//! Binary entry point for `nh-cli`, the NetworkHub command-line client.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use nh_cli::{CliError, commands, connect, output};
use serde::Serialize;

/// The NetworkHub command-line client.
#[derive(Debug, Parser)]
#[command(name = "nh-cli", version, about = "NetworkHub command-line client")]
struct Args {
    /// Directory where the daemon's socket / pipe lives. Defaults to the
    /// per-user `daemon-sim` directory; point it at the installed daemon's
    /// directory (e.g. /var/run/networkhub) in production.
    #[arg(long, global = true)]
    dir: Option<PathBuf>,
    /// Emit machine-readable JSON instead of human text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Daemon version and per-module health.
    Status,
    /// The machine's capability matrix (§3.4).
    Capabilities,
    /// Run diagnostic checks against the daemon.
    Doctor,
    /// Debugging views.
    #[command(subcommand)]
    Debug(DebugCommand),
}

#[derive(Debug, Subcommand)]
enum DebugCommand {
    /// The daemon's background tasks (module health in M0).
    Tasks,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<ExitCode, CliError> {
    let dir = args.dir.unwrap_or_else(default_dir);
    let json = args.json;

    match args.command {
        Command::Status => {
            let mut client = connect(&dir).await?;
            let report = commands::status(&mut client).await?;
            emit(&report, json, || output::render_status(&report))?;
        }
        Command::Capabilities => {
            let mut client = connect(&dir).await?;
            let report = commands::capabilities(&mut client).await?;
            emit(&report, json, || output::render_capabilities(&report))?;
        }
        Command::Debug(DebugCommand::Tasks) => {
            let mut client = connect(&dir).await?;
            let report = commands::debug_tasks(&mut client).await?;
            emit(&report, json, || output::render_tasks(&report))?;
        }
        Command::Doctor => {
            let report = commands::doctor(&dir).await;
            emit(&report, json, || output::render_doctor(&report))?;
            if !report.ok {
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints `report` as JSON or via the human renderer.
fn emit<T: Serialize>(
    report: &T,
    json: bool,
    human: impl FnOnce() -> String,
) -> Result<(), CliError> {
    if json {
        output::print_json(report)
    } else {
        println!("{}", human());
        Ok(())
    }
}

/// The default daemon directory: the per-user `daemon-sim` location.
fn default_dir() -> PathBuf {
    std::env::temp_dir().join("networkhub-sim")
}
