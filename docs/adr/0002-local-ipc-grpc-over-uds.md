# ADR-0002: Local IPC over Unix domain sockets / named pipes, authenticated by peer credentials

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-2 · ARCHITECTURE.md §6.1
- **Supersedes:** —

## Context

Given ADR-0001, clients must reach the daemon. The obvious choice — an HTTP server on
localhost — opens a listening TCP socket that any local process can connect to, requires us
to invent our own authentication, and is reachable from browsers (and therefore from any web
page the user visits, subject to CORS). For a service that controls the firewall, that is an
unacceptable default.

Unix domain sockets and Windows named pipes are filesystem/namespace objects with OS-enforced
permissions, and both platforms can tell us the *authenticated identity of the connecting
process* without us implementing anything.

## Decision

We will use **gRPC over Unix domain sockets (Linux, macOS) and named pipes (Windows)** for
all local client-to-daemon communication, authenticated by **OS peer credentials**
(`SO_PEERCRED`, `LOCAL_PEERCRED`, `GetNamedPipeClientProcessId`) mapped to a NetworkHub
principal, with authorisation enforced per-RPC. **We will not listen on TCP for local IPC.**

gRPC is chosen over a bespoke protocol because it is streaming-native (live metrics and event
feeds are core to the product), strongly typed, and generates clients for Rust, Dart, and
TypeScript from one contract (ADR-0013).

## Consequences

**Positive**
- Zero listening network sockets in the default configuration. "Secure by default" is a
  property of the architecture, not a setting.
- Authentication is delegated to the operating system, which is better at it than we are.
- Browsers cannot reach the daemon, eliminating an entire class of cross-origin attack.
- Streaming RPCs match the domain: metrics, device events, and connection state are all
  naturally push-based.

**Negative**
- Dart's `dart:io` has no named-pipe support, and its Windows `AF_UNIX` support is unverified.
  This is the single largest technical risk in Phase 0 and is why SPIKE-A (WP-0.3) is
  time-boxed in week one.
- gRPC adds `protoc`/`buf` to the toolchain and generated code to the repository.

**Neutral**
- Remote access becomes a deliberate, separate component rather than an accident of binding
  to `0.0.0.0` (ADR-0003).

## Alternatives considered

**HTTP/JSON on 127.0.0.1 with a token file** — rejected as the default for the reasons above.
Retained as the documented fallback (option A3 in SPIKE-A) *only* if Dart cannot speak either
`AF_UNIX` or named pipes on Windows, and then with compensating controls: loopback-only bind
assertions, token rotation, and Origin checks.

**Custom length-prefixed binary protocol over the same sockets** — rejected: we would rebuild
streaming, flow control, deadlines, and code generation for three client languages, and get
them subtly wrong.

**D-Bus (Linux) with per-platform equivalents** — rejected: three different IPC models means
three client implementations per language, and no shared contract.
