# ADR-0015: Signed updates with rollback; never self-update package-manager installs

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-15 · ARCHITECTURE.md §13.4
- **Supersedes:** —

## Context

The update channel is the single highest-value target in the product: compromising it yields
remote root on every installation. It is also a reliability hazard — a bad update to a service
that controls the firewall can leave machines unreachable, and if a kill switch is active,
offline with no path to a fix.

Separately, files installed by `apt` or `dnf` belong to the package manager. Software that
overwrites them corrupts the package database and breaks the user's system in ways that are
confusing and hard to reverse.

## Decision

**Integrity.** Updates are distributed as **TUF-style signed manifests** with an offline root
key held on hardware, an online targets key, per-file hashes, and anti-rollback version
counters. Signatures are verified **before any file is replaced**. Manifests are fetched over
TLS with certificate pinning.

**Reliability.** Updates apply in stages: verify signatures → stage files → start the new
daemon in `--verify` mode (load configuration, probe capabilities, exit) → switch over → health
check → **automatic rollback to the previous version on failure**. Kill-switch state surviving
the update path is explicitly tested.

**Channel policy.** Self-updating applies **only to installations we own** — MSI, DMG,
AppImage. For deb and rpm installations, the updater **notifies only**; updates flow through
the distribution package manager and our signed repository. These two paths are never mixed.

Daemon and UI ship together but tolerate one version of skew via `min_client_version`
(ADR-0013).

## Consequences

**Positive**
- A compromised update server is insufficient to ship malicious code; signing keys are
  required, and the root key is offline.
- A bad release rolls back automatically rather than requiring users to recover a machine that
  may have no network.
- Linux users get updates through the mechanism they already trust and audit.

**Negative**
- Key management is a real operational burden: a key ceremony, hardware custody, a documented
  revocation drill, and the risk of losing the root key entirely.
- Staged-apply-with-rollback is significantly more complex than replace-and-restart.
- Two update mechanisms to build and maintain, plus repository hosting.

**Neutral**
- Requires the release pipeline to produce signed manifests and an SBOM per artifact, which
  supports the supply-chain goals in ARCHITECTURE.md §10.2.

## Alternatives considered

**Simple "download and replace" self-updater** — rejected: no rollback, and signature
verification bolted on afterwards is usually verification in name only.

**Package managers only, on every platform** — rejected: Windows and macOS have no universally
installed equivalent, and requiring winget or Homebrew excludes most users.

**Self-update everywhere, including deb/rpm** — rejected: corrupts the package database and
violates the user's trust in their own system's package management.
