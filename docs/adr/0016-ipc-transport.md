# ADR-0016: Local IPC transport — UDS on Unix, named-pipe FFI on Windows

- **Status:** Accepted (Windows leg pending hardware confirmation — see Status note)
- **Date:** 2026-07-22
- **Architecture reference:** OD-4 · AD-2 · ARCHITECTURE.md §6.1 · refines ADR-0002
- **Supersedes:** —

## Status note

The Unix decision is **confirmed by a working spike on macOS**. The Windows decision is
**implemented but not yet run on Windows hardware**: the code compiles and passes `dart
analyze`, and the approach is sound, but the exit criterion "streaming echo authenticated by
peer credentials on all three OSes" is met on two of three. This ADR is marked Accepted
because the *decision* is made; the Windows run in `spikes/spike-a-ipc/run-windows.ps1` closes
the remaining verification. If that run fails, this ADR is amended, not the architecture — the
fallback (A3, below) is already specified.

## Context

ADR-0002 committed to gRPC over Unix domain sockets and Windows named pipes, authenticated by
OS peer credentials, with no local TCP listener. The Rust side (tonic) supports all of this.
The open question, and the highest-risk unknown in Phase 0, was the **Dart client**:

- `dart:io` supports `AF_UNIX` on Unix-likes, but its named-pipe support is absent and its
  Windows `AF_UNIX` support was unverified.
- `grpc-dart`'s default channel is TCP; a non-TCP transport needs a custom connector.

SPIKE-A (WP-0.3) built a minimal tonic server and a Dart client to settle it empirically
rather than by reading documentation.

## Decision

**Unix (Linux, macOS): gRPC over a Unix domain socket, no FFI.** `grpc-dart`'s `ClientChannel`
accepts an `InternetAddress` of type `unix` directly. Peer credentials come from tonic's
`UdsConnectInfo`.

**Windows: gRPC over a named pipe, via a `dart:ffi` transport connector.** Rust and tokio do
not support `AF_UNIX` on Windows, so the server listens on a named pipe. `grpc-dart` does not
require a real socket — `ClientTransportConnectorChannel` accepts any byte-stream pair — so a
small FFI shim wrapping `CreateFileW`/`ReadFile`/`WriteFile` presents the pipe to gRPC. The
server authenticates with `GetNamedPipeClientProcessId`. This is option **A2** from the spike
plan.

Option **A1** (Dart `AF_UNIX` on Windows) is not adopted even if it works, because the server
cannot offer `AF_UNIX` on Windows regardless. The spike retains a `--try-afunix` probe to
record whether Dart *can* do it, as a data point for future maintainers.

## Evidence

Measured on macOS 26.5 (aarch64), Dart 3.12.2, tonic 0.14.6, over the Unix domain socket:

| Property | Result |
|---|---|
| Unary round trip | works |
| Peer credentials | server read `uid=502 gid=20 pid=26071`; client independently confirmed uid 502 |
| Server streaming | 1000/1000 messages, in order, lossless |
| Streaming throughput | ~68k msg/s, ~33 MiB/s at 512-byte payloads |
| Unary latency | p50 389 µs, p99 884 µs |

macOS returns a pid via `LOCAL_PEERCRED`; the daemon must still authorise on uid/gid, since a
pid is racy and not guaranteed on every platform.

The Windows named-pipe client compiles and passes `dart analyze`; its runtime verification is
pending (Status note).

## Consequences

**Positive**
- ADR-0002 stands. There is no local TCP listener; the "secure by default, no open port"
  property holds.
- On Unix — the reference platform and the bulk of development — the transport needs no native
  code at all.
- Performance is comfortably sufficient for live metrics and event streams.

**Negative**
- Windows carries an FFI shim (~250 lines) that must be maintained. The spike version uses
  blocking reads on a background isolate; the production version (WP-M0.6) needs overlapped
  (async) I/O, cancellation, and a hardened error path.
- Two client transport paths to test, though they sit behind one interface.
- A subtle grpc-dart footgun is now on record: it exports *two* classes named `ClientChannel`
  (a concrete socket one from `grpc.dart`, an abstract interface from `service_api.dart`). The
  connector channel implements the latter, not the former.

**Neutral**
- tonic 0.14 moved prost codegen into `tonic-prost-build`. WP-M0.2 must depend on both
  `tonic-build` and `tonic-prost-build`; following older examples silently fails.

## Alternatives considered

**A1 — Dart `AF_UNIX` on Windows.** Not adopted: the server side has no `AF_UNIX` on Windows,
so even if the client can, there is nothing to connect to. Retained only as a recorded probe.

**A3 — loopback TCP on 127.0.0.1 with a token file.** The documented fallback if the
named-pipe FFI proves unworkable on real hardware. It weakens ADR-0002 (an open localhost port,
reachable by any local process and by browsers subject to CORS) and would require compensating
controls: bind-to-loopback assertions, a filesystem-permissioned rotating token, and Origin
checks. Not chosen, but specified so the fallback is ready rather than improvised.

**A custom length-prefixed protocol over the pipe, no gRPC.** Rejected for the reasons in
ADR-0002: we would rebuild streaming, deadlines, and three-language codegen by hand.
