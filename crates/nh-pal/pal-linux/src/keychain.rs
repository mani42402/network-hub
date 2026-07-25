//! Secret Service keyring — the Linux desktop platform secret store
//! (WP-M1.1, §7.1).
//!
//! On a Linux **desktop** session, secrets live in the freedesktop Secret
//! Service (`gnome-keyring` / `KWallet`) over the session D-Bus, as
//! generic-password items. This is the right store when NetworkHub runs in a
//! user session with a keyring daemon.
//!
//! For a **headless** system daemon (a Pi with no desktop session) there is no
//! Secret Service to reach — the daemon has no user session bus. There the
//! backend is `nh_secrets::FileKeyring` (root-only, `0700`/`0600`), and the
//! composition root falls back to it when [`SecretServiceKeyring::open`] fails.
//!
//! The FFI-free `secret-service` crate (zbus, pure Rust) seals the D-Bus plumbing
//! and encrypts the transported secret with an ephemeral DH session, so this
//! module carries no `unsafe`. A [`SecretService`] borrows itself, so we connect
//! fresh per call — a keyring is touched rarely (master-key load, rotation), so
//! the extra D-Bus round trip is immaterial.

use std::collections::HashMap;

use nh_secrets::{Keyring, SecretsError, SecretsResult};
use secret_service::EncryptionType;
use secret_service::blocking::SecretService;

/// The content type recorded on stored items (opaque binary).
const CONTENT_TYPE: &str = "application/octet-stream";

/// A keyring backed by the freedesktop Secret Service.
#[derive(Debug)]
pub struct SecretServiceKeyring {
    service: String,
}

impl SecretServiceKeyring {
    /// Opens the backend, verifying the Secret Service is reachable and its
    /// default collection can be resolved. `service` namespaces the items.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if there is no session bus or no Secret Service
    /// provider (the headless case); the composition root then falls back to a
    /// file keyring.
    pub fn open(service: impl Into<String>) -> SecretsResult<Self> {
        // Probe: connect and resolve the default collection. Dropped immediately;
        // real operations reconnect.
        let ss = connect()?;
        ss.get_default_collection().map_err(map_err)?;
        Ok(Self {
            service: service.into(),
        })
    }

    /// The attribute set that identifies an entry within our namespace.
    fn attrs<'a>(&'a self, entry: &'a str) -> HashMap<&'a str, &'a str> {
        HashMap::from([
            ("application", "networkhub"),
            ("service", self.service.as_str()),
            ("entry", entry),
        ])
    }
}

impl Keyring for SecretServiceKeyring {
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>> {
        let ss = connect()?;
        let collection = unlocked_default(&ss)?;
        let items = collection
            .search_items(self.attrs(entry))
            .map_err(map_err)?;
        match items.first() {
            Some(item) => Ok(Some(item.get_secret().map_err(map_err)?)),
            None => Ok(None),
        }
    }

    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()> {
        let ss = connect()?;
        let collection = unlocked_default(&ss)?;
        // `replace = true` overwrites an existing item with the same attributes.
        collection
            .create_item(
                &format!("NetworkHub: {entry}"),
                self.attrs(entry),
                value,
                true,
                CONTENT_TYPE,
            )
            .map_err(map_err)?;
        Ok(())
    }

    fn delete(&self, entry: &str) -> SecretsResult<()> {
        let ss = connect()?;
        let collection = unlocked_default(&ss)?;
        for item in collection
            .search_items(self.attrs(entry))
            .map_err(map_err)?
        {
            item.delete().map_err(map_err)?;
        }
        Ok(())
    }
}

/// True when the Secret Service is reachable here. The daemon uses this to decide
/// whether to fall back to a file keyring (the headless case).
#[must_use]
pub fn is_available() -> bool {
    connect().is_ok_and(|ss| ss.get_default_collection().is_ok())
}

/// Connects to the Secret Service with an encrypted (DH) session.
fn connect() -> SecretsResult<SecretService<'static>> {
    SecretService::connect(EncryptionType::Dh).map_err(map_err)
}

/// Resolves the default collection and unlocks it if necessary. On a desktop the
/// login keyring is usually already unlocked; a locked one may prompt the user.
fn unlocked_default<'a>(
    ss: &'a SecretService<'a>,
) -> SecretsResult<secret_service::blocking::Collection<'a>> {
    let collection = ss.get_default_collection().map_err(map_err)?;
    if collection.is_locked().map_err(map_err)? {
        collection.unlock().map_err(map_err)?;
    }
    Ok(collection)
}

/// Maps a Secret Service error to a message-only [`SecretsError`], never echoing
/// secret material. Taken by value so it composes as `.map_err(map_err)`.
#[allow(clippy::needless_pass_by_value)]
fn map_err(e: secret_service::Error) -> SecretsError {
    SecretsError::Keyring(format!("secret service error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full round-trip against a live Secret Service. Skips when none is
    /// reachable — the headless case (and the CI Linux runner), where the daemon
    /// uses a file keyring instead. Where a keyring *is* present (a developer
    /// desktop), it exercises the real store end to end and cleans up after
    /// itself.
    #[test]
    fn round_trips_when_service_is_available() {
        let Ok(kr) = SecretServiceKeyring::open("networkhub-test") else {
            eprintln!("no Secret Service reachable — skipping (headless/CI)");
            return;
        };

        // Start clean in case a prior aborted run left the entry behind.
        kr.delete("test-master").expect("delete");

        assert!(kr.get("test-master").expect("get").is_none());
        kr.set("test-master", b"secret-bytes").expect("set");
        assert_eq!(
            kr.get("test-master").expect("get").expect("some"),
            b"secret-bytes"
        );

        // Overwrite replaces in place.
        kr.set("test-master", b"rotated").expect("update");
        assert_eq!(
            kr.get("test-master").expect("get").expect("some"),
            b"rotated"
        );

        kr.delete("test-master").expect("delete");
        assert!(kr.get("test-master").expect("get").is_none());
    }
}
