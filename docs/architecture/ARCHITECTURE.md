# NetworkHub — Engineering Blueprint

**Version:** 1.0-draft · **Status:** Pre-implementation design · **Supersedes:** `Idea.md` (kept as product vision reference)

This document is the build contract for NetworkHub. `Idea.md` describes *what* we want; this document decides *how*, resolves the conflicts and impossibilities in the wishlist, and enumerates the edge cases and failure modes we must design for before writing code. Every major decision is recorded as an **AD-n** (Architecture Decision) so it can be referenced, challenged, and amended via ADRs later.

---

## Table of Contents

1. [Executive Summary & Key Decisions](#1-executive-summary--key-decisions)
2. [Scope, Personas, Non-Goals](#2-scope-personas-non-goals)
3. [Platform Reality Check](#3-platform-reality-check)
4. [System Architecture](#4-system-architecture)
5. [Core Engine Design](#5-core-engine-design)
6. [IPC & API Layer](#6-ipc--api-layer)
7. [Data Layer](#7-data-layer)
8. [Module Specifications](#8-module-specifications)
9. [Plugin System](#9-plugin-system)
10. [Security Architecture & Threat Model](#10-security-architecture--threat-model)
11. [Resilience & Failure-Mode Catalog](#11-resilience--failure-mode-catalog)
12. [UI Architecture](#12-ui-architecture)
13. [Packaging & Distribution](#13-packaging--distribution)
14. [Testing Strategy & CI/CD](#14-testing-strategy--cicd)
15. [Observability](#15-observability)
16. [Licensing Strategy](#16-licensing-strategy)
17. [Repository Layout & Engineering Process](#17-repository-layout--engineering-process)
18. [Roadmap with Exit Criteria](#18-roadmap-with-exit-criteria)
19. [Risk Register](#19-risk-register)
20. [Open Decisions](#20-open-decisions)

---

## 1. Executive Summary & Key Decisions

NetworkHub is a cross-platform network management platform: hotspot sharing, VPN, firewall, DHCP/DNS, monitoring, device management, automation, and a plugin ecosystem — local-first, privacy-focused, API-first.

The single most important structural insight, which reshapes everything in `Idea.md`:

> **Almost every feature in this product requires OS privileges (root / Administrator / entitlements), but a GUI must never run privileged.** Therefore NetworkHub is not "a desktop app". It is a **privileged system service (daemon)** with a **thin, unprivileged UI** attached to it. The Flutter app, the future web dashboard, the future mobile controllers, and the CLI are all just *clients* of the same daemon API.

This also solves a use case the idea doc implies but doesn't design for: **headless operation** (Raspberry Pi in a café closet, managed from a browser or phone) works for free, because the daemon never depended on a desktop session.

### Architecture Decisions (summary — details in linked sections)

| ID | Decision | Rationale (short) |
|----|----------|-------------------|
| AD-1 | **Split-privilege architecture**: privileged Rust daemon (`networkhubd`) + unprivileged clients (Flutter UI, CLI, web). | Security, headless support, crash isolation. §4 |
| AD-2 | **Local IPC is gRPC (tonic) over Unix domain sockets / Windows named pipes**, authenticated by OS peer credentials. TCP is never used for local UI↔daemon. | No open localhost port by default; OS-enforced authn. §6 |
| AD-3 | **Optional remote API is a separate gateway** (`networkhub-gateway`, axum: REST + WebSocket) that proxies the daemon API. Disabled by default; requires explicit enable + TLS + credential setup. | "Secure by default" means zero listening sockets out of the box. §6 |
| AD-4 | **Desired-state reconciliation model**: all network config (firewall rules, hotspot, VPN, shaping) is stored as declarative desired state; a reconciler continuously converges actual OS state to it. No imperative one-shot mutations. | Survives crashes, reboots, sleep/resume, external interference; makes "don't brick the network" tractable. §5.3 |
| AD-5 | **Fail-open networking safety invariant**: any NetworkHub-owned firewall/routing state must be attributable (tagged/anchored), reversible by a single documented command, and cleaned up by a boot-time deadman check. The kill switch is the *only* deliberately fail-closed feature, and it has an offline escape hatch. | A network tool that can strand the user offline is worse than no tool. §11 |
| AD-6 | **Capability-driven feature model**: at startup the platform adapter probes what the OS/driver actually supports (AP mode, band, shaping, portal, etc.) and publishes a capability report. UI renders from capabilities; unsupported features show *why*, never fail silently. | Hotspot/shaping support varies wildly by OS, driver, and even Wi-Fi chipset. §3, §5.4 |
| AD-7 | **Platform Adapter Layer (PAL) as Rust traits** with three backends (`pal-windows`, `pal-linux`, `pal-macos`) plus a `pal-mock` for tests. Core logic never touches OS APIs directly. | Testability, portability, clean seams. §5.2 |
| AD-8 | **VPN protocol engines are isolated**: WireGuard via in-process userspace implementation (kernel module on Linux when available); OpenVPN via a **bundled separate process** controlled over its management interface — never linked. | GPL isolation, crash isolation, license compliance. §8.2, §16 |
| AD-9 | **Plugins run in a WASM sandbox** (wasmtime, WASI) with capability-scoped host functions, resource limits, and mandatory Ed25519 signatures. No native plugins in v1. | Only credible way to promise "sandboxed plugins" cross-platform. §9 |
| AD-10 | **SQLite with SQLx and hand-written migrations**, split into `config.db` (settings, rules, profiles) and `metrics.db` (time-series, purgeable). Secrets are **never** in SQLite — platform keychain only, or encrypted blobs whose key lives in the keychain. | Control over schema/migrations; blast-radius separation; secret hygiene. §7 |
| AD-11 | **Linux integrates *with* NetworkManager/systemd where present** (D-Bus first, raw tools as fallback), and claims exclusive ownership only of interfaces it manages (marks them unmanaged in NM). Firewall lives in NetworkHub-owned nftables tables that coexist with firewalld/ufw. | Fighting the distro's network stack is unwinnable. §3.2, §8.3 |
| AD-12 | **macOS hotspot is explicitly degraded**: no public API exists for programmatic Internet Sharing. We ship VPN/firewall/monitoring fully; hotspot on macOS is "assisted manual" (guide + detect) until/unless Apple provides an API. | Honesty over fragile plist hacks. §3.3 |
| AD-13 | **Contract-first APIs**: protobuf is the single source of truth for daemon APIs; Dart/TS/Rust client code is generated. REST gateway is generated/mapped from the same contracts. | Prevents UI/daemon drift across 4+ client platforms. §6.3 |
| AD-14 | **Installers: MSI (WiX) on Windows, deb/rpm on Linux (AppImage for UI-only), DMG + notarized pkg with SMAppService helper on macOS.** MSIX, Snap, Flatpak deferred — their sandboxing conflicts with a privileged service. | Services and store sandboxes don't mix. §13 |
| AD-15 | **Update channel policy**: self-updater (TUF-style signed manifests) only for installs we own (MSI, DMG, AppImage); deb/rpm updates flow through the distro package manager/repo. Daemon and UI update atomically with rollback. | Never self-modify files owned by a package manager. §13.4 |

### The five hard truths this design is built around

1. **Privilege is the product.** Firewall, hotspot, DHCP, shaping, packet capture — all privileged. The architecture is shaped by *where the privilege boundary sits* (daemon) and *how authorization crosses it* (§10).
2. **Hardware/driver variance dominates hotspot UX.** Whether AP mode, STA+AP concurrency, 5 GHz AP, or client isolation work is a function of the Wi-Fi chipset and driver, not our code. Capability probing (AD-6) is not optional polish; it is the feature.
3. **We are a guest in the OS's network stack.** systemd-resolved owns port 53 semantics on Ubuntu; firewalld owns nftables opinions on Fedora; Windows AV suites install WFP callouts; macOS SIP limits what we may touch. Coexistence rules are specified per subsystem (§8).
4. **Failure modes are user-hostile by default.** A crashed daemon with leftover DROP rules, a kill switch that survives uninstall, a DHCP server leaking onto the office LAN — each is catastrophic. §11 catalogs these and each has a designed mitigation, not a hope.
5. **Licensing is architecture.** OpenVPN (GPLv2), wintun/wireguard-nt (custom terms), GeoIP databases (licensed) — each constrains *how* we integrate (process boundary vs. linking, download-at-runtime vs. bundle). §16.

---

## 2. Scope, Personas, Non-Goals

### 2.1 Personas (drive priority ordering)

| Persona | Primary jobs | Implication |
|---|---|---|
| **Home power user** | Share Wi-Fi, see who's on the network, parental-style pause/schedule, VPN | Zero-config defaults, one-click actions, no jargon |
| **Developer / remote worker** | VPN profiles, kill switch, split tunnel, quick hotspot for devices, API/CLI | CLI + API parity with UI, scriptability |
| **IT pro / small business / café / school / hotel** | Guest network, captive portal, quotas, device approval, reporting, headless box | Headless daemon + web dashboard, RBAC, audit |
| **Privacy-conscious user** | Local-first, DNS filtering, DoH/DoT, no telemetry | Opt-in-only telemetry, local analytics, data purge |

### 2.2 Non-goals (v1 — explicitly out)

- Replacing enterprise NAC/RADIUS deployments (802.1X server is out; 802.1X *client* config is out of scope v1).
- Being a general packet-analysis tool (we are not Wireshark; we do flow accounting, not payload inspection).
- DPI/content inspection of TLS traffic. Domain analytics come from DNS and SNI metadata only (§8.5).
- Router firmware (we run on general-purpose OSes; OpenWrt port is a possible future).
- Mobile *hotspot hosting* on Android/iOS (future apps are controllers/clients only).
- Windows 10 *below* 2004 and macOS below 13 (Ventura); 32-bit anything.

### 2.3 Supported targets (v1)

- **Windows 10 (2004+) / 11**, x86_64 + arm64.
- **Linux**: Ubuntu 22.04+/Debian 12+, Fedora (current−1), Arch, Raspberry Pi OS (bookworm+), x86_64 + aarch64. Kernel ≥ 5.15 assumed for full feature set (nftables, kernel WireGuard, tc-cake).
- **macOS 13+**, Apple Silicon + Intel.

---

## 3. Platform Reality Check

This section exists because the idea doc's platform table ("✅ Full" everywhere) is not achievable, and pretending otherwise would poison the whole design. Below is what each OS *actually* permits, and the strategy per platform. The output of this section is the **Capability Matrix** the runtime probes and the UI consumes (AD-6).

### 3.1 Windows

| Area | Reality | Our strategy |
|---|---|---|
| Hotspot | **Mobile Hotspot** (WinRT `NetworkOperatorTetheringManager`): supported path on Win10/11; shares an existing connection; SSID/passphrase/band configurable; client cap (typically 8); requires adapter+driver support. Legacy *Hosted Network* (`WlanHostedNetwork`) is deprecated and absent on many modern drivers. | Primary: Mobile Hotspot API. Probe support at runtime (`GetTetheringCapability`). Do **not** build on Hosted Network. Expose Windows client-count limit in capabilities. |
| Ethernet/ICS sharing | ICS via `HNetCfg.HNetShare` COM API; brittle, global (one shared connection at a time), known to reset on reboot in some configurations. | Support as "best effort" capability with explicit caveats; reconciler re-asserts ICS state on boot/resume. |
| Firewall | **WFP** (Windows Filtering Platform) user-mode API for filters at ALE/IP layers; sufficient for allow/deny per app/port/IP without a kernel driver. Per-packet inspection/shaping would need a WFP **callout driver** (signed kernel driver — heavy). | v1: user-mode WFP filters in our own sublayer with unique provider GUID. No kernel driver in v1 (defers per-device shaping on Windows, §8.3). Coexist with Windows Defender Firewall (we add filters; we never disable it). |
| DHCP/DNS for hotspot | Mobile Hotspot/ICS runs its own internal DHCP/NAT (192.168.137.0/24 by convention); we cannot replace it cleanly. | On Windows, our DHCP/DNS crates are **observers** (lease table via ARP/neighbor scan), not servers. Custom subnet/static leases = capability "unsupported" on Windows hotspot. |
| Monitoring | `GetIfTable2`/`GetIfEntry2` for interface counters; `GetExtendedTcpTable`/`GetExtendedUdpTable` + PID→process for per-app attribution; ETW for deeper events. Packet capture needs npcap (licensing!) or raw sockets (limited). | Per-interface + per-process accounting via IP Helper/ETW. **No npcap dependency** (license cost, install friction). Flow-level only. |
| Bandwidth shaping | No usable public per-client shaping API without a callout driver. | Capability "unsupported (Windows)" in v1; roadmap item tied to signed-driver investment (§19 R-7). |
| Service model | Windows Service (LocalSystem or a reduced-privilege service account + required privileges), named pipe IPC with SDDL. | `networkhubd` as a service; pipe ACL: SYSTEM, Administrators, and members of a `NetworkHub Operators` local group. |
| Signing | Kernel-free keeps us to standard Authenticode signing; SmartScreen reputation still applies. | EV code-signing cert budgeted from day one; MSI + binaries signed in CI. |

### 3.2 Linux

| Area | Reality | Our strategy |
|---|---|---|
| Network stack ownership | NetworkManager on desktops/Ubuntu/Fedora; systemd-networkd on some servers; wpa_supplicant/iwd underneath; Raspberry Pi OS uses NM (bookworm+). | **Integrate via D-Bus with NetworkManager when present** (it can create AP-mode connections itself). When we need raw control (hostapd), set the interface `unmanaged` in NM first, restore on teardown. Detect networkd and warn about split ownership. |
| Hotspot | AP mode requires driver/chipset support (nl80211 `AP` interface mode; check via `nl80211` netlink — equivalent of `iw list`). STA+AP concurrency depends on chipset combination matrix. Many USB adapters (esp. Realtek out-of-tree drivers) lie or fail. | Probe: interface modes, supported channels/bands, `valid interface combinations` (for repeater mode). Primary path NM D-Bus AP profile; advanced path (guest VLAN-less isolation, captive portal) uses **hostapd** child process with generated config + `ctrl_interface` control socket. |
| DHCP/DNS | Port 53/67 conflicts: systemd-resolved binds 127.0.0.53:53 (stub) — normally no conflict on hotspot interface IP; some distros run a global dnsmasq; firewalld may own nftables ruleset. | Run our own DHCP/DNS (embedded Rust servers, §8.4) **bound only to NetworkHub-managed interface addresses**, never wildcard. Startup preflight detects listeners on target ip:port and reports conflict with remediation, instead of failing on `EADDRINUSE` mid-flow. |
| Firewall | nftables is the ground truth on modern kernels; iptables-nft shims exist; firewalld/ufw may manage their own tables. Ordering across tables is by hook priority. | All rules live in dedicated tables `inet networkhub_*` with documented priorities; we never flush/modify foreign tables. Conflict detection reports overlapping DROP/REJECT verdicts from other tables (best effort). NAT for sharing via our own `networkhub_nat` table + `net.ipv4.ip_forward` (restored to prior value on teardown — see §11 F-9). |
| Shaping | `tc` HTB/fq_codel or CAKE per client (by IP/MAC via flower filters or per-IP classes); IFB for ingress. Fully supported. | Linux is the reference platform for per-device bandwidth limits and quotas. |
| Accounting | nftables counters per device (MAC/IP), conntrack accounting (`nf_conntrack_acct=1`), optional eBPF (cgroup/tc) for per-app later. | v1: nftables counters + conntrack; eBPF behind a capability flag (kernel/BTF variance on Pi). |
| Privileges | Daemon as root under systemd with hardening (see §10.6); or fine-grained ambient capabilities (`CAP_NET_ADMIN, CAP_NET_RAW, CAP_NET_BIND_SERVICE` + file access). Client authorization via UDS peer credentials (`SO_PEERCRED`) + membership in `networkhub` group, with polkit optional for desktop-integrated auth prompts. | Start as root (hostapd/tc/nft orchestration is painful under partial caps), with a hard systemd sandbox (ProtectHome, ProtectSystem=strict + explicit RW paths, NoNewPrivileges for child processes where possible, seccomp allowlist as hardening milestone). |
| Raspberry Pi specifics | brcmfmac supports AP but *not* STA+AP on the same band reliably; SD-card wear (metrics DB!); headless is the norm. | Metrics write batching + WAL tuning + configurable retention (§7.4); repeater capability gated by combination probe; first-boot headless provisioning flow (§8.1). |

### 3.3 macOS

| Area | Reality | Our strategy |
|---|---|---|
| Hotspot / Internet Sharing | **No public API.** Internet Sharing is toggled in System Settings; the plist/`configd` hacks are fragile, SIP-adjacent, and break across releases. `CoreWLAN` cannot create an AP. | AD-12: "assisted manual" — we detect Internet Sharing state (SCDynamicStore), deep-link the user to the settings pane, auto-detect when it activates, then layer what we legally can on top (device discovery, monitoring on the bridge interface, DNS override guidance). Marketed honestly as limited. |
| VPN | `NetworkExtension` (NEPacketTunnelProvider) requires signed app + `com.apple.developer.networking.networkextension` entitlement, system extension approval UX on first install. Alternative: utun device management with root helper (how WireGuard's own macOS non-store builds & Tunnelblick-style apps work). | v1: **privileged helper (SMAppService daemon) + utun** for WireGuard, `openvpn` child process for OpenVPN. NE-based product variant deferred (needed for App Store distribution, which we are not targeting v1). |
| Firewall | PF via `pfctl` anchors; supported and stable, but global PF enable state is shared; Apple's own `pf.conf` must remain untouched. Application Firewall (socketfilterfw) is separate. | Own anchors `networkhub/*` loaded under the system anchor point; enable PF reference-counted; never rewrite `/etc/pf.conf` — use anchor files + `pfctl -a`. |
| Monitoring | Interface counters via sysctl/route; per-process via `nettop`-style PF_SYSTEM sockets (private-ish) or NE. pcap needs root (we have the helper). | Interface + per-device (hotspot bridge) accounting v1; per-app deferred. |
| Wi-Fi info | Scanning/SSID read requires Location permission grants on modern macOS. | Capability + permission-prompt flow in UI; degrade gracefully. |
| Packaging | DMG + notarization mandatory; privileged helper via `SMAppService`; hardened runtime. | §13.3. |

### 3.4 Capability Matrix (probe → publish → render)

The daemon computes this at startup and on hardware change; every entry is `{status: supported | degraded | unsupported, reason, details}`.

| Capability key | Win | Linux | macOS |
|---|---|---|---|
| `hotspot.wifi_ap` | driver-probe | driver-probe (nl80211) | unsupported (assisted-manual) |
| `hotspot.band_5ghz` / `band_6ghz` | driver-probe | driver-probe | — |
| `hotspot.sta_ap_concurrent` (repeater) | rare | combination-probe | unsupported |
| `hotspot.custom_subnet` | unsupported | supported | — |
| `hotspot.captive_portal` | unsupported (no DHCP/DNS control) | supported | unsupported |
| `hotspot.client_isolation` | unsupported | hostapd `ap_isolate` | — |
| `sharing.ethernet` | ICS (degraded) | supported (NAT) | assisted-manual |
| `firewall.rules` | WFP | nftables | PF anchors |
| `firewall.per_device` | partial (IP/MAC volatility caveats) | supported | supported (hotspot bridge) |
| `shaping.per_device` | unsupported v1 | supported (tc) | partial (dummynet/pf, evaluate) |
| `quota.per_device` | accounting-only (no enforcement) | supported | partial |
| `vpn.wireguard` | supported (wintun) | supported (kernel/userspace) | supported (utun) |
| `vpn.openvpn` | supported (bundled proc) | supported | supported |
| `vpn.kill_switch` | WFP | nftables | PF |
| `dns.local_resolver` (DoH/DoT/filtering) | supported (per-adapter DNS pointing) | supported | supported (resolver config) |
| `monitor.per_process` | supported (IP Helper/ETW) | eBPF-gated | deferred |

**Rule:** the UI never hardcodes a platform switch. It renders from this matrix. Adding OS support = shipping a better matrix, no UI changes.

---

## 4. System Architecture

### 4.1 Process topology

```
┌────────────────────────────── unprivileged ──────────────────────────────┐
│                                                                          │
│  Flutter Desktop UI      networkhub CLI       Web Dashboard (Ph.3+)      │
│  (user session)          (scripts/headless)   (browser)                  │
│        │                        │                   │                    │
│        │  gRPC over UDS /       │                   │ HTTPS/WSS          │
│        │  named pipe            │                   │                    │
└────────┼────────────────────────┼───────────────────┼────────────────────┘
         │                        │                   │
         ▼                        ▼                   ▼
┌─────────────────────────────────────────────┐   ┌──────────────────────┐
│  networkhubd  (privileged system service)   │◄──│ networkhub-gateway   │
│                                             │   │ (REST+WS, OPTIONAL,  │
│  ┌─────────────┐  ┌──────────────────────┐  │   │ off by default,      │
│  │ API server  │  │ AuthZ / audit        │  │   │ unprivileged user)   │
│  └──────┬──────┘  └──────────────────────┘  │   └──────────────────────┘
│         ▼                                   │
│  ┌──────────────────────────────────────┐   │   ┌──────────────────────┐
│  │ Desired-state store + Reconcilers    │   │   │ Child processes:     │
│  │ (hotspot, vpn, firewall, dhcp/dns,   │◄──┼──►│  hostapd (Linux)     │
│  │  shaping, automation)                │   │   │  openvpn (all)       │
│  └──────┬───────────────────────────────┘   │   │  wasm plugin host    │
│         ▼                                   │   └──────────────────────┘
│  ┌──────────────────────────────────────┐   │
│  │ Platform Adapter Layer (PAL)         │   │
│  │  pal-windows │ pal-linux │ pal-macos │   │
│  └──────────────────────────────────────┘   │
│         │            │                      │
│  ┌──────▼─────┐ ┌────▼──────┐               │
│  │ config.db  │ │ metrics.db│  (SQLite)     │
│  └────────────┘ └───────────┘               │
└─────────────────────────────────────────────┘
```

Key properties:

- **One daemon instance per machine** (singleton via OS-level lock: named mutex on Windows, flock + systemd on Linux, launchd on macOS). Second instance exits with a clear message.
- **UI is optional and restartable at any time.** Closing the UI never affects a running hotspot/VPN. The UI is a *view* over daemon state, resynchronized on connect (no client-held authoritative state).
- **Child processes** (hostapd, openvpn, plugin host) are supervised by the daemon: monitored, restarted per policy, killed on daemon shutdown via process-group/job-object so orphans cannot survive (§11 F-4).
- **The gateway is a separate binary and OS user.** Compromise of the internet-facing HTTP surface does not directly equal root: the gateway holds a scoped daemon credential, and the daemon enforces RBAC on every call (defense in depth, §10).

### 4.2 Startup / shutdown sequence (daemon)

1. Acquire singleton lock; open logs.
2. **Deadman check** (§11): scan for NetworkHub-tagged firewall/routing/DNS state left by a previous run; reconcile against persisted desired state — *restore or remove*, never leave orphaned.
3. Open databases; run migrations (with pre-migration backup, §7.3).
4. Probe capabilities → publish matrix.
5. Start reconcilers (each independently; one failing module must not prevent others — e.g. broken hostapd must not take down VPN).
6. Start IPC server; mark ready (systemd `sd_notify` / SCM `SERVICE_RUNNING`).
7. On shutdown: stop accepting IPC → drain → per-module teardown policy (VPN kill switch persists **only** if user opted into "persistent kill switch"; hotspot stops; firewall rules persist by design — they are desired state) → flush DBs → exit.

### 4.3 Trust boundaries

| Boundary | Crossing mechanism | Enforcement |
|---|---|---|
| UI/CLI ↔ daemon | UDS / named pipe | OS peer credentials + group membership + per-operation RBAC + audit log |
| Gateway ↔ daemon | Same IPC, gateway's own identity | Gateway has a *service principal* with a role; user identity forwarded and re-checked by daemon |
| Internet ↔ gateway | HTTPS/WSS | TLS 1.3, token auth (§6.4), rate limiting, no daemon exposure |
| Daemon ↔ plugins | WASM host functions | Capability grants per plugin manifest; resource limits (§9) |
| Daemon ↔ child processes | argv/env/config files + control sockets | Generated configs in root-owned dirs (0700); control sockets ACL'd; never shell-interpolated (no `sh -c`) |
| LAN clients ↔ hotspot services (DHCP/DNS/portal) | Untrusted network input | Hardened parsers (fuzzed), rate limits, per-source throttling (§10.7) |

---

## 5. Core Engine Design

### 5.1 Rust workspace layout

```
crates/
├── nh-types/          # Shared domain types, IDs, error taxonomy, capability model (no deps on tokio/OS)
├── nh-proto/          # Protobuf contracts + generated tonic code (single source of truth)
├── nh-daemon/         # Binary: wiring, lifecycle, supervision, IPC server, authz, audit
├── nh-state/          # Desired-state store, reconciler framework, event bus
├── nh-pal/            # PAL trait definitions + pal-mock
│   ├── pal-windows/
│   ├── pal-linux/
│   └── pal-macos/
├── nh-hotspot/        # Hotspot orchestration (drives PAL + hostapd mgmt on Linux)
├── nh-vpn/            # Profile mgmt, WireGuard engine, OpenVPN process controller, kill switch logic
├── nh-firewall/       # Rule model, compiler to per-OS representation (via PAL)
├── nh-dhcp/           # Embedded DHCPv4 server (Linux-serving; observer elsewhere)
├── nh-dns/            # Embedded DNS: cache, DoH/DoT upstreams, filtering, local zone
├── nh-monitor/        # Counters, flows, speed/latency probes, metric pipeline
├── nh-devices/        # Device identity, discovery (ARP/NDP/mDNS/SSDP), OUI vendor db, state machine
├── nh-automation/     # Rule engine (triggers/conditions/actions), scheduler
├── nh-analytics/      # Aggregation, rollups, reports, export (CSV/JSON/PDF)
├── nh-plugin-host/    # wasmtime runtime, manifest verification, host API
├── nh-secrets/        # Keychain abstraction (DPAPI/Keychain/Secret Service) + encrypted-blob fallback
├── nh-updater/        # Manifest fetch/verify, staged apply, rollback (per-channel policy)
├── nh-gateway/        # Binary: REST+WS remote gateway (optional component)
└── nh-cli/            # Binary: networkhub CLI (thin gRPC client; also the headless install tool)
```

Dependency rules (enforced with cargo-deny + a CI layer check):
- `nh-types` depends on nothing internal. Everything may depend on it.
- Feature crates (`nh-hotspot`, `nh-vpn`, …) depend on `nh-pal` **traits**, never on `pal-{os}` concrete crates. Only `nh-daemon` selects backends.
- Nothing but `nh-daemon`/`nh-gateway`/`nh-cli` produces binaries.

### 5.2 Platform Adapter Layer (PAL)

One trait family per concern, sized to be implementable on all three OSes *or* explicitly optional:

```rust
trait WifiApControl   { fn probe(&self) -> ApCapabilities; async fn start(&self, cfg: ApConfig) -> Result<ApHandle>; ... }
trait FirewallBackend { fn begin(&self) -> Txn; /* rule-set transactions, atomic swap */ ... }
trait RoutingNat      { async fn enable_forwarding(&self, scope) -> Guard; async fn add_nat(&self, ...) -> Guard; }
trait TrafficShaper   { fn capabilities(&self) -> ShapingCaps; ... }
trait InterfaceInfo   { /* enumerate, watch hotplug/link events (RTNETLINK / NotifyIpInterfaceChange / SCDynamicStore) */ }
trait DnsSystemConfig { /* set per-interface system DNS; restore-on-drop guards */ }
trait ProcessAttrib   { /* per-process socket attribution where supported */ }
```

Conventions:
- **Every mutating PAL call returns a Guard or is recorded in the state journal** (§5.3) so it can be undone. RAII alone is insufficient (process may die) — guards also write a journal entry first (write-ahead intent).
- PAL methods are cancel-safe and idempotent where the OS allows ("ensure rule exists" not "add rule").
- `pal-mock` simulates interfaces, latency, and injected failures; the entire reconciler test suite runs against it on every commit, on all dev machines, no privileges needed.

### 5.3 Desired-state reconciliation (AD-4)

The heart of robustness. Modeled after how Kubernetes/NetworkManager think, scaled down:

- **Desired state** lives in `config.db` (e.g., "hotspot enabled with profile X", "firewall rule set R", "VPN P connected with kill switch"). All API mutations write desired state + an audit entry, then nudge the reconciler. The API returns when desired state is durably recorded and reconciliation has been *initiated* (long operations report progress via events, not blocked RPCs).
- **Observed state** is refreshed by watchers (netlink/IP Helper/SCDynamicStore callbacks, child process health, periodic probes).
- **Reconcilers** (one per module, independent tasks) converge observed → desired with: exponential backoff, jitter, circuit breaker (after N failures mark module `degraded` + surface actionable error, stop hot-looping), and convergence events on the bus.
- **Intent journal**: before any OS mutation, an intent record (op, target, prior value) is appended (`config.db`, synchronous). On startup the deadman check replays/undoes incomplete intents. This is what makes "we changed `ip_forward` and crashed" recoverable (§11 F-9).
- **External interference policy**: if something else deletes our nftables table or flips our firewall rules, the reconciler restores desired state, but *rate-limited with detection* — after 3 restore cycles in 60s it stops fighting, marks the module `conflicted`, and tells the user what appears to be interfering. We never enter an infinite war with firewalld or an AV product.

### 5.4 Event bus & domain events

In-process broadcast bus (tokio broadcast + typed envelopes). Everything significant is an event: `DeviceJoined`, `DeviceLeft`, `VpnStateChanged`, `RuleApplied`, `CapabilityChanged`, `QuotaExceeded`, `ConflictDetected`, `MetricTick`.

- Events feed: WebSocket/gRPC streams to clients, the automation engine (§8.7), the audit log (subset), and plugins (subset, per capability grant).
- Events are **droppable for slow consumers** (bounded ring per subscriber; a stalled UI can never backpressure the engine). Clients resync from snapshot + sequence number on reconnect — every stream starts with a full snapshot, then deltas (this kills an entire class of stale-UI bugs).

### 5.5 Concurrency model

- tokio multi-thread runtime; per-module tasks; **no global locks across modules**. Cross-module needs go through the bus or the state store.
- All blocking OS/FFI calls (WinRT, COM, D-Bus sync ops, pfctl execs) wrapped in `spawn_blocking` or dedicated threads (COM apartments need a dedicated STA thread on Windows).
- Every spawned task is registered with a supervisor (name, health, restart policy) — visible via `networkhub debug tasks` for field debugging.

---

## 6. IPC & API Layer

### 6.1 Local IPC (AD-2)

- **Transport**: Unix domain socket `/run/networkhub/daemon.sock` (0660, root:networkhub) on Linux; `~/Library/...`-independent `/var/run/networkhub.sock` on macOS; named pipe `\\.\pipe\networkhub` with SDDL on Windows.
- **Protocol**: gRPC (tonic) — typed, streaming-native (live metrics, event feeds), and code-gens to Dart (`grpc` package) and TS.
- **Authn**: OS peer credentials (SO_PEERCRED / `GetNamedPipeClientProcessId`+token query). Map OS user → NetworkHub principal.
- **Authz**: every RPC declares a required permission (`hotspot.start`, `firewall.write`, `settings.read`, …) in proto options; a single interceptor enforces it. Default local policy: members of admin group get `admin` role; other local users get `viewer` (configurable).
- **Session-independent**: works from SSH, cron, or a headless boot — nothing assumes a desktop session.

### 6.2 Remote gateway (AD-3) — optional component

- Disabled by default. Enabling requires: setting an admin password (Argon2id-hashed), TLS configuration (self-provisioned local CA + cert with proper SANs by default; user-supplied cert supported), and choosing bind scope (`lan` vs `all`, default `lan`).
- REST (mapped from the same protobuf contracts) + WebSocket event/stream endpoint.
- **Auth**: short-lived access tokens (JWT, EdDSA-signed, 15 min) + rotating refresh tokens (revocable, stored hashed); API keys for automation (scoped, prefix-identifiable `nhk_…`, hashed at rest, last-used tracking). Login rate-limited with lockout backoff. WebSocket auth via first-message token (never query-string tokens — they leak into logs).
- **Browser hardening** (matters when web dashboard lands): strict CORS (explicit origin allowlist), `Origin` check on WS upgrade, CSP on served dashboard, cookies (if ever used) `SameSite=Strict; Secure; HttpOnly` — but default is token-in-header, not cookies, which sidesteps CSRF.
- Gateway is intentionally *dumb*: validation, authn, rate limiting, translation. All authorization decisions re-checked in the daemon with the forwarded principal.

### 6.3 Contract-first workflow (AD-13)

- `nh-proto` holds `.proto` files: `devices.proto`, `hotspot.proto`, `vpn.proto`, `firewall.proto`, `monitor.proto`, `automation.proto`, `system.proto`, `events.proto`.
- CI generates: Rust (tonic), Dart (UI), TypeScript (SDK + web). Generated code is committed for Dart/TS (reproducibility) with a CI check that regeneration is clean.
- **Versioning**: proto packages versioned `networkhub.v1`; additive evolution only within v1 (new fields/optional messages); breaking changes require `v2` side-by-side. The daemon reports `min_client_version`; UI shows a clear "update the app / update the service" mismatch screen instead of undefined behavior (installers can update daemon and UI at different times — §13.4).
- Error model: canonical error codes + machine-readable `reason` + human `detail` + optional `remediation` (the UI renders remediation — "Port 53 is in use by dnsmasq (pid 412). See how to fix →").

### 6.4 API surface sketch (v1)

- `SystemService`: capabilities, health, version, audit query, backup/restore.
- `DeviceService`: list/watch devices, rename, tag, block, pause, quota, schedule.
- `HotspotService`: profiles CRUD, start/stop, status stream, connected clients stream, QR payload.
- `VpnService`: profiles CRUD/import, connect/disconnect, status stream, peer mgmt (WG), key generation, kill-switch config.
- `FirewallService`: rule CRUD (atomic rule-set apply with versioning), test/dry-run, conflict report.
- `DnsService`: upstream config (Do53/DoH/DoT), filter lists, overrides, cache stats/flush.
- `MonitorService`: live metrics stream, historical queries, speed test trigger.
- `AutomationService`: rules CRUD, run-log, manual trigger, dry-run.
- `PluginService`: list/install/enable/disable, permission grants, marketplace query (Ph.4).

---

## 7. Data Layer

### 7.1 Storage split (AD-10)

| Store | Contents | Properties |
|---|---|---|
| `config.db` (SQLite) | settings, profiles (hotspot/VPN/firewall), devices, automation rules, users/roles/API-key hashes, plugin registry, audit log, intent journal, schema_meta | Small, hot, backed up, WAL, `synchronous=FULL` for intent journal writes |
| `metrics.db` (SQLite) | time-series samples + rollups, flow summaries, speed tests | Purgeable without losing config; WAL, batched writes, `synchronous=NORMAL`; separate file ⇒ separate corruption blast radius and easy "delete my data" |
| Platform keychain (via `nh-secrets`) | Wi-Fi passphrases, VPN private keys, JWT signing key, DB field-encryption key, cloud tokens | DPAPI (machine scope) / macOS Keychain (system) / Secret Service or root-only encrypted file fallback on headless Linux |
| Filesystem | generated child-process configs (root-owned 0700 runtime dir), plugin wasm files, exported reports, log files | Runtime dir wiped at start/stop |

Secrets rule: SQLite may hold a *reference* to a secret or a ciphertext blob (AES-256-GCM, key in keychain), never plaintext. VPN private keys, hotspot PSKs, API-key material are secrets. MACs/IPs/hostnames are personal-ish data → covered by retention policy, not keychain.

### 7.2 Schema notes (beyond the obvious tables)

- All rows: `id` (UUIDv7 — time-ordered, merge-friendly for future sync), `created_at`, `updated_at` (UTC only; all local-time rendering in clients).
- `devices`: identity is a **stable synthetic ID**, not the MAC. MAC randomization (iOS/Android/Win11 default!) means one physical device may present many MACs; we correlate via (MAC, hostname, mDNS name, DHCP client-id/fingerprint) heuristics with a "merge devices" UI. This is a top-3 real-world edge case for device tracking and quotas — quotas attach to the synthetic ID.
- `firewall_rules`: rule sets are **versioned documents** (rule_set + revision), not individual mutable rows. Apply = compile revision → atomic swap → record `active_revision`. Enables dry-run diff, one-click rollback to previous revision, and clean audit.
- `audit_log`: append-only (no UPDATE/DELETE grants in code paths), hash-chained (`entry_hash = H(prev_hash‖entry)`) for tamper evidence; actor, principal type, action, target, result, source (local-ui/cli/gateway/plugin/automation).
- `intent_journal`: §5.3; rows deleted only after confirmed completion/undo.
- `schedules` stored as RRULE-like recurrences + IANA timezone name, evaluated against a monotonic-aware scheduler (§11 F-13, clock jumps).

### 7.3 Migrations, backup, corruption

- SQLx with versioned, hand-written, forward-only SQL migrations; every release knows the range it can migrate from; downgrade = restore backup.
- Automatic backup: pre-migration snapshot + daily rotating backup of `config.db` (metrics excluded by default), stored locally, optional cloud later. Backup = SQLite `VACUUM INTO` (consistent, compact).
- Corruption handling: `PRAGMA quick_check` at open; on failure → attempt `.recover`-style salvage into a fresh DB → if that fails, quarantine the file, restore latest backup, emit a prominent event. **The daemon must come up in a safe degraded mode even with a destroyed config.db** (empty desired state = no rules applied = fail-open, per AD-5).

### 7.4 Metrics retention (SD-card & disk friendliness)

Raw samples (1–5 s resolution) → kept 24–48 h → rolled to 1-min (30 d) → 1-h (2 y) → purged. Rollup job is incremental and crash-safe (watermark-based). Batched inserts (≥5 s flush) to respect Raspberry Pi SD wear. All windows user-configurable; "paranoid privacy" preset = live-only, no history.

---

## 8. Module Specifications

Each module spec: purpose → design → platform notes → edge cases (E-x) the implementation MUST handle (these become acceptance tests).

### 8.1 Hotspot & Sharing

**Design.** A hotspot *profile* (SSID, auth [WPA2/WPA3-transition default], band pref, subnet, isolation, guest flag, portal ref, schedule) + a *runtime* (state machine: `Stopped → Preflight → Starting → Running → Degraded → Stopping`). Preflight validates capabilities, radio availability, subnet conflicts, and port conflicts **before** touching anything, returning a full list of problems at once (not fail-on-first).

- Linux: NM D-Bus AP profile (simple path) or managed hostapd (advanced path: isolation, portal, fine-grained 802.11 opts). Our DHCP/DNS bound to the AP interface; NAT via `networkhub_nat` nftables + forwarding guard.
- Windows: Mobile Hotspot API; observe clients via WinRT events + ARP table; no custom subnet/DHCP (§3.1).
- macOS: assisted-manual (§3.3).
- Guest network: on Linux = separate hostapd BSS/interface where the driver's interface-combinations allow, else time-boxed credentials on the main SSID (temporary passwords via WPA2, or WPA3 SAE password rotation). Device approval = new device starts in `quarantine` firewall class (portal/DNS only) until approved, when the policy is enabled.

**Edge cases.**
- E-1 Upstream interface changes while hotspot runs (Ethernet unplugged, Wi-Fi roams, VPN becomes default route): NAT/forwarding must re-target the new upstream; combined with VPN this defines "share the VPN" vs "leak around the VPN" — explicit user choice, default share-VPN when both active.
- E-2 Subnet collision: chosen hotspot subnet (default 10.x picked at random from a pool) collides with the upstream LAN → preflight detects and auto-picks another; persisted per profile.
- E-3 Radio contention: user asks for hotspot while Wi-Fi client mode is on a different channel and chipset can't do both → capability-driven refusal with the actual reason ("your adapter cannot run AP+client on different bands").
- E-4 Sleep/resume & lid close: reconciler restarts the hotspot post-resume if desired state says running; clients will have dropped — event emitted, portal sessions kept (grace period).
- E-5 RF regulatory domain: channel lists depend on regdom; never hardcode channels; surface DFS constraints (DFS channels may take 60 s CAC or be denied).
- E-6 Hotspot start fails midway (hostapd dies, DHCP port grabbed between preflight and bind): full rollback via intent journal; the machine's networking is exactly as before.
- E-7 SSID/passphrase constraints: SSID 32-byte UTF-8 (validate byte length not char count), WPA2 PSK 8–63 ASCII; QR payload (`WIFI:T:WPA;S:…;P:…;;`) escaping of `;,:"\`.
- E-8 Headless first boot (Pi): if no config exists and `--provision` file present (`/boot/networkhub-setup.toml`), auto-start a setup hotspot with a generated password printed to console/HDMI + web setup on portal IP; else stay idle. Never an open unauthenticated setup AP by default.

### 8.2 VPN

**Design.** Profile store (WireGuard native model: interface+peers; OpenVPN: ovpn blob + credentials ref) + per-profile connection state machine (`Disconnected → Resolving → Connecting → Handshaking → Up → Reconnecting → Down(reason)`) + shared services: kill switch, split tunnel, DNS override, auto-reconnect with backoff, connectivity verification (post-up probe through the tunnel — "connected but blackholed" is a state we detect, not deny).

- **WireGuard**: Linux — kernel WG via netlink when available, else userspace; Windows — wintun adapter + userspace engine (license review §16); macOS — utun + userspace. Userspace engine choice (boringtun fork vs. embedded implementation) is Open Decision OD-2.
- **OpenVPN (AD-8)**: bundled `openvpn` binary, own process, controlled via management interface socket; credentials passed via management channel (never argv/env); config file sanitization: we parse and re-emit the profile, **stripping dangerous directives** (`up`, `down`, `script-security`, `plugin`, …) — imported `.ovpn` files are untrusted input that can otherwise execute scripts as root (top VPN-import vulnerability class).
- **Kill switch** (the one fail-closed feature): firewall-layer allow-only policy (loopback, DHCP on the physical link, the tunnel endpoint ip:port, the tun interface). Modes: `off` / `while-connected` (auto-clears on user disconnect or daemon crash — deadman removes it) / `persistent` (survives reboot; requires explicit opt-in with a scary-honest explanation + documented offline escape hatch: `networkhub vpn killswitch off` works with daemon down by removing the persisted OS rules via a tiny self-contained code path, and is printed on the opt-in screen).
- **Split tunnel**: v1 = by destination CIDR + "exclude LAN" toggle (all OSes via routes). Per-app split tunnel is per-OS hard (Linux cgroup/fwmark: doable; Windows WFP appid: doable; macOS: NE-only) → phased, capability-gated.
- **IPv6 leak policy**: if the tunnel doesn't carry IPv6, block IPv6 egress on physical interfaces while connected (default on, toggleable). DNS leak policy: while connected, only the tunnel DNS (or our local resolver forwarding through the tunnel) is reachable; firewall blocks other :53/:853/DoH-known-IPs egress *from the host* (best effort for DoH).

**Edge cases.** E-9 endpoint DNS re-resolution on reconnect (WG endpoints resolved once by design — we re-resolve on handshake failure). E-10 captive-portal networks: kill switch + hotel Wi-Fi = user can't log in; provide "pause kill switch for portal login (max 5 min, LAN+80/443 only)" flow. E-11 MTU: derive from upstream (PMTU probing fallback 1280 for v6) — silent MTU blackholes are the #1 "VPN is broken" report. E-12 overlapping address space (tunnel subnet == LAN subnet): detect, warn, prefer more-specific routes. E-13 default-route wars: two VPN profiles both wanting default route → mutual exclusion in desired-state validation. E-14 system sleep: pause reconnect attempts during sleep, verify + fast-reconnect on resume (don't burn retries into a dead network). E-15 profile import hygiene: size caps, format validation, secret extraction into keychain, no path traversal via embedded filenames (QR/zip imports).

### 8.3 Firewall

**Design.** One canonical rule model → per-OS compiler:

```
Rule { id, priority, enabled, action: Allow|Deny|Reject,
       direction, protocol, src (ip/cidr/device-ref/geo-set), dst, ports,
       iface-ref, schedule-ref?, hit-counting: bool, comment }
```

- Rules referencing **devices** resolve to current IP+MAC pairs at compile time and **recompile on device address change** (event-driven) — this is the mechanism behind block/pause/quota enforcement.
- Compilation is a pure function `RuleSetRevision → OS program` (nft ruleset text applied atomically via one transaction; WFP filter batch in a transaction; PF anchor file + `pfctl -a networkhub/main -f -`). Atomic swap or nothing; failed apply keeps the previous revision active and reports the compiler/OS error verbatim.
- **Lockout guard**: applying a rule set that would cut the *management path* (the connection this request came from, e.g. gateway on LAN, or SSH-likely port 22 to a headless box) triggers confirm-or-revert: apply with a 60 s automatic revert unless confirmed (classic remote-firewall safety, on by default for remote sessions, headless installs, and any deny-rule matching the requester's address).
- Geo/country blocking: optional IP-set feed (licensed source, OD-5), stored as named sets (nft sets / WFP ranges / pf tables), updated by the updater with signatures; feature clearly labeled "best effort".
- Modes: NetworkHub-firewall as **additive** layer (default; host firewall stays on) — we never disable Defender/ufw/firewalld; conflict *detection* reports rules of ours that can't take effect due to other tables' verdicts (Linux: analyzable; Windows/macOS: heuristic).

**Edge cases.** E-16 rule referencing a departed/merged device. E-17 IPv6 parity — every device-block covers v4+v6+MAC where the layer allows (blocking only v4 is a bypass). E-18 DHCP must keep working for the hotspot even under "deny by default" templates (implicit service rules, visible but locked). E-19 rule explosion (per-device × per-schedule): compile-time cap with counting sets instead of naive expansion on Linux. E-20 uninstall removes every filter/table/anchor (§13.5).

### 8.4 DHCP / DNS

**DHCP (Linux-serving).** Embedded DHCPv4 server (own implementation or vetted crate — OD-3): pool mgmt, static leases, reservations, lease DB in `config.db` (survives restart; leases honored across daemon restarts), options (router, DNS, MTU, captive-portal option RFC 8910/8908). **Safety interlocks**: binds only NetworkHub-managed interfaces (never wildcard); refuses to start if another DHCP server is detected answering on that segment (active probe on start); rate-limits DISCOVER floods; DHCP starvation (MAC-rotating exhaustion) mitigations: per-client-id throttling + pool-exhaustion event.

**DNS.** Embedded resolver on managed interfaces (and optionally as the *host's* resolver — the mechanism behind system-wide DoH/DoT + filtering):
- Upstreams: Do53 / DoT (rustls) / DoH; per-upstream health, RTT-based selection, fallback chain; **bootstrap problem** handled (DoH URL's own hostname resolved via bootstrap IPs or system resolver once).
- Filtering: domain lists (hosts/ABP formats), per-device policy groups (kid/guest/default) keyed to device identity, override rules; block = NXDOMAIN or 0.0.0.0 (configurable); local zone for device names (`.hub.local`… careful: avoid `.local` mDNS collision → default suffix `home.arpa` per RFC 8375).
- Cache with proper TTL handling, negative caching (RFC 2308), `flush` API; serve-stale (RFC 8767) optional for flaky upstreams.
- **Host-resolver takeover** is guarded: per-interface DNS set via PAL with restore-on-teardown guard + deadman restore; on Ubuntu integrate with systemd-resolved (set per-link DNS to us) rather than fighting the stub listener.

**Edge cases.** E-21 clients with hardcoded DNS (8.8.8.8) bypass filtering → optional enforcement firewall rule (redirect/block :53 from hotspot clients; DoH bypass acknowledged as not fully blockable). E-22 DNS rebind protection for portal/local zone (block RFC1918 answers from upstreams, configurable). E-23 EDNS0/large responses over UDP → truncation + TCP fallback correctness (test with DNSSEC-sized answers; we pass through DNSSEC records, validation optional later). E-24 mDNS coexistence: we never bind :5353; device discovery *listens* passively.

### 8.5 Monitoring & Analytics

**Pipeline.** Samplers (interface counters @1–5 s; per-device via nft/WFP counters; latency probes: configurable targets, ICMP+TCP fallback when ICMP filtered; jitter/loss via probe series; system CPU/RAM via sysinfo) → ring buffer (live streams) → batcher → `metrics.db` → rollup job → query API → exports.

- Speed test: built-in transfer test against configurable endpoints (self-hostable target spec published; no third-party lock-in), explicitly user-triggered or scheduled (it saturates the link — never automatic by surprise).
- Per-app (where capable): Windows via IP Helper tables + PID mapping; Linux eBPF later (capability-gated); macOS deferred.
- Domain analytics **only** from our DNS resolver logs (if enabled) — no packet payload inspection (privacy stance, §2.2); traffic categories via local domain-category list (plugin-extensible).
- Exports: CSV/JSON native; PDF via headless rendering in the *UI* layer (typography/i18n lives there), not the daemon.

**Edge cases.** E-25 counter wrap/reset (32-bit counters, driver resets, interface re-creation) — samplers store (raw, derived-delta) with wrap detection; never negative rates. E-26 clock jumps (NTP step, DST): all series keyed on UTC + monotonic elapsed for rate math (§11 F-13). E-27 metrics DB growth on tiny disks: hard cap with oldest-first eviction + event when evicting early.

### 8.6 Device Management

- **Discovery**: passive-first (DHCP leases we issued, ARP/NDP tables, mDNS/SSDP *listening*), active scan (ARP sweep/ping) only on managed subnets and only user-initiated or opted-in ("scan my LAN" ethics: on the *upstream* LAN we are a guest — default passive-only; active scanning of non-managed networks requires explicit enable with a notice).
- Identity resolution & merge (§7.2, MAC randomization). Vendor lookup from bundled offline OUI DB (updated with app releases; no runtime callouts).
- Controls map to other modules: block/pause → firewall class; limit/quota → shaper + accounting + automation action on exceed; schedule → automation rule sugar. **Quota semantics defined**: rolling window vs calendar month (user-chosen), counted at the accounting layer per synthetic device ID, enforcement = throttle-to-rate or block (user-chosen), with 80%/100% events.
- Signal/RSSI per client from hostapd station dump (Linux AP) / WinRT client info where exposed.

### 8.7 Automation

- Model: `Rule { trigger, conditions[], actions[], cooldown, enabled, dry_run }` — triggers = domain events + schedule ticks + metric threshold crossings (with hysteresis: `>90% for 60s`, clear at `<75%`, else flapping storms).
- Actions = a safelisted catalog of daemon operations (notify, limit device, disconnect/reconnect VPN, start/stop hotspot, run plugin action, webhook POST [gateway-mediated, with SSRF guard: no RFC1918 targets unless user opts in per-URL]).
- **Safety**: per-rule cooldown; global action budget (max N automation actions/min); loop detection (action → event → same rule chain depth cap); every run logged with inputs and outcome; `dry_run` mode logs would-have-done. No arbitrary scripting in core — complex logic belongs in plugins with their own sandbox.
- Scheduler: timezone-aware, DST-correct (skipped/ambiguous local times resolved by policy), catch-up policy per rule (run-once-if-missed vs skip) after downtime/sleep.

### 8.8 Captive Portal (Phase 2, Linux)

- Mechanics: quarantine firewall class (allow DHCP/DNS/portal IP only) + DNS answers portal IP for unauthenticated clients + HTTP :80 redirect to portal + **RFC 8910 DHCP option & RFC 8908 CapPort API** so modern OSes pop the portal sheet natively. We do **not** MITM HTTPS (impossible without trust violations) — portal detection relies on OS probe URLs (http) + RFC mechanisms.
- Portal server: axum, templated pages (customizable logo/text/languages), voucher/temporary-password auth, terms acceptance logging (configurable retention — hospitality compliance), session table (MAC+token, idle/absolute timeouts), re-auth on expiry.
- Session follows device through IP changes (keyed on MAC on our own AP where MAC is stable per association).
- Legal note surfaced in docs: operators are responsible for local lawful-intercept/data-retention rules (café/hotel jurisdictions differ); we provide the knobs (retention, ToS logging), not legal advice.

### 8.9 AI Assistant (Phase 4 — architecture hooks only in v1)

- v1 ships only the *substrate*: structured diagnostic snapshot API ("network state bundle": capabilities, interfaces, recent events, anonymized metrics summary) that any assistant (cloud LLM via user-supplied key, or local model via plugin) can consume; suggestions land as **proposed changes** (a diff against desired state) that the user reviews/applies — the assistant never mutates state directly (same RBAC path as any client, `assistant` role = read + propose).
- This keeps the security review surface small: AI is just another API client with the weakest role.

---

## 9. Plugin System

### 9.1 Model (AD-9)

- **Runtime**: wasmtime; WASM components (WASI Preview 2 / component model). SDKs: Rust (first), TypeScript via componentize-js (second). One OS process hosts all plugins (`nh-plugin-host`, unprivileged as far as possible), separate *store/instance* per plugin; per-plugin: memory cap, fuel/epoch CPU limits, wall-clock watchdog, no filesystem/network by default.
- **Host API = explicit capabilities** (declared in manifest, granted by user at install, revocable):
  `events.subscribe[topics]`, `devices.read`, `metrics.read`, `dns.resolve_hook` (filtering providers), `notify.send`, `http.fetch[allowlisted origins]` (proxied by host with SSRF guard + rate limit), `storage.kv[quota]`, `vpn.provider` / `dns.provider` (provider-model interfaces), `automation.action`.
- **Distribution & trust**: plugin = signed bundle (manifest + wasm + assets), Ed25519 signatures; roots of trust: NetworkHub official key, marketplace key (Ph.4), and user-added developer keys (with warning). Unsigned = dev-mode only (daemon flag, big banner). Manifest pins: API version range, capability list, resource requests.
- **Failure containment**: plugin crash/timeout → instance restarted with backoff → repeated failure → auto-disabled + event. A plugin can never block a reconciler (all plugin calls async with deadlines; DNS-hook plugins have a hard per-query budget [e.g. 5 ms cached path] and fail-open or fail-closed per user's filter policy).
- **No native plugins in v1** — the entire "sandboxed plugins" promise collapses otherwise. Native integrations that truly need OS access get proposed as core modules instead.

### 9.2 Provider interfaces

`dns.provider` (upstream/filter list source), `vpn.provider` (profile source/refresher — e.g. commercial VPN account integration), `notify.provider` (Telegram/Slack/email/webhook), `backup.provider`, `analytics.exporter`. Each is a versioned WIT interface; core ships reference implementations so interfaces are proven before the SDK is public (dogfooding rule: **core features consume the same provider interfaces plugins get**).

---

## 10. Security Architecture & Threat Model

### 10.1 Assets

Wi-Fi/VPN secrets & keys; admin credentials/API keys/JWT signing key; network control (firewall/routing/DNS = ability to MITM or DoS the user); traffic metadata (who talked to what — privacy-sensitive); audit integrity; update channel integrity; the user's *general network availability* (AD-5).

### 10.2 Adversaries & attack surfaces (STRIDE-organized highlights)

| Surface | Threats | Mitigations |
|---|---|---|
| Local unprivileged user/malware → daemon IPC | Elevation via daemon (any RPC that writes firewall/VPN = root-equivalent) | Peer-cred authn, group-gated socket, RBAC per-RPC, no "eval"-shaped RPCs, audit; secrets never returned over IPC (write-only; QR/export requires re-auth) |
| Hotspot clients (untrusted LAN) | DHCP/DNS/portal parser exploits, floods, DHCP starvation, ARP spoofing, client-to-client attacks, evil-twin | Memory-safe Rust parsers + fuzzing (CI), rate limits, starvation throttles, client isolation option, portal input hardening; evil-twin noted as out of our control (document SSID hygiene) |
| Imported artifacts (.ovpn, WG conf, QR, filter lists, backups) | Script execution (ovpn directives), path traversal, zip bombs, malformed parsing | Parse-and-re-emit sanitization (§8.2), directive allowlists, size/entry caps, no archive extraction outside temp jail, imports treated as hostile in tests |
| Remote gateway (opt-in) | Cred stuffing, token theft, CSRF/WS hijack, API abuse, TLS misconfig | Argon2id, rate limit + lockout, short-lived JWT + revocable refresh, Origin checks, header-token (no cookies) default, TLS1.3-only, separate process/user, daemon-side re-authz |
| Plugins | Malicious/compromised plugin, supply chain | WASM sandbox + capability grants + signatures + resource limits (§9); marketplace review later |
| Update channel | Malicious update = root RCE | TUF-style: signed manifests (offline root key, HSM/YubiKey), version anti-rollback, per-file hashes, signature verified *before* any file replacement, staged rollout + rollback (§13.4) |
| Supply chain (build) | Dependency compromise | cargo-deny (licenses/advisories), lockfiles, dependabot + review, `cargo vet`/audit trail as maturity milestone, reproducible-ish builds goal, SBOM (CycloneDX) published per release |
| Physical/other-admin | Keychain scope, DB theft | Machine-scope secrets readable only by root/SYSTEM; config.db permissions 0600 root; optional at-rest passphrase (OD-6) |
| The daemon itself | Memory bugs, logic bugs | Rust, `#![forbid(unsafe_code)]` outside PAL FFI (FFI isolated in `-sys`-style modules with review), clippy pedantic, fuzzing on all wire-facing parsers, security review gate per release (§14) |

### 10.3 AuthN/AuthZ model

Principals: local OS users (mapped), remote users (gateway accounts), API keys, plugins, automation (system principal), assistant. Roles v1: `admin`, `operator` (day-to-day ops, no security-settings writes), `viewer`, `assistant`. Full RBAC matrix is an appendix-level artifact generated from proto annotations (single source of truth ⇒ docs and enforcement can't diverge). Sensitive ops (`firewall.write`, `killswitch.persistent`, `plugin.install`, `gateway.enable`, `secrets.export`) additionally require recent re-auth (sudo-mode, 5 min) on remote sessions.

### 10.4 Cryptography inventory (no ad-hoc crypto anywhere)

| Use | Primitive | Notes |
|---|---|---|
| Secret blobs at rest | AES-256-GCM, key in OS keychain | nonce-misuse discipline: random 96-bit + key rotation support |
| Passwords | Argon2id (tuned at first-run benchmark) | per-secret salt, upgradeable params recorded per-hash |
| Tokens | JWT EdDSA (Ed25519) | key rotation with `kid`, short TTL |
| Plugin/update signatures | Ed25519 | offline roots, revocation list shipped in manifests |
| TLS | rustls, TLS 1.3 (1.2 floor for gateway if user demands) | cert pinning for our cloud endpoints only |
| Hashing/integrity | BLAKE3/SHA-256 | audit chain, file manifests |
| Randomness | OS CSPRNG (`getrandom`) | passwords/PSK generation with unambiguous alphabets |

### 10.5 Privacy commitments (product-level)

Telemetry & crash reporting **opt-in only** (off by default), with visible payload preview; analytics local by default; one-click "delete all history"; data retention windows user-controlled; no traffic payload capture ever in core; documentation of exactly what cloud sync stores when enabled (encrypted client-side, OD-7). These are testable claims — CI includes a "no unexpected egress" integration test (daemon under network capture must make zero connections with defaults beyond user-configured upstreams/updates check).

### 10.6 Daemon hardening (Linux exemplar; analogues per OS)

systemd unit: `ProtectSystem=strict` + explicit RW paths, `ProtectHome=yes`, `PrivateTmp=yes`, `NoNewPrivileges` where child-exec allows, `CapabilityBoundingSet` trimmed, `RestrictAddressFamilies`, `SystemCallFilter` allowlist (hardening milestone M-H1, after functionality stabilizes — a wrong seccomp list is an outage machine). Windows: service with unnecessary privileges removed from token where feasible. macOS: hardened runtime, minimal helper surface.

### 10.7 Untrusted-input hardening checklist (applies to every listener)

Bounded reads, length-prefix validation, per-source rate limiting, allocation caps per request, fuzz target in-tree (**a listener without a fuzz target fails CI policy**), structured decode via well-reviewed crates, timeouts on every await touching the network.

---

## 11. Resilience & Failure-Mode Catalog

The product's worst outcomes are not crashes — they are *states we leave the user's machine in*. Every failure mode below has a designed mitigation; each becomes at least one integration test (fault injection, §14).

| # | Failure | Consequence if unhandled | Designed mitigation |
|---|---|---|---|
| F-1 | Daemon crashes with firewall rules applied | Rules are desired state — fine — but half-applied *revision* could block traffic unexpectedly | Atomic rule-set swap only (§8.3); startup deadman reconciles active revision vs. desired |
| F-2 | Daemon crashes with `while-connected` kill switch active | **User offline forever** | Kill-switch rules carry a mode marker; deadman on next start removes non-persistent kill-switch rules; watchdog: if daemon doesn't come back (service restart policy exhausted), OS-level cleanup — Linux: `ExecStopPost` + a boot-time oneshot unit that clears non-persistent NetworkHub tables if the daemon isn't running; Windows: service recovery actions + on-start check; documented manual escape hatch (CLI standalone path, §8.2) |
| F-3 | Power loss mid-mutation (e.g. after enabling `ip_forward`, before NAT rule) | Silent config drift, security hole (forwarding without firewall) | Write-ahead intent journal with prior values; replay/undo on boot (§5.3) |
| F-4 | Daemon killed −9; hostapd/openvpn orphaned | Ghost AP, ghost VPN, port squatting | Children in same process group/job object (kill-on-close); PID+start-time recorded; startup adopts-or-kills matching orphans |
| F-5 | `config.db` corrupted | Total loss of config, or crash-loop | Quarantine + backup restore + safe degraded boot with empty desired state (fail-open) (§7.3) |
| F-6 | Another agent (firewalld, AV, GPO, IT script) fights our rules | Infinite reconcile war, CPU burn, log spam | Interference detection: back off after 3 cycles/60 s, mark `conflicted`, name the suspect (§5.3) |
| F-7 | Hotspot DHCP leaks onto the upstream LAN (mis-bind, bridge misconfig) | We DHCP-poison the office network — reputationally fatal | Interface-scoped bind only + startup self-test (probe on non-managed segments must not see us) + refuse-on-foreign-DHCP-detected (§8.4) |
| F-8 | Update bricks the daemon | Networking features dead, maybe kill-switch stuck | Staged apply: new binary starts in `--verify` mode (loads config, probes, exits OK) before switchover; automatic rollback to previous version on failed health check; kill-switch state preserved across update path tested explicitly |
| F-9 | `ip_forward`/sysctl values we changed persist after uninstall | Host silently acts as router | All sysctl changes journaled with prior value; uninstaller + deadman restore them |
| F-10 | Gateway exposed to the internet by user error | Brute force → network control | Off by default; enabling to `all` interface shows explicit warning + requires strong password + rate limiting + optional auto-disable after N days of no use |
| F-11 | Clock jumps (NTP, DST, manual) | Schedules misfire, quotas reset wrongly, JWT flaps | Monotonic clocks for rates/timeouts; UTC + tz-database for schedules; quota windows anchored to wall-clock with jump-detection (>5 min jump → recompute, never double-charge); JWT clock-skew leeway 60 s |
| F-12 | Disk full | DB writes fail; daemon crash-loop | Metrics eviction first (§8.5 E-27), config writes ≤ few KB with explicit failure event; daemon stays up read-only |
| F-13 | Interface renamed/re-enumerated (USB adapter re-plug, predictable-name changes) | Rules/profiles bound to stale names | Interfaces referenced by stable identity (MAC+type+path heuristic) resolved at compile-time, re-resolved on hotplug events |
| F-14 | Sleep/resume mid-VPN/hotspot | Zombie states, battery drain from reconnect storms | Power events subscribed on all OSes; state machines have explicit `Suspended` handling (§8.1 E-4, §8.2 E-14) |
| F-15 | Two NetworkHub features conflict (VPN kill switch + hotspot sharing + firewall deny-all template) | User-inflicted lockout with confusing symptoms | **Cross-module desired-state validation** pass before apply: known-bad combinations rejected with explanation (e.g. kill switch blocks hotspot client traffic unless share-VPN routing is on) |
| F-16 | Locale/encoding: non-ASCII SSIDs, IDN domains, Windows OEM codepages in child output | Mojibake, spoofing, parser crashes | UTF-8 everywhere internally, byte-length validation (E-7), IDN displayed as punycode+unicode pair in security contexts, child-process output decoded per-OS explicitly |

**Global invariant (AD-5) restated as a test:** *from any single crash/kill/power-loss at any point, the next daemon start (or, failing that, the boot-time cleanup path) returns the system to either the user's desired state or the OS pre-NetworkHub state — never a third thing.* CI fault-injection suite kills the daemon at randomized points inside every mutating flow and asserts this.

---

## 12. UI Architecture

### 12.1 Structure (Flutter)

- **Stack**: Flutter (Material 3, light/dark, adaptive), riverpod (state), go_router (nav), freezed (models — largely generated from proto), grpc-dart channel to daemon, flutter_secure_storage only for UI-local prefs (real secrets live daemon-side), fl_chart initially (revisit if perf demands custom painters for live charts).
- **Layered**: `presentation → application (controllers/providers) → data (generated gRPC clients + repository wrappers)`. No widget ever imports gRPC directly; repositories expose typed streams that already implement snapshot+delta resync (§5.4) and reconnect/backoff — connection state is a first-class app-wide banner ("Service unavailable — retrying"), not per-screen spinners.
- **Capability-driven rendering** (AD-6): a `Capability` provider gates every feature widget; `unsupported` renders an explainer card (reason + link), `degraded` renders with caveat chip. No platform `if (Platform.isWindows)` in feature code.
- Long operations: RPC returns fast; progress arrives as events → UI shows operation timeline (e.g. hotspot preflight results itemized).
- **Elevation UX**: the UI never elevates. If the daemon isn't installed/running: a dedicated installer/repair flow launches the OS-appropriate privileged installer once (UAC/pkexec/osascript admin prompt), not per-operation prompts.

### 12.2 UX safety patterns

Destructive/risky actions (deny-all templates, persistent kill switch, gateway enable) use explicit confirmation with consequence text + the revert window pattern (§8.3 lockout guard) surfaced in UI. Every error shown = human sentence + `remediation` from the error contract (§6.3) + copyable diagnostic ID that greps the log.

### 12.3 Accessibility & i18n (day one, cheap now / expensive later)

Semantics labels on all controls, keyboard-complete navigation, contrast-checked palettes in both themes, animation-reduced mode. All strings через ARB from the first commit (English only shipped; structure ready); RTL smoke test in CI (golden tests in ar locale). Units/locale-aware formatting for bytes/dates (binary vs decimal units — user setting, default SI decimal with tooltip).

### 12.4 CLI

`networkhub` CLI ships v1 (not an afterthought): status, hotspot start/stop, vpn connect, device list/block, firewall apply/rollback, capability report, `doctor` (diagnostics bundle), `debug tasks`. It is the headless admin story and the support-debugging story. Machine-readable `--json` on every command.

---

## 13. Packaging & Distribution

### 13.1 Windows

MSI (WiX v4): installs service + UI + CLI, creates `NetworkHub Operators` group, service recovery policy, signed (EV cert). Per-machine install only (service requirement). MSIX deferred (packaged-service friction, AD-14). Winget manifest published from CI.

### 13.2 Linux

- **deb + rpm** (primary): daemon systemd unit (+ boot-cleanup oneshot, F-2), UI desktop entry, CLI; postinst creates `networkhub` group; apt/dnf repository hosted by us (signed, with key rotation story) for updates.
- **AppImage**: UI+CLI only, connects to a separately installed daemon (clearly documented); for "try it" flows.
- Snap/Flatpak: deferred — strict confinement is incompatible with our daemon; classic snap possible later. Raspberry Pi: same deb via our apt repo (arm64/armhf as needed).

### 13.3 macOS

DMG containing app; on first run, installs privileged helper via `SMAppService` (admin prompt once); helper = the daemon (launchd). Hardened runtime, Developer ID signing, notarization, stapling — all in CI. Homebrew cask published.

### 13.4 Updates (AD-15)

- Channels: `stable` / `beta`. Manifest (TUF-style roles: offline root, online targets key; anti-rollback counters) fetched over pinned TLS; delta optional later.
- Self-update applies to MSI/DMG/AppImage installs; deb/rpm users update via repo (updater only *notifies*). Never mix.
- Order: verify sigs → stage → daemon `--verify` self-check → swap → health check → rollback on failure (F-8). UI and daemon ship together but tolerate one-version skew via `min_client_version` (§6.3).

### 13.5 Uninstall contract (tested in CI per-OS)

Removes: service/unit/helper + boot-cleanup hooks, all firewall artifacts (WFP provider/sublayer/filters; nft tables; pf anchors), DNS/system-config restorations, sysctl restorations (F-9), runtime dirs. Asks about: config/keychain data (default keep, offer purge). Leaves: nothing else. A machine after install+use+uninstall must diff clean (integration test uses VM snapshots).

---

## 14. Testing Strategy & CI/CD

### 14.1 Test pyramid (what runs where)

| Layer | Scope | Runs on |
|---|---|---|
| Unit | pure logic: rule compiler, parsers, state machines, schedulers, quota math | every commit, all OS runners |
| Property/fuzz | DHCP/DNS/portal/import parsers (cargo-fuzz targets, §10.7); rule-compiler round-trips; proptest on state machines | fuzz smoke every PR; long fuzz nightly |
| PAL-mock integration | full reconciler flows incl. fault injection (random kill points, F-catalog assertions) against pal-mock | every commit, no privileges needed |
| Linux netns integration | real nftables/tc/DHCP/DNS/WG in network namespaces (mac80211_hwsim for Wi-Fi AP paths!) — the crown jewel: full hotspot+portal+quota E2E without hardware | every PR (Linux runner, root in container/VM) |
| OS-matrix E2E | real service install, VPN connect, firewall apply, uninstall-diff-clean, on Windows/macOS/Ubuntu VMs (snapshot-revert) | nightly + release gate |
| Hardware lab (manual+scheduled) | real Wi-Fi adapters matrix (Intel AX, Broadcom Pi, common Realtek USB), STA+AP combos, real phones as clients | pre-release checklist |
| UI | widget tests, golden tests (light/dark/RTL), integration against a daemon-simulator binary (pal-mock daemon, also used for UI dev without root — **daemon-sim is a first-class dev tool**) | every commit |
| Load/soak | 100+ simulated clients (netns), 72 h soak with leak tracking (RSS, fd, task counts) | weekly + release gate |
| Security | cargo-deny/audit, gateway DAST (zap baseline), dependency review, secrets-scan, SBOM diff review; annual external pentest once gateway/portal ship | per PR / release / annual |

Coverage target ≥90 % on `nh-state`, `nh-firewall` compiler, parsers, quota/schedule math (the logic that corrupts silently); no vanity global number.

### 14.2 CI/CD (GitHub Actions)

PR: lint (fmt, clippy -D warnings, dart analyze) → unit+mock-integration (3 OS) → netns suite → proto-breaking-change check (buf) → license/advisory gates → build all targets. Release: tag → full matrix builds (x86_64+aarch64) → sign (Authenticode via HSM-backed key in secured runner; notarize macOS; GPG-sign repos) → package (MSI/deb/rpm/AppImage/DMG) → E2E gate on packaged artifacts → staged rollout (beta → stable) with update-manifest publishing. Release artifacts carry SBOM + checksums + signatures; build provenance (SLSA-style attestation) as maturity goal.

---

## 15. Observability

- **Logging**: `tracing` with structured JSON file output (rotating, size-capped) + human console mode; levels per-module at runtime via CLI; **redaction layer** (PSKs, keys, tokens typed as `Secret<T>` which cannot Display — compile-time enforcement, plus regex scrub as belt-and-braces).
- **Diagnostics bundle** (`networkhub doctor --bundle`): logs, capability report, sanitized config, versions, recent events — with an interactive redaction preview; this is the support workflow.
- **Self-metrics**: daemon exports its own health (task states, reconcile latencies, queue depths, child restarts) via the API; optional Prometheus text endpoint on the gateway (off by default) for the Grafana crowd; OpenTelemetry export optional behind a flag.
- **Crash reporting**: opt-in Sentry (self-hosted-friendly), minidumps scrubbed, symbol upload in CI.

---

## 16. Licensing Strategy

- **Core repo**: Apache-2.0 (patent grant matters for network code) for all first-party crates/UI. CLA or DCO decision = OD-8.
- **GPL isolation (AD-8)**: `openvpn` (GPLv2) bundled as an unmodified separate executable, process-boundary only — no linking; its source offer/attribution shipped in third-party notices. hostapd (BSD) fine. wintun / wireguard-nt: prebuilt signed drivers with specific redistribution terms — **legal review required before bundling** (R-6); fallback: guided component download at first use (like many WG apps do).
- **"WireGuard" is a registered trademark** (Jason Donenfeld) — naming/claims must follow the trademark policy; same care for "OpenVPN".
- Third-party data: OUI DB (IEEE, fine), GeoIP (MaxMind GeoLite requires account/EULA + redistribution limits → user-fetched with our tooling, or licensed commercial feed for enterprise, OD-5), domain filter lists (varied licenses — plugin-delivered, license shown per list).
- Dual licensing (open core + commercial enterprise features per roadmap Ph.3) requires: clean module boundary (enterprise features live in separate crates/repo behind the plugin/provider interfaces where possible), copyright ownership clarity from first external contribution (hence OD-8 now, not later).
- `cargo-deny` license allowlist in CI from day one; every release publishes THIRD-PARTY-NOTICES.

---

## 17. Repository Layout & Engineering Process

```
networkhub/
├── apps/
│   └── desktop/                  # Flutter app (mobile/web join in their phases)
├── crates/                       # §5.1 workspace
├── proto/                        # .proto sources (nh-proto builds from here)
├── sdk/
│   ├── plugin-rust/              # plugin SDK + examples
│   └── plugin-ts/
├── packaging/
│   ├── windows-msi/  linux-deb/  linux-rpm/  appimage/  macos/
├── docs/
│   ├── adr/                      # ADR-0001..., template included; AD-1..15 backfilled as ADRs
│   ├── architecture/             # this document, split per-section as it grows
│   ├── ops/                      # install, headless, troubleshooting
│   └── security/                 # threat model, disclosure policy (security.md), audit reports
├── tests/
│   ├── netns/                    # Linux namespace E2E harness
│   ├── e2e/                      # OS-matrix packaged-artifact tests
│   └── fault/                    # fault-injection scenarios (F-catalog)
├── tools/                        # daemon-sim, devcerts, release scripts
└── .github/workflows/
```

Process rules:
- **ADRs are mandatory** for decisions touching: privilege boundary, wire contracts, storage schema, crypto, licensing. This document's AD-1..15 get backfilled as ADR-0001..0015 at repo init.
- Conventional commits + semver; proto and DB migrations reviewed by a designated owner (CODEOWNERS).
- `SECURITY.md` with disclosure policy + security contact from the first public release.
- Definition of Done for any feature: capability entry + error remediation texts + audit events + tests at the layers in §14.1 + docs page + F-catalog review ("what new ways can this strand the user?").

---

## 18. Roadmap with Exit Criteria

Reordered from `Idea.md` to retire the highest-risk unknowns first (privilege plumbing, capability probing, reconciliation) inside a **walking skeleton**, then vertical feature slices. Each milestone has objective exit criteria — no "90 % done" limbo.

### M0 — Walking Skeleton (the architecture, proven end-to-end)
Daemon installs & runs as service on all 3 OSes; UDS/pipe gRPC with peer-cred auth; capability probe + matrix API; Flutter UI connects, renders capabilities, live interface list + throughput chart (first real event stream); CLI `status`; config.db with migrations; intent journal + deadman (trivial case); CI matrix + netns harness skeleton + uninstall-diff-clean test.
**Exit:** fresh VM → install → see live network state in UI → uninstall → clean diff. On all three OSes.

### M1 — MVP part 1: VPN (highest user value, least hardware variance)
WireGuard (all OSes) + OpenVPN (bundled, sanitized import); profiles, connect/disconnect, status, DNS override; kill switch `while-connected` incl. F-2 deadman test; auto-reconnect; import via file/QR.
**Exit:** F-2/F-3 fault tests green; VPN leak test suite (DNS/IPv6/reconnect-window) green on all OSes; soak 72 h with flapping upstream.

### M2 — MVP part 2: Hotspot + Devices + Monitoring (Linux full; Windows native-API; macOS assisted)
Hotspot profiles/preflight/state machine; Linux DHCP+DNS serving; device list with identity/merge; block/pause via firewall classes; live + historical charts; retention/rollups.
**Exit:** E-1..E-8 tests green; hwsim E2E green; hardware-lab checklist passed on reference adapters (incl. Pi); F-7 self-test demonstrably refuses foreign-segment serving.

### M3 — MVP release hardening
REST gateway (localhost/LAN opt-in) + API keys; `doctor`; packaging polish (signing, notarization, repos); update pipeline with rollback; docs; security review + fuzz corpus baseline; public beta.
**Exit:** external smoke testers on all OSes; crash-free sessions >99 %; update+rollback exercised in production channel.

### Phase 2 — Professional (post-MVP, each its own milestone)
Firewall rule UI + revisions + lockout guard → per-device shaping/quotas (Linux) → DNS filtering + DoH/DoT + host-resolver mode → captive portal (Linux) → automation engine → plugin SDK stabilization (provider interfaces dogfooded internally in Ph.2, SDK public at its end) → QR onboarding polish.

### Phase 3 — Enterprise
Multi-user/RBAC full matrix, SSO (OIDC first; SAML if customer-driven), audit export, enterprise policy templates, cloud sync (E2E-encrypted, OD-7), HA story *scoped honestly* (config replication + cold standby — not live network-state failover), web dashboard (the gateway grows a first-class SPA using the TS SDK).

### Phase 4 — Ecosystem
Mobile controllers (Flutter, same gRPC-over-TLS via gateway), plugin marketplace (review pipeline + signing service), AI assistant on the §8.9 substrate, third-party integrations.

**Cross-cutting hardening milestones**: M-H1 seccomp/least-priv tightening (post-M2 stability); M-H2 external pentest (pre-Ph.2 GA of gateway/portal); M-H3 reproducible builds/SLSA.

---

## 19. Risk Register

| ID | Risk | L×I | Mitigation / trigger |
|---|---|---|---|
| R-1 | Wi-Fi driver/adapter variance makes hotspot flaky on user hardware | H×H | Capability probing (AD-6), hardware lab matrix, published compatibility list, in-UI probe report users can share; brutal honesty in marketing |
| R-2 | macOS constraints disappoint users expecting parity | H×M | AD-12 messaging from the first screenshot; macOS value led by VPN/firewall/monitoring |
| R-3 | Windows per-device shaping/quota gap undermines "device management" pitch | M×M | Accounting-only + block/pause still works; evaluate WFP callout driver investment only on demand signal (budget: driver dev + EV/WHQL) |
| R-4 | Fighting OS/network agents (firewalld, AV, GPO) creates unfixable support load | M×H | Interference detection + named-suspect diagnostics (F-6); documented coexistence matrix; `doctor` bundle |
| R-5 | Scope explosion (the Idea list is 3 products) | H×H | This document's phase gates; Definition of Done; features enter only via roadmap review |
| R-6 | wintun/wireguard-nt redistribution or signing issue late in dev | M×H | Legal review during M0 (before code depends on it); fallback = guided download flow |
| R-7 | Privileged daemon vulnerability = root RCE | L×H | §10 posture, fuzzing gates, pentest M-H2, fast-patch update channel, minimized RPC surface |
| R-8 | Solo/small-team bus factor on 3-OS PAL expertise | H×M | pal-mock keeps 90 % of dev OS-independent; per-OS knowledge captured in ADRs + PAL docs as it's learned |
| R-9 | SQLite on SD (Pi) wears cards / corrupts on power cuts | M×M | §7.4 batching, WAL, quick_check+backup+degraded-boot (F-5) |
| R-10 | Update infrastructure compromise | L×H | TUF roles, offline root, staged rollout, anti-rollback, revocation drill documented |

### Compatibility test matrix (minimum hardware lab)

Intel AX2xx (Win/Linux), Broadcom (Pi 4/5), one mainstream Realtek USB (worst-case driver), Apple Silicon Mac, one Win11 ARM device (later); clients: recent iOS + Android (MAC randomization on), Windows laptop, smart-home device (DHCP oddities).

---

## 20. Open Decisions

To resolve before/during the milestone that needs them:

| ID | Decision | Needed by | Leaning |
|---|---|---|---|
| OD-1 | Working name / trademark check ("NetworkHub" is likely contested) | Public beta | do the search early; renames get costlier |
| OD-2 | Userspace WireGuard engine: fork boringtun vs alternatives vs own (maintenance vs control) | M1 | boringtun-family fork with upstream tracking |
| OD-3 | DHCP/DNS: embed existing crates (audit them) vs in-house minimal servers | M2 | in-house DHCPv4 (small RFC surface, we need tight control); DNS: build on `hickory-dns` libraries |
| OD-4 | gRPC-Dart maturity check on all desktop targets vs fallback (JSON-RPC over same sockets) | M0 spike | spike in week 1; contract stays proto either way |
| OD-5 | GeoIP source & licensing for country blocking | Ph.2 firewall | user-fetched GeoLite + commercial option for enterprise |
| OD-6 | Optional at-rest passphrase for config.db (headless boxes without keychain) | M3 | offer file-key + TPM-backed where available |
| OD-7 | Cloud sync design: E2E encryption scheme & conflict model (UUIDv7 + CRDT-lite vs last-writer-wins per document) | Ph.3 | per-document LWW with vector stamps; full CRDT only if collaboration demands |
| OD-8 | DCO vs CLA; copyright aggregation for dual-licensing | first external PR | DCO + explicit dual-license grant note |
| OD-9 | PDF report generation placement (UI-side rendering vs headless service for scheduled emailed reports) | Ph.2 analytics | UI-side v1; gateway-side render worker when scheduled reports arrive |

---

*End of blueprint. Amendments happen via ADRs in `docs/adr/`; this document is updated to reference them. First implementation step when we begin: Milestone 0 walking skeleton, starting with the proto contracts and the PAL trait definitions — the two artifacts everything else compiles against.*


