# Packaging & the uninstall contract (WP-M0.13)

NetworkHub ships as native installers, one per platform. Every installer is held
to the **uninstall contract** (ADR-0014, §13.5): a machine after
install → use → uninstall must be indistinguishable from before. All artifacts
here are **unsigned / dev-signed**; real Authenticode / Developer ID signing and
notarization are M3 (gated on PRE-1 / PRE-2).

## Formats and builders

| Platform | Format | Builder | Contents | Status |
|---|---|---|---|---|
| Linux | deb + rpm | [`packaging/linux/build.sh`](../../packaging/linux/build.sh) (nfpm) | daemon + CLI + systemd units | ✅ builds locally |
| Linux | AppImage | [`packaging/appimage/build.sh`](../../packaging/appimage/build.sh) | UI + CLI only (no daemon) | authored `[you-verify]` |
| macOS | DMG | [`packaging/macos/build-dmg.sh`](../../packaging/macos/build-dmg.sh) (hdiutil) | app + daemon + CLI + launchd plists | ✅ builds locally |
| Windows | MSI | [`packaging/windows-msi/build.ps1`](../../packaging/windows-msi/build.ps1) (WiX v4) | daemon (as a service) + CLI | authored `[you-verify]` |

One nfpm config produces both deb and rpm, so their maintainer scriptlets stay
in sync. The AppImage deliberately carries no privileged daemon — it talks to a
separately-installed daemon or to `daemon-sim`.

## Building

```sh
# Build the release binaries first (per target OS):
cargo build --release -p nh-daemon -p nh-cli

# Linux packages (deb + rpm) — needs nfpm on PATH:
packaging/linux/build.sh                 # → packaging/out/

# macOS DMG — optionally build the GUI first:
(cd apps/desktop && flutter build macos --release)
packaging/macos/build-dmg.sh             # → packaging/out/

# Windows MSI (PowerShell, WiX v4 `wix` on PATH):
packaging\windows-msi\build.ps1          # → packaging\out\

# Linux AppImage — needs the Flutter Linux build + appimagetool:
(cd apps/desktop && flutter build linux --release)
packaging/appimage/build.sh              # → packaging/out/
```

## Where things go (Linux deb/rpm)

| Path | What |
|---|---|
| `/usr/libexec/networkhub/nh-daemon` | the privileged daemon |
| `/usr/bin/nh-cli` | the CLI client |
| `/usr/lib/systemd/system/networkhub{,-cleanup}.service` | the units |
| `/var/lib/networkhub/` | config + state (removed only on deb `purge`) |
| `/run/networkhub/` | runtime socket / lock (removed on uninstall) |

The maintainer scriptlets create the `networkhub` group, enable the cleanup
deadman + main service, and on removal run `nh-daemon cleanup` to reclaim any
NetworkHub-owned OS state **before** the binary is deleted — the uninstall
contract as a package scriptlet.

## The uninstall contract, verified

The differ [`state-diff`](../../tools/state-diff) snapshots the network-relevant
machine state (firewall, routes, DNS, services, sysctls) and diffs two
snapshots. A non-empty diff means NetworkHub changed the machine and did not
change it back — a failing test.

```sh
state-diff snapshot before.json
# install the package, use it, uninstall it
state-diff snapshot after.json
state-diff diff before.json after.json   # exit 0 = clean; 1 = leftover state
```

[`tests/e2e/uninstall-contract.sh`](../../tests/e2e/uninstall-contract.sh)
automates this. It runs in the **nightly** `uninstall-contract` job on hosted
runners (a clean-VM proxy). Today it runs the full contract on Linux (build deb →
install → use → purge → diff); the macOS and Windows package-install flows are a
follow-up and skip cleanly.

### `[you-verify]` — the real VM gate (the M0 exit criterion)

Hosted runners approximate a clean machine but are not a substitute for a real
snapshot-revert VM per OS. On each of your Windows / Linux / macOS VMs:

1. Take a clean VM snapshot.
2. `state-diff snapshot before.json`.
3. Install the package; start the daemon; drive the CLI / UI.
4. Uninstall the package.
5. `state-diff snapshot after.json`.
6. `state-diff diff before.json after.json` — **must be empty**.
7. Revert the VM snapshot for the next OS/run.

A clean diff on all three OSes is the M0 gate. Any leftover (a firewall rule, a
service, a sysctl, a file) is a bug in the corresponding uninstall path.
