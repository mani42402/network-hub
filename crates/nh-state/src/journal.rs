//! The intent journal — write-ahead logging of OS mutations (§5.3, AD-5).
//!
//! Before the daemon changes any OS state (a firewall rule, a `sysctl`, a DNS
//! setting), it records an **intent**: what it is about to do, and the prior
//! value, so the change can be undone even if the process dies the instant after
//! it is applied. On the next start the deadman check replays the journal: any
//! intent that was recorded but never confirmed complete is rolled back to its
//! prior value. This is what turns "we changed `ip_forward` and crashed" from a
//! silent security hole into a recoverable event (F-3, F-9).
//!
//! This module defines the journal's shape and logic plus an in-memory store for
//! tests. The durable SQLite-backed store lands in WP-M0.5; the reconciler
//! framework here is written against the [`JournalStore`] trait so it does not
//! care which backend is used.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A single write-ahead intent: one OS mutation the daemon is about to perform,
/// recorded with enough information to undo it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    /// Unique id for this intent (UUIDv7, time-ordered).
    pub id: String,
    /// The module recording it, e.g. `firewall`, `routing`, for diagnostics.
    pub module: String,
    /// A stable operation name, e.g. `sysctl.ip_forward`, so a replayer knows
    /// how to undo it.
    pub operation: String,
    /// The value before the change, serialized. Restoring this exact value is
    /// the point (F-9: not merely "off", but whatever it was).
    pub prior_value: String,
    /// The value being set, for audit and idempotency checks.
    pub new_value: String,
    /// Where in its lifecycle this intent is.
    pub state: IntentState,
}

/// The lifecycle of an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    /// Recorded, but the OS mutation has not been confirmed applied. If the
    /// process dies now, the deadman rolls this back to `prior_value`.
    Pending,
    /// The OS mutation completed and is the durable desired state. No rollback
    /// on replay.
    Committed,
    /// The change was undone (deliberately reverted). Kept briefly for audit,
    /// then pruned.
    Reverted,
}

/// Storage for intents. The reconciler framework writes through this before and
/// after each mutation. Implemented in-memory here (tests) and by SQLite in
/// WP-M0.5.
#[async_trait]
pub trait JournalStore: Send + Sync {
    /// Appends a new `Pending` intent, synchronously durable before it returns
    /// (the write-ahead guarantee — the record must survive a crash that happens
    /// immediately after).
    async fn append(&self, intent: Intent) -> JournalResult<()>;

    /// Marks an intent `Committed`: the mutation succeeded and is now desired
    /// state.
    async fn commit(&self, id: &str) -> JournalResult<()>;

    /// Marks an intent `Reverted`.
    async fn mark_reverted(&self, id: &str) -> JournalResult<()>;

    /// Returns all intents still `Pending` — the ones a deadman replay must roll
    /// back, oldest first.
    async fn pending(&self) -> JournalResult<Vec<Intent>>;

    /// Removes committed/reverted intents older than needed, to bound growth.
    async fn prune_finished(&self) -> JournalResult<usize>;
}

/// Result type for journal operations.
pub type JournalResult<T> = Result<T, JournalError>;

/// A journal storage failure.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    /// The backing store failed (disk, database).
    #[error("journal storage error: {0}")]
    Storage(String),
    /// An operation referenced an intent id that does not exist.
    #[error("no such intent: {0}")]
    NotFound(String),
}

/// An in-memory [`JournalStore`] for tests. Not durable — a real crash loses it,
/// which is exactly why production uses the SQLite store (WP-M0.5).
#[derive(Debug, Default)]
pub struct InMemoryJournal {
    intents: tokio::sync::Mutex<Vec<Intent>>,
}

impl InMemoryJournal {
    /// A new, empty in-memory journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The full journal contents, for test assertions.
    pub async fn all(&self) -> Vec<Intent> {
        self.intents.lock().await.clone()
    }
}

#[async_trait]
impl JournalStore for InMemoryJournal {
    async fn append(&self, intent: Intent) -> JournalResult<()> {
        self.intents.lock().await.push(intent);
        Ok(())
    }

    async fn commit(&self, id: &str) -> JournalResult<()> {
        set_state(&self.intents, id, IntentState::Committed).await
    }

    async fn mark_reverted(&self, id: &str) -> JournalResult<()> {
        set_state(&self.intents, id, IntentState::Reverted).await
    }

    async fn pending(&self) -> JournalResult<Vec<Intent>> {
        Ok(self
            .intents
            .lock()
            .await
            .iter()
            .filter(|i| i.state == IntentState::Pending)
            .cloned()
            .collect())
    }

    async fn prune_finished(&self) -> JournalResult<usize> {
        let mut intents = self.intents.lock().await;
        let before = intents.len();
        intents.retain(|i| i.state == IntentState::Pending);
        Ok(before - intents.len())
    }
}

async fn set_state(
    intents: &tokio::sync::Mutex<Vec<Intent>>,
    id: &str,
    state: IntentState,
) -> JournalResult<()> {
    let mut intents = intents.lock().await;
    let intent = intents
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or_else(|| JournalError::NotFound(id.to_owned()))?;
    intent.state = state;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn intent(id: &str, prior: &str, new: &str) -> Intent {
        Intent {
            id: id.to_owned(),
            module: "routing".to_owned(),
            operation: "sysctl.ip_forward".to_owned(),
            prior_value: prior.to_owned(),
            new_value: new.to_owned(),
            state: IntentState::Pending,
        }
    }

    #[tokio::test]
    async fn pending_lists_only_uncommitted() {
        let j = InMemoryJournal::new();
        j.append(intent("1", "0", "1")).await.unwrap();
        j.append(intent("2", "0", "1")).await.unwrap();
        j.commit("1").await.unwrap();

        let pending = j.pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "2");
    }

    #[tokio::test]
    async fn commit_and_revert_change_state() {
        let j = InMemoryJournal::new();
        j.append(intent("1", "0", "1")).await.unwrap();
        j.commit("1").await.unwrap();
        assert!(j.pending().await.unwrap().is_empty());

        j.append(intent("2", "0", "1")).await.unwrap();
        j.mark_reverted("2").await.unwrap();
        assert!(j.pending().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn commit_of_unknown_id_errors() {
        let j = InMemoryJournal::new();
        assert!(matches!(
            j.commit("nope").await,
            Err(JournalError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn prune_keeps_only_pending() {
        let j = InMemoryJournal::new();
        j.append(intent("1", "0", "1")).await.unwrap();
        j.append(intent("2", "0", "1")).await.unwrap();
        j.commit("1").await.unwrap();
        assert_eq!(j.prune_finished().await.unwrap(), 1);
        assert_eq!(j.all().await.len(), 1);
    }
}
