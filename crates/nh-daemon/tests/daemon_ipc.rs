//! End-to-end IPC tests: a real daemon over a Unix socket, a real gRPC client,
//! and the authorization layer enforcing over the wire (WP-M0.6 exit criteria).

#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use nh_daemon::ipc::fixed_resolver;
use nh_daemon::lifecycle::{Daemon, DaemonConfig};
use nh_daemon::principal::Role;
use nh_proto::v1::GetVersionRequest;
use nh_proto::v1::system_service_client::SystemServiceClient;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};

/// Starts a daemon in a temp dir with the given resolver, returning its socket
/// path and a shutdown trigger. Waits until the socket is accepting.
///
/// The daemon runs on its own thread with a current-thread runtime — exactly how
/// it runs in production (a `block_on` in `main`), which is also why its future
/// need not be `Send`. Running it here via `tokio::spawn` would (correctly)
/// require `Send`, which the sqlx migration transaction's borrowed connection
/// does not satisfy.
async fn start_daemon(
    resolver: nh_daemon::ipc::PrincipalResolver,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    tokio::sync::oneshot::Sender<()>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = DaemonConfig::in_dir(dir.path());
    config.resolver = resolver;
    let socket = config.socket_path.clone();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let shutdown = async {
                let _ = rx.await;
            };
            let _ = Daemon::run(config, shutdown).await;
        });
    });

    // Wait for the socket to accept connections.
    for _ in 0..100 {
        if UnixStream::connect(&socket).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    (dir, socket, tx)
}

/// Builds a gRPC client channel over the daemon's Unix socket.
async fn connect(socket: &std::path::Path) -> SystemServiceClient<tonic::transport::Channel> {
    let socket = socket.to_path_buf();
    // The URI is ignored for a UDS connector, but tonic requires a valid one.
    let channel = Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(tower::service_fn(move |_: Uri| {
            let socket = socket.clone();
            async move {
                let stream = UnixStream::connect(socket).await?;
                Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
            }
        }))
        .await
        .unwrap();
    SystemServiceClient::new(channel)
}

#[tokio::test]
async fn admin_can_call_system_service() {
    // An admin principal calling GetVersion (requires SystemRead) succeeds, and
    // the response carries real version data.
    let (_dir, socket, shutdown) = start_daemon(fixed_resolver(&[Role::Admin], "test-admin")).await;
    let mut client = connect(&socket).await;

    let resp = client
        .get_version(GetVersionRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.api_version, "v1");
    assert!(!resp.daemon_version.is_empty());
    assert_eq!(resp.os, std::env::consts::OS);

    let _ = shutdown.send(());
}

#[tokio::test]
async fn viewer_can_read_system() {
    // Viewer holds SystemRead, so GetVersion is allowed.
    let (_dir, socket, shutdown) =
        start_daemon(fixed_resolver(&[Role::Viewer], "test-viewer")).await;
    let mut client = connect(&socket).await;
    assert!(client.get_version(GetVersionRequest {}).await.is_ok());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn unauthenticated_principal_is_rejected() {
    // A principal with NO roles is denied even SystemRead — the layer rejects it
    // over the wire with PermissionDenied. This is the "unauthorized user is
    // rejected" exit criterion, end to end.
    let (_dir, socket, shutdown) = start_daemon(fixed_resolver(&[], "nobody")).await;
    let mut client = connect(&socket).await;

    let err = client.get_version(GetVersionRequest {}).await.unwrap_err();
    assert_eq!(
        err.code(),
        tonic::Code::PermissionDenied,
        "expected PermissionDenied, got {err:?}"
    );

    let _ = shutdown.send(());
}

#[tokio::test]
async fn capabilities_reports_os() {
    let (_dir, socket, shutdown) = start_daemon(fixed_resolver(&[Role::Viewer], "v")).await;
    let mut client = connect(&socket).await;
    let resp = client
        .get_capabilities(nh_proto::v1::GetCapabilitiesRequest {})
        .await
        .unwrap()
        .into_inner();
    assert!(resp.capabilities.contains_key("system.os"));
    let _ = shutdown.send(());
}
