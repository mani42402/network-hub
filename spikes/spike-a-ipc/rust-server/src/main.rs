//! SPIKE-A server: a tonic gRPC service over a Unix domain socket (Unix) or a
//! named pipe (Windows), reporting the OS-authenticated identity of the caller.
//!
//! This exists to answer one question before ADR-0002 is load-bearing: can a
//! Dart client speak gRPC over a transport that is not TCP, on all three
//! platforms, including streaming and peer-credential authentication?
//!
//! Throwaway code. Clarity over polish; it is deleted once ADR-0016 is written.

use std::path::Path;

use tonic::{Request, Response, Status};

pub mod spike {
    #![allow(clippy::pedantic, clippy::all, missing_docs)]
    tonic::include_proto!("spike.v1");
}

use spike::spike_service_server::{SpikeService, SpikeServiceServer};
use spike::{EchoRequest, EchoResponse, PeerIdentity, Tick, TicksRequest};

#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/networkhub-spike-a.sock";

#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\networkhub-spike-a";

#[derive(Default)]
struct Spike;

#[tonic::async_trait]
impl SpikeService for Spike {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        let peer = peer_identity(&request);
        let message = request.into_inner().message;
        eprintln!("[server] echo: {message:?} from {peer:?}");
        Ok(Response::new(EchoResponse {
            message: format!("echo: {message}"),
            peer: Some(peer),
        }))
    }

    type TicksStream = tokio_stream::wrappers::ReceiverStream<Result<Tick, Status>>;

    async fn ticks(
        &self,
        request: Request<TicksRequest>,
    ) -> Result<Response<Self::TicksStream>, Status> {
        let req = request.into_inner();
        let count = req.count.max(1);
        let payload = vec![0xABu8; req.payload_bytes as usize];

        // A small buffer on purpose: it exercises backpressure, which is the
        // property that matters for the real event bus (a stalled UI must never
        // be able to block the engine).
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        tokio::spawn(async move {
            for seq in 0..u64::from(count) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64;

                let tick = Tick {
                    seq,
                    unix_micros: now,
                    payload: payload.clone(),
                };
                if tx.send(Ok(tick)).await.is_err() {
                    eprintln!("[server] client dropped the stream at seq {seq}");
                    return;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}

/// Extracts the caller's identity from the OS, never from the request body.
#[cfg(unix)]
fn peer_identity<T>(request: &Request<T>) -> PeerIdentity {
    use tonic::transport::server::UdsConnectInfo;

    let Some(info) = request.extensions().get::<UdsConnectInfo>() else {
        return PeerIdentity {
            transport: "uds".into(),
            notes: "no UdsConnectInfo in extensions".into(),
            ..Default::default()
        };
    };

    match info.peer_cred {
        Some(cred) => PeerIdentity {
            uid: cred.uid(),
            gid: cred.gid(),
            // macOS does not return a pid via LOCAL_PEERCRED; Linux does via
            // SO_PEERCRED. The daemon must therefore treat pid as advisory and
            // authorise on uid/gid.
            pid: cred.pid().unwrap_or(-1).into(),
            transport: "uds".into(),
            notes: if cred.pid().is_none() {
                "pid unavailable on this platform".into()
            } else {
                String::new()
            },
        },
        None => PeerIdentity {
            transport: "uds".into(),
            notes: "peer credentials unavailable".into(),
            ..Default::default()
        },
    }
}

#[cfg(windows)]
fn peer_identity<T>(_request: &Request<T>) -> PeerIdentity {
    // Filled in by the named-pipe path below, which resolves the client PID
    // with GetNamedPipeClientProcessId at accept time.
    PeerIdentity {
        transport: "named-pipe".into(),
        notes: "see windows_pipe module".into(),
        ..Default::default()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = SpikeServiceServer::new(Spike);

    #[cfg(unix)]
    {
        use tokio::net::UnixListener;
        use tokio_stream::wrappers::UnixListenerStream;

        // A stale socket file from a killed run would make bind fail. This is a
        // miniature version of the deadman problem the daemon has for real
        // (ARCHITECTURE.md §11): clean up what a previous instance left behind.
        if Path::new(SOCKET_PATH).exists() {
            std::fs::remove_file(SOCKET_PATH)?;
            eprintln!("[server] removed stale socket at {SOCKET_PATH}");
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;

        // 0660: owner and group only. The real daemon narrows this further with
        // a dedicated group (ADR-0002); world-writable would defeat the point
        // of using a filesystem socket at all.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o660))?;
        }

        eprintln!("[server] listening on unix:{SOCKET_PATH}");
        println!("READY");

        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await?;
    }

    #[cfg(windows)]
    {
        windows_pipe::serve(service).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Windows named-pipe transport.
//
// Rust and tokio do not support AF_UNIX on Windows, so named pipes are the
// server-side answer. The open question this spike exists to settle is on the
// *client* side: dart:io cannot open a named pipe, so the Dart client needs an
// FFI shim (see ../dart-client/lib/named_pipe_windows.dart).
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod windows_pipe {
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use tonic::transport::server::Connected;

    /// Wrapper so tonic can attach connection info to a named pipe.
    pub struct PipeStream {
        inner: NamedPipeServer,
        client_pid: u32,
    }

    #[derive(Clone)]
    pub struct PipeConnectInfo {
        pub client_pid: u32,
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

    /// Resolves the PID of the process on the other end of the pipe. This is
    /// the Windows equivalent of SO_PEERCRED and the basis of authentication.
    fn client_pid(server: &NamedPipeServer) -> u32 {
        use std::os::windows::io::AsRawHandle;
        let mut pid: u32 = 0;
        // SAFETY: the handle is valid for the lifetime of `server`, and `pid`
        // is a valid out-pointer for a single u32.
        let ok = unsafe {
            windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(
                server.as_raw_handle() as isize,
                &raw mut pid,
            )
        };
        if ok == 0 { 0 } else { pid }
    }

    pub async fn serve<S>(service: S) -> Result<(), Box<dyn std::error::Error>>
    where
        S: tower::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<tonic::body::Body>,
                Error = std::convert::Infallible,
            > + tonic::server::NamedService
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Send + 'static,
    {
        let pipe_name = super::PIPE_NAME;
        eprintln!("[server] listening on {pipe_name}");
        println!("READY");

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<PipeStream, io::Error>>(8);

        tokio::spawn(async move {
            let mut server = match ServerOptions::new()
                .first_pipe_instance(true)
                .create(pipe_name)
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

                server = match ServerOptions::new().create(pipe_name) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(e)).await;
                        return;
                    }
                };

                if tx
                    .send(Ok(PipeStream {
                        inner: connected,
                        client_pid: pid,
                    }))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });

        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(tokio_stream::wrappers::ReceiverStream::new(rx))
            .await?;
        Ok(())
    }
}
