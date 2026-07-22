//! The singleton lock — exactly one daemon per machine (AD-1, §4.1).
//!
//! A second daemon starting while one runs must fail fast with a clear message,
//! not race the first over the socket and the database. On Unix an advisory
//! `flock` on a lock file gives that: the lock is held for the process's
//! lifetime and released automatically if it dies, so a crash never leaves a
//! stale lock (unlike a PID file). The Windows equivalent (a named mutex) lands
//! with the service integration in WP-M0.7.

use std::path::Path;

use crate::lifecycle::DaemonError;

/// Holds the singleton lock. Dropping it releases the lock.
#[derive(Debug)]
pub(crate) struct SingletonLock {
    #[cfg(unix)]
    _flock: nix::fcntl::Flock<std::fs::File>,
    #[cfg(not(unix))]
    _placeholder: (),
}

/// Acquires the singleton lock, or returns [`DaemonError::AlreadyRunning`] if
/// another instance holds it.
///
/// # Errors
/// Returns [`DaemonError::Io`] if the lock file cannot be created, or
/// [`DaemonError::AlreadyRunning`] if the lock is already held.
pub(crate) fn acquire(lock_path: &Path) -> Result<SingletonLock, DaemonError> {
    #[cfg(unix)]
    {
        use nix::fcntl::{Flock, FlockArg};

        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(lock_path)?;

        match Flock::lock(file, FlockArg::LockExclusiveNonblock) {
            Ok(flock) => Ok(SingletonLock { _flock: flock }),
            // The lock is held by another instance (or a transient error).
            Err(_) => Err(DaemonError::AlreadyRunning),
        }
    }

    #[cfg(not(unix))]
    {
        // Windows singleton (named mutex) lands in WP-M0.7. Until then, creating
        // the lock file is enough to keep the API consistent.
        let _ = lock_path;
        Ok(SingletonLock { _placeholder: () })
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_fails_while_first_held() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("daemon.lock");

        let first = acquire(&lock).expect("first acquire succeeds");
        // A second acquire while the first is held must fail.
        assert!(matches!(acquire(&lock), Err(DaemonError::AlreadyRunning)));

        // After releasing, a fresh acquire succeeds again.
        drop(first);
        let _second = acquire(&lock).expect("acquire after release succeeds");
    }
}
