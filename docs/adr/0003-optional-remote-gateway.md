# ADR-0003: Remote access is a separate, opt-in gateway process

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-3 · ARCHITECTURE.md §6.2
- **Supersedes:** —

## Context

IT professionals, café operators, and headless Raspberry Pi users need remote administration
and a REST API. But an internet-reachable HTTP surface on a process that controls the firewall
is the highest-risk component in the product, and most users will never need it.

## Decision

We will implement remote access as a **separate binary (`nh-gateway`), running as its own
unprivileged OS user**, which proxies the daemon API and exposes REST + WebSocket over TLS. It
is **disabled by default**. Enabling it requires explicitly setting an administrator password,
configuring TLS, and choosing a bind scope (defaulting to LAN rather than all interfaces).

The gateway is deliberately dumb: it terminates TLS, authenticates, rate-limits, and
translates. **Every authorisation decision is re-checked by the daemon** against the forwarded
principal.

## Consequences

**Positive**
- A default install has no listening network socket at all.
- Compromising the HTTP surface does not directly yield root: the attacker lands in an
  unprivileged process holding a scoped credential, and the daemon still enforces its own
  authorisation.
- The gateway can be updated, restarted, or disabled without touching network state.
- Web dashboard and mobile controllers (Phase 3/4) get a well-defined home.

**Negative**
- A second process to package, supervise, configure, and keep version-compatible.
- Two authorisation checks means two places to keep a permission model correct — mitigated by
  generating both from the same protobuf annotations (ADR-0013).

**Neutral**
- Users who want remote access must take a deliberate action, which is a support cost and a
  security benefit.

## Alternatives considered

**Serve REST directly from the daemon behind a feature flag** — rejected: a bug in HTTP
parsing, TLS handling, or session management would be a bug in the root process.

**Require an external reverse proxy (nginx/Caddy)** — rejected: pushes TLS and authentication
onto users who are least equipped to get them right, and makes the "works on a Pi out of the
box" story much worse.

**Cloud relay only, no direct remote access** — rejected: contradicts the local-first,
cloud-optional principle, and fails for air-gapped and privacy-focused deployments.
