# Contributing to NetworkHub

Read [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) first. It is
the contract this codebase is built against — decisions, threat model, and the catalog of
failure modes we design around. Code that contradicts it does not get merged; the document
gets amended by ADR instead, or it wins.

## Setting up

```bash
tools/setup-macos.sh          # or setup-linux.sh, or setup-windows.ps1
cargo build --workspace
cargo test --workspace
```

The toolchain is pinned in `rust-toolchain.toml`. Do not change your system default Rust;
`rustup` will fetch the pinned compiler when you build inside this repository.

## Before you push

```bash
cargo fmt --all
cargo lint                    # alias for clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-layers      # architecture dependency rules
cargo deny check              # licences and advisories
```

CI runs exactly these on Windows, Linux, and macOS. "It works on my machine" is not a
defence when the machine is one of three.

## Engineering rules

These are not style preferences. Each one exists because violating it produced, or would
produce, a specific class of failure in a product that runs as root and controls networks.

**1. `unsafe_code` is forbidden** everywhere except the three `pal-*` backends, which are the
designated FFI boundary. There, it is `deny` — individual modules may opt in with
`#[allow(unsafe_code)]` and a `SAFETY:` comment justifying *every* block. If you find
yourself wanting `unsafe` outside a `pal-*` crate, the design is wrong.

**2. Never shell out through an interpreter.** Child processes are spawned with an argv
array, never `sh -c` or string interpolation. We pass user-controlled SSIDs, interface names,
and file paths to `hostapd`, `openvpn`, `nft`, and `pfctl`; one interpolated string is a root
shell.

**3. Secrets are `Secret<T>`**, a type that cannot be `Display`ed or `Debug`ged. This is
compile-time enforcement, not discipline. Anything that would burn a user if it appeared in a
log or a support bundle is a secret: VPN private keys, Wi-Fi passphrases, API key material,
tokens.

**4. Every network-facing parser ships with a fuzz target in the same pull request.** DHCP,
DNS, captive portal, VPN profile imports, plugin manifests — all of it parses input from
attackers. A listener without a fuzz target fails review.

**5. Mutating OS state requires a rollback path.** Before changing a firewall rule, a
`sysctl`, or a DNS setting, write an intent-journal record with the prior value. The question
to answer for every change is: *if the process dies right here, what does the next start do?*

**6. Ask the failure question.** Every feature adds a line to the failure-mode catalog
(ARCHITECTURE.md §11) or explicitly confirms it adds none: **what new way does this give us
to strand the user offline, or leave state behind after uninstall?**

**7. No feature branch older than about three days.** Contract-first development via protobuf
makes small increments practical. Long branches on a project this size are where correctness
goes to die.

### Sanctioned lint escape hatches

`clippy::unwrap_used` is a warning, and CI denies warnings. In test modules, add at the top of
the file:

```rust
#![cfg_attr(test, allow(clippy::unwrap_used))]
```

Anywhere else, an `#[allow]` needs a comment explaining why the invariant holds. "It cannot
fail" is not an explanation; "the length was checked on the line above" is.

## Architecture layer rules

`cargo xtask check-layers` enforces the dependency rules from ARCHITECTURE.md §5.1:

1. `nh-types` depends on nothing internal — everything may depend on it
2. Feature crates depend on the `nh-pal` **traits**, never on `pal-linux`/`pal-windows`/
   `pal-macos`. Only `nh-daemon` selects a backend. This is what keeps business logic
   testable against `pal-mock` without root or hardware.
3. Only `nh-daemon`, `nh-gateway`, and `nh-cli` produce binaries
4. Nothing depends on `nh-daemon` — it is the composition root

`tools/verify-layer-enforcement.sh` tests the checker itself by introducing each forbidden
edge and asserting it is rejected. Run it if you change the rules.

## Architecture Decision Records

Any change touching these needs an ADR in [`docs/adr/`](docs/adr/) before the code:

- The privilege boundary (what runs as root, and how authorisation crosses the line)
- Wire contracts (protobuf, REST, plugin interfaces)
- Storage schema, migrations, or where secrets live
- Cryptography — primitives, key handling, signing
- Licensing, or how a third-party component is integrated

Copy `docs/adr/0000-template.md`, take the next number, and open it with the pull request.
ADRs are immutable once accepted: to change a decision, write a new ADR that supersedes it.
The history of *why* is the point.

## Definition of Done

A feature is not done when the code exists. It is done when:

1. Unit tests cover the logic, and the coverage targets in ARCHITECTURE.md §14.1 hold for
   the logic-heavy crates
2. A capability matrix entry exists if behaviour varies by platform
3. Every user-triggerable failure has an error contract entry with `remediation` text — a
   human sentence telling them what to do, not a stack trace
4. State mutations emit audit events
5. Tests exist at every layer §14.1 requires — unit, mock-integration, netns, end-to-end
6. The failure question (rule 6) is answered, with new failure modes added to §11 *and* a
   fault-injection test
7. Documentation exists for the user-facing behaviour
8. The uninstall contract still holds — if the feature touches OS state, the uninstall test
   covers removing it

## Commits and pull requests

[Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`,
`refactor:`, `test:`, `chore:`, `perf:`, `build:`, `ci:`. Reference the work package where
one applies — `feat(nh-state): reconciler backoff (WP-M0.4)`.

Pull requests should state what was **proven** versus what is **assumed**, and name the
platforms it was verified on. Precision about what you did not test is more valuable than
confidence about what you did.
