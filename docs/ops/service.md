# Running NetworkHub as a system service

The daemon (`nh-daemon`) is a privileged, always-on system service (AD-1). This
page documents what gets installed on each OS, how to install and remove it, and
the **verification protocol** for the WP-M0.7 exit criteria — reboot survival,
crash recovery, and the F-2 boot-cleanup safety net. The uninstall path is the
uninstall contract (ADR-0014): everything an install creates, an uninstall
removes.

The three OSes share one design and differ only in the init system:

| | Linux | macOS | Windows |
|---|---|---|---|
| Init system | systemd | launchd | Service Control Manager |
| Main unit | `networkhub.service` | `com.networkhub.daemon` | service `networkhubd` |
| Boot-cleanup (F-2) | `networkhub-cleanup.service` (oneshot, `Before=network-pre.target`) | `com.networkhub.cleanup` (`RunAtLoad`) | on-start step inside the service |
| Readiness signal | `sd_notify` (`Type=notify`) | (none; launchd keepalive) | SCM `Running` status |
| Restart on crash | `Restart=on-failure` | `KeepAlive` | recovery actions (3× restart, 2s delay) |
| Binary path | `/usr/libexec/networkhub/nh-daemon` | `/usr/local/libexec/networkhub/nh-daemon` | chosen at install (`current_exe`) |
| Runtime dir | `/run/networkhub` | `/var/run/networkhub` | `%ProgramData%\NetworkHub` (planned) |

## The boot-cleanup deadman (F-2)

The single most important safety property: **a broken daemon must never strand
the machine.** From M1 the daemon can install a kill-switch firewall rule; if it
then crashes or fails to start, that rule would keep the machine offline with no
daemon left to remove it. The boot-cleanup runs *independently of the daemon*,
early at boot, and reclaims NetworkHub-owned OS state (see
[`cleanup.rs`](../../crates/nh-daemon/src/cleanup.rs)). On Linux and macOS it is
a separate boot job ordered before the network comes up; on Windows, which has no
equivalent strict ordering, the service performs the same check on start before
it reports `Running`.

In M0 the only owned state is a stale socket and the intent journal, so the
cleanup is mostly machinery — but it is built now, before anything can need it,
which is the whole point.

## Install / uninstall

Build first: `cargo build --release -p nh-daemon`.

### Linux

```sh
sudo packaging/linux-systemd/install.sh          # installs, enables, starts
sudo packaging/linux-systemd/uninstall.sh        # stops, disables, removes
sudo packaging/linux-systemd/uninstall.sh --purge  # also removes /var/lib/networkhub + group
```

Install creates the `networkhub` group (socket group-access, ADR-0002), installs
the binary and both units, and enables the cleanup + main services.

### macOS

```sh
sudo packaging/macos/install.sh
sudo packaging/macos/uninstall.sh [--purge]
```

The manual `launchctl bootstrap` path is used until a Developer ID certificate
(PRE-2) enables the signed `SMAppService` registration (ADR-0012).

### Windows (elevated PowerShell / cmd)

```
nh-daemon.exe install      :: register with the SCM, auto-start, restart-on-failure
nh-daemon.exe uninstall    :: stop and remove
```

`install` registers the service to run `nh-daemon service`, which is the SCM
entry point (never run `service` by hand). See **Status** below for what is
verified.

## Verification protocol (WP-M0.7 exit criteria)

These require real hardware + privileges and are the `[you-verify]` steps. Run
one block per OS.

### 1. Survives reboot

1. Install (above). Confirm running:
   - Linux: `systemctl status networkhub.service` → `active (running)`
   - macOS: `sudo launchctl print system/com.networkhub.daemon` → `state = running`
   - Windows: `sc query networkhubd` → `STATE: RUNNING`
2. Reboot.
3. After boot, confirm it is running again **and** that the cleanup ran before it
   (Linux: `journalctl -u networkhub-cleanup.service -b`; macOS:
   `/var/log/networkhub-cleanup.log`; Windows: Event Log / the daemon's
   `on-start cleanup complete` log line).

### 2. Recovers from a crash (kill -9)

1. Find the PID (`systemctl show -p MainPID networkhub.service` / `launchctl` /
   `sc query`).
2. `kill -9 <pid>` (Windows: `taskkill /F /PID <pid>`).
3. Within the restart delay, confirm a **new** PID is serving. The service
   manager restarts it per policy (systemd `on-failure`, launchd `KeepAlive`,
   SCM recovery actions).

### 3. F-2 boot-cleanup reclaims orphaned state

This is the safety net itself. In M0, simulate with the socket; from M1, repeat
with a real tagged firewall artifact.

1. Stop the daemon but leave state behind:
   - Create a stale socket/marker in the runtime dir, or (M1+) install a
     NetworkHub-tagged firewall rule, then `kill -9` the daemon so it cannot
     clean up.
2. Prevent the daemon from starting (temporarily rename the binary, or mask the
   main unit: `systemctl mask networkhub.service`).
3. Reboot.
4. Confirm the **cleanup job still ran** and removed the orphaned artifact even
   though the daemon never started. This proves the safety net does not depend on
   a healthy daemon.
5. Unmask / restore the binary and reboot back to a healthy state.

## Status (what is verified, what is hardware-gated)

Consistent with the project's Windows posture, runtime verification that CI
cannot perform is marked for the hardware pass:

- **Verified in CI (all 3 OSes):** the daemon builds and lints; the boot-cleanup
  logic and the singleton lock have unit tests that run on every OS; the Windows
  named-pipe transport has an in-process client↔server test that runs on
  `windows-latest` (accepts a connection, resolves the client PID, round-trips
  bytes). The Unix end-to-end IPC + authorization test runs on Linux/macOS.
- **Hardware-gated (`[you-verify]`):** the three protocols above; the Windows SCM
  install/reboot/recovery runtime (the SCM cannot be exercised in CI).
- **Deferred to M1, with reason:**
  - *Windows pipe ACL (SDDL).* The pipe currently carries its default security
    descriptor. The security boundary is still enforced: every RPC is authorized
    by peer credential, and a pipe client is granted only the read-only role in
    M0. The SDDL narrowing (SYSTEM / Administrators / `NetworkHub Operators`) is
    added and verified on the box in M1.
  - *Windows privileged-role mapping.* M0 maps every identified pipe client to
    Viewer (fail-closed); mapping an elevated/admin caller to the Admin role
    needs client-token/SID inspection and arrives with M1's first privileged RPC.
  - *Windows descendant-kill (Job Object).* `kill_on_drop` handles the immediate
    child today; the Job Object arrives with the first real supervised child in
    M1 (M0 spawns none).
