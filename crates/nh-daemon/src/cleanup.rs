//! Boot-time deadman cleanup (F-2, AD-5).
//!
//! This runs at boot **before** the daemon, as a systemd oneshot / launchd job /
//! Windows on-start step. Its whole reason to exist is the case the daemon's own
//! startup deadman cannot cover: the daemon crashes or fails to start, leaving
//! NetworkHub-owned OS state behind. If that state includes a kill-switch
//! firewall rule, the machine is stranded offline with no daemon to fix it. The
//! boot-cleanup reclaims that state independently, so a broken daemon can never
//! leave the machine wedged.
//!
//! For M0 the owned OS state is limited to runtime files (a stale socket) and the
//! intent journal; the reconcilers that create firewall/`sysctl` state land from
//! M1, and each registers its reclaim step here. The mechanism and its ordering
//! are established now so that safety net exists before anything can need it.

use std::path::{Path, PathBuf};

use nh_state::journal::{IntentState, JournalStore};
use nh_state::persist::Database;

/// What the cleanup found and did, for logging and the exit report.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    /// The daemon was already running, so nothing was reclaimed.
    pub daemon_live: bool,
    /// A stale socket file was removed.
    pub removed_stale_socket: bool,
    /// How many intents were left pending by a previous run (candidates for
    /// rollback once the owning modules exist).
    pub pending_intents: usize,
}

/// Paths the cleanup operates on.
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// The IPC socket path.
    pub socket_path: PathBuf,
    /// The `config.db` path.
    pub db_path: PathBuf,
}

impl CleanupConfig {
    /// A config rooted at `dir` (matching [`crate::lifecycle::DaemonConfig`]).
    #[must_use]
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            socket_path: dir.join("daemon.sock"),
            db_path: dir.join("config.db"),
        }
    }
}

/// Runs the boot-time cleanup.
///
/// # Errors
/// Returns a message if the database exists but cannot be read at all (beyond
/// the corruption recovery `Database::open` already performs). A missing
/// database is not an error — there is simply nothing to reconcile.
pub async fn run(config: &CleanupConfig) -> Result<CleanupReport, String> {
    let mut report = CleanupReport::default();

    // 1. If a daemon is already accepting connections, it owns its state — leave
    //    everything alone. (On a real boot this oneshot runs before the daemon,
    //    so this is the rare race guard, not the common path.)
    if daemon_is_live(&config.socket_path).await {
        report.daemon_live = true;
        return Ok(report);
    }

    // 2. A stale socket file from an unclean exit would block the daemon's bind.
    //    Remove it. (The daemon also does this on start; doing it here means a
    //    daemon that never starts still leaves a clean socket path.)
    if is_stale_socket(&config.socket_path) {
        let _ = std::fs::remove_file(&config.socket_path);
        report.removed_stale_socket = true;
    }

    // 3. Reconcile the intent journal. In M0 no module has written OS-affecting
    //    intents, so this reports the count; from M1 each pending intent is
    //    dispatched to its module's reclaim step (the registry below).
    if config.db_path.exists() {
        match Database::open(&config.db_path).await {
            Ok((db, _report)) => {
                let pending = db.journal().pending().await.map_err(|e| e.to_string())?;
                report.pending_intents = pending.len();
                // M1+: for each pending intent, look up its module in the reclaim
                // registry and revert to prior_value, then mark it reverted.
                for intent in &pending {
                    debug_assert_eq!(intent.state, IntentState::Pending);
                }
                db.close().await;
            }
            // Database::open already recovers from corruption; a hard error here
            // is unusual but must not wedge the boot — report and continue.
            Err(e) => return Err(format!("cleanup could not open config.db: {e}")),
        }
    }

    Ok(report)
}

/// Whether a daemon is accepting connections on the socket (versus a stale file).
// Genuinely async on Unix (the connect awaits); the Windows named-pipe client
// open is synchronous, so the `async` is unused only there.
#[cfg_attr(not(unix), allow(clippy::unused_async))]
async fn daemon_is_live(socket_path: &Path) -> bool {
    #[cfg(unix)]
    {
        tokio::net::UnixStream::connect(socket_path).await.is_ok()
    }
    #[cfg(not(unix))]
    {
        use tokio::net::windows::named_pipe::ClientOptions;

        // On Windows a daemon is "live" if its named pipe accepts (or is busy
        // serving) a client. ERROR_PIPE_BUSY (231) means instances exist but are
        // all in use — still a live daemon.
        const ERROR_PIPE_BUSY: i32 = 231;
        let pipe = crate::lifecycle::windows_pipe_name(socket_path);
        match ClientOptions::new().open(&pipe) {
            Ok(_) => true,
            Err(e) => e.raw_os_error() == Some(ERROR_PIPE_BUSY),
        }
    }
}

/// Whether the path is a socket file that nothing is listening on.
fn is_stale_socket(socket_path: &Path) -> bool {
    // Present on disk but not connectable (checked by the caller before this).
    socket_path.exists()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use nh_state::journal::Intent;

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
    async fn removes_a_stale_socket_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = CleanupConfig::in_dir(dir.path());
        // Create a plain file at the socket path (a stale leftover, nothing
        // listening).
        std::fs::write(&config.socket_path, b"").unwrap();

        let report = run(&config).await.unwrap();
        assert!(report.removed_stale_socket);
        assert!(!config.socket_path.exists(), "stale socket must be gone");
    }

    #[tokio::test]
    async fn reports_pending_intents_from_a_previous_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = CleanupConfig::in_dir(dir.path());

        // Seed config.db with a pending intent, as a crashed run would leave.
        {
            let (db, _) = Database::open(&config.db_path).await.unwrap();
            db.journal().append(intent("orphan")).await.unwrap();
            db.close().await;
        }

        let report = run(&config).await.unwrap();
        assert_eq!(report.pending_intents, 1);
        assert!(!report.daemon_live);
    }

    #[tokio::test]
    async fn missing_database_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = CleanupConfig::in_dir(dir.path());
        let report = run(&config).await.unwrap();
        assert_eq!(report, CleanupReport::default());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn skips_when_a_daemon_is_live() {
        use tokio::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let config = CleanupConfig::in_dir(dir.path());
        // Bind a real listener so the socket is live.
        let _listener = UnixListener::bind(&config.socket_path).unwrap();

        let report = run(&config).await.unwrap();
        assert!(report.daemon_live, "must not touch a live daemon's state");
        assert!(!report.removed_stale_socket);
    }
}
