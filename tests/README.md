# Test suites

See ARCHITECTURE.md §14.1 for the full pyramid.

| Directory | Scope | Runs |
|---|---|---|
| `netns/` | Linux namespace + `mac80211_hwsim` end-to-end. Full hotspot, DHCP, DNS, and device control without hardware. | Every PR |
| `e2e/` | Packaged-artifact tests on real VMs: install, use, uninstall, state-diff clean. | Nightly + release gate |
| `fault/` | Fault injection against the §11 catalog. Kills the daemon at randomised points and asserts the fail-open invariant (ADR-0005). | Every PR (mock), nightly (netns/VM) |
