//! The durable [`JournalStore`] — SQLite-backed intent journal.
//!
//! This is the production backing for the write-ahead intent journal defined in
//! [`crate::journal`]. `append` is synchronously durable (SQLite in FULL
//! synchronous mode, ADR-0010), which is what makes the write-ahead guarantee
//! real: a record survives a crash that happens the instant after it returns.

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use crate::journal::{Intent, IntentState, JournalError, JournalResult, JournalStore};

/// A [`JournalStore`] backed by `config.db`.
#[derive(Debug, Clone)]
pub struct SqliteJournal {
    pool: SqlitePool,
}

impl SqliteJournal {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn state_str(s: IntentState) -> &'static str {
    match s {
        IntentState::Pending => "pending",
        IntentState::Committed => "committed",
        IntentState::Reverted => "reverted",
    }
}

fn parse_state(s: &str) -> IntentState {
    match s {
        "committed" => IntentState::Committed,
        "reverted" => IntentState::Reverted,
        // Anything unexpected is treated as pending, so a mangled row is rolled
        // back rather than silently trusted — fail safe.
        _ => IntentState::Pending,
    }
}

// Takes the error by value because it is used as a `map_err` callback, which
// hands over an owned error.
#[allow(clippy::needless_pass_by_value)]
fn storage(e: sqlx::Error) -> JournalError {
    JournalError::Storage(e.to_string())
}

#[async_trait]
impl JournalStore for SqliteJournal {
    async fn append(&self, intent: Intent) -> JournalResult<()> {
        sqlx::query(
            "INSERT INTO intent_journal
             (id, module, operation, prior_value, new_value, state, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&intent.id)
        .bind(&intent.module)
        .bind(&intent.operation)
        .bind(&intent.prior_value)
        .bind(&intent.new_value)
        .bind(state_str(intent.state))
        .bind(now_secs())
        .execute(&self.pool)
        .await
        .map_err(storage)?;
        Ok(())
    }

    async fn commit(&self, id: &str) -> JournalResult<()> {
        set_state(&self.pool, id, IntentState::Committed).await
    }

    async fn mark_reverted(&self, id: &str) -> JournalResult<()> {
        set_state(&self.pool, id, IntentState::Reverted).await
    }

    async fn pending(&self) -> JournalResult<Vec<Intent>> {
        let rows = sqlx::query(
            "SELECT id, module, operation, prior_value, new_value, state
             FROM intent_journal WHERE state = 'pending' ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?;

        Ok(rows.iter().map(row_to_intent).collect())
    }

    async fn prune_finished(&self) -> JournalResult<usize> {
        let result = sqlx::query("DELETE FROM intent_journal WHERE state != 'pending'")
            .execute(&self.pool)
            .await
            .map_err(storage)?;
        Ok(usize::try_from(result.rows_affected()).unwrap_or(usize::MAX))
    }
}

async fn set_state(pool: &SqlitePool, id: &str, state: IntentState) -> JournalResult<()> {
    let result = sqlx::query("UPDATE intent_journal SET state = ? WHERE id = ?")
        .bind(state_str(state))
        .bind(id)
        .execute(pool)
        .await
        .map_err(storage)?;
    if result.rows_affected() == 0 {
        return Err(JournalError::NotFound(id.to_owned()));
    }
    Ok(())
}

fn row_to_intent(row: &sqlx::sqlite::SqliteRow) -> Intent {
    Intent {
        id: row.get("id"),
        module: row.get("module"),
        operation: row.get("operation"),
        prior_value: row.get("prior_value"),
        new_value: row.get("new_value"),
        state: parse_state(&row.get::<String, _>("state")),
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .try_into()
        .unwrap_or(0)
}
