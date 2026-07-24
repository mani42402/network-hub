//! Binary entry point for `nh-daemon`, the privileged system service.
//!
//! Subcommands:
//! - `run` (default): start the daemon and serve until signalled.
//! - `cleanup`: the boot-time deadman (F-2) — reclaim state a previous run left
//!   behind, then exit. A systemd oneshot / launchd job invokes this *before*
//!   `run`, so a broken daemon can never strand the machine.
//! - `service` / `install` / `uninstall` (Windows only): run under the Service
//!   Control Manager, and register/remove the service. On Windows there is no
//!   separate boot unit, so `service` performs the on-start orphan check itself
//!   before serving.

use std::future::Future;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use nh_daemon::cleanup::CleanupConfig;
use nh_daemon::lifecycle::{Daemon, DaemonConfig};

/// The NetworkHub daemon.
#[derive(Debug, Parser)]
#[command(
    name = "nh-daemon",
    version,
    about = "NetworkHub privileged system service"
)]
struct Args {
    /// Directory for the socket, database, and lock file.
    #[arg(long, default_value = "/var/run/networkhub", global = true)]
    dir: PathBuf,
    /// Emit machine-readable JSON logs (production). Human logs otherwise.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the daemon (the default).
    Run,
    /// Reclaim state left by a previous run, then exit (the boot-time deadman).
    Cleanup,
    /// Run under the Windows Service Control Manager (invoked by the SCM).
    #[cfg(windows)]
    Service,
    /// Register the Windows service with the SCM (requires elevation).
    #[cfg(windows)]
    Install,
    /// Remove the Windows service (requires elevation).
    #[cfg(windows)]
    Uninstall,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if let Err(e) = nh_daemon::logging::init(args.json) {
        eprintln!("failed to initialise logging: {e}");
        return ExitCode::FAILURE;
    }

    match args.command.unwrap_or(Command::Run) {
        Command::Run => block_on(run(args.dir)),
        Command::Cleanup => block_on(cleanup(args.dir)),
        #[cfg(windows)]
        Command::Service => service(args.dir),
        #[cfg(windows)]
        Command::Install => install(),
        #[cfg(windows)]
        Command::Uninstall => uninstall(),
    }
}

/// Builds a multi-thread runtime and drives `fut` to completion on it. The
/// daemon is awaited on the main task (never `spawn`ed), so its non-`Send`
/// internals (the sqlx pool) never cross a thread boundary.
fn block_on<F: Future<Output = ExitCode>>(fut: F) -> ExitCode {
    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(fut),
        Err(e) => {
            eprintln!("failed to start async runtime: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(dir: PathBuf) -> ExitCode {
    let config = DaemonConfig::in_dir(&dir);
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };
    match Daemon::run(config, shutdown).await {
        Ok(()) => {
            tracing::info!("daemon stopped");
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!("daemon failed: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn cleanup(dir: PathBuf) -> ExitCode {
    let config = CleanupConfig::in_dir(&dir);
    match nh_daemon::cleanup::run(&config).await {
        Ok(report) => {
            tracing::info!(?report, "boot-time cleanup complete");
            ExitCode::SUCCESS
        }
        Err(e) => {
            // A cleanup failure must not wedge the boot: log it and let the boot
            // proceed. The daemon's own startup deadman is the second line.
            tracing::error!("boot-time cleanup error (continuing): {e}");
            ExitCode::SUCCESS
        }
    }
}

/// Runs the daemon under the Windows SCM. The SCM invokes the body on its own
/// thread with a stop token; we build a runtime there and run the daemon with
/// the stop token as its shutdown future, after the on-start orphan check (F-2).
#[cfg(windows)]
fn service(dir: PathBuf) -> ExitCode {
    let result = pal_windows::service::run(move |stop| {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("failed to start async runtime: {e}");
                return;
            }
        };
        rt.block_on(async move {
            // On-start orphan check (F-2): reclaim state a previous run left,
            // before serving. A cleanup failure must not wedge startup.
            let cleanup_cfg = CleanupConfig::in_dir(&dir);
            match nh_daemon::cleanup::run(&cleanup_cfg).await {
                Ok(report) => tracing::info!(?report, "on-start cleanup complete"),
                Err(e) => tracing::error!("on-start cleanup error (continuing): {e}"),
            }

            let config = DaemonConfig::in_dir(&dir);
            let shutdown = async move {
                stop.wait().await;
                tracing::info!("SCM stop received");
            };
            if let Err(e) = Daemon::run(config, shutdown).await {
                tracing::error!("daemon failed: {e}");
            }
        });
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("service dispatch failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Registers the Windows service with the SCM.
#[cfg(windows)]
fn install() -> ExitCode {
    match pal_windows::service::install(&["service"]) {
        Ok(()) => {
            println!(
                "installed service {} (restart-on-failure)",
                pal_windows::service::SERVICE_NAME
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("install failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Removes the Windows service from the SCM.
#[cfg(windows)]
fn uninstall() -> ExitCode {
    match pal_windows::service::uninstall() {
        Ok(()) => {
            println!("uninstalled service {}", pal_windows::service::SERVICE_NAME);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("uninstall failed: {e}");
            ExitCode::FAILURE
        }
    }
}
