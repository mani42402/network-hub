# SPIKE-A — Dart ↔ Rust IPC transport

**Question:** Can a Dart client speak gRPC to a Rust (tonic) server over a Unix domain socket
or a Windows named pipe, with streaming and OS peer-credential authentication — so that
ADR-0002's "no local TCP listener" decision holds?

**Answer:** Yes on Unix (proven on macOS). Yes on Windows by design, via a `dart:ffi`
named-pipe shim (implemented, pending a run on Windows hardware).

Outcome recorded in [ADR-0016](../../docs/adr/0016-ipc-transport.md). This is throwaway spike
code; it is deleted once WP-M0.2/WP-M0.6 build the real transport.

## Layout

```
proto/spike.proto              Minimal service: unary Echo + server-streaming Ticks
rust-server/                   tonic server: UDS (Unix) / named pipe (Windows)
dart-client/
  bin/client.dart              The test: unary, latency, streaming, peer-cred check
  lib/named_pipe_windows.dart  dart:ffi named-pipe transport connector (A2)
run.sh                         End-to-end on Linux/macOS
run-windows.ps1                End-to-end on Windows
```

## Running

**Linux / macOS** (no elevation):

```bash
./run.sh
```

**Windows** (no elevation), from an environment with Rust and the Dart SDK:

```powershell
powershell -ExecutionPolicy Bypass -File run-windows.ps1
# optional data point: does Dart support AF_UNIX on Windows at all?
powershell -ExecutionPolicy Bypass -File run-windows.ps1 -TryAfUnix
```

A run prints per-step OK/FAIL and ends with `SPIKE-A: PASS` or `FAIL`.

## What "pass" proves

1. Dart speaks gRPC over the non-TCP transport (no open localhost port).
2. The server learns the caller's identity from the OS (`uid`/`gid`/`pid` on Unix, client PID
   on Windows) — the authentication mechanism in ADR-0002.
3. Server streaming is lossless and ordered — required by the live metrics and event feeds.
4. Latency and throughput are adequate for a responsive UI.

## For the Windows verifier

This is the one piece of SPIKE-A that has not run on real hardware. If `run-windows.ps1`:

- **passes** → update ADR-0016's Status to fully Accepted and note the measured numbers;
- **fails** → capture the exact error (the client prints a stack trace), and fall back to
  option A3 in ADR-0016 (loopback TCP + token file). Do not improvise a different transport;
  the fallback is already specified.

Known limitation of the spike shim, to fix in the production version (WP-M0.6): it uses
blocking `ReadFile` on a background isolate. Production needs overlapped (async) I/O and
cancellation.
