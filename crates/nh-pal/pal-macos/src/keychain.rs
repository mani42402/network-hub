//! Keychain-backed keyring — the macOS platform secret store (WP-M1.1, §7.1).
//!
//! Secrets are stored as **generic-password** items in a keychain via
//! Security.framework. A generic password survives a reboot, is protected by the
//! OS, and is invisible to `config.db`. For the daemon (which runs as root) the
//! target is the **System keychain** at `/Library/Keychains/System.keychain`;
//! elsewhere it falls back to the caller's default keychain.
//!
//! The FFI is sealed inside the `security-framework` crate — this module holds no
//! `unsafe` of its own, only the safe [`Keyring`] mapping the daemon wires in as
//! the macOS backend.

use nh_secrets::{Keyring, SecretsError, SecretsResult};
use security_framework::base::Error as SecError;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::os::macos::passwords::find_generic_password;

/// The System keychain — the machine-wide store a root daemon writes to.
const SYSTEM_KEYCHAIN: &str = "/Library/Keychains/System.keychain";

/// `errSecItemNotFound` — the item does not exist.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// A keyring backed by a macOS keychain's generic-password items.
pub struct KeychainKeyring {
    keychain: SecKeychain,
    service: String,
}

impl std::fmt::Debug for KeychainKeyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `SecKeychain` is opaque and secret-adjacent; show only the namespace.
        f.debug_struct("KeychainKeyring")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

impl KeychainKeyring {
    /// Opens the backend against the **System keychain** — the correct store for
    /// the root daemon. `service` namespaces the items (use the product name).
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the System keychain cannot be opened (e.g. the
    /// process is not root); callers fall back to a file keyring.
    pub fn system(service: impl Into<String>) -> SecretsResult<Self> {
        let keychain = SecKeychain::open(SYSTEM_KEYCHAIN).map_err(map_err)?;
        Ok(Self {
            keychain,
            service: service.into(),
        })
    }

    /// Opens the backend against the caller's **default keychain** (the login
    /// keychain in a user session). A fallback for non-root/desktop use.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if there is no accessible default keychain.
    pub fn default_keychain(service: impl Into<String>) -> SecretsResult<Self> {
        let keychain = SecKeychain::default().map_err(map_err)?;
        Ok(Self {
            keychain,
            service: service.into(),
        })
    }

    /// Wraps an explicit keychain — used by tests to target a hermetic temporary
    /// keychain instead of the login/System one.
    #[must_use]
    pub fn with_keychain(keychain: SecKeychain, service: impl Into<String>) -> Self {
        Self {
            keychain,
            service: service.into(),
        }
    }
}

impl Keyring for KeychainKeyring {
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>> {
        match find_generic_password(
            Some(std::slice::from_ref(&self.keychain)),
            &self.service,
            entry,
        ) {
            Ok((password, _item)) => Ok(Some(password.as_ref().to_vec())),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }

    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()> {
        // Update in place if the item exists, otherwise add it — the keychain
        // rejects a duplicate add.
        match find_generic_password(
            Some(std::slice::from_ref(&self.keychain)),
            &self.service,
            entry,
        ) {
            Ok((_password, mut item)) => {
                item.set_password(value).map_err(map_err)?;
                Ok(())
            }
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => self
                .keychain
                .set_generic_password(&self.service, entry, value)
                .map_err(map_err),
            Err(e) => Err(map_err(e)),
        }
    }

    fn delete(&self, entry: &str) -> SecretsResult<()> {
        match find_generic_password(
            Some(std::slice::from_ref(&self.keychain)),
            &self.service,
            entry,
        ) {
            Ok((_password, item)) => {
                item.delete();
                Ok(())
            }
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(map_err(e)),
        }
    }
}

/// True when a keychain-backed keyring can be opened here (System keychain if
/// root, else the default keychain). The daemon uses this to decide whether to
/// fall back to a file keyring.
#[must_use]
pub fn is_available() -> bool {
    SecKeychain::open(SYSTEM_KEYCHAIN).is_ok() || SecKeychain::default().is_ok()
}

/// Maps a Security.framework error to a message-only [`SecretsError`], never
/// echoing secret material.
fn map_err(e: SecError) -> SecretsError {
    SecretsError::Keyring(format!("keychain error (OSStatus {}): {e}", e.code()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_framework::os::macos::keychain::CreateOptions;

    /// A hermetic temporary keychain so the tests never touch the login or
    /// System keychain (and so they run unattended in CI).
    fn temp_keyring() -> (tempfile::TempDir, KeychainKeyring) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("networkhub-test.keychain");
        let keychain = CreateOptions::new()
            .password("test-keychain-password")
            .create(&path)
            .expect("create temp keychain");
        (
            dir,
            KeychainKeyring::with_keychain(keychain, "networkhub-test"),
        )
    }

    #[test]
    fn round_trips_a_secret() {
        let (_dir, kr) = temp_keyring();

        assert!(kr.get("absent").expect("get").is_none());

        kr.set("master", b"wireguard-key").expect("set");
        assert_eq!(
            kr.get("master").expect("get").expect("some"),
            b"wireguard-key"
        );

        // Overwrite updates in place.
        kr.set("master", b"rotated-key").expect("update");
        assert_eq!(
            kr.get("master").expect("get").expect("some"),
            b"rotated-key"
        );

        kr.delete("master").expect("delete");
        assert!(kr.get("master").expect("get").is_none());
        kr.delete("master").expect("delete-missing is ok");
    }

    #[test]
    fn distinct_entries_are_independent() {
        let (_dir, kr) = temp_keyring();
        kr.set("a", b"one").expect("set a");
        kr.set("b", b"two").expect("set b");
        assert_eq!(kr.get("a").expect("get").expect("some"), b"one");
        assert_eq!(kr.get("b").expect("get").expect("some"), b"two");
    }
}
