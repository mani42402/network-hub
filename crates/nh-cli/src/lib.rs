//! Command-line client and the headless administration surface.
//!
//! **Status (M0 subset, WP-M0.11):** a gRPC client for the daemon's
//! `SystemService`, exposing `status`, `capabilities`, `doctor`, and
//! `debug tasks`, each with `--json` for scripting. It connects over the same
//! local transport the daemon serves — a Unix domain socket, or a named pipe on
//! Windows — and never depends on the daemon crate (it speaks only the proto
//! contract). Interface and throughput views arrive with the UI (WP-M0.12).

pub mod client;
pub mod commands;
pub mod output;

pub use client::{Client, connect};

/// A CLI-level failure.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// The daemon could not be reached at the configured location.
    #[error("cannot reach the daemon: {0}")]
    Connect(String),
    /// The daemon rejected or failed a call.
    #[error("daemon call failed: {0}")]
    Rpc(#[from] tonic::Status),
    /// Serializing the JSON output failed.
    #[error("output error: {0}")]
    Output(#[from] serde_json::Error),
}
