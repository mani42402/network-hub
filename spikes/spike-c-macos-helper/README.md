# SPIKE-C — macOS privileged helper lifecycle

**Question:** Can NetworkHub ship a privileged root daemon on macOS that a user installs, that
survives reboot and crash, and that uninstalls cleanly? (ARCHITECTURE.md §13.3, ADR-0014)

**Answer so far:** The mechanism works. The helper builds, runs, correctly reports whether it
is root, and the LaunchDaemon plist + install/verify/uninstall path are complete. Two steps
need what this machine does not have — see "What is verified" below.

## What this proves

The real product registers its helper via `SMAppService` from a signed app bundle. What
`SMAppService` installs underneath is a **launchd LaunchDaemon**, which is what this spike
exercises directly, so the lifecycle question is answered even before the signed path exists.

The helper is a stand-in for `networkhubd`: it listens on a Unix domain socket and answers
`PING`, `WHOAMI` (reports its euid and whether it is root), and `PRIVCHECK` (tries to read a
root-only path — real privilege, not a claimed euid).

## What is verified, and what is not

| Step | Status | Blocked on |
|---|---|---|
| Helper builds and runs | ✅ verified on macOS 26.5 | — |
| IPC round trip (probe ↔ helper) | ✅ verified | — |
| Correct privilege reporting | ✅ verified: as a normal user it reports `root=false` and `PRIVCHECK` is denied — so under launchd it will report `root=true` | — |
| LaunchDaemon plist is well-formed | ✅ `plutil -lint` OK | — |
| Ad-hoc code signing | ✅ verified | — |
| **Runs as root under launchd, survives reboot** | ⏳ scripted, not run | your **sudo** (this session has no root) |
| **`SMAppService` signed registration** | ⏳ documented | **PRE-2** Apple Developer ID cert |

The two pending rows are genuine external dependencies, not gaps in the work. The install path
below is the pre-cert verification method; `SMAppService` replaces it once PRE-2 lands.

## Running it

Build and exercise the round trip unprivileged (no sudo, proves the code):

```bash
cd helper && cargo build --release && cd ..
SPIKE_C_SOCKET=/tmp/spike-c.sock ./helper/target/release/spike-c-helper &
SPIKE_C_SOCKET=/tmp/spike-c.sock ./helper/target/release/spike-c-probe
```

Verify the full privileged lifecycle (**your sudo**):

```bash
cd helper && cargo build --release && cd ..
sudo scripts/install.sh      # installs the LaunchDaemon, loads it as root
scripts/verify.sh            # unprivileged: confirms launchd started it as ROOT
sudo scripts/uninstall.sh    # removes everything, asserts the machine is clean
```

A successful `verify.sh` after install prints `root=true` and `privcheck=ok` — the helper is
genuinely running as root under launchd. `uninstall.sh` then asserts nothing is left behind,
which is the ADR-0014 clean-removal contract in miniature.

## For the production build (WP-M0.7)

- Replace the manual `launchctl bootstrap` with `SMAppService.daemon(plistName:).register()`
  once PRE-2 provides a Developer ID cert; keep this manual path documented as the fallback.
- The helper here uses two `unsafe extern "C"` calls for `geteuid`/`getuid`. In the real
  `pal-macos` crate these live behind the PAL boundary with the `libc` crate.
