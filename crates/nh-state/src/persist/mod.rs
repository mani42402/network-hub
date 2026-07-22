//! SQLite persistence — `config.db` (ADR-0010, §7).
//!
//! The durable home for everything the daemon must not lose: settings, the
//! intent journal, the audit log, and desired-state documents. Three properties
//! matter as much as the storage itself:
//!
//! - **Migrations are forward-only and versioned**, with a backup taken before
//!   any upgrade, so a bad migration is recoverable.
//! - **Corruption cannot crash-loop the daemon** (F-5). On open, a failed
//!   integrity check quarantines the bad file, restores the newest backup, or —
//!   failing that — starts empty. An empty `config.db` means no desired state,
//!   which is fail-open by construction (AD-5): no rules applied.
//! - **The audit log is tamper-evident** (§7.2): a hash chain makes silent
//!   edits detectable.
//!
//! Enabled by the `persist` feature so framework-only consumers never link sqlx.

mod audit;
mod journal;
mod migrations;
mod settings;

pub use audit::{AuditEntry, AuditLog, TamperCheck};
pub use journal::SqliteJournal;
pub use settings::Settings;

use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

/// The `config.db` connection and the operations over it.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
}

/// What happened when the database was opened — surfaced so the daemon can emit
/// an event and, in the recovery cases, warn the user (F-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenReport {
    /// Opened an existing, healthy database.
    Healthy,
    /// The file did not exist; a fresh empty database was created.
    CreatedFresh,
    /// The database was corrupt and was restored from a backup. The bad file was
    /// quarantined at the given path.
    RestoredFromBackup {
        /// Where the corrupt file was moved.
        quarantined: PathBuf,
        /// Which backup was restored.
        restored_from: PathBuf,
    },
    /// The database was corrupt and no usable backup existed, so a fresh empty
    /// database was created. Desired state is lost, but the daemon comes up
    /// (fail-open): empty desired state means nothing is applied.
    StartedEmptyAfterCorruption {
        /// Where the corrupt file was moved.
        quarantined: PathBuf,
    },
}

/// A persistence error.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// The underlying SQLite driver failed.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    /// A filesystem operation (backup, quarantine) failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A migration failed to apply.
    #[error("migration {version} failed: {source}")]
    Migration {
        /// The migration version that failed.
        version: u32,
        /// The underlying error.
        source: sqlx::Error,
    },
}

impl Database {
    /// Opens (or creates) `config.db` at `path`, recovering from corruption and
    /// applying migrations. Returns the database and a report of what happened.
    ///
    /// # Errors
    /// Returns [`DbError`] only for failures that recovery cannot handle (e.g. the
    /// directory is unwritable). Corruption itself is *recovered*, not returned
    /// as an error — that is the whole point (F-5).
    pub async fn open(path: impl AsRef<Path>) -> Result<(Self, OpenReport), DbError> {
        let path = path.as_ref().to_path_buf();
        let existed = path.exists();

        // First attempt: open and integrity-check the existing file.
        if existed {
            match Self::try_open_healthy(&path).await {
                Ok(db) => {
                    db.migrate().await?;
                    return Ok((db, OpenReport::Healthy));
                }
                Err(_corrupt) => {
                    // Fall through to the recovery path below.
                    return Self::recover(&path).await;
                }
            }
        }

        // No existing file: create fresh.
        let db = Self::create_and_migrate(&path).await?;
        Ok((db, OpenReport::CreatedFresh))
    }

    /// The connection pool, for the sub-stores.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Closes the database, awaiting the final WAL checkpoint. Prefer this over
    /// dropping the handle when the on-disk file must be fully settled before the
    /// next step (e.g. a backup copy, or a controlled shutdown) — a bare drop
    /// closes the pool in the background, which can race a subsequent file
    /// operation.
    pub async fn close(self) {
        self.pool.close().await;
    }

    /// The schema version currently applied to this database.
    ///
    /// # Errors
    /// Returns [`DbError`] if the query fails.
    pub async fn schema_version(&self) -> Result<u32, DbError> {
        let v: i64 = sqlx::query("SELECT COALESCE(MAX(version), 0) AS v FROM schema_meta")
            .fetch_one(&self.pool)
            .await?
            .try_get("v")?;
        Ok(u32::try_from(v).unwrap_or(0))
    }

    /// The highest schema version this build knows how to migrate to. After a
    /// successful open, [`schema_version`](Self::schema_version) equals this.
    #[must_use]
    pub fn latest_known_version() -> u32 {
        migrations::latest_version()
    }

    /// The audit log over this database.
    #[must_use]
    pub fn audit(&self) -> AuditLog {
        AuditLog::new(self.pool.clone())
    }

    /// The intent journal over this database.
    #[must_use]
    pub fn journal(&self) -> SqliteJournal {
        SqliteJournal::new(self.pool.clone())
    }

    /// The settings/state-document store over this database.
    #[must_use]
    pub fn settings(&self) -> Settings {
        Settings::new(self.pool.clone())
    }

    /// Writes a consistent, compact backup via `VACUUM INTO`, named with a
    /// timestamp, and prunes to the newest `keep` backups.
    ///
    /// # Errors
    /// Returns [`DbError`] if the backup write or pruning fails.
    pub async fn backup(&self, keep: usize) -> Result<PathBuf, DbError> {
        let backup_path = self.backup_path();
        // VACUUM INTO produces a clean, defragmented copy — better than a file
        // copy, which could capture a torn WAL.
        let sql = format!("VACUUM INTO '{}'", backup_path.display());
        sqlx::query(&sql).execute(&self.pool).await?;
        self.prune_backups(keep)?;
        Ok(backup_path)
    }

    // --- internals ---------------------------------------------------------

    async fn try_open_healthy(path: &Path) -> Result<Self, DbError> {
        let pool = Self::connect(path, false).await?;
        // PRAGMA quick_check returns a single row "ok" on a healthy database.
        let rows = sqlx::query("PRAGMA quick_check").fetch_all(&pool).await?;
        let ok = rows.len() == 1 && rows[0].try_get::<String, _>(0).is_ok_and(|s| s == "ok");
        if ok {
            Ok(Self {
                pool,
                path: path.to_path_buf(),
            })
        } else {
            pool.close().await;
            Err(DbError::Sqlite(sqlx::Error::Protocol(
                "integrity check failed".into(),
            )))
        }
    }

    async fn recover(path: &Path) -> Result<(Self, OpenReport), DbError> {
        // Quarantine the corrupt file so it is preserved for inspection and out
        // of the way.
        let quarantined = Self::quarantine(path)?;

        // Try the newest backup.
        if let Some(backup) = Self::newest_backup(path) {
            std::fs::copy(&backup, path)?;
            if let Ok(db) = Self::try_open_healthy(path).await {
                db.migrate().await?;
                return Ok((
                    db,
                    OpenReport::RestoredFromBackup {
                        quarantined,
                        restored_from: backup,
                    },
                ));
            }
            // The backup was also unusable; remove the failed restore attempt.
            let _ = std::fs::remove_file(path);
        }

        // No usable backup: start empty. Fail-open — an empty config.db applies
        // no rules (AD-5).
        let db = Self::create_and_migrate(path).await?;
        Ok((db, OpenReport::StartedEmptyAfterCorruption { quarantined }))
    }

    async fn create_and_migrate(path: &Path) -> Result<Self, DbError> {
        let pool = Self::connect(path, true).await?;
        let db = Self {
            pool,
            path: path.to_path_buf(),
        };
        db.migrate().await?;
        Ok(db)
    }

    async fn connect(path: &Path, create: bool) -> Result<SqlitePool, DbError> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(create)
            // config.db is small and precious: WAL for concurrency, FULL
            // synchronous so the intent journal's write-ahead guarantee holds
            // across a power cut (ADR-0010).
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Full)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        Ok(pool)
    }

    async fn migrate(&self) -> Result<(), DbError> {
        migrations::run(&self.pool, &self.path).await
    }

    fn quarantine(path: &Path) -> Result<PathBuf, DbError> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut quarantined = path.to_path_buf();
        let name = format!(
            "{}.corrupt-{stamp}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("config.db")
        );
        quarantined.set_file_name(name);
        std::fs::rename(path, &quarantined)?;
        // WAL/SHM siblings would confuse a fresh open; move them aside too.
        for ext in ["-wal", "-shm"] {
            let sibling = sibling(path, ext);
            if sibling.exists() {
                let _ = std::fs::rename(&sibling, sibling.with_extension(format!("corrupt{ext}")));
            }
        }
        Ok(quarantined)
    }

    fn backup_path(&self) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let mut p = self.path.clone();
        let name = format!(
            "{}.backup-{stamp}",
            self.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("config.db")
        );
        p.set_file_name(name);
        p
    }

    fn newest_backup(path: &Path) -> Option<PathBuf> {
        let dir = path.parent()?;
        let prefix = format!(
            "{}.backup-",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("config.db")
        );
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix))
            })
            // Names end with a monotonic nanosecond stamp, so lexicographic max
            // is the newest.
            .max()
    }

    fn prune_backups(&self, keep: usize) -> Result<(), DbError> {
        let Some(dir) = self.path.parent() else {
            return Ok(());
        };
        let prefix = format!(
            "{}.backup-",
            self.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("config.db")
        );
        let mut backups: Vec<PathBuf> = std::fs::read_dir(dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
            .collect();
        backups.sort();
        // Keep the newest `keep`; remove the rest.
        if backups.len() > keep {
            for old in &backups[..backups.len() - keep] {
                let _ = std::fs::remove_file(old);
            }
        }
        Ok(())
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_path_buf();
    let name = format!(
        "{}{suffix}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.db")
    );
    p.set_file_name(name);
    p
}
