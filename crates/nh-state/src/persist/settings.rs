//! Durable settings and desired-state documents.
//!
//! The persistent counterpart to the in-memory [`crate::store::StateStore`]: a
//! typed key/value store with per-key revisions. The daemon hydrates the
//! in-memory store from here on boot and writes through on change, so desired
//! state survives a restart (§5.3) — the property the reconciliation model
//! depends on.

use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::{Row, SqlitePool};

use super::DbError;

/// A typed, revisioned settings store over `config.db`.
#[derive(Debug, Clone)]
pub struct Settings {
    pool: SqlitePool,
}

impl Settings {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Writes a value under `key`, bumping its revision. Returns the new revision.
    ///
    /// # Errors
    /// Returns [`DbError`] if serialization or the write fails.
    pub async fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<i64, DbError> {
        let json = serde_json::to_string(value)
            .map_err(|e| DbError::Sqlite(sqlx::Error::Encode(Box::new(e))))?;
        // Upsert: insert at revision 1, or bump the existing revision.
        let revision: i64 = sqlx::query(
            "INSERT INTO settings (key, value, revision, updated_at)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value = ?2,
                revision = revision + 1,
                updated_at = ?3
             RETURNING revision",
        )
        .bind(key)
        .bind(&json)
        .bind(now_secs())
        .fetch_one(&self.pool)
        .await?
        .try_get("revision")?;
        Ok(revision)
    }

    /// Reads and deserializes a value, with its revision, or `None` if absent.
    ///
    /// # Errors
    /// Returns [`DbError`] if the read fails or the stored bytes do not match `T`.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<(T, i64)>, DbError> {
        let row = sqlx::query("SELECT value, revision FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(None),
            Some(row) => {
                let json: String = row.get("value");
                let revision: i64 = row.get("revision");
                let value = serde_json::from_str(&json)
                    .map_err(|e| DbError::Sqlite(sqlx::Error::Decode(Box::new(e))))?;
                Ok(Some((value, revision)))
            }
        }
    }

    /// Deletes a key. Returns whether it existed.
    ///
    /// # Errors
    /// Returns [`DbError`] if the delete fails.
    pub async fn delete(&self, key: &str) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// All keys, for hydrating the in-memory store on boot.
    ///
    /// # Errors
    /// Returns [`DbError`] if the query fails.
    pub async fn keys(&self) -> Result<Vec<String>, DbError> {
        let rows = sqlx::query("SELECT key FROM settings ORDER BY key")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(|r| r.get::<String, _>("key")).collect())
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .try_into()
        .unwrap_or(0)
}
