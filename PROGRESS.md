# NetworkHub — Progress

Updated as work lands. Companion to
[EXECUTION_PLAN.md](docs/architecture/EXECUTION_PLAN.md). The rule this file
follows: **state what was proven and where, and state plainly what was not.**

---

## Current: Milestone M0 — Walking Skeleton (in progress)

Work happens on branch `phase/m0-walking-skeleton` (PR #2 to `main`); each work
package is a commit, CI gates every push, and the branch merges to `main` when
M0's exit criterion is met. `main` holds the Phase 0 baseline.

| WP | What | Status |
|----|------|--------|
| M0.1 | `nh-types` — domain vocabulary, `Secret<T>`, errors, capabilities | ✅ CI-green 3-OS |
| M0.2 | `nh-proto` — API contracts, Rust/Dart/TS codegen, provable authz coverage | ✅ CI-green 3-OS |
| M0.3 | `nh-pal` — 7 PAL traits + mock with fault injection | ✅ CI-green 3-OS |
| M0.4 | `nh-state` — desired-state store, reconciler, event bus, intent journal | ✅ CI-green 3-OS |
| M0.5 | `config.db` — storage, migrations, resilience | ✅ CI-green 3-OS |
| M0.6 | `nh-daemon` — lifecycle, IPC (UDS/pipe), authorization, child supervision | ✅ CI-green 3-OS |
| M0.7 | Service integration + boot-cleanup (F-2): units/plists, install scripts, Windows SCM + named-pipe transport | ✅ CI-green 3-OS; runtime `[you-verify]` per [ops/service.md](docs/ops/service.md) |
| M0.8 | Capability probe framework (AD-6): `SystemProbe` per OS + `build_matrix` (§3.4), live `Capabilities` with re-probe + change event | ✅ CI-green 3-OS |
| M0.9 | `InterfaceInfo` + counters + watchers per OS (RTNETLINK / PF_ROUTE / NotifyIpInterfaceChange), shared diff/identity (F-13) watch engine | ✅ CI-green 3-OS; hotplug soak `[you-verify]` |
| M0.10 | `nh-monitor` (live throughput): monotonic-clock rate derivation (E-25/E-26), per-interface ring buffer, broadcast stream API | ✅ CI-green 3-OS; 5%-accuracy check `[you-verify]` |
| M0.11–M0.14 | CLI + daemon-sim, Flutter UI, packaging, fault suite | pending |

**M0 exit criterion:** fresh VM → install → live network state in the UI →
uninstall → clean state-diff, on all three OSes.

---

## Phase 0 — Foundations (complete)

All six Phase 0 work packages are implemented and committed. Two exit criteria
remain open because they need resources this development machine (macOS, no root,
no signing certs, no GitHub remote) does not have. They are listed explicitly
below rather than quietly marked done.

### Done

| WP | What | Commit | Verified on |
|----|------|--------|-------------|
| 0.1 | Repo, 22-crate workspace, architecture layer enforcement, ADR-0001..0015 | `f20d792` | macOS |
| 0.2 | 3-OS CI pipeline, nightly skeleton, self-verifying guard rails | `e74f622` | macOS (locally); **CI never executed — no remote** |
| 0.3 | SPIKE-A: Dart↔Rust IPC transport (ADR-0016) | `b771f3e` | macOS (UDS); Windows path compiles only |
| 0.4 | SPIKE-B/C/D: privileged service lifecycle on 3 OSes | `7dd7d9b` | macOS (C partial); B/D cross-compile only |
| 0.5 | state-diff tool + netns/hwsim harness | `9642589` | macOS (state-diff E2E); Linux/Windows compile only |
| 0.6 | Procurement kickoff | — | **your action** — see Blocked |

### The big risk is retired

**SPIKE-A answered the project's highest-risk unknown the good way.** Dart speaks
gRPC over a Unix domain socket with peer-credential authentication and lossless
streaming — proven on macOS. ADR-0002 ("no local TCP listener") stands. The
Windows named-pipe path is implemented and compiles; it needs one run on the
Windows box to be fully confirmed, with a specified fallback (ADR-0016 A3) if it
does not.

### Open exit criteria (need you / hardware)

These are the honest gaps. None blocks starting M0; all should close as the
resources arrive.

1. **CI has never run.** The pipeline is written and every command passes
   locally on macOS, but the WP-0.2 exit criteria — a deliberate clippy warning
   blocked on all three OSes, >70% cache hit — require a GitHub remote and a
   push. → needs the repo created (Blocked #1).
2. **SPIKE-A Windows leg** — `spikes/spike-a-ipc/run-windows.ps1`, ~10 min on the
   Windows box.
3. **SPIKE-B (Windows service)** — `spikes/spike-b-windows-service/scripts/run.ps1`,
   elevated, on the Windows box.
4. **SPIKE-C (macOS root helper)** — `spikes/spike-c-macos-helper/scripts/`, needs
   your **sudo**; the `SMAppService` signed path additionally needs PRE-2.
5. **SPIKE-D (Linux systemd + F-2 boot cleanup)** —
   `spikes/spike-d-linux-systemd/scripts/run.sh`, needs a **Linux box** + sudo.

What cross-compilation already caught, so it will not waste your time on the
other machines: a `c_char` pointer-type mismatch (Linux), a moved
`windows-service` method, a relocated pipe constant, and a missing `windows-sys`
feature (Windows). The remaining runtime steps are genuinely runtime-only.

### Blocked on you

1. **Create the GitHub remote.** Outward-facing, so not done unilaterally. Give a
   name + private/public and it is one command (`gh repo create`), then CI runs
   and open criterion #1 closes. `tools/setup-branch-protection.sh` applies the
   merge policy afterward.
2. **Procurement (PRE-1..PRE-6, PRE-8).** Lead-time items that gate *shipping*,
   not building. Most urgent: **PRE-5 trademark search on "NetworkHub"** (the
   name is generic and near-certainly contested; it gets baked into the domain
   and certs) and **PRE-6 wintun/wireguard-nt licensing** (must resolve before
   M1's WireGuard work).
3. **Placeholders:** repo URL in `Cargo.toml`, `@REPLACE-ME` in CODEOWNERS, and
   the security contact in `SECURITY.md`. The last matters most — a disclosure
   policy pointing at an unread inbox is worse than none.
4. **`Idea.md`** was removed from the repo; ARCHITECTURE.md still cites it as the
   vision reference. Restore it, or say so and the citation is dropped.

### Phase 0 gate checklist (ARCHITECTURE.md / EXECUTION_PLAN.md §4)

- [x] Workspace builds on the dev machine; layer rules enforced and self-tested
- [x] CI written for 3 OSes, blocking bad PRs — **pending first run**
- [x] SPIKE-A concluded → transport decided (ADR-0016)
- [x] SPIKE-B/C/D concluded → each with a demo or a documented, compiling, ready-to-run path
- [x] state-diff works (proven on macOS)
- [x] netns + hwsim harness functional (smoke test written; runs on Linux)
- [x] ADRs 0001–0015 backfilled; ADR-0016 added
- [ ] Procurement in flight — **awaiting you**
- [x] No known unknown remains that could invalidate AD-1, AD-2, or AD-8

**Assessment:** Phase 0's purpose — retire the unknowns that could force a
rewrite — is achieved. The privilege split (AD-1), the IPC transport (AD-2), and
the VPN isolation approach (AD-8) all survived contact. What is left is execution
verification on the other two machines and the CI remote, none of which can
change an architectural decision.

---

## Next: Milestone M0 — Walking Skeleton

First work packages (EXECUTION_PLAN.md §5): WP-M0.1 `nh-types` (domain
vocabulary, `Secret<T>`, error taxonomy, capability model) and WP-M0.2
`nh-proto` (the protobuf contracts). Both are pure Rust, buildable and fully
verifiable on this machine with no external dependency — so M0 can begin
immediately in parallel with the procurement and remote setup above.

Carried into M0 from the spikes:
- tonic 0.14 needs both `tonic-build` and `tonic-prost-build` (WP-M0.2)
- grpc-dart exports two `ClientChannel` types; use the abstract one (WP-M0.12)
- the production Windows pipe transport needs overlapped async I/O, not the
  spike's blocking-read-on-an-isolate (WP-M0.6)
