//! Reversible platform changes — the fail-open mechanism at the PAL boundary.
//!
//! Every mutating PAL operation returns a [`Guard`]: a handle that can undo the
//! change it made. This is the in-process half of the fail-open invariant
//! (AD-5, §5.2). The *durable* half is the intent journal in `nh-state`, which
//! records a serializable revert **before** a change is applied, so a process
//! that dies mid-operation is still recovered on next boot. The two work
//! together: the journal survives a crash; the Guard makes the happy-path
//! rollback of a multi-step operation clean.
//!
//! Reverting is **explicit**, never on `Drop`. Undoing an OS change is async and
//! can fail, and a silent best-effort revert in a destructor would swallow those
//! failures exactly when they matter. So a dropped-but-uncommitted Guard logs a
//! warning rather than pretending it reverted; the journal is what guarantees
//! eventual cleanup.

use async_trait::async_trait;

use crate::error::PalResult;

/// Describes how to undo one applied change. Implemented per operation by each
/// backend (and by the mock). Must be `Send + Sync` because the daemon holds
/// guards across await points and threads.
#[async_trait]
pub trait Revert: Send + Sync {
    /// Undoes the change. Idempotent where the OS allows, so a revert that runs
    /// after a partial external cleanup does not error.
    async fn revert(&self) -> PalResult<()>;
}

/// A handle to an applied platform change.
///
/// Hold it for as long as the change should persist. Call [`Guard::commit`] when
/// the change is part of the durable desired state and must not be rolled back,
/// or [`Guard::revert`] to undo it.
#[must_use = "a Guard that is neither committed nor reverted logs a warning on drop"]
pub struct Guard {
    label: String,
    revert: Option<Box<dyn Revert>>,
}

impl Guard {
    /// Creates a guard for an applied change, with a human label for logs and a
    /// revert action.
    pub fn new(label: impl Into<String>, revert: Box<dyn Revert>) -> Self {
        Self {
            label: label.into(),
            revert: Some(revert),
        }
    }

    /// A guard for a change that needs no undo (a no-op mutation, or one the OS
    /// reverses itself). Committing or dropping it does nothing.
    pub fn noop(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            revert: None,
        }
    }

    /// The human label, for logs and diagnostics.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Marks the change permanent: it is part of the desired state and will not
    /// be rolled back. Consumes the guard without reverting.
    pub fn commit(mut self) {
        self.revert = None;
    }

    /// Undoes the change now. Consumes the guard.
    ///
    /// # Errors
    /// Returns the platform error if the revert itself fails.
    pub async fn revert(mut self) -> PalResult<()> {
        if let Some(revert) = self.revert.take() {
            revert.revert().await
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Show the label and whether a revert is still pending; the boxed revert
        // action itself is opaque.
        f.debug_struct("Guard")
            .field("label", &self.label)
            .field("pending", &self.revert.is_some())
            .finish()
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // A guard dropped while still holding a revert was neither committed nor
        // reverted — a programming error. We cannot revert here (Drop is sync;
        // revert is async), so we make the omission loud. The journal, not this
        // destructor, is the durable safety net.
        if self.revert.is_some() {
            // tracing is not a dependency of this leaf crate; use eprintln so the
            // warning is never silently lost. The daemon's logging layer will
            // capture stderr.
            eprintln!(
                "warning: PAL Guard '{}' dropped without commit or revert; \
                 rely on the intent journal for cleanup",
                self.label
            );
        }
    }
}

/// A group of changes applied together that must all succeed or all be undone —
/// the transaction pattern behind atomic multi-step operations (§5.2).
///
/// A caller applies each step, pushing its guard. On success it [`commit`]s; on
/// failure it [`rollback`]s, which reverts the acquired changes in reverse order.
/// This is what makes "start the hotspot" leave the machine exactly as it was if
/// any step fails partway (E-6).
///
/// [`commit`]: Transaction::commit
/// [`rollback`]: Transaction::rollback
#[must_use = "a Transaction should be committed or rolled back"]
#[derive(Debug, Default)]
pub struct Transaction {
    guards: Vec<Guard>,
}

impl Transaction {
    /// A new, empty transaction.
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }

    /// Records a successfully-applied step.
    pub fn push(&mut self, guard: Guard) {
        self.guards.push(guard);
    }

    /// The number of applied steps so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.guards.len()
    }

    /// Whether no steps have been applied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// Commits every step: the whole operation is now durable desired state.
    pub fn commit(self) {
        for guard in self.guards {
            guard.commit();
        }
    }

    /// Reverts every applied step, in reverse order (the opposite of how they
    /// were applied), so dependencies unwind correctly.
    ///
    /// Reverts are attempted for *all* steps even if some fail; the collected
    /// errors are returned so the caller can surface a conflict without leaving
    /// further steps un-reverted. An empty vec means a fully clean rollback.
    pub async fn rollback(self) -> Vec<crate::error::PalError> {
        let mut errors = Vec::new();
        for guard in self.guards.into_iter().rev() {
            if let Err(e) = guard.revert().await {
                errors.push(e);
            }
        }
        errors
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, Ordering};

    use super::*;

    /// A test revert that increments a shared counter, so we can observe how
    /// many reverts ran and in what order.
    struct CountingRevert {
        counter: Arc<AtomicI32>,
        // Records the order value this revert writes, to check reverse order.
        marker: i32,
        order_log: Arc<std::sync::Mutex<Vec<i32>>>,
    }

    #[async_trait]
    impl Revert for CountingRevert {
        async fn revert(&self) -> PalResult<()> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            self.order_log.lock().unwrap().push(self.marker);
            Ok(())
        }
    }

    fn guard(
        counter: &Arc<AtomicI32>,
        marker: i32,
        log: &Arc<std::sync::Mutex<Vec<i32>>>,
    ) -> Guard {
        Guard::new(
            format!("step-{marker}"),
            Box::new(CountingRevert {
                counter: counter.clone(),
                marker,
                order_log: log.clone(),
            }),
        )
    }

    #[tokio::test]
    async fn commit_does_not_revert() {
        let counter = Arc::new(AtomicI32::new(0));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        guard(&counter, 1, &log).commit();
        assert_eq!(counter.load(Ordering::SeqCst), 0, "commit must not revert");
    }

    #[tokio::test]
    async fn revert_runs_the_undo() {
        let counter = Arc::new(AtomicI32::new(0));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        guard(&counter, 1, &log).revert().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn transaction_rollback_reverts_in_reverse_order() {
        let counter = Arc::new(AtomicI32::new(0));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let mut txn = Transaction::new();
        txn.push(guard(&counter, 1, &log));
        txn.push(guard(&counter, 2, &log));
        txn.push(guard(&counter, 3, &log));
        assert_eq!(txn.len(), 3);

        let errors = txn.rollback().await;
        assert!(errors.is_empty());
        assert_eq!(counter.load(Ordering::SeqCst), 3, "all three reverted");
        // Applied 1,2,3 → reverted 3,2,1.
        assert_eq!(*log.lock().unwrap(), vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn transaction_commit_keeps_everything() {
        let counter = Arc::new(AtomicI32::new(0));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut txn = Transaction::new();
        txn.push(guard(&counter, 1, &log));
        txn.push(guard(&counter, 2, &log));
        txn.commit();
        assert_eq!(counter.load(Ordering::SeqCst), 0, "commit reverts nothing");
    }

    #[tokio::test]
    async fn noop_guard_is_inert() {
        Guard::noop("nothing").commit();
        Guard::noop("nothing").revert().await.unwrap();
    }
}
