//! Daemon lifecycle — the startup and shutdown sequence (§4.2).
//!
//! Startup, in order: acquire the singleton lock so only one daemon runs; run
//! the deadman check for state a previous run left behind; open `config.db` and
//! migrate it (recovering from corruption); probe capabilities; build services;
//! start the IPC server; signal ready. Shutdown drains the IPC surface and lets
//! each module tear down. The firewall/kill-switch teardown policy and boot-time
//! cleanup unit land in WP-M0.7; the sequence and the deadman hook are here.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use nh_state::journal::JournalStore;
use nh_state::persist::Database;
use tonic::transport::Server;

use crate::ipc::{self, PrincipalResolver};
use crate::services::system::{SystemServiceImpl, SystemState};
use crate::{capabilities, singleton};

/// Configuration for a daemon instance.
pub struct DaemonConfig {
    /// Where the IPC socket / pipe is created.
    pub socket_path: PathBuf,
    /// Where `config.db` lives.
    pub db_path: PathBuf,
    /// The singleton lock file.
    pub lock_path: PathBuf,
    /// How connections are mapped to principals (injectable for tests).
    pub resolver: PrincipalResolver,
}

impl std::fmt::Debug for DaemonConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonConfig")
            .field("socket_path", &self.socket_path)
            .field("db_path", &self.db_path)
            .field("lock_path", &self.lock_path)
            .finish_non_exhaustive()
    }
}

impl DaemonConfig {
    /// A config rooted at `dir`, with the default principal resolver.
    #[must_use]
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            socket_path: dir.join("daemon.sock"),
            db_path: dir.join("config.db"),
            lock_path: dir.join("daemon.lock"),
            resolver: ipc::default_resolver(),
        }
    }
}

/// A running daemon. Held so the singleton lock stays acquired for its lifetime.
#[derive(Debug)]
pub struct Daemon {
    _lock: singleton::SingletonLock,
}

/// A daemon lifecycle error.
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    /// Another instance already holds the singleton lock.
    #[error("another daemon instance is already running")]
    AlreadyRunning,
    /// The database could not be opened (beyond what corruption recovery handles).
    #[error("database error: {0}")]
    Database(#[from] nh_state::persist::DbError),
    /// The intent journal could not be read (deadman check).
    #[error("journal error: {0}")]
    Journal(#[from] nh_state::journal::JournalError),
    /// An IO or transport error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The gRPC server failed.
    #[error("server error: {0}")]
    Server(String),
}

impl Daemon {
    /// Runs the daemon to completion, serving until `shutdown` resolves.
    ///
    /// # Errors
    /// Returns [`DaemonError`] if startup fails (lock held, unrecoverable DB
    /// error, socket bind failure) or the server errors while running.
    pub async fn run<F>(config: DaemonConfig, shutdown: F) -> Result<(), DaemonError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // 1. Singleton lock: exactly one daemon per machine (AD-1).
        let lock = singleton::acquire(&config.lock_path)?;
        // On Unix the lock guard's Drop releases the flock; the Windows lock is a
        // placeholder until WP-M0.7, so the binding looks inert there.
        #[cfg_attr(not(unix), allow(clippy::no_effect_underscore_binding))]
        let _daemon = Daemon { _lock: lock };
        tracing::info!("acquired singleton lock");

        // 2. Open config.db (recovers from corruption, F-5) and migrate.
        let (db, report) = Database::open(&config.db_path).await?;
        tracing::info!(?report, "opened config.db");

        // 3. Deadman check: state a previous run left pending. In M0 no module
        //    has written real OS intents yet, so this is the machinery; the
        //    per-module rollback dispatch lands with those modules (WP-M0.7+).
        let pending = db.journal().pending().await?;
        if pending.is_empty() {
            tracing::info!("deadman check: no pending intents");
        } else {
            tracing::warn!(
                count = pending.len(),
                "deadman check: pending intents from a previous run; rollback dispatch lands in WP-M0.7"
            );
        }

        // 4. Probe capabilities (provisional until WP-M0.8).
        let capabilities = capabilities::probe();
        tracing::info!(count = capabilities.len(), "probed capabilities");

        // 5. Build services over shared state.
        let system = SystemServiceImpl::new(Arc::new(SystemState { capabilities }));

        // Extract owned values so no borrow of `config` (or the sqlx pool)
        // crosses the long-lived serve await — a borrow held across the spawned
        // server future is not `Send`-general enough for tokio::spawn.
        let socket_path = config.socket_path.clone();
        let resolver = config.resolver.clone();

        // M0.6: SystemService does not read config.db, so the connection is
        // released before serving. Feature services (from M0.7) keep it open.
        drop(db);

        // 6. Start the IPC server and signal ready.
        serve(socket_path, resolver, system, shutdown).await
    }
}

/// Binds the local transport and serves the gRPC services with authorization,
/// until `shutdown` resolves. Takes owned values so nothing is borrowed across
/// the await.
// The Windows branch is a stub until WP-M0.7, so it has no await there.
#[cfg_attr(not(unix), allow(clippy::unused_async))]
async fn serve<F>(
    socket_path: PathBuf,
    resolver: PrincipalResolver,
    system: SystemServiceImpl,
    shutdown: F,
) -> Result<(), DaemonError>
where
    F: Future<Output = ()> + Send + 'static,
{
    use nh_proto::v1::system_service_server::SystemServiceServer;

    let layer = ipc::authz_layer(resolver);
    let builder = Server::builder()
        .layer(layer)
        .add_service(SystemServiceServer::new(system));

    #[cfg(unix)]
    {
        use tokio::net::UnixListener;
        use tokio_stream::wrappers::UnixListenerStream;

        // A stale socket from an unclean prior exit would block bind — the
        // miniature deadman for the socket itself (§4.2).
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }
        let listener = UnixListener::bind(&socket_path)?;
        set_socket_permissions(&socket_path)?;

        tracing::info!(socket = %socket_path.display(), "IPC listening");
        // The readiness marker the CLI and service manager wait on.
        println!("READY");

        builder
            .serve_with_incoming_shutdown(UnixListenerStream::new(listener), shutdown)
            .await
            .map_err(|e| DaemonError::Server(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(unix))]
    {
        // The Windows named-pipe transport lands with the service integration
        // (WP-M0.7); the pieces were proven in SPIKE-A.
        let _ = (builder, shutdown, socket_path);
        Err(DaemonError::Server(
            "Windows named-pipe IPC is implemented in WP-M0.7".to_owned(),
        ))
    }
}

#[cfg(unix)]
fn set_socket_permissions(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // 0660: owner and group only. WP-M0.7 narrows the group to a dedicated
    // `networkhub` group; the socket is never world-accessible (ADR-0002).
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))
}
