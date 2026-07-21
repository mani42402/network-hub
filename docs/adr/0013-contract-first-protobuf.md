# ADR-0013: Contract-first APIs with protobuf as the single source of truth

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-13 · ARCHITECTURE.md §6.3
- **Supersedes:** —

## Context

The daemon will eventually serve at least five client surfaces: the Flutter desktop app, the
CLI, the REST gateway, a web dashboard, and mobile controllers — in three languages. Any
hand-maintained interface definition across that many consumers drifts, and the drift is
discovered by users rather than by the compiler.

The daemon and the UI can also be at different versions on the same machine, because
installers update them at different moments and package managers do not guarantee atomicity.

## Decision

**Protobuf definitions in `proto/` are the single source of truth.** Rust (tonic), Dart, and
TypeScript clients are generated from them; generated Dart and TypeScript are committed with a
CI check that regeneration produces no diff. `buf` enforces backward compatibility in CI.

Two further requirements ride on the same contract:

- **Per-RPC permission annotations** as custom proto options, read by the authorisation
  interceptor. The permission model and its documentation are generated from one place, so
  enforcement and documentation cannot diverge, and a test asserts that no RPC lacks an
  annotation.
- **Version negotiation**: the daemon publishes `min_client_version`; a mismatched client sees
  an explicit "update the app" screen instead of undefined behaviour.

Error responses carry a canonical code, a machine-readable `reason`, a human `detail`, and an
optional `remediation` string that the UI renders directly.

## Consequences

**Positive**
- Contract changes break the build, not the user.
- A new client platform costs a code-generation step, not a reimplementation.
- Authorisation coverage is provable rather than reviewed by eye.
- Streaming contracts (snapshot-then-delta) are expressed once and honoured by every client.

**Negative**
- `protoc` and `buf` join the required toolchain on all three platforms.
- Generated code in the repository creates review noise and merge conflicts.
- Protobuf's evolution rules (additive-only within a major version) constrain API design;
  genuine breaking changes require a parallel `v2` package.

**Neutral**
- Forces API design to happen before implementation, which slows the first feature and speeds
  every subsequent one.

## Alternatives considered

**OpenAPI/JSON-first** — rejected: weaker streaming story, and code generation quality for
Dart and Rust is markedly worse.

**Hand-written clients per platform** — rejected: guaranteed drift across five surfaces, and
no mechanical way to catch it.

**GraphQL** — rejected: solves a query-flexibility problem we do not have, and adds a
non-trivial server surface to a privileged process.
