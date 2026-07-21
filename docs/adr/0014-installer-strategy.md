# ADR-0014: Native installers only — MSI, deb/rpm, DMG; store sandboxes deferred

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-14 · ARCHITECTURE.md §13
- **Supersedes:** —

## Context

NetworkHub installs a privileged system service, creates an OS group, registers boot-time
cleanup hooks, and must uninstall all of that cleanly. Modern application-store packaging
formats — MSIX, Snap (strict), Flatpak, and the Mac App Store — are built around sandboxed,
unprivileged applications. They actively prevent what this product must do, and their
confinement models fight the service lifecycle.

## Decision

Version 1 ships **native installers**:

- **Windows**: MSI built with WiX v4, per-machine, installing the service with recovery
  actions and creating a `NetworkHub Operators` group. Signed with an EV certificate.
- **Linux**: **deb and rpm** as the primary formats, with a signed apt/dnf repository.
  **AppImage** for the UI and CLI only, connecting to a separately installed daemon.
- **macOS**: **DMG**, with the privileged helper registered via `SMAppService`, hardened
  runtime, Developer ID signing, and notarisation.

**MSIX, Snap, and Flatpak are deferred**, not rejected forever; a classic (unconfined) Snap
remains possible later.

Every installer is held to the **uninstall contract**: firewall artifacts, DNS and system
configuration changes, `sysctl` values, services, and runtime directories are all removed, and
a machine after install-use-uninstall must diff clean against its pre-install snapshot.

## Consequences

**Positive**
- Each platform's native mechanism does what we need without fighting confinement.
- The uninstall contract is enforceable and automatically testable via VM snapshots and the
  state differ.
- Signed repositories give Linux users a normal, trusted update path.

**Negative**
- No app-store distribution, and therefore no store discovery or store-managed updates.
- Five packaging formats to build, sign, and test on every release.
- Windows per-machine installation always requires elevation; there is no portable mode.

**Neutral**
- Directly determines the update strategy: installs we own can self-update, distribution
  packages must not (ADR-0015).

## Alternatives considered

**MSIX on Windows** — rejected for version 1: packaged services and the MSIX container model
add friction for a system service, for benefits (store presence, clean uninstall) we already
obtain by other means.

**Flatpak/Snap as the primary Linux format** — rejected: strict confinement is fundamentally
incompatible with a privileged network daemon.

**Portable/no-install mode** — rejected: incompatible with running as a system service that
must survive reboot.
