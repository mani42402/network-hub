//! The singleton lock — exactly one daemon per machine (AD-1, §4.1).
//!
//! A second daemon starting while one runs must fail fast with a clear message,
//! not race the first over the socket and the database. On Unix an advisory
//! `flock` on a lock file gives that. On Windows the equivalent is opening the
//! lock file with **no sharing** (`share_mode(0)`): a second open fails with a
//! sharing violation. Both hold the lock for the process's lifetime and release
//! it automatically when the process dies, so a crash never leaves a stale lock
//! (unlike a PID file). Neither path needs `unsafe`.

use std::path::Path;

use crate::lifecycle::DaemonError;

/// Holds the singleton lock. Dropping it releases the lock.
#[derive(Debug)]
pub(crate) struct SingletonLock {
    #[cfg(unix)]
    _flock: nix::fcntl::Flock<std::fs::File>,
    // On Windows the held, no-sharing file handle *is* the lock; closing it (on
    // drop or process exit) releases it.
    #[cfg(not(unix))]
    _file: std::fs::File,
}

/// Acquires the singleton lock, or returns [`DaemonError::AlreadyRunning`] if
/// another instance holds it.
///
/// # Errors
/// Returns [`DaemonError::Io`] if the lock file cannot be created, or
/// [`DaemonError::AlreadyRunning`] if the lock is already held.
pub(crate) fn acquire(lock_path: &Path) -> Result<SingletonLock, DaemonError> {
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use nix::fcntl::{Flock, FlockArg};

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
        use std::os::windows::fs::OpenOptionsExt;

        // share_mode(0): open for exclusive use. A second daemon's open fails
        // with ERROR_SHARING_VIOLATION (32) — that, specifically, means "already
        // running"; any other error is a real IO failure and is surfaced as one.
        const ERROR_SHARING_VIOLATION: i32 = 32;
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .share_mode(0)
            .open(lock_path)
        {
            Ok(file) => Ok(SingletonLock { _file: file }),
            Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => {
                Err(DaemonError::AlreadyRunning)
            }
            Err(e) => Err(DaemonError::Io(e)),
        }
    }
}

#[cfg(test)]
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
