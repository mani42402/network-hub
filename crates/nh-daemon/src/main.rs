//! Binary entry point for `nh-daemon`, the privileged system service.

use std::path::PathBuf;

use clap::Parser;
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
    #[arg(long, default_value = "/var/run/networkhub")]
    dir: PathBuf,
    /// Emit machine-readable JSON logs (production). Human logs otherwise.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Args::parse();

    if let Err(e) = nh_daemon::logging::init(args.json) {
        eprintln!("failed to initialise logging: {e}");
        return std::process::ExitCode::FAILURE;
    }

    let config = DaemonConfig::in_dir(&args.dir);

    // Shut down cleanly on Ctrl-C / SIGTERM.
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };

    match Daemon::run(config, shutdown).await {
        Ok(()) => {
            tracing::info!("daemon stopped");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!("daemon failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
