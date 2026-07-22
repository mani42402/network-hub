//! Persistence and resilience tests for config.db (WP-M0.5 exit criteria).
//!
//! Proves the three properties that make the storage layer trustworthy:
//! corruption cannot crash-loop the daemon (F-5), the audit log detects
//! tampering (§7.2), and migrations take a backup and reach the latest schema.

#![cfg(feature = "persist")]
#![allow(clippy::unwrap_used)]

use std::io::Write;

use nh_state::journal::{Intent, IntentState, JournalStore};
use nh_state::persist::{AuditEntry, Database, OpenReport, TamperCheck};

fn audit(action: &str) -> AuditEntry {
    AuditEntry {
        actor: "user:1".into(),
        source: "local-ui".into(),
        action: action.into(),
        target: "device:abc".into(),
        result: "ok".into(),
    }
}

fn intent(id: &str) -> Intent {
    Intent {
        id: id.into(),
        module: "routing".into(),
        operation: "sysctl.ip_forward".into(),
        prior_value: "0".into(),
        new_value: "1".into(),
        state: IntentState::Pending,
    }
}

#[tokio::test]
async fn opens_fresh_and_reaches_latest_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");

    let (db, report) = Database::open(&path).await.unwrap();
    assert_eq!(report, OpenReport::CreatedFresh);
    assert_eq!(
        db.schema_version().await.unwrap(),
        Database::latest_known_version()
    );
}

#[tokio::test]
async fn reopen_is_healthy_and_preserves_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");

    let (db, _) = Database::open(&path).await.unwrap();
    db.settings().put("hotspot.enabled", &true).await.unwrap();
    db.close().await;
    // Reopen: existing healthy database, data intact.
    let (db, report) = Database::open(&path).await.unwrap();
    assert_eq!(report, OpenReport::Healthy);
    let (value, revision): (bool, i64) =
        db.settings().get("hotspot.enabled").await.unwrap().unwrap();
    assert!(value);
    assert_eq!(revision, 1);
}

#[tokio::test]
async fn settings_revision_bumps_on_each_write() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Database::open(dir.path().join("config.db")).await.unwrap();
    let s = db.settings();
    assert_eq!(s.put("k", &1u32).await.unwrap(), 1);
    assert_eq!(s.put("k", &2u32).await.unwrap(), 2);
    let (v, rev): (u32, i64) = s.get("k").await.unwrap().unwrap();
    assert_eq!((v, rev), (2, 2));
}

#[tokio::test]
async fn durable_journal_round_trips_and_lists_pending() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Database::open(dir.path().join("config.db")).await.unwrap();
    let j = db.journal();

    j.append(intent("1")).await.unwrap();
    j.append(intent("2")).await.unwrap();
    j.commit("1").await.unwrap();

    let pending = j.pending().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "2");
    assert_eq!(pending[0].prior_value, "0");
}

#[tokio::test]
async fn pending_intents_survive_reopen() {
    // The write-ahead guarantee across a "restart": a pending intent written
    // before the process stops is still pending — and thus rolled back — on the
    // next open (F-3).
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");
    let (db, _) = Database::open(&path).await.unwrap();
    db.journal().append(intent("survivor")).await.unwrap();
    db.close().await;
    let (db, _) = Database::open(&path).await.unwrap();
    let pending = db.journal().pending().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, "survivor");
}

#[tokio::test]
async fn audit_chain_is_intact_and_detects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Database::open(dir.path().join("config.db")).await.unwrap();
    let log = db.audit();

    log.append(&audit("vpn.connect")).await.unwrap();
    log.append(&audit("firewall.apply")).await.unwrap();
    log.append(&audit("device.block")).await.unwrap();

    // A freshly-written chain verifies.
    assert_eq!(
        log.verify().await.unwrap(),
        TamperCheck::Intact { entries: 3 }
    );

    // Tamper: edit a past entry's action directly in the table, bypassing the
    // append path. The hash chain must catch it.
    sqlx::query("UPDATE audit_log SET action = 'device.allow' WHERE seq = 2")
        .execute(db.pool())
        .await
        .unwrap();

    match log.verify().await.unwrap() {
        TamperCheck::Broken { at_seq } => {
            assert_eq!(at_seq, 2, "tamper detected at the edited row");
        }
        TamperCheck::Intact { .. } => panic!("tampering went undetected"),
    }
}

#[tokio::test]
async fn deleting_an_audit_row_breaks_the_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (db, _) = Database::open(dir.path().join("config.db")).await.unwrap();
    let log = db.audit();
    for a in ["a", "b", "c"] {
        log.append(&audit(a)).await.unwrap();
    }
    // Delete the middle row: the following row's prev_hash no longer matches.
    sqlx::query("DELETE FROM audit_log WHERE seq = 2")
        .execute(db.pool())
        .await
        .unwrap();
    assert!(matches!(
        log.verify().await.unwrap(),
        TamperCheck::Broken { .. }
    ));
}

#[tokio::test]
async fn corruption_recovers_from_backup() {
    // F-5: a corrupt database is quarantined and restored from a backup, and the
    // daemon comes up with its data — never a crash loop.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");

    // Establish data and take a backup.
    let (db, _) = Database::open(&path).await.unwrap();
    db.settings()
        .put("important", &"keep me".to_string())
        .await
        .unwrap();
    db.backup(3).await.unwrap();
    db.close().await;

    // Corrupt the live database by overwriting it with garbage.
    corrupt_file(&path);

    // Reopen: recovery restores from the backup.
    let (db, report) = Database::open(&path).await.unwrap();
    match report {
        OpenReport::RestoredFromBackup { quarantined, .. } => {
            assert!(
                quarantined.exists(),
                "the corrupt file was preserved for inspection"
            );
        }
        other => panic!("expected RestoredFromBackup, got {other:?}"),
    }
    // The restored data is present.
    let (value, _): (String, i64) = db.settings().get("important").await.unwrap().unwrap();
    assert_eq!(value, "keep me");
}

#[tokio::test]
async fn corruption_without_backup_starts_empty_but_comes_up() {
    // F-5 worst case: corrupt database, no backup. The daemon must still come up
    // — with empty desired state, which applies no rules (fail-open, AD-5) —
    // rather than crash-loop.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");
    let (db, _) = Database::open(&path).await.unwrap();
    db.settings().put("gone", &1u32).await.unwrap();
    // No backup taken.
    db.close().await;
    corrupt_file(&path);

    let (db, report) = Database::open(&path).await.unwrap();
    match report {
        OpenReport::StartedEmptyAfterCorruption { quarantined } => {
            assert!(quarantined.exists());
        }
        other => panic!("expected StartedEmptyAfterCorruption, got {other:?}"),
    }
    // Empty: the old data is gone, but the database is usable and at latest schema.
    assert!(db.settings().get::<u32>("gone").await.unwrap().is_none());
    assert_eq!(
        db.schema_version().await.unwrap(),
        Database::latest_known_version()
    );
}

#[tokio::test]
async fn backups_rotate_to_the_keep_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.db");
    let (db, _) = Database::open(&path).await.unwrap();
    db.settings().put("k", &1u32).await.unwrap();

    for _ in 0..5 {
        db.backup(2).await.unwrap();
    }
    // Only the newest 2 backups remain.
    let backups = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".backup-"))
        .count();
    assert_eq!(backups, 2);
}

/// Replaces a database file with non-SQLite bytes so the next open sees
/// corruption. The caller must `db.close().await` first.
///
/// The file is fully removed and rewritten (not truncated in place), and synced
/// to disk, so the corruption is deterministic even under heavy parallel test
/// load — an in-place truncate can race the OS releasing the closed handle.
fn corrupt_file(path: &std::path::Path) {
    // Remove the WAL/SHM siblings so the garbage main file is what gets opened.
    for ext in ["-wal", "-shm"] {
        let sibling = path.with_file_name(format!(
            "{}{ext}",
            path.file_name().unwrap().to_str().unwrap()
        ));
        let _ = std::fs::remove_file(sibling);
    }
    // Remove and recreate rather than truncate, to avoid any lingering handle.
    let _ = std::fs::remove_file(path);
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"this is not a sqlite database, it is garbage bytes not a header")
        .unwrap();
    // Force the garbage to disk before the reopen reads it.
    f.sync_all().unwrap();
}
