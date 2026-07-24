# Packaging

Native installers only; store sandboxes deferred (ADR-0014).

| Directory | Format | Builder | Work package |
|---|---|---|---|
| `linux/` | deb + rpm (one nfpm config) | `linux/build.sh` (nfpm) | WP-M0.13 ✅ |
| `linux-systemd/` | systemd units + manual install scripts | — | WP-M0.7 ✅ |
| `windows-msi/` | MSI (WiX v4) | `windows-msi/build.ps1` | WP-M0.13 |
| `appimage/` | AppImage (UI + CLI only) | `appimage/build.sh` | WP-M0.13 |
| `macos/` | DMG + `SMAppService` helper + plists | `macos/build-dmg.sh` | WP-M0.13 |

`linux/` supersedes the planned separate `linux-deb/` / `linux-rpm/` dirs: nfpm
produces both formats from one config, so they share scriptlets and stay in
sync. A signed apt/yum repo is a later (release) concern.

Every installer is held to the uninstall contract: a machine after
install-use-uninstall must diff clean against its pre-install snapshot.
