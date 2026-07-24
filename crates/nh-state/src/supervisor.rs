//! Task supervision (§5.5).
//!
//! Every long-lived task the daemon spawns — reconcilers, watchers, child-process
//! monitors — is registered here with a name and a health state. This is what
//! backs `networkhub debug tasks`: when something is wrong in the field, an
//! operator can see which task is unhealthy without a debugger. The supervisor
//! also records restart policy, so a task that dies can be brought back per
//! policy rather than silently vanishing.
//!
//! For WP-M0.4 the supervisor is the registry and health model; wiring specific
//! tasks (and automatic restart) happens as those tasks land.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// The health of a supervised task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum TaskHealth {
    /// Starting up.
    Starting,
    /// Running normally.
    Healthy,
    /// Running but impaired; carries a reason.
    Degraded {
        /// Why the task is impaired.
        reason: String,
    },
    /// Stopped and will not restart (either deliberately or policy-exhausted).
    Stopped {
        /// Why it stopped.
        reason: String,
    },
}

/// What to do when a supervised task exits unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Never restart; exiting is expected.
    Never,
    /// Restart on failure, up to a bounded number of attempts.
    OnFailure {
        /// Maximum restart attempts before giving up and marking `Stopped`.
        max_restarts: u32,
    },
    /// Always restart (bounded), for tasks that must run for the daemon's life.
    Always {
        /// Maximum restart attempts before giving up.
        max_restarts: u32,
    },
}

/// A snapshot of one supervised task, for `debug tasks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInfo {
    /// The task's unique name.
    pub name: String,
    /// Its current health.
    pub health: TaskHealth,
    /// Its restart policy.
    pub policy: RestartPolicy,
    /// How many times it has been restarted.
    pub restarts: u32,
}

/// The registry of supervised tasks.
#[derive(Debug, Default)]
pub struct Supervisor {
    tasks: Mutex<BTreeMap<String, TaskInfo>>,
}

impl Supervisor {
    /// A new, empty supervisor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a task as `Starting`. Replaces any existing entry with the same
    /// name (a fresh registration resets restart counts).
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    pub fn register(&self, name: impl Into<String>, policy: RestartPolicy) {
        let name = name.into();
        self.tasks.lock().expect("supervisor lock").insert(
            name.clone(),
            TaskInfo {
                name,
                health: TaskHealth::Starting,
                policy,
                restarts: 0,
            },
        );
    }

    /// Updates a task's health. No-op if the task is not registered.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    pub fn set_health(&self, name: &str, health: TaskHealth) {
        if let Some(task) = self.tasks.lock().expect("supervisor lock").get_mut(name) {
            task.health = health;
        }
    }

    /// Records that a task was restarted, incrementing its counter and marking it
    /// `Starting`. Returns the new restart count, or `None` if unregistered.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    pub fn record_restart(&self, name: &str) -> Option<u32> {
        let mut tasks = self.tasks.lock().expect("supervisor lock");
        let task = tasks.get_mut(name)?;
        task.restarts += 1;
        task.health = TaskHealth::Starting;
        Some(task.restarts)
    }

    /// Whether a task may restart again under its policy, given its current
    /// restart count.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    #[must_use]
    pub fn may_restart(&self, name: &str) -> bool {
        let tasks = self.tasks.lock().expect("supervisor lock");
        let Some(task) = tasks.get(name) else {
            return false;
        };
        match task.policy {
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure { max_restarts } | RestartPolicy::Always { max_restarts } => {
                task.restarts < max_restarts
            }
        }
    }

    /// A snapshot of all tasks, sorted by name — the `debug tasks` output.
    ///
    /// # Panics
    /// Panics only if the internal lock is poisoned.
    #[must_use]
    pub fn snapshot(&self) -> Vec<TaskInfo> {
        self.tasks
            .lock()
            .expect("supervisor lock")
            .values()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn register_and_snapshot() {
        let s = Supervisor::new();
        s.register(
            "reconciler.hotspot",
            RestartPolicy::Always { max_restarts: 3 },
        );
        let snap = s.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].name, "reconciler.hotspot");
        assert_eq!(snap[0].health, TaskHealth::Starting);
    }

    #[test]
    fn health_updates() {
        let s = Supervisor::new();
        s.register("watcher", RestartPolicy::Never);
        s.set_health("watcher", TaskHealth::Healthy);
        assert_eq!(s.snapshot()[0].health, TaskHealth::Healthy);
    }

    #[test]
    fn restart_policy_bounds_restarts() {
        let s = Supervisor::new();
        s.register("t", RestartPolicy::OnFailure { max_restarts: 2 });
        assert!(s.may_restart("t"));
        assert_eq!(s.record_restart("t"), Some(1));
        assert!(s.may_restart("t"));
        assert_eq!(s.record_restart("t"), Some(2));
        assert!(!s.may_restart("t"), "exhausted the 2-restart budget");
    }

    #[test]
    fn never_policy_forbids_restart() {
        let s = Supervisor::new();
        s.register("t", RestartPolicy::Never);
        assert!(!s.may_restart("t"));
    }

    #[test]
    fn unregistered_task_may_not_restart() {
        let s = Supervisor::new();
        assert!(!s.may_restart("ghost"));
        assert_eq!(s.record_restart("ghost"), None);
    }
}
