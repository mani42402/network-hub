//! gRPC over a Windows named pipe (ADR-0002, ADR-0016).
//!
//! Rust and tokio do not support `AF_UNIX` on Windows, so the daemon listens on
//! a **named pipe** — a namespace object with OS-enforced access control, which
//! satisfies ADR-0002's "no local TCP listener". Each accepted connection is
//! wrapped as a [`PipeStream`] carrying the caller's process id (resolved with
//! `GetNamedPipeClientProcessId`, the Windows analogue of `SO_PEERCRED`), which
//! the daemon's principal resolver reads from the request's extensions.
//!
//! **Deferred (hardware-verified in M1):** a restrictive pipe ACL via an SDDL
//! security descriptor, narrowing connect rights to SYSTEM, Administrators, and
//! the `NetworkHub Operators` group. Until then the pipe carries its default
//! descriptor; the boundary is still enforced, because every RPC is authorized
//! by peer credential at the [`crate`]-consuming daemon's authorization layer,
//! and the M0 default role for a pipe client is read-only.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::server::Connected;

/// How many accepted-but-not-yet-served connections may queue before the
/// acceptor applies backpressure. A slow consumer must not make it spin.
const ACCEPT_BACKLOG: usize = 8;

/// The identity of the process on the client end of a pipe connection.
///
/// tonic inserts this into each request's extensions when serving a stream of
/// [`PipeStream`]s, so the principal resolver can read it with
/// `req.extensions().get::<PipeConnectInfo>()`.
#[derive(Debug, Clone, Copy)]
pub struct PipeConnectInfo {
    /// The client process id from `GetNamedPipeClientProcessId`, or `0` if it
    /// could not be resolved (treated as unauthenticated by the resolver).
    pub client_pid: u32,
}

/// An accepted named-pipe connection, adapted to tonic's transport server.
pub struct PipeStream {
    inner: NamedPipeServer,
    client_pid: u32,
}

impl std::fmt::Debug for PipeStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // NamedPipeServer is not Debug; expose only the identity we carry.
        f.debug_struct("PipeStream")
            .field("client_pid", &self.client_pid)
            .finish_non_exhaustive()
    }
}

impl Connected for PipeStream {
    type ConnectInfo = PipeConnectInfo;

    fn connect_info(&self) -> Self::ConnectInfo {
        PipeConnectInfo {
            client_pid: self.client_pid,
        }
    }
}

impl AsyncRead for PipeStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for PipeStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Resolves the client process id of a connected pipe — the Windows analogue of
/// `SO_PEERCRED`, and the basis of peer authentication. Returns `0` on failure.
// This crate is the designated FFI boundary (`unsafe_code = "deny"`, not
// "forbid"); the single unsafe call below is annotated with its own SAFETY note.
#[allow(unsafe_code)]
fn client_pid(server: &NamedPipeServer) -> u32 {
    use windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId;

    let handle = server.as_raw_handle();
    let mut pid: u32 = 0;
    // SAFETY: `handle` is a valid, open pipe handle owned by `server` and live
    // for the duration of this call; `&raw mut pid` is a valid, writable
    // out-pointer for exactly one `u32`. The call writes `pid` and returns
    // non-zero on success, which is the only case in which we read it.
    let ok = unsafe { GetNamedPipeClientProcessId(handle.cast(), &raw mut pid) };
    if ok == 0 { 0 } else { pid }
}

/// Starts accepting connections on `pipe_name` and returns a stream of them,
/// suitable for `Server::serve_with_incoming_shutdown`.
///
/// A background task owns the pipe instances: it creates one, waits for a
/// client, resolves the client's pid, hands the connection downstream, and
/// immediately creates the next instance so one is always listening. When the
/// returned stream is dropped (the server has stopped), the task's next send
/// fails and it exits, closing the idle listening instance.
///
/// Must be called from within a tokio runtime (it spawns the acceptor task).
///
/// # Panics
/// Does not panic; a failure to create the first (or any subsequent) pipe
/// instance is delivered as an error item on the returned stream.
#[must_use]
pub fn incoming(pipe_name: &str) -> ReceiverStream<io::Result<PipeStream>> {
    let pipe_name = pipe_name.to_owned();
    let (tx, rx) = tokio::sync::mpsc::channel::<io::Result<PipeStream>>(ACCEPT_BACKLOG);

    tokio::spawn(async move {
        // `first_pipe_instance(true)` fails if another process already owns this
        // pipe name — the transport-level singleton, complementing the daemon's
        // own singleton lock.
        let mut server = match ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
        {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Err(e)).await;
                return;
            }
        };

        loop {
            if let Err(e) = server.connect().await {
                let _ = tx.send(Err(e)).await;
                return;
            }
            let connected = server;
            let pid = client_pid(&connected);

            // Open the next instance before handing off the current one, so a
            // client connecting immediately after does not get ERROR_PIPE_BUSY.
            server = match ServerOptions::new().create(&pipe_name) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };

            let stream = PipeStream {
                inner: connected,
                client_pid: pid,
            };
            if tx.send(Ok(stream)).await.is_err() {
                return; // the consumer (server) is gone.
            }
        }
    });

    ReceiverStream::new(rx)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::ClientOptions;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn accepts_connection_resolves_pid_and_round_trips() {
        // A per-process pipe name so concurrent test binaries do not collide.
        let pipe = format!(r"\\.\pipe\networkhub-test-{}", std::process::id());
        let mut incoming = incoming(&pipe);

        // The first server instance is created inside the spawned acceptor, so a
        // client may briefly see ERROR_FILE_NOT_FOUND / ERROR_PIPE_BUSY. Retry.
        let mut client = loop {
            match ClientOptions::new().open(&pipe) {
                Ok(c) => break c,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        };

        let mut accepted = incoming
            .next()
            .await
            .expect("a connection is produced")
            .expect("no io error accepting");

        // Peer credential: the connection reports this very test process.
        assert_eq!(
            accepted.connect_info().client_pid,
            std::process::id(),
            "resolved client pid should be this process"
        );

        // A byte round-trip proves the wrapped stream does real IO both ways.
        client.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 4];
        accepted.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping", "server should read what the client wrote");

        accepted.write_all(b"pong").await.unwrap();
        let mut back = [0u8; 4];
        client.read_exact(&mut back).await.unwrap();
        assert_eq!(&back, b"pong", "client should read what the server wrote");
    }
}
