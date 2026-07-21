# ADR-0001: Split-privilege architecture — privileged daemon, unprivileged clients

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-1 · ARCHITECTURE.md §4
- **Supersedes:** —

## Context

Nearly every capability NetworkHub offers requires operating-system privilege: firewall rules
(root/Administrator), hotspot hosting, DHCP server sockets on port 67, tunnel interface
creation, traffic shaping, and raw interface statistics. A graphical application must never
run with those privileges — it loads fonts, renders untrusted strings, links a large UI
toolkit, and is exposed to the user's session.

The product also needs to work with no desktop session at all: a Raspberry Pi in a café
closet, administered remotely, is a first-class deployment target rather than an afterthought.

## Decision

We will build NetworkHub as **one privileged system service** (`networkhubd`) that owns all
network state, plus **thin unprivileged clients** — the Flutter desktop application, the CLI,
and later a web dashboard and mobile controllers — which are pure API consumers holding no
authoritative state of their own.

## Consequences

**Positive**
- The attack surface running as root is a Rust service with a narrow, typed API, not a GUI
  framework and its transitive dependency tree.
- Headless operation is free: the daemon never depended on a session, so a Pi install and a
  desktop install are the same product.
- The UI can crash, be closed, or be updated independently. A running hotspot or VPN is
  unaffected — an important property, since users close windows.
- Multiple simultaneous clients (desktop app, CLI script, phone) work with no extra design.

**Negative**
- Every feature pays an IPC tax: contracts, serialisation, versioning, and an authorisation
  check at the boundary.
- Installation is heavier than a portable application: a service, an installer, and elevation
  at install time.
- Daemon and UI can be at different versions, requiring explicit skew handling (see ADR-0013).

**Neutral**
- Forces authorisation to be designed once, at one boundary, rather than scattered.
- Creates the need for a mock daemon so UI development does not require root (`daemon-sim`).

## Alternatives considered

**Single privileged application** — the whole product runs elevated. Rejected: it puts a UI
toolkit and its dependencies in the root attack surface, prompts for elevation constantly,
and cannot work headless.

**Unprivileged app with per-operation elevation** (UAC/polkit prompt per action) — rejected:
unusable prompt fatigue for a product whose core loop is continuous reconciliation, and it
cannot maintain long-lived state such as a running hotspot.

**Helper tool invoked on demand** (setuid or a small privileged shim) — rejected: the
reconciliation model (ADR-0004) requires a continuously running process that watches OS state
and converges it; an on-demand shim cannot detect external interference or recover after a
crash.
