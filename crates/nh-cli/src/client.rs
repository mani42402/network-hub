//! Connecting to the daemon over its local transport.
//!
//! The daemon serves gRPC over a Unix domain socket (`<dir>/daemon.sock`) or,
//! on Windows, a named pipe whose name is derived from that socket path. The CLI
//! cannot depend on the daemon crate, so the pipe-name derivation is duplicated
//! here as a convention both sides agree on.

use std::path::{Path, PathBuf};

use nh_proto::v1::system_service_client::SystemServiceClient;
use tonic::transport::{Channel, Endpoint, Uri};

use crate::CliError;

/// A connected `SystemService` client.
pub type Client = SystemServiceClient<Channel>;

/// The socket file name the daemon binds under its directory.
const SOCKET_FILE: &str = "daemon.sock";

/// Connects to the daemon whose transport lives under `dir`.
///
/// # Errors
/// Returns [`CliError::Connect`] if the socket/pipe cannot be reached.
pub async fn connect(dir: &Path) -> Result<Client, CliError> {
    let socket = dir.join(SOCKET_FILE);
    let channel = build_channel(socket).await?;
    Ok(SystemServiceClient::new(channel))
}

/// A dummy authority; the connector ignores it, but tonic requires a valid URI.
fn endpoint() -> Endpoint {
    Endpoint::try_from("http://[::1]:50051").expect("static endpoint uri is valid")
}

#[cfg(unix)]
async fn build_channel(socket: PathBuf) -> Result<Channel, CliError> {
    use tokio::net::UnixStream;

    endpoint()
        .connect_with_connector(tower::service_fn(move |_: Uri| {
            let socket = socket.clone();
            async move {
                let stream = UnixStream::connect(socket).await?;
                Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
            }
        }))
        .await
        .map_err(|e| CliError::Connect(e.to_string()))
}

#[cfg(windows)]
async fn build_channel(socket: PathBuf) -> Result<Channel, CliError> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let pipe = windows_pipe_name(&socket);
    endpoint()
        .connect_with_connector(tower::service_fn(move |_: Uri| {
            let pipe = pipe.clone();
            async move {
                let client = ClientOptions::new().open(&pipe)?;
                Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(client))
            }
        }))
        .await
        .map_err(|e| CliError::Connect(e.to_string()))
}

/// Mirrors `nh_daemon::lifecycle::windows_pipe_name`: the CLI cannot depend on
/// the daemon, so the two agree on this derivation by convention.
#[cfg(windows)]
fn windows_pipe_name(socket: &Path) -> String {
    let sanitized: String = socket
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!(r"\\.\pipe\networkhub-{sanitized}")
}
