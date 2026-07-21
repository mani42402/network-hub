# ADR-0012: macOS hotspot is assisted-manual, not programmatic

- **Status:** Accepted
- **Date:** 2026-07-22
- **Architecture reference:** AD-12 · ARCHITECTURE.md §3.3
- **Supersedes:** —

## Context

macOS provides **no public API for programmatically enabling Internet Sharing**. `CoreWLAN`
cannot create an access point. The known workarounds manipulate `configd` preference plists
and private frameworks: they are fragile, adjacent to System Integrity Protection, break
across macOS releases, and would likely be treated as unsupported behaviour by Apple.

The product's platform table originally claimed full hotspot support on macOS. That claim
cannot be met.

## Decision

macOS hotspot support is **explicitly degraded and honestly labelled**. We will:

- detect Internet Sharing state via SystemConfiguration/SCDynamicStore,
- deep-link the user into the correct System Settings pane,
- auto-detect when sharing becomes active, and then
- layer everything we legitimately can on top: device discovery and monitoring on the bridge
  interface, DNS guidance, and per-device visibility.

We will **not** manipulate private plists or use private frameworks. The capability matrix
reports `hotspot.wifi_ap` as `unsupported` on macOS with the reason stated, and marketing
material will not claim otherwise.

VPN, firewall, DNS, monitoring, and device management remain fully supported on macOS; those
are what lead the macOS story.

## Consequences

**Positive**
- No fragile code that breaks on every macOS update and generates support load we cannot fix.
- No risk of the product being flagged for private API use, which would also block any future
  App Store or notarisation path.
- Honest expectations, set before purchase rather than after.

**Negative**
- macOS users get a visibly lesser product for one headline feature, and some will be
  disappointed regardless of how clearly we communicate it.
- Competitors willing to use fragile hacks may appear more capable.

**Neutral**
- If Apple ever exposes an API, this ADR is superseded and the capability flips with no UI
  changes — which is precisely what the capability model (ADR-0006) buys us.

## Alternatives considered

**Manipulate the Internet Sharing plists and restart `configd`** — rejected: breaks across
releases, may require disabling SIP, and produces failures we cannot diagnose or fix.

**Ship a NetworkExtension-based virtual AP** — not viable: `NetworkExtension` provides tunnel
and filter providers, not Wi-Fi access point hosting.

**Omit macOS entirely** — rejected: macOS is well served by the rest of the feature set, and
excluding it would forfeit a large share of the developer and remote-worker audience.
