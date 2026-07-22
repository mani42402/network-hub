//! The reconciler framework (§5.3, AD-4).
//!
//! A reconciler continuously converges *observed* OS state toward *desired*
//! state. The daemon runs one per module (hotspot, vpn, firewall, …), each an
//! independent task so a broken one cannot take down the others. This module is
//! the generic runtime that drives any [`Reconciler`]: it retries with backoff,
//! trips a circuit breaker into `Degraded` after repeated failures, and detects
//! external interference to enter `Conflicted` rather than fighting forever.
//!
//! A reconcile is triggered by a [`TriggerReason`]: a *desired* change (the user
//! edited config) is legitimate and resets interference tracking; an *observed*
//! change after convergence (something external reverted our state) is what the
//! interference detector counts.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::backoff::Backoff;
use crate::interference::InterferenceTracker;

/// The result of one reconcile attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// Observed state now matches desired; nothing more until state changes.
    Converged,
    /// Progress was made but more work remains; the runtime reconciles again
    /// immediately.
    Progress,
    /// The attempt failed; the runtime retries with backoff. The string is a
    /// log/diagnostic message (never a secret).
    Failed(String),
}

/// A unit of convergence logic. One per module.
#[async_trait]
pub trait Reconciler: Send + Sync {
    /// The module name, for status, logs, and `debug tasks`.
    fn name(&self) -> &str;

    /// Attempts to converge observed state toward desired state once. Called by
    /// the runtime; must be idempotent (a reconcile that runs twice must not
    /// double-apply — §5.2).
    async fn reconcile(&self) -> ReconcileOutcome;
}

/// Why a reconcile was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerReason {
    /// The desired state changed (user/API action). Legitimate; resets
    /// interference tracking.
    DesiredChanged,
    /// The observed OS state changed underneath us. After convergence this is a
    /// candidate for interference (something is reverting our work).
    ObservedChanged,
}

/// The current status of a reconciler, published for the API and `debug tasks`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcilerStatus {
    /// Not yet run.
    Idle,
    /// Actively working toward convergence.
    Converging,
    /// Observed matches desired.
    Converged,
    /// Repeated failures tripped the circuit breaker. Carries the last error and
    /// the failure count.
    Degraded {
        /// The most recent failure message.
        error: String,
        /// How many consecutive failures have occurred.
        consecutive_failures: u32,
    },
    /// External interference detected; the reconciler has stopped fighting.
    /// Carries a human description naming the likely suspect.
    Conflicted {
        /// What appears to be reverting our state.
        suspect: String,
    },
}

/// Tuning for a reconciler runtime.
#[derive(Debug, Clone)]
pub struct ReconcilerConfig {
    /// Consecutive failures before entering `Degraded`.
    pub circuit_breaker_threshold: u32,
    /// Restore cycles within the window before entering `Conflicted`.
    pub interference_threshold: u32,
    /// The interference sliding-window length.
    pub interference_window: Duration,
    /// Retry backoff.
    pub backoff: Backoff,
}

impl Default for ReconcilerConfig {
    fn default() -> Self {
        Self {
            circuit_breaker_threshold: 5,
            interference_threshold: 3,
            interference_window: Duration::from_mins(1),
            backoff: Backoff::reconciler_default(),
        }
    }
}

/// A running reconciler. Dropping the handle shuts the reconciler down.
#[derive(Debug)]
pub struct ReconcilerHandle {
    name: String,
    status_rx: watch::Receiver<ReconcilerStatus>,
    cmd_tx: mpsc::Sender<Command>,
}

#[derive(Debug)]
enum Command {
    Trigger(TriggerReason),
    Shutdown,
}

/// What woke the runtime out of a post-failure backoff wait.
enum RetryWake {
    /// The backoff elapsed; retry the reconcile.
    Retry,
    /// A trigger arrived during the wait.
    Trigger(TriggerReason),
    /// Shutdown was requested (or the handle was dropped).
    Shutdown,
}

impl ReconcilerHandle {
    /// The module name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The current status.
    #[must_use]
    pub fn status(&self) -> ReconcilerStatus {
        self.status_rx.borrow().clone()
    }

    /// Asks the reconciler to reconcile, stating why. Non-blocking; the request
    /// is queued.
    pub async fn trigger(&self, reason: TriggerReason) {
        let _ = self.cmd_tx.send(Command::Trigger(reason)).await;
    }

    /// Waits until the status changes from its current value, returning the new
    /// status. Used by tests and by the daemon to react to transitions.
    ///
    /// # Errors
    /// Returns `None` if the reconciler task has stopped.
    pub async fn await_status_change(&mut self) -> Option<ReconcilerStatus> {
        self.status_rx.changed().await.ok()?;
        Some(self.status_rx.borrow().clone())
    }

    /// Signals the reconciler to stop.
    pub async fn shutdown(&self) {
        let _ = self.cmd_tx.send(Command::Shutdown).await;
    }
}

/// Spawns a reconciler task and returns a handle to it. The task reconciles once
/// on start, then whenever triggered.
#[must_use]
pub fn spawn(reconciler: Arc<dyn Reconciler>, config: ReconcilerConfig) -> ReconcilerHandle {
    let name = reconciler.name().to_owned();
    let (status_tx, status_rx) = watch::channel(ReconcilerStatus::Idle);
    let (cmd_tx, cmd_rx) = mpsc::channel(64);

    let runtime = Runtime {
        reconciler,
        config: config.clone(),
        status_tx,
        cmd_rx,
        backoff: config.backoff,
        consecutive_failures: 0,
        interference: InterferenceTracker::new(
            config.interference_threshold,
            config.interference_window,
        ),
    };
    tokio::spawn(runtime.run());

    ReconcilerHandle {
        name,
        status_rx,
        cmd_tx,
    }
}

struct Runtime {
    reconciler: Arc<dyn Reconciler>,
    config: ReconcilerConfig,
    status_tx: watch::Sender<ReconcilerStatus>,
    cmd_rx: mpsc::Receiver<Command>,
    backoff: Backoff,
    consecutive_failures: u32,
    interference: InterferenceTracker,
}

/// The result of one reconcile, from the runtime's point of view.
enum Step {
    /// Converged; wait for the next trigger.
    Done,
    /// Made progress; reconcile again immediately.
    Again,
    /// Failed; schedule a backoff retry (whether or not the breaker tripped).
    Retry,
}

impl Runtime {
    async fn run(mut self) {
        // Reconcile once on start to converge from whatever the initial state is.
        let mut want_reconcile = true;

        loop {
            if want_reconcile {
                match self.do_reconcile().await {
                    Step::Again => continue,
                    Step::Retry => {
                        // A failure ALWAYS schedules a retry via the backoff
                        // timer, independent of the circuit breaker (which only
                        // changes the reported status). Race the timer against
                        // an incoming command so a fresh trigger preempts the
                        // wait.
                        match self.wait_retry_or_command().await {
                            RetryWake::Shutdown => break,
                            RetryWake::Retry => want_reconcile = true,
                            RetryWake::Trigger(reason) => {
                                want_reconcile = self.handle_trigger(reason);
                            }
                        }
                        continue;
                    }
                    // Converged: fall through to wait for the next command,
                    // which reassigns want_reconcile.
                    Step::Done => {}
                }
            }

            // Converged/idle: wait for the next command only.
            match self.cmd_rx.recv().await {
                None | Some(Command::Shutdown) => break,
                Some(Command::Trigger(reason)) => {
                    want_reconcile = self.handle_trigger(reason);
                }
            }
        }
    }

    /// Runs one reconcile and updates status.
    async fn do_reconcile(&mut self) -> Step {
        self.set_status(ReconcilerStatus::Converging);
        match self.reconciler.reconcile().await {
            ReconcileOutcome::Converged => {
                self.consecutive_failures = 0;
                self.backoff.reset();
                self.set_status(ReconcilerStatus::Converged);
                Step::Done
            }
            ReconcileOutcome::Progress => Step::Again,
            ReconcileOutcome::Failed(error) => {
                self.consecutive_failures += 1;
                if self.consecutive_failures >= self.config.circuit_breaker_threshold {
                    self.set_status(ReconcilerStatus::Degraded {
                        error,
                        consecutive_failures: self.consecutive_failures,
                    });
                }
                Step::Retry
            }
        }
    }

    /// Decides what a trigger means given the current status. Returns whether to
    /// reconcile now.
    fn handle_trigger(&mut self, reason: TriggerReason) -> bool {
        match reason {
            TriggerReason::DesiredChanged => {
                // A legitimate change: forget any interference history and work.
                self.interference.clear();
                true
            }
            TriggerReason::ObservedChanged => {
                let converged = matches!(*self.status_tx.borrow(), ReconcilerStatus::Converged);
                let conflicted = matches!(
                    *self.status_tx.borrow(),
                    ReconcilerStatus::Conflicted { .. }
                );

                // If already conflicted, keep ignoring observed churn until the
                // desired state changes — we have stopped fighting (§5.3).
                if conflicted {
                    return false;
                }

                // An observed change after we had converged is a restore cycle:
                // something reverted our work.
                if converged && self.interference.record(Instant::now()) {
                    self.set_status(ReconcilerStatus::Conflicted {
                        suspect: format!(
                            "another agent keeps reverting {}'s state; \
                             stopped reconciling to avoid a loop",
                            self.reconciler.name()
                        ),
                    });
                    return false;
                }
                true
            }
        }
    }

    /// After a failed reconcile, waits out the backoff delay, but lets an
    /// incoming command preempt it (a fresh trigger should not wait behind a
    /// long backoff).
    async fn wait_retry_or_command(&mut self) -> RetryWake {
        let delay = self.backoff.next_delay();
        tokio::select! {
            cmd = self.cmd_rx.recv() => match cmd {
                None | Some(Command::Shutdown) => RetryWake::Shutdown,
                Some(Command::Trigger(reason)) => RetryWake::Trigger(reason),
            },
            () = tokio::time::sleep(delay) => RetryWake::Retry,
        }
    }

    fn set_status(&self, status: ReconcilerStatus) {
        // send_if_modified avoids waking watchers when nothing changed. The
        // comparison borrows `status`; the else branch then moves it in, so
        // there is no clone.
        self.status_tx.send_if_modified(|current| {
            if *current == status {
                false
            } else {
                *current = status;
                true
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// A reconciler whose outcome is scripted by an atomic, so tests can make it
    /// converge, fail, or report progress on demand.
    struct ScriptedReconciler {
        name: String,
        // 0 = converge, 1 = fail, 2 = progress-then-converge
        mode: Arc<AtomicU32>,
        calls: Arc<AtomicU32>,
    }

    #[async_trait]
    impl Reconciler for ScriptedReconciler {
        fn name(&self) -> &str {
            &self.name
        }
        async fn reconcile(&self) -> ReconcileOutcome {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match self.mode.load(Ordering::SeqCst) {
                1 => ReconcileOutcome::Failed("scripted failure".into()),
                2 => {
                    // First call Progress, then Converged.
                    self.mode.store(0, Ordering::SeqCst);
                    ReconcileOutcome::Progress
                }
                _ => ReconcileOutcome::Converged,
            }
        }
    }

    fn scripted(mode: u32) -> (Arc<ScriptedReconciler>, Arc<AtomicU32>, Arc<AtomicU32>) {
        let mode = Arc::new(AtomicU32::new(mode));
        let calls = Arc::new(AtomicU32::new(0));
        let r = Arc::new(ScriptedReconciler {
            name: "test".into(),
            mode: mode.clone(),
            calls: calls.clone(),
        });
        (r, mode, calls)
    }

    async fn wait_for_status(h: &mut ReconcilerHandle, want: &ReconcilerStatus) {
        if h.status() == *want {
            return;
        }
        for _ in 0..50 {
            if let Some(s) = h.await_status_change().await
                && s == *want
            {
                return;
            }
        }
        panic!("status never reached {want:?}; last was {:?}", h.status());
    }

    #[tokio::test]
    async fn converges_on_start() {
        let (r, _mode, calls) = scripted(0);
        let mut h = spawn(r, ReconcilerConfig::default());
        wait_for_status(&mut h, &ReconcilerStatus::Converged).await;
        assert!(calls.load(Ordering::SeqCst) >= 1);
        h.shutdown().await;
    }

    #[tokio::test]
    async fn progress_then_converges() {
        let (r, _mode, calls) = scripted(2);
        let mut h = spawn(r, ReconcilerConfig::default());
        wait_for_status(&mut h, &ReconcilerStatus::Converged).await;
        // Progress + Converged = at least two calls.
        assert!(calls.load(Ordering::SeqCst) >= 2);
        h.shutdown().await;
    }

    #[tokio::test(start_paused = true)]
    async fn repeated_failure_trips_circuit_breaker() {
        let (r, _mode, _calls) = scripted(1); // always fails
        let config = ReconcilerConfig {
            circuit_breaker_threshold: 3,
            backoff: Backoff::new(Duration::from_millis(10), Duration::from_millis(10), 0.0),
            ..Default::default()
        };
        let mut h = spawn(r, config);
        // The runtime reconciles on start (fail 1), then retries on the backoff
        // timer (fail 2, 3) → Degraded.
        wait_for_status(&mut h, &{
            ReconcilerStatus::Degraded {
                error: "scripted failure".into(),
                consecutive_failures: 3,
            }
        })
        .await;
        h.shutdown().await;
    }

    #[tokio::test(start_paused = true)]
    async fn observed_churn_after_convergence_becomes_conflicted() {
        let (r, _mode, _calls) = scripted(0); // always converges
        let config = ReconcilerConfig {
            interference_threshold: 3,
            ..Default::default()
        };
        let mut h = spawn(r, config);
        wait_for_status(&mut h, &ReconcilerStatus::Converged).await;

        // Simulate an external agent reverting our state repeatedly: each
        // observed change re-converges, and after 3 the runtime gives up.
        for _ in 0..3 {
            h.trigger(TriggerReason::ObservedChanged).await;
            // let it process
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        wait_for_status(
            &mut h,
            &ReconcilerStatus::Conflicted {
                suspect: "another agent keeps reverting test's state; \
                          stopped reconciling to avoid a loop"
                    .into(),
            },
        )
        .await;
        h.shutdown().await;
    }

    #[tokio::test]
    async fn desired_change_clears_interference() {
        let (r, _mode, _calls) = scripted(0);
        let config = ReconcilerConfig {
            interference_threshold: 2,
            ..Default::default()
        };
        let mut h = spawn(r, config);
        wait_for_status(&mut h, &ReconcilerStatus::Converged).await;

        // One observed churn (below threshold of 2).
        h.trigger(TriggerReason::ObservedChanged).await;
        tokio::time::sleep(Duration::from_millis(1)).await;
        // A legitimate desired change resets the interference count.
        h.trigger(TriggerReason::DesiredChanged).await;
        wait_for_status(&mut h, &ReconcilerStatus::Converged).await;

        // Another single observed churn should NOT trip conflict (count was reset).
        h.trigger(TriggerReason::ObservedChanged).await;
        tokio::time::sleep(Duration::from_millis(1)).await;
        assert_eq!(h.status(), ReconcilerStatus::Converged);
        h.shutdown().await;
    }
}
