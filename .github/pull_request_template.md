<!--
Precision about what you did NOT test is more valuable than confidence about
what you did. A reviewer who knows the gaps can decide; one who is reassured
cannot.
-->

## What and why

<!-- What changes, and what problem it solves. Reference the work package where
     one applies, e.g. WP-M0.4. -->

## Proven vs. assumed

**Proven** — what you actually observed working, and where:

<!-- e.g. "Reconciler converges after injected failure at every step — mock
     integration suite, 200 randomised runs, Linux + macOS." -->

**Assumed** — what should work but was not exercised:

<!-- e.g. "Windows path compiles but was not run on hardware; no Wi-Fi adapter
     available." Being explicit here is the point of this section. -->

**Platforms verified on:** <!-- Linux / Windows / macOS / CI only -->

## The failure question

> What new way does this give us to strand the user offline, or leave OS state
> behind after uninstall?

<!-- Answer it, or state "none — this change touches no OS state." If it adds a
     failure mode, it belongs in ARCHITECTURE.md §11 with a fault test. -->

- [ ] No new failure modes, **or** new modes are documented in §11 with a fault test
- [ ] Uninstall contract still holds (or is untouched by this change)

## Definition of Done

<!-- Delete rows that genuinely do not apply; do not silently skip them. -->

- [ ] Unit tests cover the logic
- [ ] Capability matrix entry exists, if behaviour varies by platform
- [ ] User-triggerable failures have error-contract entries with `remediation` text
- [ ] State mutations emit audit events
- [ ] Tests at every layer ARCHITECTURE.md §14.1 requires
- [ ] Network-facing parsers ship with their fuzz target **in this pull request**
- [ ] Documentation updated
- [ ] ADR added, if this touches the privilege boundary, wire contracts, storage
      schema, cryptography, or licensing

## Checks

- [ ] `cargo fmt --all && cargo lint && cargo test --workspace`
- [ ] `cargo xtask check-layers && cargo deny check`

## Notes for the reviewer

<!-- Anything you are unsure about, alternatives you rejected, or areas where a
     second opinion would genuinely help. -->
