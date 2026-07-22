//! The tamper-evident audit log (§7.2, §10.2).
//!
//! Every security-relevant action is recorded append-only, and each entry's hash
//! chains to the previous one: `entry_hash = BLAKE3(prev_hash ‖ canonical(entry))`.
//! Editing or deleting a past entry breaks the chain from that point on, so
//! tampering is *detectable* even by someone with database write access —
//! exactly the property an audit log needs to be worth anything.
//!
//! The log is append-only in code: there is no update or delete path. Pruning,
//! when it lands, will re-anchor rather than edit.

use sqlx::{Row, SqlitePool};

use super::DbError;

/// The genesis hash that anchors an empty chain.
const GENESIS: &str = "genesis";

/// One audit record, as written and as read back for verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// Who performed the action (a principal id, or "system"/"automation").
    pub actor: String,
    /// Where it came from: `local-ui`, `cli`, `gateway`, `plugin`, `automation`.
    pub source: String,
    /// What was done, e.g. `firewall.apply`, `vpn.connect`.
    pub action: String,
    /// What it was done to, e.g. a device id or a profile name.
    pub target: String,
    /// The outcome: `ok`, `denied`, `error`.
    pub result: String,
}

/// The result of verifying the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TamperCheck {
    /// Every entry's hash matches; the log is intact.
    Intact {
        /// How many entries were verified.
        entries: u64,
    },
    /// The chain is broken at the given sequence number — that row or an earlier
    /// one was altered or removed.
    Broken {
        /// The sequence where verification first failed.
        at_seq: i64,
    },
}

/// The append-only, hash-chained audit log over `config.db`.
#[derive(Debug, Clone)]
pub struct AuditLog {
    pool: SqlitePool,
}

impl AuditLog {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Appends an entry, chaining it to the current tip. Returns the new entry's
    /// sequence number.
    ///
    /// # Errors
    /// Returns [`DbError`] if the write fails.
    pub async fn append(&self, entry: &AuditEntry) -> Result<i64, DbError> {
        // Take the tip hash under the same connection to keep the chain linear.
        // A serialized writer (config.db has few writers) makes this safe; a
        // busy multi-writer design would wrap this in an EXCLUSIVE transaction.
        let mut tx = self.pool.begin().await?;

        let prev_hash: String =
            sqlx::query("SELECT entry_hash FROM audit_log ORDER BY seq DESC LIMIT 1")
                .fetch_optional(&mut *tx)
                .await?
                .and_then(|r| r.try_get::<String, _>(0).ok())
                .unwrap_or_else(|| GENESIS.to_owned());

        let created_at = now_secs();
        let entry_hash = hash_entry(&prev_hash, entry, created_at);

        let seq: i64 = sqlx::query(
            "INSERT INTO audit_log
             (actor, source, action, target, result, created_at, prev_hash, entry_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING seq",
        )
        .bind(&entry.actor)
        .bind(&entry.source)
        .bind(&entry.action)
        .bind(&entry.target)
        .bind(&entry.result)
        .bind(created_at)
        .bind(&prev_hash)
        .bind(&entry_hash)
        .fetch_one(&mut *tx)
        .await?
        .try_get::<i64, _>("seq")?;

        tx.commit().await?;
        Ok(seq)
    }

    /// Recomputes the whole chain and reports whether it is intact.
    ///
    /// # Errors
    /// Returns [`DbError`] if reading the log fails.
    pub async fn verify(&self) -> Result<TamperCheck, DbError> {
        let rows = sqlx::query(
            "SELECT seq, actor, source, action, target, result, created_at, prev_hash, entry_hash
             FROM audit_log ORDER BY seq ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut expected_prev = GENESIS.to_owned();
        let mut count = 0u64;
        for row in &rows {
            let seq: i64 = row.get("seq");
            let stored_prev: String = row.get("prev_hash");
            let stored_hash: String = row.get("entry_hash");
            let created_at: i64 = row.get("created_at");
            let entry = AuditEntry {
                actor: row.get("actor"),
                source: row.get("source"),
                action: row.get("action"),
                target: row.get("target"),
                result: row.get("result"),
            };

            // The recorded prev_hash must match the running chain, and the
            // recorded entry_hash must match a fresh recomputation. Either
            // mismatch means this row (or an earlier one) was altered.
            if stored_prev != expected_prev {
                return Ok(TamperCheck::Broken { at_seq: seq });
            }
            let recomputed = hash_entry(&stored_prev, &entry, created_at);
            if recomputed != stored_hash {
                return Ok(TamperCheck::Broken { at_seq: seq });
            }
            expected_prev = stored_hash;
            count += 1;
        }
        Ok(TamperCheck::Intact { entries: count })
    }

    /// The number of entries, for tests and diagnostics.
    ///
    /// # Errors
    /// Returns [`DbError`] if the count query fails.
    pub async fn len(&self) -> Result<u64, DbError> {
        let n: i64 = sqlx::query("SELECT COUNT(*) AS n FROM audit_log")
            .fetch_one(&self.pool)
            .await?
            .try_get("n")?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Whether the audit log has no entries.
    ///
    /// # Errors
    /// Returns [`DbError`] if the count query fails.
    pub async fn is_empty(&self) -> Result<bool, DbError> {
        Ok(self.len().await? == 0)
    }
}

/// Hashes an entry over its predecessor and its canonical content. Field
/// separators (`\x1f`, the ASCII unit separator) prevent a field-boundary
/// ambiguity attack (e.g. actor="a" action="bc" hashing the same as
/// actor="ab" action="c").
fn hash_entry(prev_hash: &str, entry: &AuditEntry, created_at: i64) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in [
        prev_hash,
        &entry.actor,
        &entry.source,
        &entry.action,
        &entry.target,
        &entry.result,
        &created_at.to_string(),
    ] {
        hasher.update(part.as_bytes());
        hasher.update(b"\x1f");
    }
    hasher.finalize().to_hex().to_string()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        .try_into()
        .unwrap_or(0)
}
