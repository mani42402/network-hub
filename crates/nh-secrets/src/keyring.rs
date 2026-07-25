//! The keyring abstraction and its portable backends.
//!
//! A [`Keyring`] holds a handful of small, named secrets — in practice just the
//! master key. The platform backends (DPAPI / Keychain / Secret Service) land
//! separately; the two here are portable and pure-Rust: [`MemoryKeyring`] for
//! tests and [`FileKeyring`], the root-only file store that is the headless
//! Linux fallback (§7.1).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::error::SecretsResult;

/// Stores small named secrets (the master key) in an OS-appropriate vault.
///
/// Values are opaque bytes. Implementations must keep them out of any location a
/// non-privileged user or the config database can read.
pub trait Keyring: Send + Sync {
    /// Reads the value for `entry`, or `None` if it has never been set.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the backend fails (not for a missing entry —
    /// that is `Ok(None)`).
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>>;

    /// Stores `value` under `entry`, replacing any previous value.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the backend fails.
    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()>;

    /// Removes `entry`. Removing a missing entry succeeds.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the backend fails.
    fn delete(&self, entry: &str) -> SecretsResult<()>;
}

/// An in-memory keyring for tests. Never persists — losing the process loses the
/// key, which is exactly why production uses a real backend.
#[derive(Debug, Default)]
pub struct MemoryKeyring {
    entries: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryKeyring {
    /// A new, empty in-memory keyring.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Keyring for MemoryKeyring {
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>> {
        Ok(self.lock().get(entry).cloned())
    }
    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()> {
        self.lock().insert(entry.to_owned(), value.to_vec());
        Ok(())
    }
    fn delete(&self, entry: &str) -> SecretsResult<()> {
        self.lock().remove(entry);
        Ok(())
    }
}

/// A root-only file keyring — the headless Linux fallback when there is no
/// desktop keyring (§7.1). Each entry is a separate file with `0600` permissions
/// in a `0700` directory, so only the owner (root, in production) can read it.
/// The secret blobs it protects are AES-256-GCM sealed regardless, so the file
/// holds only the master key.
#[derive(Debug)]
pub struct FileKeyring {
    dir: PathBuf,
}

impl FileKeyring {
    /// Opens (creating if needed) a file keyring rooted at `dir`. On Unix the
    /// directory is forced to `0700`.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the directory cannot be created or secured.
    pub fn open(dir: impl Into<PathBuf>) -> SecretsResult<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        secure_dir(&dir)?;
        Ok(Self { dir })
    }

    /// The path for an entry, with the name sanitised so it cannot escape `dir`.
    fn path(&self, entry: &str) -> PathBuf {
        let safe: String = entry
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.key"))
    }
}

impl Keyring for FileKeyring {
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>> {
        match std::fs::read(self.path(entry)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()> {
        write_owner_only(&self.path(entry), value)
    }
    fn delete(&self, entry: &str) -> SecretsResult<()> {
        match std::fs::remove_file(self.path(entry)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(unix)]
fn secure_dir(dir: &Path) -> SecretsResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

// Signature must match the Unix variant (which genuinely can fail), so the
// always-`Ok` non-Unix body trips `unnecessary_wraps` — a cfg-gated false
// positive only clippy on a non-Unix target sees.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn secure_dir(_dir: &Path) -> SecretsResult<()> {
    // Non-Unix uses a platform keychain, not this fallback; nothing to harden.
    Ok(())
}

#[cfg(unix)]
fn write_owner_only(path: &Path, data: &[u8]) -> SecretsResult<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(data)?;
    f.sync_all()?;
    // Force the mode even if the file already existed with looser bits.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_owner_only(path: &Path, data: &[u8]) -> SecretsResult<()> {
    std::fs::write(path, data)?;
    Ok(())
}
