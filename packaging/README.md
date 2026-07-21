# Packaging

Native installers only; store sandboxes deferred (ADR-0014).

| Directory | Format | Work package |
|---|---|---|
| `windows-msi/` | MSI (WiX v4) | WP-M0.13 |
| `linux-deb/`, `linux-rpm/` | deb, rpm + signed repo | WP-M0.13 |
| `appimage/` | AppImage (UI + CLI only) | WP-M0.13 |
| `macos/` | DMG + SMAppService helper | WP-M0.13 |

Every installer is held to the uninstall contract: a machine after
install-use-uninstall must diff clean against its pre-install snapshot.
