//! `daemon-sim` — the daemon runnable **unprivileged**, for UI and CLI
//! development without root (§14.1).
//!
//! It runs the exact same lifecycle and serves the exact same gRPC contracts as
//! the real `nh-daemon`, but in a user-writable directory and treating the
//! connecting user as an administrator (it is that user's own private
//! simulation). This is how the Flutter UI and the CLI are developed and how UI
//! CI runs — no root, no real network changes.
//!
//! In M0 the only PAL-derived surface is the capability matrix, which the real
//! probe reports honestly even unprivileged (it marks `system.privileged` as
//! degraded). When the daemon starts serving interfaces and throughput
//! (WP-M0.12), `daemon-sim` wires those to `pal-mock` so the UI sees realistic
//! devices and traffic without hardware.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use nh_daemon::ipc::fixed_resolver;
use nh_daemon::lifecycle::{Daemon, DaemonConfig};
use nh_daemon::principal::Role;

/// The NetworkHub daemon in unprivileged simulation mode.
#[derive(Debug, Parser)]
#[command(
    name = "daemon-sim",
    version,
    about = "NetworkHub daemon (unprivileged simulation)"
)]
struct Args {
    /// Directory for the socket, database, and lock (must be user-writable).
    /// Defaults to a per-user temp directory.
    #[arg(long)]
    dir: Option<PathBuf>,
    /// Emit machine-readable JSON logs.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    if let Err(e) = nh_daemon::logging::init(args.json) {
        eprintln!("failed to initialise logging: {e}");
        return ExitCode::FAILURE;
    }

    let dir = args.dir.unwrap_or_else(default_sim_dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::error!("cannot create sim directory {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }

    let mut config = DaemonConfig::in_dir(&dir);
    // A local simulation: the connecting user is its owner, so grant admin. The
    // real daemon maps OS peer credentials to roles instead (ADR-0002).
    config.resolver = fixed_resolver(&[Role::Admin], "daemon-sim");

    tracing::info!(
        dir = %dir.display(),
        "daemon-sim starting (unprivileged); connect with: nh-cli --dir {} status",
        dir.display()
    );

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };
    match Daemon::run(config, shutdown).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("daemon-sim failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The default per-user simulation directory.
fn default_sim_dir() -> PathBuf {
    std::env::temp_dir().join("networkhub-sim")
}
