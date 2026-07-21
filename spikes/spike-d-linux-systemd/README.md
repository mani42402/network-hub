# SPIKE-D — Linux systemd lifecycle, hardening, and the F-2 boot-cleanup

**Questions** (ARCHITECTURE.md §3.2, §10.6, §11; ADR-0005):

1. Does the daemon run under a hardened systemd unit (`ProtectSystem=strict` and friends) and
   still work?
2. Can it signal readiness via `sd_notify`, so `systemctl start` blocks until it is actually up?
3. **F-2:** if the daemon dies uncleanly and does not come back, does a boot-time cleanup unit
   reclaim its orphaned state — so the machine is never stranded by leftover firewall rules?

Question 3 is the important one. It is the safety net for the entire product (ADR-0005): the
promise that no crash can leave the user offline. This spike proves the *mechanism* — a
oneshot cleanup unit ordered before the daemon and before networking — using a stand-in "state
tag" file in place of real nftables tables and sysctl changes.

## What is verified, and what is not

| Item | Status | Needs |
|---|---|---|
| Helper + cleanup compile for Linux (x86-64 and aarch64) | ✅ cross-checked from macOS | — |
| systemd unit files well-formed | ✅ syntax-checked | — |
| Hardened unit runs and functions | ⏳ scripted | a **Linux box**, sudo |
| `sd_notify` readiness | ⏳ scripted | Linux, sudo |
| **F-2 boot-cleanup reclaims orphaned state** | ⏳ scripted | Linux, sudo |
| Clean uninstall | ⏳ scripted | Linux, sudo |

Everything is written and compiles; the runtime rows need a Linux machine with `systemd`,
which this development session (macOS) does not have. `scripts/run.sh` executes all four in
sequence.

## Running it (on a Linux box)

```bash
cd helper && cargo build --release && cd ..
sudo scripts/run.sh
```

The script ends with `SPIKE-D: PASS` only if all four hold, including the F-2 step: it kills
the daemon with `SIGKILL` (so its clean-shutdown handler never runs), leaves the state tag
orphaned, then runs the cleanup oneshot and asserts the orphan is gone.

## Design notes carried into the real build

- **`sd_notify` without libsystemd.** The daemon writes `READY=1` to `$NOTIFY_SOCKET` itself
  (a stable, documented protocol), including the abstract-socket (`@`) case. The real daemon
  may use the `sd-notify` crate, but the dependency-free path is proven here.
- **Two units, ordered.** `networkhub-spike-d-cleanup.service` (oneshot, `DefaultDependencies=no`,
  `Before=network-pre.target`) runs before `networkhub-spike-d.service`. This ordering is what
  makes the deadman reclaim orphaned state *before* anything can depend on the network.
- **Hardening minus seccomp.** The unit sets `ProtectSystem=strict`, `NoNewPrivileges`,
  `MemoryDenyWriteExecute`, and the rest, but deliberately omits `SystemCallFilter`: a wrong
  seccomp allowlist is an outage, and it is scheduled as its own milestone (M-H1) once the
  daemon's real syscall surface is known.
- **c-string FFI portability.** `c"..."` literals are `*const u8`; C `unlink` wants
  `*const c_char`, which is `i8` on x86-64 and `u8` on aarch64. The cast to `c_char` is correct
  on both — a bug the macOS→Linux cross-check caught before it reached hardware.
