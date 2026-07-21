# Security Policy

NetworkHub runs a privileged system service that controls firewall rules, routing, DNS, and
VPN tunnels. A vulnerability here is not a bug — it is root-equivalent access to someone's
network. We treat reports accordingly.

## Reporting a vulnerability

> **TODO(WP-0.1):** replace with a monitored address before the repository becomes public.
> A security policy pointing at an unread inbox is worse than none.

**Do not open a public issue for a security problem.**

- Email: `security@REPLACE-ME` (PGP key: *to be published*)
- Or use GitHub's [private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)

Please include: affected version and platform, reproduction steps, and what an attacker
gains. If you have a proof of concept, say so — we will arrange a secure channel rather than
ask you to attach it to an email.

## What to expect

| Stage | Target |
|---|---|
| Acknowledgement of your report | 48 hours |
| Initial assessment and severity rating | 7 days |
| Fix or documented mitigation | 90 days, sooner for actively exploited issues |
| Public advisory and credit | Coordinated with you |

We will keep you informed as the fix progresses, and we will credit you in the advisory
unless you prefer otherwise.

## Scope

**In scope** — anything that crosses a trust boundary described in ARCHITECTURE.md §4.3:

- Privilege escalation through the daemon IPC surface (any local user gaining network control)
- Authentication or authorisation bypass in the daemon, gateway, or captive portal
- Memory-safety or logic flaws in parsers exposed to untrusted input: DHCP, DNS, captive
  portal, imported VPN profiles (`.ovpn`, WireGuard configs, QR payloads), plugin bundles
- Plugin sandbox escape (WASM host boundary)
- Update-channel weaknesses: signature bypass, rollback attacks, manifest tampering
- Secret disclosure: VPN keys, Wi-Fi passphrases, API keys, tokens — in logs, diagnostics
  bundles, exports, or over the wire
- **Leaving the user offline or with orphaned OS state** — see "Availability is a security
  property" below
- TLS/crypto misuse in the gateway or cloud sync

**Out of scope**

- Attacks requiring physical access to an unlocked machine, or an existing root/Administrator shell
- Denial of service through legitimate resource exhaustion by an authorised administrator
- Social engineering, or vulnerabilities in third-party VPN servers you connect to
- Missing hardening that has no demonstrated impact (send it as a normal issue — we want it,
  it just is not an advisory)
- Findings against a deliberately weakened configuration (dev-mode unsigned plugins, an
  intentionally disabled kill switch)

## Availability is a security property here

Most projects would not call "the app crashed" a security issue. For NetworkHub it can be
the worst outcome available: a crashed daemon that leaves a kill switch active can strand a
machine with no network access, and orphaned firewall rules can outlive an uninstall.

So we treat these as security-class reports:

- Any state where NetworkHub leaves a machine unable to reach the network without a
  documented, working escape hatch
- Firewall rules, routes, `sysctl` changes, or DNS configuration that survive uninstallation
- A DHCP or DNS server of ours answering on a network segment it does not manage

The design intent — and the invariant to test against — is in ARCHITECTURE.md §11: *from any
single crash, kill, or power loss, the next start returns the system to either the user's
desired state or the pre-NetworkHub state, never a third thing.*

## Supported versions

Pre-1.0, only the latest release receives security fixes. This table becomes meaningful at
1.0.

## Our commitments

- Security fixes ship through the update channel with the same signing and rollback
  guarantees as any release (ARCHITECTURE.md §13.4)
- Advisories are published with a CVE where applicable, and an SBOM accompanies every release
- We will not pursue legal action against good-faith research that respects this policy,
  avoids privacy violations and service degradation, and gives us reasonable time to fix
