//! Runtime selection of the platform secret keyring (WP-M1.1, §7.1).
//!
//! The composition root is the only place allowed to wire concrete `pal-*`
//! backends (AD-7), so backend *selection* lives here. Each OS has a native
//! store, with the root-only file keyring as the universal fallback:
//!
//! | OS      | native store                        | fallback            |
//! |---------|-------------------------------------|---------------------|
//! | Linux   | Secret Service (user session)       | `FileKeyring`       |
//! | macOS   | System keychain (root) / default    | `FileKeyring`       |
//! | Windows | DPAPI machine-scope (under the dir) | `FileKeyring`       |
//!
//! The fallback is what a headless Linux daemon (a Pi with no session bus)
//! actually uses — the file keyring is root-only (`0700`/`0600`) and every
//! secret it guards is AES-256-GCM sealed regardless (resolving OD-6). Selection
//! never fails: an unavailable native store degrades to the file keyring rather
//! than taking the daemon down.

use std::path::Path;
use std::sync::Arc;

use nh_secrets::{FileKeyring, Keyring, SecretsResult};

/// The service namespace for the platform stores (keychain "service", Secret
/// Service attribute). Windows DPAPI namespaces by directory instead, so this is
/// only referenced by the macOS and Linux arms.
#[cfg(any(target_os = "macos", target_os = "linux"))]
const SERVICE: &str = "networkhub";

/// Opens the best available secret keyring for this host. Tries the OS-native
/// store first and falls back to a root-only file keyring under `dir/secrets`.
///
/// # Errors
/// Returns [`nh_secrets::SecretsError`] only if even the file-keyring fallback
/// cannot be created (e.g. `dir` is not writable) — the native store never
/// propagates an error, it just declines and we fall back.
pub fn open(dir: &Path) -> SecretsResult<Arc<dyn Keyring>> {
    if let Some(native) = native_keyring(dir) {
        return Ok(native);
    }
    tracing::info!("secrets: using root-only file keyring (no platform store available)");
    file_keyring(dir)
}

/// The universal fallback: a root-only file keyring under `dir/secrets`.
fn file_keyring(dir: &Path) -> SecretsResult<Arc<dyn Keyring>> {
    Ok(Arc::new(FileKeyring::open(dir.join("secrets"))?))
}

#[cfg(target_os = "linux")]
fn native_keyring(_dir: &Path) -> Option<Arc<dyn Keyring>> {
    match pal_linux::SecretServiceKeyring::open(SERVICE) {
        Ok(kr) => {
            tracing::info!("secrets: using the Secret Service");
            Some(Arc::new(kr))
        }
        Err(e) => {
            tracing::debug!(error = %e, "secrets: Secret Service unavailable; falling back");
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn native_keyring(_dir: &Path) -> Option<Arc<dyn Keyring>> {
    // The System keychain is writable only by root — opening it succeeds for any
    // user, but a write would fail — so only the root daemon targets it. A
    // desktop/dev process uses its default (login) keychain.
    if nix::unistd::geteuid().is_root()
        && let Ok(kr) = pal_macos::KeychainKeyring::system(SERVICE)
    {
        tracing::info!("secrets: using the System keychain");
        return Some(Arc::new(kr));
    }
    match pal_macos::KeychainKeyring::default_keychain(SERVICE) {
        Ok(kr) => {
            tracing::info!("secrets: using the default (login) keychain");
            Some(Arc::new(kr))
        }
        Err(e) => {
            tracing::debug!(error = %e, "secrets: no keychain available; falling back");
            None
        }
    }
}

#[cfg(windows)]
fn native_keyring(dir: &Path) -> Option<Arc<dyn Keyring>> {
    match pal_windows::DpapiKeyring::open(dir.join("secrets")) {
        Ok(kr) => {
            tracing::info!("secrets: using the DPAPI machine-scope keyring");
            Some(Arc::new(kr))
        }
        Err(e) => {
            tracing::debug!(error = %e, "secrets: DPAPI unavailable; falling back");
            None
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn native_keyring(_dir: &Path) -> Option<Arc<dyn Keyring>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nh_secrets::SecretStore;

    /// The file fallback integrates with [`SecretStore`] and survives a restart:
    /// a store opened over a fresh file keyring at the same dir opens a blob the
    /// first store sealed. This is the "secrets survive a reboot" property.
    ///
    /// The test targets the fallback directly rather than `open()` so it is
    /// hermetic on every platform — the native `open()` path would touch a
    /// process-global store (the login keychain) and could prompt for access.
    /// The native backends are covered by the `pal-*` crates' own tests.
    #[test]
    fn file_fallback_seals_and_survives_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");

        let sealed = {
            let keyring = file_keyring(dir.path()).expect("open file keyring");
            let store = SecretStore::open(keyring).expect("open store");
            let sealed = store.seal(b"wireguard-private-key").expect("seal");
            assert_ne!(sealed.as_slice(), b"wireguard-private-key");
            assert_eq!(
                store.unseal(&sealed).expect("unseal"),
                b"wireguard-private-key"
            );
            sealed
        };

        // A second store over a freshly-opened keyring at the same dir loads the
        // same master key and opens the earlier blob.
        let keyring = file_keyring(dir.path()).expect("reopen file keyring");
        let store = SecretStore::open(keyring).expect("reopen store");
        assert_eq!(
            store.unseal(&sealed).expect("unseal after reopen"),
            b"wireguard-private-key"
        );
    }
}
