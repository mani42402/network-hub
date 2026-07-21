# NetworkHub

A cross-platform network management platform: hotspot sharing, VPN, firewall, DHCP/DNS,
monitoring, device management, and automation — local-first, privacy-focused, API-first.

> **Status: pre-alpha.** The workspace is scaffolded and the architecture is settled, but no
> feature is implemented yet. Nothing here is usable as a product. See
> [`docs/architecture/EXECUTION_PLAN.md`](docs/architecture/EXECUTION_PLAN.md) for what is
> being built and in what order.

---

## What this is

NetworkHub is **not a desktop app**. Almost every capability it offers — firewall rules,
hotspot hosting, DHCP leases, VPN tunnels, traffic shaping — requires operating-system
privileges, and a GUI must never run privileged. So the product is:

- **`networkhubd`** — a privileged system service that owns all network state, and
- **thin, unprivileged clients** — a Flutter desktop app, a CLI, and (later) a web dashboard
  and mobile controllers, all speaking the same gRPC API.

A useful side effect: because the daemon never depended on a desktop session, **headless
operation works for free** — a Raspberry Pi in a café closet, administered from a browser
or a phone.

Read [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) before writing
code. It is the contract, not background reading.

## Supported platforms

| Platform | Status |
|---|---|
| Linux (Ubuntu 22.04+, Debian 12+, Fedora, Arch, Raspberry Pi OS) | Reference platform — full feature set |
| Windows 10 (2004+) / 11 | Full, with documented gaps (per-device shaping, custom hotspot subnets) |
| macOS 13+ | VPN, firewall, and monitoring in full; hotspot is assisted-manual — Apple exposes no public API for programmatic Internet Sharing |

Capabilities are **probed at runtime**, not assumed. The UI renders from a capability matrix
and explains *why* something is unavailable rather than failing silently.

## Repository layout

```
crates/            Rust workspace — the engine
  nh-types/          Shared vocabulary; depends on nothing internal
  nh-proto/          Protobuf contracts — single source of truth for all client APIs
  nh-state/          Desired-state store, reconcilers, event bus, intent journal
  nh-pal/            Platform Adapter Layer traits + mock backend
    pal-linux/  pal-windows/  pal-macos/
  nh-{firewall,vpn,hotspot,dhcp,dns,monitor,devices,...}/
  nh-daemon/         The privileged service (binary)
  nh-gateway/        Optional remote REST/WebSocket gateway (binary)
  nh-cli/            Command-line client (binary)
apps/desktop/      Flutter desktop application
proto/             .proto sources
sdk/               Plugin SDKs (Rust, TypeScript)
packaging/         MSI, deb, rpm, AppImage, DMG
docs/              Architecture, ADRs, ops guides, security
tests/             netns, end-to-end, and fault-injection suites
tools/             xtask and developer scripts
```

## Getting started

Run the setup script for your platform, then build:

```bash
tools/setup-macos.sh          # or setup-linux.sh, or setup-windows.ps1
cargo build --workspace
cargo xtask check-layers      # architecture rules
```

The repo pins its own Rust toolchain via `rust-toolchain.toml`, so `rustup` fetches the
right compiler automatically. You do not need to change your system default.

## Development rules that are enforced, not suggested

These are checked by CI and will fail your build. The reasoning is in
[`CONTRIBUTING.md`](CONTRIBUTING.md) and ARCHITECTURE.md §1.6.

- `unsafe_code = "forbid"` everywhere except the three `pal-*` FFI backends
- Clippy pedantic, `-D warnings`
- **Architecture layer rules** — feature crates may not depend on a concrete PAL backend;
  `nh-types` stays a leaf; only three crates produce binaries (`cargo xtask check-layers`)
- Permissive licences only — a GPL dependency fails the build (`cargo deny check`)
- Every network-facing parser ships with its fuzz target in the same pull request

## Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](docs/architecture/ARCHITECTURE.md) | The design contract: decisions, threat model, failure catalog |
| [EXECUTION_PLAN.md](docs/architecture/EXECUTION_PLAN.md) | Work packages, sequencing, exit criteria |
| [docs/adr/](docs/adr/) | Architecture Decision Records — how decisions change |
| [SECURITY.md](SECURITY.md) | Vulnerability disclosure policy |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development workflow and engineering standards |

## Licence

Apache-2.0. See [LICENSE](LICENSE) and — once dependencies exist — `THIRD-PARTY-NOTICES`.

OpenVPN (GPLv2) is integrated as a **separate process**, never linked, to keep the licence
boundary clean. See ARCHITECTURE.md §16.
