# ADR-0010: SQLite via SQLx, split config/metrics stores, secrets in the platform keychain

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-10 · ARCHITECTURE.md §7
- **Supersedes:** —

## Context

The daemon must persist configuration, device records, audit logs, and time-series metrics,
locally, with no server, on hardware ranging from a workstation to a Raspberry Pi writing to
an SD card. It also handles material that must never sit in a database file: VPN private keys,
Wi-Fi passphrases, API key material, and token-signing keys.

Metrics and configuration have opposite characteristics. Metrics are high-volume, append-heavy,
and disposable. Configuration is small, precious, and must survive.

## Decision

- **SQLite** as the storage engine, accessed through **SQLx** with **hand-written,
  forward-only migrations**. We prefer SQLx over an ORM because schema and migration control
  matter more to us than mapping convenience, and because migrations here are a correctness
  and data-loss concern.
- **Two database files**: `config.db` (settings, profiles, devices, audit log, intent journal
  — WAL, synchronous writes for journal entries) and `metrics.db` (time-series and rollups —
  batched writes, purgeable).
- **Secrets never go in SQLite.** They live in the platform keychain (DPAPI machine scope,
  macOS system Keychain, Secret Service, or a root-only encrypted file for headless Linux).
  The database may hold a *reference* or a ciphertext blob whose key lives in the keychain.

## Consequences

**Positive**
- Deleting all history is one file, with no risk to configuration. "Delete my data" becomes
  trivially correct.
- Corruption blast radius is separated: a damaged `metrics.db` never costs the user their
  firewall rules or VPN profiles.
- Different durability settings per store: expensive synchronous writes where they matter,
  batched writes where they do not — which is what makes SD-card longevity manageable.
- Stealing the database file yields no secrets.

**Negative**
- No cross-database transactions or joins; anything spanning both stores is coordinated in
  application code.
- Two connection pools, two migration histories, two backup policies.
- Keychain APIs differ per platform and are awkward for a system service, especially on
  headless Linux where no keyring daemon exists — hence a fallback we must design carefully.

**Neutral**
- Hand-written migrations are more work per change and considerably safer at upgrade time.
- Requires an explicit corruption-recovery path: `quick_check` on open, quarantine, restore
  from backup, and a safe degraded boot with empty desired state — which is fail-open by
  construction (ADR-0005).

## Alternatives considered

**One database for everything** — rejected: metrics growth and churn would put configuration
at risk, and purging history would mean deleting rows from the same file that holds the user's
settings.

**An embedded key-value store (sled, RocksDB)** — rejected: we want relational queries and
ad-hoc inspection for analytics and support, and SQLite's operational track record is
unmatched.

**A full ORM (SeaORM/Diesel)** — rejected: the abstraction obscures exactly the schema and
migration details we most need to control.

**Encrypting the whole database (SQLCipher)** — rejected as the primary mechanism: it needs a
key from somewhere, which is the original problem, and it would not stop a compromised daemon
from reading everything anyway. Considered as optional at-rest defence for headless machines.
