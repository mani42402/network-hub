//! End-to-end CLI tests: the real command functions against a real daemon over
//! a Unix socket (WP-M0.11 exit criteria — parseable output, same contracts).

#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use nh_daemon::ipc::fixed_resolver;
use nh_daemon::lifecycle::{Daemon, DaemonConfig};
use nh_daemon::principal::Role;
use tokio::net::UnixStream;

/// Starts a daemon in a temp dir (admin resolver, as `daemon-sim` does) and
/// waits until its socket accepts. Runs it on its own current-thread runtime,
/// exactly as production does (a `block_on` in `main`).
async fn start_daemon() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    tokio::sync::oneshot::Sender<()>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = DaemonConfig::in_dir(dir.path());
    config.resolver = fixed_resolver(&[Role::Admin], "test-admin");
    let socket = config.socket_path.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let _ = Daemon::run(config, async move {
                let _ = rx.await;
            })
            .await;
        });
    });

    for _ in 0..100 {
        if UnixStream::connect(&socket).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    (dir, socket, tx)
}

#[tokio::test]
async fn status_reports_version_and_serving_health() {
    let (dir, _socket, shutdown) = start_daemon().await;
    let mut client = nh_cli::connect(dir.path()).await.unwrap();

    let report = nh_cli::commands::status(&mut client).await.unwrap();
    assert_eq!(report.api_version, "v1");
    assert!(!report.daemon_version.is_empty());
    assert_eq!(report.overall, "serving");
    assert!(report.modules.iter().any(|m| m.name == "ipc"));

    // Output is stable, parseable JSON.
    let json = serde_json::to_string(&report).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["api_version"], "v1");

    let _ = shutdown.send(());
}

#[tokio::test]
async fn capabilities_returns_the_full_matrix() {
    let (dir, _socket, shutdown) = start_daemon().await;
    let mut client = nh_cli::connect(dir.path()).await.unwrap();

    let report = nh_cli::commands::capabilities(&mut client).await.unwrap();
    assert!(report.capabilities.contains_key("system.os"));
    assert!(report.capabilities.contains_key("vpn.wireguard"));
    // Every entry has a non-empty, named status and reason.
    for (key, cap) in &report.capabilities {
        assert!(!cap.status.is_empty(), "{key} has empty status");
        assert_ne!(cap.status, "unspecified", "{key} status not mapped");
        assert!(!cap.reason.trim().is_empty(), "{key} has empty reason");
    }

    let _ = shutdown.send(());
}

#[tokio::test]
async fn doctor_passes_against_a_live_daemon() {
    let (dir, _socket, shutdown) = start_daemon().await;
    let report = nh_cli::commands::doctor(dir.path()).await;
    assert!(report.ok, "doctor should pass, got: {:?}", report.checks);
    assert!(report.checks.iter().all(|c| c.ok));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn doctor_reports_unreachable_daemon() {
    // A directory with no daemon running: doctor fails the reachability check
    // rather than erroring out.
    let dir = tempfile::tempdir().unwrap();
    let report = nh_cli::commands::doctor(dir.path()).await;
    assert!(!report.ok);
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.name == "daemon reachable" && !c.ok)
    );
}

#[tokio::test]
async fn debug_tasks_lists_modules() {
    let (dir, _socket, shutdown) = start_daemon().await;
    let mut client = nh_cli::connect(dir.path()).await.unwrap();
    let report = nh_cli::commands::debug_tasks(&mut client).await.unwrap();
    assert!(report.modules.iter().any(|m| m.name == "ipc"));
    let _ = shutdown.send(());
}
