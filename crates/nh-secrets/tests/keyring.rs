//! Backend tests for the portable keyrings.
//!
//! `MemoryKeyring` is exercised through `SecretStore`'s unit tests already; here
//! we cover the `Keyring` contract directly and, crucially, the on-disk
//! hardening of `FileKeyring` that backs the "unreadable by a non-root user"
//! exit criterion (§7.1). The permission assertions are Unix-only.

#![allow(clippy::unwrap_used)]

use nh_secrets::{FileKeyring, Keyring, MemoryKeyring, SecretStore};
use std::sync::Arc;

/// The `Keyring` contract, run against any backend: missing → `None`, set then
/// get round-trips, overwrite replaces, delete removes, deleting-missing is ok.
fn assert_keyring_contract(kr: &dyn Keyring) {
    assert!(
        kr.get("absent").unwrap().is_none(),
        "missing entry reads as None"
    );

    kr.set("k", b"first").unwrap();
    assert_eq!(kr.get("k").unwrap().unwrap(), b"first");

    kr.set("k", b"second").unwrap();
    assert_eq!(kr.get("k").unwrap().unwrap(), b"second", "set overwrites");

    kr.delete("k").unwrap();
    assert!(kr.get("k").unwrap().is_none(), "delete removes the entry");

    kr.delete("k").unwrap(); // deleting a missing entry succeeds
}

#[test]
fn memory_keyring_honours_the_contract() {
    assert_keyring_contract(&MemoryKeyring::new());
}

#[test]
fn file_keyring_honours_the_contract() {
    let dir = tempfile::tempdir().unwrap();
    let kr = FileKeyring::open(dir.path()).unwrap();
    assert_keyring_contract(&kr);
}

#[test]
fn file_keyring_persists_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let kr = FileKeyring::open(dir.path()).unwrap();
        kr.set("master", b"key-bytes").unwrap();
    }
    // A fresh handle over the same directory sees the stored value.
    let kr = FileKeyring::open(dir.path()).unwrap();
    assert_eq!(kr.get("master").unwrap().unwrap(), b"key-bytes");
}

/// Entry names that could otherwise escape the directory are sanitised, so a
/// hostile name cannot write outside the keyring dir.
#[test]
fn file_keyring_sanitises_entry_names() {
    let dir = tempfile::tempdir().unwrap();
    let kr = FileKeyring::open(dir.path()).unwrap();
    kr.set("../escape", b"nope").unwrap();

    // Nothing was written to the parent of the keyring directory.
    let parent = dir.path().parent().unwrap();
    let escaped = parent.join("escape.key");
    assert!(
        !escaped.exists(),
        "entry name must not escape the keyring dir"
    );
    // ...and the value is retrievable under its original (sanitised) name.
    assert_eq!(kr.get("../escape").unwrap().unwrap(), b"nope");
}

/// A `SecretStore` can use the file keyring end-to-end, and its master key
/// survives a reopen — the property that lets secrets survive a reboot.
#[test]
fn secret_store_over_file_keyring_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let sealed = {
        let kr = Arc::new(FileKeyring::open(dir.path()).unwrap());
        SecretStore::open(kr)
            .unwrap()
            .seal(b"vpn-private-key")
            .unwrap()
    };
    // Reopen from a cold keyring on disk — same master key, blob still opens.
    let kr = Arc::new(FileKeyring::open(dir.path()).unwrap());
    let store = SecretStore::open(kr).unwrap();
    assert_eq!(store.unseal(&sealed).unwrap(), b"vpn-private-key");
}

/// The core of the "unreadable by non-root" exit criterion: on Unix the keyring
/// directory is `0700` and each entry file is `0600`, so no group/other bit is
/// ever set.
#[cfg(unix)]
#[test]
fn file_keyring_is_owner_only_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let kr = FileKeyring::open(dir.path()).unwrap();
    kr.set("master", b"secret").unwrap();

    let dir_mode = std::fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        dir_mode, 0o700,
        "keyring dir must be 0700, got {dir_mode:o}"
    );

    let file = dir.path().join("master.key");
    let file_mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        file_mode, 0o600,
        "entry file must be 0600, got {file_mode:o}"
    );
}

/// Even if an entry file somehow pre-exists with loose permissions, writing to
/// it forces the mode back to `0600`.
#[cfg(unix)]
#[test]
fn file_keyring_tightens_a_preexisting_loose_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let kr = FileKeyring::open(dir.path()).unwrap();

    // Plant a world-readable file where the entry will live.
    let file = dir.path().join("master.key");
    std::fs::write(&file, b"old").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

    kr.set("master", b"new").unwrap();

    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "write must tighten mode to 0600, got {mode:o}");
    assert_eq!(kr.get("master").unwrap().unwrap(), b"new");
}
