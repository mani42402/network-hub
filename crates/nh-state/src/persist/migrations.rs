//! Forward-only, versioned schema migrations (ADR-0010, §7.3).
//!
//! Each migration is a `(version, SQL)` pair applied in order. The current
//! version is recorded in `schema_meta`; on open, every migration newer than the
//! recorded version is applied inside a transaction, and a backup is taken first
//! so a bad upgrade is recoverable. There is no down-migration by design —
//! downgrades restore a backup instead.

use std::path::Path;

use sqlx::{Row, SqlitePool};

use super::DbError;

/// The ordered set of migrations. Append only; never edit or renumber an
/// existing entry, or databases in the field will not match.
const MIGRATIONS: &[(u32, &str)] = &[(
    1,
    r"
    -- Key/value settings and desired-state documents. `revision` bumps on every
    -- write so reconcilers have a cheap change check (mirrors the in-memory
    -- StateStore).
    CREATE TABLE settings (
        key        TEXT    PRIMARY KEY NOT NULL,
        value      TEXT    NOT NULL,
        revision   INTEGER NOT NULL DEFAULT 1,
        updated_at INTEGER NOT NULL
    ) STRICT;

    -- Write-ahead intent journal (AD-5, F-3/F-9). `state` is pending/committed/
    -- reverted; pending rows are what a boot-time deadman replays.
    CREATE TABLE intent_journal (
        id          TEXT    PRIMARY KEY NOT NULL,
        module      TEXT    NOT NULL,
        operation   TEXT    NOT NULL,
        prior_value TEXT    NOT NULL,
        new_value   TEXT    NOT NULL,
        state       TEXT    NOT NULL,
        created_at  INTEGER NOT NULL
    ) STRICT;
    CREATE INDEX idx_intent_state ON intent_journal(state);

    -- Tamper-evident audit log (§7.2). Append-only: every row's entry_hash
    -- chains to the previous row's hash, so a silent edit breaks the chain.
    CREATE TABLE audit_log (
        seq        INTEGER PRIMARY KEY AUTOINCREMENT,
        actor      TEXT    NOT NULL,
        source     TEXT    NOT NULL,
        action     TEXT    NOT NULL,
        target     TEXT    NOT NULL,
        result     TEXT    NOT NULL,
        created_at INTEGER NOT NULL,
        prev_hash  TEXT    NOT NULL,
        entry_hash TEXT    NOT NULL
    ) STRICT;
    ",
)];

/// The highest migration version this build knows about.
#[must_use]
pub(super) fn latest_version() -> u32 {
    MIGRATIONS.iter().map(|(v, _)| *v).max().unwrap_or(0)
}

/// Applies every migration newer than the recorded schema version.
pub(super) async fn run(pool: &SqlitePool, db_path: &Path) -> Result<(), DbError> {
    // The version bookkeeping table exists before anything else.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_meta (
            version    INTEGER PRIMARY KEY NOT NULL,
            applied_at INTEGER NOT NULL
        ) STRICT",
    )
    .execute(pool)
    .await?;

    let current: u32 = sqlx::query("SELECT COALESCE(MAX(version), 0) AS v FROM schema_meta")
        .fetch_one(pool)
        .await?
        .try_get::<i64, _>("v")
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    let pending: Vec<&(u32, &str)> = MIGRATIONS.iter().filter(|(v, _)| *v > current).collect();
    if pending.is_empty() {
        return Ok(());
    }

    // Back up before upgrading, so a failed migration can be rolled back by
    // restoring the pre-migration file (§7.3). Best-effort: a backup failure on
    // a brand-new empty database should not block the first migration.
    if db_path.exists() {
        let _ = backup_before_migration(pool, db_path).await;
    }

    for (version, sql) in pending {
        let mut tx = pool.begin().await?;
        // Execute the whole migration as raw SQL so SQLite parses the statements
        // and comments itself. (Hand-splitting on ';' is fragile — a semicolon
        // inside a comment string mis-splits a statement.)
        sqlx::raw_sql(sql)
            .execute(&mut *tx)
            .await
            .map_err(|source| DbError::Migration {
                version: *version,
                source,
            })?;
        sqlx::query("INSERT INTO schema_meta (version, applied_at) VALUES (?, ?)")
            .bind(i64::from(*version))
            .bind(now_secs())
            .execute(&mut *tx)
            .await
            .map_err(|source| DbError::Migration {
                version: *version,
                source,
            })?;
        tx.commit().await.map_err(|source| DbError::Migration {
            version: *version,
            source,
        })?;
    }

    Ok(())
}

async fn backup_before_migration(pool: &SqlitePool, db_path: &Path) -> Result<(), DbError> {
    let mut backup = db_path.to_path_buf();
    let name = format!(
        "{}.pre-migration",
        db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.db")
    );
    backup.set_file_name(name);
    let sql = format!("VACUUM INTO '{}'", backup.display());
    // Overwrite any stale pre-migration backup.
    let _ = std::fs::remove_file(&backup);
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .try_into()
        .unwrap_or(0)
}
