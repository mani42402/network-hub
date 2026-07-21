# ADR-0006: Capability-driven feature model

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-6 · ARCHITECTURE.md §3.4, §5.4
- **Supersedes:** —

## Context

Whether a given machine can host a Wi-Fi access point, run access-point and client mode
simultaneously, use 5 GHz for an AP, isolate clients, or shape per-device bandwidth is not a
function of the operating system alone. It depends on the Wi-Fi chipset, the driver, the
regulatory domain, and sometimes the specific firmware. Realtek USB adapters routinely
misreport their own capabilities. macOS has no public hotspot API at all; Windows cannot
provide a custom hotspot subnet.

Encoding this as `if (Platform.isWindows)` in the UI would produce a codebase where every new
platform quirk touches presentation code, and where users see controls that fail rather than
explanations.

## Decision

The daemon will **probe actual capabilities at startup and on hardware change**, publishing a
capability matrix in which every entry is `{status: supported | degraded | unsupported,
reason, details}`. The UI renders exclusively from this matrix. Unsupported features display
the reason; they are never hidden silently and never presented as working controls.

**No platform conditionals in feature code.** Adding support for a platform means shipping a
better matrix, not editing the UI.

## Consequences

**Positive**
- Honest UX: users learn *why* their adapter cannot do something, which is actionable
  (buy a different adapter) rather than mysterious.
- Driver variance — the largest support-load risk in the product (R-1) — is surfaced as data
  we can collect and publish as a compatibility list.
- Platform differences live in one place, keeping the UI codebase genuinely single-source.

**Negative**
- Probing must be implemented per platform before the features that depend on it, and probes
  can be slow or themselves unreliable.
- The capability model must be designed up front with the full key set, even where entries
  are `unsupported` for a long time.
- Drivers lie. A probe that trusts the driver is a probe that reports success and then fails,
  so some capabilities need empirical verification rather than a query.

**Neutral**
- Creates a natural mechanism for staged rollout: ship a capability as `unsupported` and flip
  it when the backend lands.

## Alternatives considered

**Try the operation and handle failure** — rejected: failures arrive after partial state
change, at the worst moment, with unhelpful OS error codes.

**Static per-platform feature tables** — rejected: cannot express chipset and driver variance,
which is where the actual variation lives.

**Hide unsupported features entirely** — rejected: users conclude the feature is missing and
that the product is deficient, rather than that their hardware is the constraint.
