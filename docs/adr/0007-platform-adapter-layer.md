# ADR-0007: Platform Adapter Layer as Rust traits with a mock backend

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-7 · ARCHITECTURE.md §5.2
- **Supersedes:** —

## Context

The same logical operations — apply a firewall rule set, start an access point, set
per-interface DNS, enable forwarding — are implemented through entirely different mechanisms
on each platform: WFP and WinRT on Windows, netlink/nftables/hostapd on Linux, PF anchors and
SystemConfiguration on macOS. Without a seam, this variance spreads through every feature
crate, and no logic can be tested without the corresponding OS and hardware.

## Decision

We will define the platform boundary as **Rust traits** in `nh-pal`, with four
implementations: `pal-linux`, `pal-windows`, `pal-macos`, and `pal-mock`. Feature crates
depend on the **traits only**; `nh-daemon` is the sole crate that selects a concrete backend.
This is enforced mechanically by `cargo xtask check-layers` rather than left to discipline.

Two conventions make the boundary useful rather than decorative:

- Every mutating call either returns a rollback guard or writes an intent-journal record
  first, because RAII alone does not survive a killed process (ADR-0004).
- Calls are idempotent where the OS permits — "ensure this rule exists", not "add this rule".

## Consequences

**Positive**
- The majority of logic — reconcilers, rule compilation, state machines, quota arithmetic,
  scheduling — is testable against `pal-mock` on any machine, with no root and no hardware.
- Fault injection becomes straightforward: the mock can fail at any step, including after a
  partial mutation.
- Adding a platform is a bounded task with a clear definition of complete.
- `unsafe` FFI is confined to three crates, which are the only ones permitted to contain it.

**Negative**
- Trait shape must be designed before three backends exist, and a wrong shape is the most
  likely source of mid-project refactoring (flagged as a risk in WP-M0.3).
- There is a real temptation to leak platform concepts into trait signatures; resisting it
  costs design time on every addition.
- A mock that diverges from real behaviour produces tests that pass while the product breaks.

**Neutral**
- Makes `daemon-sim` — the unprivileged mock daemon used for UI development and UI CI —
  nearly free.

## Alternatives considered

**Conditional compilation (`#[cfg(target_os)]`) inside feature crates** — rejected: nothing
becomes testable off-platform, and platform logic tangles with domain logic permanently.

**A process-boundary platform helper with a serialised protocol** — rejected: serialisation
overhead and a second supervision problem, for a boundary that does not need process
isolation.

**Runtime dynamic dispatch via a plugin interface** — rejected: no benefit over static traits
here, and it makes the privileged code path harder to audit.
