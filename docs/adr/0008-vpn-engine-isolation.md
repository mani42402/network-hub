# ADR-0008: VPN engines isolated — WireGuard in-process, OpenVPN as a child process

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-8 · ARCHITECTURE.md §8.2, §16
- **Supersedes:** —

## Context

Two VPN protocols matter for the MVP. They have very different integration profiles:

- **WireGuard** is small, has permissive-licensed implementations, and benefits from tight
  integration (state, statistics, roaming, and the kill switch all interact).
- **OpenVPN** is a large, mature C codebase licensed **GPLv2**. Linking it into our
  Apache-2.0 daemon would impose GPL obligations on our own source — incompatible with the
  planned commercial enterprise tier — and would place a large C parser inside the root
  process.

Additionally, imported `.ovpn` files are untrusted input that can contain directives (`up`,
`down`, `script-security`, `plugin`) which execute arbitrary commands as root. This is a
well-known vulnerability class in VPN clients.

## Decision

- **WireGuard**: implemented in-process — kernel WireGuard via netlink on Linux where
  available, userspace engine otherwise, `wintun` on Windows, `utun` on macOS.
- **OpenVPN**: bundled as an **unmodified separate executable**, supervised as a child process
  and controlled over its management interface. Never linked. Credentials pass over the
  management channel, never via argv or environment variables.
- **All imported profiles are parsed and re-emitted through a sanitiser** that strips every
  directive capable of executing code.

## Consequences

**Positive**
- The GPL boundary is a process boundary, which is unambiguous and defensible.
- An OpenVPN crash or memory-safety bug cannot corrupt the daemon.
- The script-injection vulnerability class is closed by construction: we never pass through a
  profile we did not generate ourselves.
- WireGuard gets the deep integration its feature set needs.

**Negative**
- Packaging and shipping a third-party binary per platform, with its own update and CVE
  tracking obligations, plus a source offer and attribution.
- Two very different code paths for what users perceive as one feature.
- The management interface is a less expressive control channel than a library API; some
  OpenVPN behaviours are only observable by parsing its log output.
- Sanitising profiles will reject some legitimate advanced configurations, which will generate
  support requests.

**Neutral**
- Establishes the child-process supervision machinery (process groups, job objects,
  kill-on-close) that `hostapd` later reuses.

## Alternatives considered

**Link OpenVPN as a library** — rejected: GPL obligations on our source, and a large C
attack surface inside the root process.

**Drop OpenVPN, ship WireGuard only** — considered and retained as a descoping lever, not a
default: corporate and legacy deployments still require OpenVPN.

**Pass `.ovpn` files through unmodified** — rejected outright. This is remote code execution
as root, triggered by opening a file a user was emailed.
