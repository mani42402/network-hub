# NetworkHub — Execution Plan

**Companion to:** [ARCHITECTURE.md](ARCHITECTURE.md) · **Version:** 1.0 · **Execution model:** Solo developer + Claude Code · **Environment:** Windows + Linux + macOS machines available, Wi-Fi hardware lab available

This document turns the architecture into an ordered build queue. Every unit of work is a **Work Package (WP)** with dependencies, deliverables, exit criteria, and an explicit statement of who verifies what. Architecture decisions are referenced as `AD-n`, failure modes as `F-n`, edge cases as `E-n`, open decisions as `OD-n`, risks as `R-n` — all defined in ARCHITECTURE.md.

**Ground rule for this project: a work package is not done when the code exists. It is done when its exit criteria are demonstrably met on real machines.** No WP is marked complete on the strength of "it compiles."

---

## Table of Contents

1. [How We Work](#1-how-we-work)
2. [Prerequisites — Start These Today](#2-prerequisites--start-these-today)
3. [Phase Map & Honest Timeline](#3-phase-map--honest-timeline)
4. [Phase 0 — Foundations](#4-phase-0--foundations)
5. [Milestone M0 — Walking Skeleton](#5-milestone-m0--walking-skeleton)
6. [Milestone M1 — VPN](#6-milestone-m1--vpn)
7. [Milestone M2 — Hotspot, Devices, Monitoring](#7-milestone-m2--hotspot-devices-monitoring)
8. [Milestone M3 — MVP Hardening & Public Beta](#8-milestone-m3--mvp-hardening--public-beta)
9. [Phase 2 — Professional](#9-phase-2--professional)
10. [Phase 3 — Enterprise](#10-phase-3--enterprise)
11. [Phase 4 — Ecosystem](#11-phase-4--ecosystem)
12. [Continuous Tracks](#12-continuous-tracks)
13. [Decision Calendar](#13-decision-calendar)
14. [Descoping Levers](#14-descoping-levers)
15. [Progress Tracking](#15-progress-tracking)

---

## 1. How We Work

### 1.1 Roles

| Role | Who | Responsibility |
|---|---|---|
| **Implementer** | Claude Code (me) | Writes all code, tests, configs, docs. Proposes designs within the architecture. Runs everything runnable in the current session's environment. |
| **Director / Verifier** | You | Approves gates, resolves `OD-n` decisions, runs hardware-dependent verification, handles procurement (certs/accounts), makes product calls. |
| **Referee** | ARCHITECTURE.md | When we disagree or drift, the document wins — or we amend it via an ADR. No silent deviation. |

### 1.2 The cross-platform loop (important — this shapes everything)

I execute on whichever machine you run me on. Three verification mechanisms, used deliberately:

| Mechanism | Use for | Notes |
|---|---|---|
| **CI (GitHub Actions, 3-OS matrix)** | Default verification for all platform code | Set up in WP-0.2 *before* product code. Every push builds and tests on Windows, Linux, macOS. This is the workhorse — it catches ~80% of cross-platform breakage without you doing anything. |
| **Claude Code on the target machine** | Hard platform debugging: WFP filters, hostapd, NetworkExtension, service install quirks | For these sessions, run me on that box. Platform WPs below are tagged **`[on-target]`** where this is expected. |
| **You, by hand** | Hardware lab (Wi-Fi adapters, real phone clients), VM snapshot install/uninstall diffs, UX judgment | Tagged **`[you-verify]`**. I'll always hand you an explicit, copy-pasteable test protocol — never "please test it." |

**Practical consequence:** keep the repo on git from day one (WP-0.1) and push often. The Windows/Linux boxes pull; CI verifies; we don't hand-carry files.

### 1.3 Work package lifecycle

```
Ready  →  I implement  →  Self-verify (tests+CI green)  →  Hand you a verification protocol
       →  You verify (if [you-verify]/[on-target])  →  GATE: approve or rework  →  Done
```

I will not start a WP whose dependencies aren't Done. If I'm blocked on a decision, I'll ask rather than guess — and I'll tell you what I'd pick if you'd rather I just chose.

### 1.4 Definition of Done (per ARCHITECTURE.md §17, enforced per WP)

A feature WP is Done only with **all** of:

1. Code + unit tests (coverage targets per §14.1 for the logic-heavy crates)
2. Capability matrix entry (AD-6) if it's platform-variable
3. Error contract entries with `remediation` text (§6.3) — every failure the user can trigger has a human sentence
4. Audit events for every state mutation (§7.2)
5. Tests at the layers §14.1 requires (unit / mock-integration / netns / E2E as applicable)
6. **F-catalog review**: "what new way can this feature strand the user offline or leave OS state behind?" — new failure modes get added to §11 and get a fault test
7. Docs page (user-facing) + ADR if an architectural decision was made
8. Uninstall contract still clean (§13.5) — if the feature touches OS state, the uninstall test covers it

### 1.5 Cadence & checkpoints

- **Per WP**: I report status with what's proven vs. assumed. No "should work."
- **Per milestone gate**: full exit-criteria walkthrough on real machines before the next milestone starts. These gates are not negotiable — they're the mechanism that prevents the "90% done for six months" failure.
- **Weekly**: PROGRESS.md updated (§15), risks re-rated, next-week queue confirmed.

### 1.6 Non-negotiable engineering rules

1. **Fail-open invariant (AD-5)** is checked at every WP touching OS state. The fault-injection suite grows with the code, not after it.
2. **No `sh -c` / shell interpolation ever** for child processes — argv arrays only (§4.3).
3. **Secrets are `Secret<T>`** — a type that can't `Display`. Enforced at compile time from the first secret we touch (WP-M1.1).
4. **No feature branches longer than ~3 days.** Contract-first (proto) makes this possible; long-lived branches on a project this size are where correctness goes to die.
5. **Every network-facing parser ships with its fuzz target in the same PR** (§10.7). CI policy, not a suggestion.
6. **`#![forbid(unsafe_code)]`** in every crate except designated FFI modules in `pal-*`, which carry a safety comment per `unsafe` block.

---

## 2. Prerequisites — Start These Today

These have **procurement lead times measured in days-to-weeks** and will block releases if started late. They run in parallel with Phase 0 coding — nothing here blocks writing code, but all of it blocks shipping.

| ID | Item | Lead time | Blocks | Notes |
|---|---|---|---|---|
| **PRE-1** | **Windows EV code-signing certificate** (or OV + SmartScreen reputation build-up) | 1–3 weeks (identity validation) | M3 release, all signed Windows builds | Requires business identity verification. EV avoids SmartScreen warnings from day one — worth it. Hardware token or cloud HSM (Azure Trusted Signing is the cheaper modern path). |
| **PRE-2** | **Apple Developer Program** membership | 1–7 days | macOS notarization, M3 | $99/yr. Needed for Developer ID signing + notarization. Get the Developer ID Application **and** Developer ID Installer certs. |
| **PRE-3** | **Domain + update-manifest hosting** | 1–3 days | M3 updater, apt/dnf repos | Needs a stable domain for pinned TLS (§10.4) and repo hosting. |
| **PRE-4** | **TUF signing root key ceremony** (offline root, online targets key) | 1 day, but must be deliberate | M3 update channel | Offline root on a hardware key (YubiKey), stored physically. Document the ceremony and the revocation drill (R-10). Do not improvise this later. |
| **PRE-5** | **Trademark search for "NetworkHub"** (OD-1) | 1–2 weeks for a proper search | Public beta, domain, marketing | Almost certainly contested — the name is generic and widely used. Renaming after beta is expensive. Do this before PRE-3 buys a domain. |
| **PRE-6** | **Legal review: wintun / wireguard-nt redistribution** (R-6, AD-8) | 1–2 weeks | M1 Windows WireGuard | If we can't bundle, fallback is guided first-run download. Must know **before** M1 code depends on it. Also review OpenVPN GPLv2 bundling posture and WireGuard/OpenVPN trademark policies (§16). |
| **PRE-7** | **GitHub org + repo + Actions minutes budget** | Same day | Everything | Private repo initially. Actions on 3 OSes burns minutes fast — budget for a paid tier or self-hosted runners on your own machines (you have all three — self-hosted runners are a genuinely good fit here and free). |
| **PRE-8** | **Hardware lab inventory** | 1–2 weeks shipping | M2 hotspot validation | See §2.1. |
| **PRE-9** | **Sentry (self-hosted or SaaS) + symbol upload** | 1 day | M3 crash reporting | Opt-in only (§10.5). Can defer to M3. |

### 2.1 Hardware lab shopping list (PRE-8)

Per ARCHITECTURE.md §19 compatibility matrix. You have the machines; this is the Wi-Fi/client side:

| Item | Why | Priority |
|---|---|---|
| Intel AX200/AX210 adapter (or a laptop with one) | Best-case Linux/Windows AP support, STA+AP concurrency reference | **Must** |
| Raspberry Pi 4 or 5 + SD cards | brcmfmac AP path, headless story, SD-wear validation (R-9), aarch64 build target | **Must** |
| Cheap Realtek USB Wi-Fi adapter (e.g. RTL8188/8812 family) | Worst-case driver: lies about capabilities, out-of-tree drivers. If we survive this, we survive the field. | **Must** |
| iPhone (recent iOS) + Android phone (recent) | MAC randomization (the device-identity edge case, §7.2), captive portal sheet behavior (RFC 8908/8910) | **Must** |
| A cheap smart-home device (plug/bulb) | DHCP oddities, non-standard clients | Should |
| USB Ethernet adapter | Interface hotplug/rename testing (F-13), Ethernet sharing | Should |
| Managed switch or second router | Upstream LAN conflict scenarios (E-2 subnet collision, F-7 foreign DHCP detection) | Nice |

### 2.2 Toolchain (install on all three machines — Phase 0 blocks on this)

Rust (pinned via `rust-toolchain.toml`), Flutter SDK (pinned), `protoc` + `buf`, Node (for TS SDK codegen), platform tooling: WiX v4 + Windows SDK (Win), `nftables`/`hostapd`/`iproute2`/`libnl` dev headers (Linux), Xcode CLT (macOS). I'll produce exact per-OS setup scripts in WP-0.1.

---

## 3. Phase Map & Honest Timeline

### 3.1 The honest version

You asked for the complete product with no shortcuts. Here is what that actually costs, solo with AI assistance, at near-full-time effort:

| Phase | Scope | Estimate | Cumulative |
|---|---|---|---|
| **Phase 0** | Foundations, CI, spikes | 2–3 weeks | ~3 wks |
| **M0** | Walking skeleton, 3 OSes | 5–7 weeks | ~2.5 months |
| **M1** | VPN (WireGuard + OpenVPN, kill switch, leak prevention) | 6–9 weeks | ~4.5 months |
| **M2** | Hotspot, devices, monitoring, firewall, DHCP/DNS | 9–12 weeks | ~7.5 months |
| **M3** | Gateway, packaging, signing, updates, security review, beta | 5–7 weeks | **~9 months → MVP public beta** |
| **Phase 2** | Firewall UI, shaping/quotas, DNS filtering, captive portal, automation, plugin SDK | 6–8 months | ~16 months |
| **Phase 3** | RBAC, SSO, cloud sync, audit export, web dashboard | 6–9 months | ~23 months |
| **Phase 4** | Mobile apps, marketplace, AI assistant, integrations | 6+ months | **~2.5 years → complete** |

**Read that honestly.** AI assistance makes code generation fast; it does not compress:
- Waiting for driver-specific bugs to reproduce on real hardware
- Certificate/notarization/procurement lead times
- Soak tests (72h is 72h)
- Your review and verification bandwidth
- The genuinely slow parts: WFP semantics, hostapd tuning, macOS entitlement fights

The MVP at ~9 months is a real, shippable, production product — VPN + hotspot + monitoring + device management, signed and updatable on three platforms. That's the number that matters most. Everything after is expansion on a proven base.

### 3.2 Sequencing rationale (why this order)

- **Phase 0 before code**: CI and spikes first because a failed spike (especially SPIKE-A, §4.3) changes core architecture. Discovering that in month 3 is a disaster; in week 2 it's a Tuesday.
- **M0 before features** (your call, and the right one): the privilege boundary, IPC, capability plumbing, and uninstall-clean contract are load-bearing for every feature. Retrofitting them under shipped features is the single most expensive mistake available to us.
- **M1 (VPN) before M2 (hotspot)**: VPN has the highest user value per unit of risk. It's mostly software — minimal hardware variance — so it validates the daemon/reconciler/state-machine architecture under real conditions before we add the chaos of Wi-Fi drivers. It also forces us to build the firewall primitives (kill switch) that M2's device blocking then reuses.
- **M2 last in MVP**: highest variance, needs the hardware lab, and benefits from every lesson learned in M1.

---

## 4. Phase 0 — Foundations

**Goal:** A repo that builds on three OSes in CI, with the riskiest unknowns resolved before they can cost us a rewrite.
**Duration:** 2–3 weeks · **Gate:** CI green on 3 OSes + all spikes concluded with written findings.

### WP-0.1 — Repository & workspace skeleton
**Depends:** PRE-7 · **Estimate:** 2–3 days

**Deliverables:**
- `git init`, GitHub repo, branch protection, CODEOWNERS
- Cargo workspace with all crates from §5.1 as empty stubs with correct dependency edges (the dependency rules are enforceable from day one)
- `rust-toolchain.toml`, `.editorconfig`, `rustfmt.toml`, `clippy.toml` (pedantic), Flutter version pin
- `LICENSE` (Apache-2.0), `SECURITY.md`, `CONTRIBUTING.md`, `README.md`
- `docs/adr/` with template + **ADR-0001..0015 backfilled from AD-1..AD-15** (ARCHITECTURE.md §1) — this is how the decisions become amendable rather than folklore
- Per-OS dev setup scripts (`tools/setup-{windows,linux,macos}.{ps1,sh}`)
- `cargo-deny` config with license allowlist (§16) — active from the first dependency

**Exit criteria:** `cargo build --workspace` succeeds on all three machines. `cargo deny check` passes. Dependency-layer check script rejects an intentionally-bad edge (test it by adding one, watching it fail, removing it).

**`[you-verify]`:** Run the setup script on each machine, confirm clean build.

---

### WP-0.2 — CI pipeline skeleton
**Depends:** WP-0.1 · **Estimate:** 3–4 days

**Deliverables:**
- GitHub Actions: 3-OS matrix (windows-latest, ubuntu-latest, macos-latest) + aarch64 cross-build for Pi
- Jobs: fmt → clippy `-D warnings` → build → test → `cargo deny` → `dart analyze` (once Flutter lands in M0)
- Caching (sccache/Swatinem), concurrency groups, required-status-checks wired to branch protection
- **Self-hosted runner setup on your three machines** (optional but recommended — free, faster, and lets CI touch real hardware later for M2)
- Nightly workflow stub (fuzz, soak) — empty but wired

**Exit criteria:** A PR with a deliberate clippy warning is blocked by CI on all three OSes. Cache hit rate >70% on second run.

**Why this is before product code:** Every later WP's "verified" claim depends on CI being trustworthy. Building it first also means we never have a "make CI green" cleanup sprint.

---

### WP-0.3 — SPIKE-A: Dart ↔ Rust IPC transport (OD-4) 🔴 **HIGHEST RISK**
**Depends:** WP-0.1 · **Estimate:** 3 days, hard time-box · **`[on-target]` all three**

**The problem:** AD-2 says gRPC over Unix domain sockets / Windows named pipes. Rust (tonic) handles this fine. **Dart is the question mark.** `dart:io` supports Unix domain sockets on Unix-likes, but **Windows named pipe support is not available in `dart:io`**, and `grpc-dart` needs a custom transport connector to use anything other than TCP. Windows 10 1803+ does support `AF_UNIX` sockets natively, but Dart's support for them on Windows is the specific unknown.

**Tasks:**
1. Minimal tonic server + grpc-dart client over UDS on **Linux** and **macOS** — including streaming RPCs (we need server-streaming for events, §5.4)
2. Same on **Windows**: try (a) `AF_UNIX` via `dart:io`, (b) named pipe via `dart:ffi` shim wrapping `CreateFile`/`ReadFile`/`WriteFile` presented as a `Stream`/`IOSink` pair
3. Verify peer-credential authentication works end to end on each (`SO_PEERCRED`, `LOCAL_PEERCRED`, `GetNamedPipeClientProcessId`)
4. Measure: throughput and latency for a 1 Hz metrics stream and a burst of 1000 events

**Decision output (write into ADR-0016):** one of —
- **A1** UDS everywhere (best case, Dart supports AF_UNIX on Windows)
- **A2** UDS on Unix + `dart:ffi` named-pipe shim on Windows (most likely; ~200 lines of FFI, maintainable)
- **A3** Fallback: loopback TCP on 127.0.0.1 with a filesystem-permissioned token file for auth (weakens AD-2 — an open localhost port — and requires compensating controls: token rotation, strict `SO_REUSEADDR` handling, bind-to-loopback-only assertions). **Only if A1 and A2 both fail.**

**Exit criteria:** A working streaming echo demo, authenticated by peer credentials, on all three OSes, with the chosen approach documented and its rough edges listed.

---

### WP-0.4 — SPIKE-B..E: Platform privilege & lifecycle spikes
**Depends:** WP-0.1 · **Estimate:** 4–5 days total · **`[on-target]`**

Four small, parallel, throwaway spikes. Each answers "can we actually do the thing the architecture assumes?"

| Spike | Question | Success looks like |
|---|---|---|
| **SPIKE-B** (Windows) | Windows Service install/start/stop/uninstall from a Rust binary; named-pipe server with SDDL restricting to SYSTEM + Administrators + a custom group; query client token for group membership; service recovery actions | A service that survives reboot, restarts on crash, and rejects a non-group client |
| **SPIKE-C** (macOS) | `SMAppService` privileged daemon registration from an app bundle; helper runs as root under launchd; clean unregister; Developer ID signing chain works with an ad-hoc/dev cert | Install → helper responds → uninstall → nothing left in `launchctl print-disabled system` |
| **SPIKE-D** (Linux) | systemd unit with the §10.6 hardening set (`ProtectSystem=strict` etc.), UDS with group ownership, `sd_notify` readiness, **plus the boot-cleanup oneshot unit pattern for F-2** | Daemon starts hardened; a deliberately-orphaned nft table is cleaned by the boot unit when the daemon doesn't come back |
| **SPIKE-E** (all) | wintun (Win) / utun (macOS) / kernel-WG-via-netlink (Linux) — can we create a tunnel interface and pass a packet? Ties to PRE-6 licensing. | A ping through a loopback-ish tunnel on each OS |

**Exit criteria:** Written findings in `docs/adr/` for each; any that fail get a documented Plan B *before* M1 depends on them.

---

### WP-0.5 — Test infrastructure bootstrap
**Depends:** WP-0.2 · **Estimate:** 3–4 days

**Deliverables:**
- **VM snapshot harness**: scripted clean-VM creation + snapshot revert for Windows and Linux (and a macOS VM if feasible on your Apple Silicon box — Tart/UTM). This powers the uninstall-diff-clean test (§13.5) that gates M0.
- **System-state differ**: a tool that snapshots the "network-relevant" state of a machine (firewall tables/filters/anchors, routes, sysctls, services, DNS config, files in our dirs) and diffs two snapshots. This is the single most valuable test tool in the project — it's how we prove AD-5 and the uninstall contract, over and over, for years.
- **netns harness skeleton** (Linux): create namespaces, veth pairs, run the daemon inside, assert connectivity. `mac80211_hwsim` loading verified (this gives us virtual Wi-Fi for hotspot E2E without hardware — huge for M2).
- Nightly fuzz job wired (no targets yet).

**Exit criteria:** `tools/state-diff snapshot A` → make a manual firewall change → `snapshot B` → diff correctly identifies it, on all three OSes. netns harness brings up two namespaces that can ping each other. `mac80211_hwsim` creates virtual radios.

**Why here and not later:** if we build this after M2, we'll have spent months manually eyeballing "did the uninstall leave junk?" and we'll have gotten it wrong.

---

### WP-0.6 — Procurement kickoff
**Depends:** none · **Estimate:** your time, ~1 day of forms · **`[you-verify]`**

Kick off PRE-1 through PRE-6 and PRE-8. These run in the background for weeks. **PRE-5 (trademark) and PRE-6 (wintun legal) are the two that can force rework if they come back late and negative.**

---

### 🚦 Phase 0 Gate

- [ ] CI green on 3 OSes, blocking bad PRs
- [ ] SPIKE-A concluded → IPC transport decided and documented (ADR-0016)
- [ ] SPIKE-B/C/D/E concluded → each with a working demo or a documented Plan B
- [ ] State-differ tool works on all three OSes
- [ ] netns + hwsim harness functional
- [ ] ADRs 0001–0015 backfilled; procurement in flight
- [ ] **No known unknowns remaining that could invalidate AD-1, AD-2, or AD-8**

---

## 5. Milestone M0 — Walking Skeleton

**Goal:** Prove the entire architecture end to end with the thinnest possible feature: see your network interfaces and live throughput in the UI, served by a privileged daemon, on all three OSes, installed and uninstalled cleanly.
**Duration:** 5–7 weeks · **Gate:** fresh VM → install → live state in UI → uninstall → state-diff clean, on Windows, Linux, macOS.

Everything in M0 is load-bearing. There is no throwaway code here (unlike Phase 0 spikes).

### WP-M0.1 — `nh-types`: domain vocabulary
**Depends:** Phase 0 gate · **Estimate:** 3–4 days

**Deliverables:** IDs (UUIDv7 newtypes per entity), `Secret<T>` (no `Display`/`Debug` leak — compile-time secret hygiene, §1.6), the **error taxonomy** (canonical code + machine `reason` + human `detail` + `remediation`, §6.3), the **capability model** (`Capability{status, reason, details}`, AD-6), interface/address/MAC types with correct parsing and display, byte/rate units.

**Exit criteria:** Zero internal deps. 100% unit-tested parsing/display incl. IPv6 forms, MAC formats, non-ASCII SSID byte-length validation (E-7). `Secret<T>` provably doesn't leak — a compile-fail test (`trybuild`) asserts `println!("{}", secret)` does not compile.

---

### WP-M0.2 — `nh-proto`: contracts + codegen pipeline (AD-13)
**Depends:** WP-M0.1, SPIKE-A · **Estimate:** 4–5 days

**Deliverables:** `proto/networkhub/v1/{system,events,monitor}.proto` (M0 subset: capabilities, health, version, interface list, metric stream, event stream). Per-RPC **permission annotations** (custom proto options) that the authz interceptor reads — this is what makes §10.3's "docs can't diverge from enforcement" true. Codegen for Rust (tonic), Dart, TypeScript. `buf` breaking-change check in CI. Generated Dart/TS committed with a CI freshness check.

**Exit criteria:** Changing a proto and forgetting to regenerate fails CI. A breaking proto change is rejected by `buf`. Snapshot+delta stream contract (§5.4) is expressed in the proto itself (every stream's first message is a snapshot).

---

### WP-M0.3 — `nh-pal`: trait definitions + `pal-mock` (AD-7)
**Depends:** WP-M0.1 · **Estimate:** 4–5 days

**Deliverables:** The PAL traits from §5.2 — full signatures for all of them, even the ones M0 doesn't implement (the shape needs to be right before three backends grow against it). `pal-mock` with: simulated interfaces, injectable failures, injectable latency, simulated hotplug events, and a scriptable scenario format.

**Exit criteria:** `pal-mock` can simulate: interface up/down, address change, interface rename (F-13), and an operation that fails after partially mutating state (the input to fault testing). Reviewed against §3's platform notes so no trait assumes a capability one OS can't provide.

**Risk note:** getting trait shape wrong here is the most likely source of mid-project refactoring. I'll draft the traits and walk you through the three-OS implementability of each before writing backends.

---

### WP-M0.4 — `nh-state`: desired-state store, reconciler framework, event bus, intent journal (AD-4)
**Depends:** WP-M0.1, WP-M0.3 · **Estimate:** 6–8 days

The architectural heart. Generic framework, no feature-specific logic.

**Deliverables:**
- Desired-state store with typed documents + revisions
- Reconciler trait + runtime: backoff with jitter, circuit breaker, `degraded`/`conflicted` states, convergence events
- **Interference detection** (§5.3): 3 restore cycles in 60s → stop fighting, mark conflicted, name the suspect
- Event bus: typed envelopes, bounded per-subscriber rings, **drop-for-slow-consumers** (a stalled UI can never backpressure the engine), sequence numbers for resync
- **Intent journal**: write-ahead intent records with prior values; replay/undo API
- Task supervisor (named tasks, health, restart policy) backing `debug tasks`

**Exit criteria:** Full reconciler test suite against `pal-mock`, including: converge from arbitrary start state; survive injected failure at every step; interference back-off triggers correctly; slow subscriber drops events without stalling producers; intent journal replay restores prior state after simulated crash at N randomized points.

---

### WP-M0.5 — `config.db`: storage, migrations, resilience
**Depends:** WP-M0.1 · **Estimate:** 4–5 days

**Deliverables:** SQLx setup, migration runner (forward-only, versioned), M0 schema subset (settings, audit_log, intent_journal, schema_meta), **hash-chained append-only audit log** (§7.2), `VACUUM INTO` backup + rotation, `PRAGMA quick_check` on open, corruption → quarantine → restore-backup → **safe degraded boot with empty desired state** (F-5, and note: empty desired state = fail-open, which is exactly AD-5).

**Exit criteria:** Deliberately corrupt the DB → daemon boots degraded, restores from backup, emits the event, never crash-loops. Audit chain detects tampering (flip a byte in a row → verification fails, loudly). Migration from an older schema version works; a backup is taken first.

---

### WP-M0.6 — `nh-daemon`: lifecycle, IPC server, authn/authz, audit
**Depends:** WP-M0.2, WP-M0.4, WP-M0.5, SPIKE-A · **Estimate:** 6–8 days · **`[on-target]`**

**Deliverables:**
- Singleton lock per OS; startup sequence exactly as §4.2 (deadman check → DB → migrations → capabilities → reconcilers → IPC → ready signal)
- IPC server on the SPIKE-A-chosen transport, per-OS socket/pipe with correct permissions
- **Peer-credential authn** → principal mapping; **RBAC interceptor** driven by the proto permission annotations (WP-M0.2)
- Audit log wiring: every mutation, with actor/source
- `tracing` setup with JSON output, rotation, **redaction layer** (§15)
- Graceful shutdown with per-module teardown policy
- Child-process supervision primitives (process group / job object, kill-on-close — F-4) — no children yet, but the machinery lands here

**Exit criteria:** Unauthorized local user is rejected. Every RPC is permission-checked (a test enumerates all RPCs and asserts none is missing an annotation — this catches the "forgot to annotate the dangerous one" bug class permanently). Logs contain no secrets under a scrub test. Clean shutdown leaves no children.

---

### WP-M0.7 — Service integration + boot-cleanup (F-2 machinery)
**Depends:** WP-M0.6, SPIKE-B/C/D · **Estimate:** 5–6 days · **`[on-target]` all three**

**Deliverables:** systemd unit (hardened per §10.6, minus seccomp — that's M-H1) + **boot-cleanup oneshot unit**; Windows Service with recovery actions + on-start orphan check; launchd/`SMAppService` helper. The `networkhub` / `NetworkHub Operators` group creation per OS. `sd_notify`/SCM readiness signaling.

**Exit criteria:** Service survives reboot on all three. Kill -9 the daemon → it restarts per policy. **Simulate F-2**: leave a tagged firewall artifact, prevent the daemon from starting, reboot → boot-cleanup removes it. This is the safety net for the entire product; it gets built now, not when the kill switch ships.

---

### WP-M0.8 — Capability probe framework (AD-6)
**Depends:** WP-M0.3, WP-M0.6 · **Estimate:** 4–5 days · **`[on-target]`**

**Deliverables:** Probe framework + the M0 probes: OS/version, interface enumeration and types, admin privilege state, and the *structure* for all capability keys in §3.4 (returning `unsupported/unknown` where not yet implemented). Re-probe on hardware change events. Capability API + event on change.

**Exit criteria:** The full §3.4 matrix is returned by the API on all three OSes, with honest statuses and human-readable reasons for every non-supported entry. This artifact is what the UI renders — it must be complete in shape even when mostly "not yet implemented."

---

### WP-M0.9 — `pal-linux` / `pal-windows` / `pal-macos`: `InterfaceInfo` + watchers
**Depends:** WP-M0.3, WP-M0.8 · **Estimate:** 7–9 days (all three) · **`[on-target]` each**

The first real platform code. Interface enumeration, addresses, link state, counters, and **change watchers**: RTNETLINK (Linux), `NotifyIpInterfaceChange` (Windows), SCDynamicStore (macOS). Stable interface identity resolution (F-13).

**Exit criteria:** Unplug/replug a USB Ethernet adapter on each OS → event fires within 2s, identity remains stable, no leaks over 1000 hotplug cycles (scripted). Counter wrap/reset handled (E-25) — never a negative rate.

---

### WP-M0.10 — `nh-monitor` (M0 subset): live throughput
**Depends:** WP-M0.9, WP-M0.4 · **Estimate:** 3–4 days

Per-interface counter sampling → rate derivation with monotonic-clock deltas (E-26, F-11) → ring buffer → stream API. No persistence yet (that's M2).

**Exit criteria:** Live rates match `iftop`/Task Manager/Activity Monitor within 5%. Clock step (`date -s`) does not produce absurd rates.

---

### WP-M0.11 — `daemon-sim` + `nh-cli` (M0 subset)
**Depends:** WP-M0.6 · **Estimate:** 3–4 days

**`daemon-sim`**: the daemon wired to `pal-mock`, runnable **unprivileged** — this is how UI development happens without root, and how UI CI runs. First-class tool, per §14.1.
**`nh-cli`**: `status`, `capabilities`, `debug tasks`, `doctor` skeleton, `--json` on everything.

**Exit criteria:** `daemon-sim` runs as a normal user on all three OSes and serves the same contracts. CLI output is stable, parseable JSON.

---

### WP-M0.12 — Flutter app skeleton
**Depends:** WP-M0.2, WP-M0.11 · **Estimate:** 8–10 days

**Deliverables:** App shell (Material 3, light/dark, adaptive), riverpod + go_router, generated gRPC clients, **repository layer implementing snapshot+delta resync and reconnect/backoff** (§5.4) with an app-wide connection banner, **capability-driven rendering** with `unsupported`/`degraded` explainer widgets (AD-6 — and zero `Platform.isX` in feature code), interface list screen, live throughput chart, error rendering that shows `remediation` + copyable diagnostic ID. ARB i18n scaffolding + accessibility semantics from the first widget (§12.3).

**Exit criteria:** Kill the daemon while the UI is open → banner appears, no crash, no stale data shown as live; restart it → auto-reconnect and resync. Golden tests pass in light/dark/RTL. Keyboard-complete navigation.

---

### WP-M0.13 — Packaging v0 + uninstall contract
**Depends:** WP-M0.7, WP-M0.12, WP-0.5 · **Estimate:** 6–8 days · **`[on-target]` + `[you-verify]`**

**Deliverables:** MSI (WiX v4), deb + rpm, DMG with `SMAppService` helper install. Unsigned/dev-signed at this stage (real signing is M3, gated on PRE-1/PRE-2). Uninstall paths per §13.5.

**Exit criteria — this is the M0 gate itself:** On a **fresh VM snapshot** per OS: state-diff snapshot → install → use the app → uninstall → state-diff snapshot → **clean diff** (modulo intentionally-kept user data, which the differ knows about). Automated in the nightly workflow.

---

### WP-M0.14 — Fault-injection suite v1
**Depends:** WP-M0.4, WP-M0.7, WP-M0.13 · **Estimate:** 4–5 days

Codify the AD-5 global invariant as an automated test: kill the daemon at randomized points inside every mutating flow that exists so far; assert the system returns to desired state or pre-NetworkHub state, never a third thing. Plus explicit tests for F-1, F-3, F-4, F-5, F-9, F-12, F-13.

**Exit criteria:** Suite runs in CI (mock-level) and nightly (netns + VM level). It must be *easy to add a case* — every future feature WP adds to it.

---

### 🚦 M0 Gate

- [ ] Fresh VM → install → **live network state visible in UI** → uninstall → **clean state diff**, on Windows, Linux, macOS
- [ ] Daemon survives reboot, crash-restart, and kill -9 without leaving OS state behind
- [ ] Capability matrix returned honestly on all three OSes
- [ ] `daemon-sim` enables unprivileged UI development
- [ ] Fault-injection suite green; F-2 boot-cleanup demonstrated
- [ ] CI runs the whole thing on every push
- [ ] **The architecture is proven. Nothing built after this gets rebuilt because of a foundational surprise.**

---

## 6. Milestone M1 — VPN

**Goal:** Production-quality VPN: WireGuard and OpenVPN on all three OSes, with a kill switch that cannot strand the user, and leak prevention that survives adversarial testing.
**Duration:** 6–9 weeks · **Gate:** leak test suite green on 3 OSes + F-2/F-3 fault tests green + 72h soak with flapping upstream.

### WP-M1.1 — `nh-secrets`: keychain abstraction
**Depends:** M0 gate · **Estimate:** 4–5 days · **`[on-target]`**

DPAPI machine-scope (Windows), macOS system Keychain, Secret Service (Linux desktop) + **root-only encrypted-file fallback for headless Linux** (the Pi case — no desktop keyring). AES-256-GCM blob encryption with key in keychain (§7.1). Key rotation support.

**Exit criteria:** Secrets survive reboot, are unreadable by non-root users (verified by test), and never appear in `config.db` or logs. Headless Linux path works on a Pi with no desktop session. OD-6 resolved and documented.

---

### WP-M1.2 — VPN profile model, storage, and **hostile-input import**
**Depends:** WP-M1.1 · **Estimate:** 5–6 days

**Deliverables:** WireGuard profile model (interface + peers), OpenVPN profile model, import from file / paste / QR. **The sanitizer** (§8.2, and this is a genuine security boundary): parse `.ovpn`, **strip `up`/`down`/`script-security`/`plugin` and every other directive that can execute code as root**, re-emit a clean config. Extract secrets to keychain on import. Size caps, path-traversal rejection, malformed-input hardening (E-15).

**Exit criteria:** A corpus of malicious `.ovpn` files (script injection, path traversal, zip bombs, absurd sizes, encoding tricks) is rejected or neutralized — **with a fuzz target in the same PR** (§1.6). Round-trip: import → export → re-import is stable.

---

### WP-M1.3 — WireGuard engine + platform tunnels
**Depends:** WP-M1.1, SPIKE-E, PRE-6 · **Estimate:** 8–11 days · **`[on-target]` all three**

Kernel WG via netlink on Linux when available, userspace engine otherwise; wintun on Windows; utun on macOS. OD-2 (userspace engine choice) resolved here. Handshake, rekey, roaming, peer stats.

**Exit criteria:** Connect to a real WireGuard server from all three OSes; sustained transfer at expected throughput; roaming (change network mid-session) recovers; endpoint re-resolution on handshake failure works (E-9).

**Contingency:** if PRE-6 forbids bundling wintun, fall back to guided first-run download (decided before this WP starts, not during).

---

### WP-M1.4 — Firewall PAL primitives (kill-switch subset)
**Depends:** M0 gate · **Estimate:** 6–8 days · **`[on-target]` all three**

nftables tables/chains/sets with our naming + priorities (Linux); WFP provider/sublayer/filters in our own GUID namespace (Windows); PF anchors under `networkhub/*` with reference-counted PF enable (macOS). **Atomic apply or nothing.** Tagging so every artifact is attributable and removable (AD-5).

**Exit criteria:** Rules apply atomically; a failed apply leaves the previous state intact; **every artifact we create is found and removed by the state-differ + uninstaller**; coexistence verified — firewalld/ufw (Linux), Defender Firewall (Windows), Apple's pf.conf (macOS) all still function and are untouched.

---

### WP-M1.5 — VPN state machine, reconciler, connectivity verification
**Depends:** WP-M1.3, WP-M0.4 · **Estimate:** 5–6 days

Full state machine (§8.2), auto-reconnect with backoff, **post-up connectivity verification through the tunnel** (detecting "connected but blackholed" as a distinct state), sleep/resume handling (E-14, F-14), MTU derivation and PMTU fallback (E-11 — the #1 "VPN is broken" report), default-route conflict rejection (E-13).

**Exit criteria:** Survives: upstream loss, sleep/resume, server restart, DNS failure of the endpoint, MTU-blackhole path. Reconnect storms are impossible (verified by counting attempts during a 1h simulated outage).

---

### WP-M1.6 — Kill switch (the fail-closed exception) 🔴
**Depends:** WP-M1.4, WP-M1.5, WP-M0.7 · **Estimate:** 5–7 days · **`[on-target]`**

Three modes (`off` / `while-connected` / `persistent`), the allow-list policy (§8.2), **deadman removal of non-persistent rules** (F-2), and the **offline escape hatch**: `networkhub vpn killswitch off` that works with the daemon dead, via a minimal self-contained code path — printed on the opt-in screen.

**Exit criteria:** The F-2 scenario, run for real on all three OSes: kill switch active → `kill -9` the daemon → prevent restart → **reboot** → machine has internet (non-persistent) or the escape hatch works in under 30 seconds (persistent). Uninstall with an active persistent kill switch leaves a working network. This WP does not pass on a mock.

---

### WP-M1.7 — Leak prevention: DNS, IPv6, and the reconnect window
**Depends:** WP-M1.6 · **Estimate:** 5–6 days

DNS override + enforcement (only tunnel DNS reachable while up), IPv6 egress blocking when the tunnel is v4-only (default on), and the **reconnect-window leak** (traffic escaping between tunnel-down and kill-switch-reassert — this is where most VPN clients actually leak). Captive-portal pause flow (E-10): time-boxed, LAN + 80/443 only.

**Exit criteria:** **Adversarial leak test suite**, run under packet capture on all three OSes: no plaintext egress during connect, disconnect, crash, sleep/resume, network change, or forced reconnect. IPv6 leak test with a v6-capable upstream. DNS leak test against a monitored resolver.

---

### WP-M1.8 — Split tunnel (CIDR-based) + OpenVPN controller
**Depends:** WP-M1.5, WP-M1.2 · **Estimate:** 7–9 days

Split tunnel by destination CIDR + exclude-LAN toggle, with overlapping-subnet detection (E-12). OpenVPN as a supervised child process (AD-8) driven over its management interface, **credentials via management channel never argv/env**, GPL process isolation maintained.

**Exit criteria:** OpenVPN connects to real servers (test against 2–3 common configs incl. TLS-auth and cert+password). Killing the daemon kills the openvpn child (F-4). Split-tunnel routes verified by traceroute per destination class.

---

### WP-M1.9 — VPN UI + M1 hardening
**Depends:** WP-M1.7, WP-M1.8, WP-M0.12 · **Estimate:** 6–8 days

Profile list/import/QR, connect flow with real progress from events, status with live stats, kill-switch config **with the honest scary-explanation opt-in** (§12.2), split-tunnel editor, error rendering with remediation.

**Exit criteria:** Full UX pass on all three OSes; every failure mode from M1 produces an actionable message, not a stack trace.

---

### WP-M1.10 — M1 soak & fault campaign
**Depends:** all M1 · **Estimate:** 4–5 days (+72h wall clock)

72h soak with deliberately flapping upstream; fault-injection extended to all M1 flows; memory/fd/task-leak tracking.

**Exit criteria:** 72h, zero leaks (flat RSS/fd/task counts), zero stranded-network events, no unrecovered states.

---

### 🚦 M1 Gate

- [ ] WireGuard + OpenVPN connect reliably on Windows, Linux, macOS
- [ ] Adversarial leak suite green (DNS, IPv6, reconnect window) under packet capture
- [ ] **F-2 verified physically**: no crash, kill, or reboot with kill switch active can strand the machine offline
- [ ] Malicious-profile corpus rejected; fuzz targets in CI
- [ ] 72h soak clean
- [ ] Uninstall still diff-clean with VPN artifacts present

---

## 7. Milestone M2 — Hotspot, Devices, Monitoring

**Goal:** The feature set that defines the product: share your connection, see and control every device on it, and understand your network over time. Linux gets the full stack; Windows uses native APIs with documented gaps; macOS is assisted-manual (AD-12).
**Duration:** 9–12 weeks · **Gate:** E-1..E-8 tests green + hwsim E2E green + hardware lab checklist passed + F-7 self-test proven.

This is the highest-variance milestone. The `mac80211_hwsim` harness (WP-0.5) is what keeps it tractable — most of it develops against virtual radios, with real hardware as the validation step rather than the development loop.

### WP-M2.1 — `metrics.db` + monitoring pipeline (full)
**Depends:** M1 gate · **Estimate:** 6–7 days

Persistence, batched writes (SD-card friendly, R-9), incremental crash-safe rollups (raw → 1min → 1hour), retention policy + enforcement, disk-pressure eviction (F-12, E-27), historical query API, latency/jitter/loss probes (ICMP with TCP fallback for filtered networks), CPU/RAM sampling.

**Exit criteria:** 30 days of simulated data rolls up correctly; killing the daemon mid-rollup resumes without double-counting or gaps; disk-full behavior is graceful; a Pi running 7 days shows acceptable write volume (measured, not assumed).

---

### WP-M2.2 — `nh-devices`: discovery and the identity problem
**Depends:** WP-M2.1 · **Estimate:** 7–9 days

Passive discovery (ARP/NDP tables, mDNS/SSDP listening, DHCP leases once M2.4 lands), opt-in active scan with the ethics guard (§8.6 — passive-only on upstream LANs by default), bundled offline OUI database, and **identity resolution across MAC randomization** (§7.2): synthetic stable device ID, correlation heuristics (MAC + hostname + mDNS name + DHCP fingerprint), manual merge/split UI support.

**Exit criteria:** An iPhone and an Android phone with MAC randomization enabled, joining repeatedly over days, resolve to **one device each**, not twelve. Verified on real hardware — this is a `[you-verify]` item and it's the one most likely to need iteration.

---

### WP-M2.3 — Firewall: full rule model, compiler, revisions, lockout guard
**Depends:** WP-M1.4 · **Estimate:** 8–10 days

The complete rule model (§8.3), the pure-function compiler (`RuleSetRevision → OS program`), **versioned rule-set documents with atomic swap and one-click rollback**, device-reference resolution with recompile-on-address-change, dry-run diff, implicit service rules (E-18 — DHCP must keep working under deny-all), IPv6 parity enforcement (E-17 — a v4-only block is a bypass), rule-explosion caps (E-19), and the **lockout guard** (60s auto-revert unless confirmed) for remote/headless sessions.

**Exit criteria:** Property tests on the compiler (round-trip, no rule silently dropped, IPv6 parity asserted). Applying a self-locking rule set over SSH auto-reverts and restores access. Rule sets of 500+ device rules compile in under 1s and apply atomically.

---

### WP-M2.4 — `nh-dhcp`: DHCPv4 server + safety interlocks 🔴
**Depends:** WP-M2.3 · **Estimate:** 7–8 days

Full DHCPv4 (pools, static leases, reservations, options incl. RFC 8910 captive-portal), lease persistence across restarts, and the **safety interlocks that prevent F-7** — the failure mode where we DHCP-poison someone's office network and destroy our reputation: interface-scoped binding only (never wildcard), active foreign-DHCP-server probe on start with refuse-to-serve, DISCOVER flood rate limiting, starvation-attack throttling (per-client-id), pool-exhaustion events.

**Exit criteria:** **F-7 proven in netns**: a daemon configured for interface A does not answer DHCP on segment B, and refuses to start serving when another DHCP server is detected. Fuzz target for the packet parser (§1.6). RFC conformance against real clients (phones, laptops, the smart-home device).

---

### WP-M2.5 — `nh-dns`: resolver, cache, upstreams
**Depends:** WP-M2.4 · **Estimate:** 8–9 days

Resolver on managed interfaces, DoH/DoT upstreams (rustls) with health/RTT selection and **bootstrap handling**, cache with correct TTL/negative caching/serve-stale, local zone using `home.arpa` (RFC 8375 — not `.local`, which collides with mDNS, E-24), DNS rebind protection (E-22), EDNS0/TCP-fallback correctness (E-23). Filtering lists land in Phase 2; the hooks land here. OD-3 resolved (`hickory-dns` libraries as the base).

**Exit criteria:** Passes a DNS conformance suite; fuzz target green; DoH/DoT verified against real providers incl. bootstrap-from-cold; never binds :5353.

---

### WP-M2.6 — `nh-hotspot`: orchestration + preflight
**Depends:** WP-M2.4, WP-M2.5, WP-M0.8 · **Estimate:** 6–7 days

Profile model, the state machine (§8.1), and **preflight** — which validates capabilities, radio availability, subnet conflicts (E-2, with auto-pick from a pool), and port conflicts, returning **all** problems at once rather than failing on the first. Full rollback on mid-start failure via the intent journal (E-6). QR payload generation with correct escaping (E-7).

**Exit criteria:** Every E-1..E-8 edge case has a test. A failed start leaves the machine byte-identical to before (state-differ proves it).

---

### WP-M2.7 — `pal-linux` hotspot: NetworkManager + hostapd paths
**Depends:** WP-M2.6 · **Estimate:** 9–11 days · **`[on-target]` + `[you-verify]`**

NM D-Bus AP path (simple), managed-hostapd path (advanced: client isolation, portal support, fine 802.11 control), interface ownership handoff (mark unmanaged in NM, restore on teardown — AD-11), nl80211 capability probing (interface modes, bands, channels, **valid interface combinations** for repeater mode), regulatory-domain awareness (E-5, incl. DFS), NAT + `ip_forward` with journaled prior values (F-9), station dump for per-client RSSI.

**Exit criteria:** Works on: Intel AX (best case), Pi brcmfmac (constrained — STA+AP correctly reported as limited), Realtek USB (worst case — capability probe must not believe the driver's lies). Hotspot survives sleep/resume (E-4) and upstream change (E-1). Teardown restores NM ownership and `ip_forward`.

---

### WP-M2.8 — `pal-windows` hotspot: Mobile Hotspot (WinRT)
**Depends:** WP-M2.6 · **Estimate:** 6–7 days · **`[on-target]`**

`NetworkOperatorTetheringManager` with runtime capability probing, client enumeration and events, the client-count limit surfaced in capabilities, ICS for Ethernet sharing (best-effort, reconciler re-asserts after reboot). Custom subnet/DHCP correctly reported **unsupported** (§3.1) — no fragile hacks.

**Exit criteria:** Starts/stops reliably; client list matches reality; capability matrix honestly reports the Windows gaps; the UI shows *why*, not a broken control.

---

### WP-M2.9 — `pal-macos` hotspot: assisted-manual (AD-12)
**Depends:** WP-M2.6 · **Estimate:** 3–4 days · **`[on-target]`**

Detect Internet Sharing state via SCDynamicStore, deep-link to the Settings pane, auto-detect activation, then layer what we legitimately can on top (device discovery and monitoring on the bridge interface). Location-permission flow for Wi-Fi info (§3.3).

**Exit criteria:** The UX is honest and useful — a guided flow, not an error. No plist hacking, no SIP-adjacent tricks.

---

### WP-M2.10 — Device controls: block, pause, and the accounting base
**Depends:** WP-M2.3, WP-M2.2 · **Estimate:** 5–6 days

Block/pause via firewall device classes, per-device accounting via nftables counters / WFP / PF, quota semantics defined and implemented at the accounting layer keyed to synthetic device ID (§8.6), 80%/100% events. Enforcement (throttle/block) on Linux; Windows is accounting-only per §3.1/R-3, reported honestly in capabilities.

**Exit criteria:** Blocking a device blocks it on v4 **and** v6 (E-17). A device that changes IP stays blocked (recompile-on-change). Quota counting survives daemon restart and clock jumps (F-11).

---

### WP-M2.11 — netns + hwsim E2E harness (full)
**Depends:** WP-M2.7, WP-M2.4, WP-M2.5 · **Estimate:** 5–6 days

The crown-jewel test (§14.1): virtual Wi-Fi AP + virtual clients in namespaces, running the real daemon, exercising hotspot → DHCP → DNS → device discovery → block/quota end to end, plus the 100+ simulated client load test.

**Exit criteria:** Full hotspot E2E in CI on every PR, no hardware needed. 100 simulated clients associate, get leases, and are individually controllable.

---

### WP-M2.12 — UI: hotspot, devices, monitoring
**Depends:** WP-M2.10, WP-M2.1, WP-M0.12 · **Estimate:** 10–12 days

Hotspot setup + QR + live client list, device list with identity/merge UX (the merge flow matters — M2.2), per-device controls, monitoring dashboards (live/daily/weekly/monthly), analytics views and CSV/JSON export (PDF is UI-side per OD-9).

**Exit criteria:** Full UX pass on three OSes with capability-driven degradation rendering correctly (a Windows user sees why custom subnets are unavailable; a macOS user gets the assisted flow).

---

### WP-M2.13 — Hardware lab validation campaign
**Depends:** all M2 · **Estimate:** 5–7 days · **`[you-verify]`** (I write the protocol; you run it)

The compatibility matrix from ARCHITECTURE.md §19, executed as a scripted checklist per adapter and client device, with results published as the public compatibility list (R-1 mitigation).

---

### 🚦 M2 Gate

- [ ] E-1..E-8 covered by tests; hwsim E2E green in CI
- [ ] **F-7 proven**: cannot serve DHCP on a foreign segment; refuses when another server is present
- [ ] Hardware matrix passed on Intel AX, Pi, and the worst-case Realtek adapter
- [ ] MAC-randomizing phones resolve to stable single identities
- [ ] 100-client load test green
- [ ] Uninstall diff-clean with hotspot/DHCP/DNS/firewall artifacts present

---

## 8. Milestone M3 — MVP Hardening & Public Beta

**Goal:** Ship it. Signed, notarized, updatable, documented, security-reviewed.
**Duration:** 5–7 weeks · **Gate:** public beta live with update+rollback exercised in production.

| WP | Scope | Est. | Notes |
|---|---|---|---|
| **WP-M3.1** | **`nh-gateway`**: REST + WebSocket, TLS 1.3, Argon2id auth, JWT+refresh, API keys, rate limiting/lockout, Origin/CORS hardening, separate OS user, SSRF guard | 8–10 d | Off by default (AD-3). Enabling to `all` interfaces requires the explicit warning flow (F-10). Daemon re-checks authz on every forwarded call. |
| **WP-M3.2** | **Signing & notarization**: Authenticode via HSM-backed key in CI, macOS Developer ID + notarization + stapling, GPG-signed apt/dnf repos | 5–7 d | Blocked on PRE-1/PRE-2/PRE-3. Start the moment certs arrive. |
| **WP-M3.3** | **Update pipeline** (AD-15): TUF-style signed manifests, anti-rollback, staged apply with `--verify` self-check, health check, **automatic rollback** (F-8), channel policy (self-update for MSI/DMG/AppImage; notify-only for deb/rpm) | 7–9 d | Blocked on PRE-4. The rollback path is tested by shipping a deliberately-broken build to a test channel. |
| **WP-M3.4** | **`doctor` + diagnostics bundle**: logs, capabilities, sanitized config, recent events, with interactive redaction preview | 3–4 d | This is the support workflow for the next two years. Worth doing well. |
| **WP-M3.5** | **Security review pass**: fuzz corpus baseline for every listener, `cargo audit`/`deny` clean, gateway DAST (ZAP baseline), secrets scan, SBOM (CycloneDX) generation, **no-unexpected-egress test** (§10.5), threat-model review against shipped code | 5–7 d | Findings become blocking bugs. This is a gate, not a report. |
| **WP-M3.6** | **Docs**: install guides ×3 OSes, headless/Pi guide, troubleshooting, coexistence matrix (firewalld/AV/GPO), API reference from proto, privacy statement, THIRD-PARTY-NOTICES | 5–6 d | |
| **WP-M3.7** | **Beta release**: repo/CDN hosting, telemetry opt-in flow, Sentry (opt-in), feedback channel, staged rollout beta→stable | 3–4 d | |

### 🚦 M3 Gate — **MVP SHIPPED**

- [ ] Signed, notarized installers for Windows, Linux (deb/rpm/AppImage), macOS
- [ ] Update + **rollback** exercised in the live beta channel
- [ ] Security review findings closed; fuzz running nightly; SBOM published
- [ ] External beta testers on all three OSes; crash-free sessions >99%
- [ ] Docs complete enough that a stranger can install headless on a Pi

---

## 9. Phase 2 — Professional

**Duration:** 6–8 months. Each item is a milestone with its own gate; they ship incrementally to beta users rather than as one big release.

| Milestone | Scope | Est. | Key risks/notes |
|---|---|---|---|
| **P2.1 Firewall UI & rule management** | Rule editor, revision history + rollback UI, dry-run diff view, conflict report, templates | 4–5 wks | Lockout guard UX (§12.2) is the make-or-break detail |
| **P2.2 Shaping & quotas** | `tc` HTB/CAKE per device, IFB ingress, quota enforcement, schedules | 5–6 wks | Linux-only (R-3); Windows gap documented. Evaluate macOS dummynet. |
| **P2.3 DNS filtering & host-resolver mode** | Filter lists (hosts/ABP), per-device policy groups, DoH/DoT for the host, systemd-resolved integration, bypass enforcement (E-21) | 5–6 wks | Host-resolver takeover needs the restore-guard + deadman path proven (§8.4) |
| **P2.4 Captive portal** | Quarantine class, RFC 8908/8910 CapPort, portal server, vouchers/temp passwords, sessions, ToS logging | 6–8 wks | Linux-only. Legal/retention knobs documented (§8.8). Real-device testing across iOS/Android/Windows portal detection. |
| **P2.5 Automation engine** | Trigger/condition/action model, hysteresis, cooldowns, **loop detection**, action budget, dry-run, tz-correct scheduler with catch-up policy | 5–6 wks | The safety rails (§8.7) are the hard part, not the rule engine |
| **P2.6 Plugin SDK** | wasmtime host, capability grants, Ed25519 signing, resource limits, WIT provider interfaces, Rust SDK then TS SDK, **core features migrated onto the provider interfaces first** (dogfooding rule, §9.2) | 8–10 wks | Do not publish the SDK until core consumes it — otherwise we ship interfaces that don't work |
| **P2.7 Hardening M-H1 + M-H2** | seccomp allowlist, least-privilege tightening, **external pentest** before gateway/portal GA | 3–4 wks + vendor time | Pentest scheduling has lead time; book early |

---

## 10. Phase 3 — Enterprise

**Duration:** 6–9 months.

| Milestone | Scope | Est. |
|---|---|---|
| **P3.1 Multi-user & full RBAC** | Complete role matrix generated from proto annotations, sudo-mode re-auth for sensitive ops, per-user audit attribution | 5–6 wks |
| **P3.2 SSO** | OIDC first (SAML only if a customer demands it), group→role mapping, session lifecycle | 5–7 wks |
| **P3.3 Audit & compliance** | Audit export (SIEM formats), retention policies, tamper-evidence verification tooling, enterprise policy templates | 4–5 wks |
| **P3.4 Cloud sync** | E2E-encrypted settings sync, conflict model per OD-7, backup/restore, remote management via gateway | 8–10 wks |
| **P3.5 Web dashboard** | SPA on the TS SDK served by the gateway, CSP-hardened, feature parity for the ops surface | 8–10 wks |
| **P3.6 HA (scoped honestly)** | Config replication + cold standby. **Not** live network-state failover — that's a different product and we say so. | 4–5 wks |

---

## 11. Phase 4 — Ecosystem

**Duration:** 6+ months.

| Milestone | Scope | Est. |
|---|---|---|
| **P4.1 Mobile controllers** | Flutter iOS/Android apps as gateway clients over TLS, push notifications, QR pairing | 10–12 wks |
| **P4.2 Plugin marketplace** | Submission/review pipeline, signing service, catalog, revenue sharing infrastructure | 10–14 wks |
| **P4.3 AI assistant** | Diagnostic snapshot API, propose-diff model with `assistant` role (read + propose only, never direct mutation — §8.9), local model via plugin, cloud via user key | 6–8 wks |
| **P4.4 Integrations** | Home Assistant, Prometheus/Grafana, notification providers, third-party VPN accounts | ongoing |

---

## 12. Continuous Tracks

These run through every phase, not as separate milestones. Budget ~15–20% of ongoing capacity.

| Track | Practice |
|---|---|
| **Security** | Fuzz targets with every parser; `cargo deny`/`audit` per PR; dependency review; threat-model revisit each milestone gate; annual pentest from P2.7 |
| **Fault catalog** | Every feature WP asks "what new way can this strand the user?" (DoD #6). §11 of ARCHITECTURE.md grows; the fault suite grows with it. |
| **Docs** | Written with the feature, not after. A feature without a docs page isn't Done. |
| **Compatibility** | Hardware matrix re-run each release; public compatibility list maintained (R-1) |
| **Performance** | Soak + leak tracking each milestone; regression thresholds in CI |
| **ADRs** | Every decision touching privilege, contracts, storage, crypto, or licensing |

---

## 13. Decision Calendar

Open decisions from ARCHITECTURE.md §20, with the **latest** point they can be resolved without causing rework:

| Decision | Resolve by | Why then |
|---|---|---|
| **OD-4** IPC transport | **Phase 0, WP-0.3** | Changes core architecture if it fails. Highest-risk unknown in the project. |
| **PRE-6 / R-6** wintun licensing | **Before WP-M1.3** | Determines bundle vs. download for Windows WireGuard |
| **OD-2** userspace WireGuard engine | WP-M1.3 | |
| **OD-6** at-rest passphrase / headless key storage | WP-M1.1 | The Pi has no keyring; the fallback design is needed then |
| **OD-1** product name / trademark | **Before public beta (M3)** — but start the search in Phase 0 | Renaming after beta is expensive; domain + signing certs encode the name |
| **OD-3** DHCP/DNS implementation base | WP-M2.4 | |
| **OD-9** PDF rendering placement | P2 analytics | |
| **OD-5** GeoIP source | P2.1 firewall | |
| **OD-8** DCO vs CLA | Before the first external contribution | Dual-licensing (P3) requires clean copyright from the start |
| **OD-7** cloud sync encryption/conflict model | P3.4 design | |

---

## 14. Descoping Levers

If ~9 months to MVP is too long, these are the honest levers — in the order I'd pull them. Each has a stated cost.

| Lever | Saves | Cost |
|---|---|---|
| **Ship MVP as Linux + Windows; macOS in M3.5** | ~3–4 wks | macOS users wait. Low architectural risk since PAL is already abstracted — but do *not* remove macOS from M0, or the PAL traits get bi-platform-biased. |
| **Defer OpenVPN to Phase 2 (WireGuard only at MVP)** | ~2 wks | WireGuard covers most modern use; OpenVPN matters for corporate/legacy users. Reversible. |
| **Defer the gateway to Phase 2** | ~2 wks | No remote/headless web access at MVP; CLI still covers headless. Reduces security surface at launch — arguably a *benefit*. |
| **Windows hotspot deferred (Linux + macOS-assisted only)** | ~1.5 wks | Weakens the Windows story significantly. I'd resist this. |
| **Reduce M2 to hotspot + devices, defer analytics/history to P2** | ~2–3 wks | Live monitoring stays; history/reports wait. Metrics schema must still land in M2 or data is lost forever. |

**Levers I would not pull, at any deadline:** the M0 skeleton, the fault-injection suite, the uninstall contract, the leak test suite, the F-7 DHCP interlocks, or the security review. Each of these is the difference between a product and a liability.

---

## 15. Progress Tracking

**`PROGRESS.md`** at repo root, updated weekly, structured as:

```markdown
## Current: M0 — Walking Skeleton  (week 3 of ~6)
### Done
- WP-M0.1 nh-types ✅ (gate: trybuild secret test green)
### In flight
- WP-M0.4 nh-state — reconciler core done, intent journal in progress
### Blocked
- WP-M0.7 — waiting on your verification of SPIKE-C findings
### Risks changed this week
- R-6 wintun: legal review came back ambiguous → escalating, may need download fallback
### Next week
- WP-M0.5, WP-M0.6
```

Plus: GitHub milestones mirroring M0–M3/P2–P4, issues per WP with the exit criteria pasted into the issue body (so "done" is never ambiguous), and the risk register (§19 of ARCHITECTURE.md) re-rated at every milestone gate.

---

## 16. What Happens Next

The immediate queue, in order:

1. **You**: kick off procurement (PRE-1, PRE-2, PRE-5, PRE-6 especially — the ones with legal/identity lead times), and confirm the hardware lab list.
2. **Me**: WP-0.1 (repo + workspace + ADR backfill) — I can start this immediately.
3. **Me**: WP-0.2 (CI on three OSes).
4. **Both**: WP-0.3 SPIKE-A — the Dart↔Rust IPC transport question. This is the one that could change the architecture, so it happens in week 1–2, not later. You'll need to run me on the Windows box for the Windows half.

Say the word and I'll start with WP-0.1.

---

*This plan is a living document. It is amended when reality disagrees with it — which it will — via the same ADR discipline as the architecture. What does not change without an explicit decision: the gates, the Definition of Done, and the non-negotiable engineering rules in §1.6.*


