# ADR-0005: Fail-open safety invariant — never strand the user offline

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-5 · ARCHITECTURE.md §11
- **Supersedes:** —

## Context

The worst outcomes in this product category are not crashes; they are the states a crash
leaves behind. A firewall tool that dies holding a default-deny rule set, or a VPN kill switch
that survives the process that created it, leaves the user with no network and no obvious way
to recover — possibly no way to search for the fix, because they cannot reach the internet.
Orphaned rules that survive uninstallation are worse still: the user has removed our software
and their machine is still broken.

## Decision

We adopt a global invariant, tested rather than hoped for:

> **From any single crash, kill, or power loss at any point, the next daemon start — or,
> failing that, the boot-time cleanup path — returns the system to either the user's desired
> state or the pre-NetworkHub state. Never a third thing.**

This requires that all NetworkHub-owned OS state be:

1. **Attributable** — every rule, table, anchor, filter, and `sysctl` change is tagged or
   namespaced so it can be identified as ours,
2. **Reversible** — removable by a single documented command, and
3. **Deadman-checked** — a boot-time path cleans up non-persistent state if the daemon does
   not come back.

The **VPN kill switch is the sole deliberate exception**, since a fail-open kill switch is not
a kill switch. It is therefore constrained: non-persistent modes are removed by the deadman
check, and the persistent mode requires explicit opt-in and ships with an offline escape hatch
that works with the daemon dead.

## Consequences

**Positive**
- The most user-hostile failure class in the product is designed out rather than patched later.
- Gives fault-injection testing a precise oracle: kill the process anywhere, assert one of two
  acceptable end states.
- Makes the uninstall contract (ADR-0014) mechanically checkable via state diffing.

**Negative**
- Every feature that touches OS state costs more: tagging, journaling, a cleanup path, and a
  fault test. There is no cheap way to add a firewall rule.
- Boot-time cleanup machinery must exist on all three platforms before the first rule ships
  (WP-M0.7), ahead of any feature that needs it.

**Neutral**
- Elevates availability to a security property, which is reflected in SECURITY.md's scope.

## Alternatives considered

**Best-effort cleanup with documentation for manual recovery** — rejected: the user cannot
read online documentation when they have no network.

**Fail-closed by default for safety** — rejected: appropriate for a firewall appliance whose
operator has console access, actively harmful for consumer software on a personal laptop.

**Rely on OS-level transient state** (rules that vanish on reboot) — rejected: not available
consistently across platforms, and incompatible with configuration that must survive reboot.
