# ADR-0011: Coexist with the distribution's network stack; own only what we manage

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-11 · ARCHITECTURE.md §3.2, §8.3
- **Supersedes:** —

## Context

A Linux desktop already has an opinionated network stack. NetworkManager owns interfaces and
will reassert that ownership. `systemd-resolved` owns DNS resolution and binds a stub listener.
`firewalld` or `ufw` manages nftables with its own model. Raspberry Pi OS, Ubuntu, Fedora, and
Arch differ in which of these are present and how they are configured.

Software that flushes the firewall, seizes interfaces, or rewrites `resolv.conf` will appear
to work on the developer's machine and break unpredictably everywhere else — and will be
blamed for breakage it did not cause.

## Decision

We will behave as a **guest in the existing stack**:

- Integrate with **NetworkManager over D-Bus** when present; use raw tooling (`hostapd`) only
  where necessary, and then mark the interface `unmanaged` in NetworkManager first and restore
  ownership on teardown.
- Place all firewall rules in **dedicated nftables tables** (`inet networkhub_*`) with
  documented hook priorities. We never flush or modify tables belonging to `firewalld`, `ufw`,
  or anyone else, and we never disable the host firewall.
- Bind DHCP and DNS services **only to addresses on interfaces we manage**, never a wildcard.
- Integrate with `systemd-resolved` by setting per-link DNS rather than fighting for port 53.
- Journal and restore any `sysctl` we change, `net.ipv4.ip_forward` above all.

Where we detect a conflict we cannot resolve, we **report it with the probable culprit named**
rather than forcing our way through.

## Consequences

**Positive**
- Installing NetworkHub does not break the user's existing network configuration.
- Our state is clearly attributable, which is what makes clean uninstallation (ADR-0005,
  ADR-0014) achievable.
- Conflicts become diagnosable support conversations instead of mysteries.

**Negative**
- Two code paths for hotspot on Linux (NetworkManager and `hostapd`), each needing testing.
- D-Bus integration is more work than shelling out, and NetworkManager's API has its own
  version variance.
- Some capabilities are unavailable when another component owns the resource, so we must
  degrade and explain rather than override.

**Neutral**
- Distribution-specific behaviour is discovered empirically; the coexistence matrix is a
  living document rather than a one-time design.

## Alternatives considered

**Take exclusive ownership of the network stack** — rejected: hostile to users, unwinnable
against NetworkManager's reconciliation, and guarantees blame for unrelated breakage.

**Require the user to disable NetworkManager/firewalld** — rejected: a support and trust
disaster, and impossible for managed corporate machines.

**Support only one configuration (NetworkManager-only)** — rejected: excludes servers,
Raspberry Pi variants, and minimal distributions that are core to the headless use case.
