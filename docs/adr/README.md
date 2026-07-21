# Architecture Decision Records

Each ADR records one decision, the forces behind it, and what it costs us. They are
**immutable once accepted**: to change a decision, write a new ADR that supersedes the old
one and update the old one's Status line. The value of this directory is the record of why we
believed something at the time — rewriting history destroys it.

Start from [`0000-template.md`](0000-template.md).

## When an ADR is required

Per [CONTRIBUTING.md](../../CONTRIBUTING.md), before code that changes:

- the privilege boundary — what runs as root, and how authorisation crosses the line
- wire contracts — protobuf, REST, plugin interfaces
- storage schema, migrations, or where secrets live
- cryptography — primitives, key handling, signing
- licensing, or how a third-party component is integrated

## Index

ADR-0001 through ADR-0015 are backfilled from decisions AD-1..AD-15 in
[ARCHITECTURE.md §1](../architecture/ARCHITECTURE.md). They were made during architecture
design and recorded here so they can be challenged and superseded like any other decision.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-split-privilege-architecture.md) | Split-privilege architecture — privileged daemon, unprivileged clients | Accepted |
| [0002](0002-local-ipc-grpc-over-uds.md) | Local IPC over Unix sockets / named pipes, peer-credential auth | Accepted |
| [0003](0003-optional-remote-gateway.md) | Remote access is a separate, opt-in gateway process | Accepted |
| [0004](0004-desired-state-reconciliation.md) | Desired-state reconciliation, not imperative mutation | Accepted |
| [0005](0005-fail-open-safety-invariant.md) | Fail-open safety invariant — never strand the user offline | Accepted |
| [0006](0006-capability-driven-features.md) | Capability-driven feature model | Accepted |
| [0007](0007-platform-adapter-layer.md) | Platform Adapter Layer as Rust traits with a mock backend | Accepted |
| [0008](0008-vpn-engine-isolation.md) | WireGuard in-process, OpenVPN as a child process | Accepted |
| [0009](0009-wasm-plugin-sandbox.md) | Plugins in a WASM sandbox with capability grants and signatures | Accepted |
| [0010](0010-sqlite-split-stores.md) | SQLite via SQLx, split config/metrics stores, secrets in keychain | Accepted |
| [0011](0011-linux-coexistence.md) | Coexist with the distribution's network stack | Accepted |
| [0012](0012-macos-hotspot-degraded.md) | macOS hotspot is assisted-manual, not programmatic | Accepted |
| [0013](0013-contract-first-protobuf.md) | Contract-first APIs with protobuf as single source of truth | Accepted |
| [0014](0014-installer-strategy.md) | Native installers only; store sandboxes deferred | Accepted |
| [0015](0015-update-channel-policy.md) | Signed updates with rollback; never self-update package installs | Accepted |

## Pending

Decisions identified but not yet made are tracked as **OD-1..OD-9** in
[ARCHITECTURE.md §20](../architecture/ARCHITECTURE.md), with the point at which each must be
resolved in [EXECUTION_PLAN.md §13](../architecture/EXECUTION_PLAN.md). Each becomes an ADR
when decided.

The next one due is **OD-4** (local IPC transport), which SPIKE-A in WP-0.3 resolves and which
will be recorded as ADR-0016. It is the highest-risk open question in the project: if Dart
cannot speak Unix sockets or named pipes on Windows, ADR-0002 needs amending.
