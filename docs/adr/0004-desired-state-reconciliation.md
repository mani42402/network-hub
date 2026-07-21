# ADR-0004: Desired-state reconciliation, not imperative mutation

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-4 · ARCHITECTURE.md §5.3
- **Supersedes:** —

## Context

Network state on a real machine is under constant external pressure: interfaces come and go,
the machine sleeps and resumes, DHCP leases change the upstream, users reboot, `firewalld`
rewrites tables, antivirus products install their own filters, and NetworkManager reasserts
ownership of a device. A design that applies configuration once — "add rule, done" — silently
drifts out of correctness and cannot recover from a crash mid-change.

## Decision

We will model all network configuration as **declarative desired state** persisted in
`config.db`, with per-module **reconcilers** that continuously converge observed OS state
toward it. API mutations write desired state and return; convergence progress is reported via
events rather than by blocking the call.

Three mechanisms make this safe:

1. An **intent journal**: before any OS mutation, an intent record with the prior value is
   written synchronously, so an interrupted change can be replayed or undone at next start.
2. **Circuit breaking**: repeated failures mark a module `degraded` with an actionable error
   rather than hot-looping.
3. **Interference detection**: if something external repeatedly reverts our state, we stop
   after three restore cycles in sixty seconds, mark the module `conflicted`, and name the
   probable culprit. We never enter an infinite war with `firewalld` or an antivirus product.

## Consequences

**Positive**
- Crash, power loss, reboot, and sleep/resume all become the same recovery path.
- The fail-open invariant (ADR-0005) becomes testable: reconciliation has a defined target.
- External interference is detected and *reported* rather than silently fought or silently lost.
- Long operations stop blocking API calls, which makes the UI responsive by construction.

**Negative**
- Substantially more machinery than direct mutation, and it must be built before the first
  feature that uses it (WP-M0.4) — real up-front cost with no visible product.
- Every OS mutation must be expressible as convergence toward a target, which is awkward for
  genuinely imperative operations (a speed test, a DNS cache flush). These are modelled as
  commands outside the reconciliation loop.
- Synchronous journal writes add latency to every mutation.

**Neutral**
- Requires state watchers (netlink, `NotifyIpInterfaceChange`, SCDynamicStore) as a
  prerequisite rather than an optimisation.

## Alternatives considered

**Imperative apply with rollback on error** — rejected: handles the error path but not the
*interrupted* path, and cannot recover state after an external actor changes it.

**Apply on startup only, then trust the OS** — rejected: drift is guaranteed on a machine that
sleeps, roams between networks, and runs other network software.

**Full transactional model with two-phase commit across subsystems** — rejected as
over-engineered: the OS provides no such guarantees, so we would be simulating a transaction
we cannot actually enforce.
