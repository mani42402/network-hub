# Continuous Integration

CI is the mechanism that makes "verified" mean something on a product that ships to three
operating systems. A change proven on one machine is a change proven for a third of the
product.

## What runs when

| Workflow | Trigger | Purpose |
|---|---|---|
| [`ci.yml`](../../.github/workflows/ci.yml) | Every pull request and push to `main` | The merge gate |
| [`nightly.yml`](../../.github/workflows/nightly.yml) | 03:00 UTC daily, or manually | Fuzzing, soak, end-to-end, uninstall contract — too slow for the gate |

Both can be run on demand from the Actions tab (`workflow_dispatch`).

## The merge gate (`ci.yml`)

| Job | Platform | What it proves |
|---|---|---|
| **Invariants** | Linux | Formatting, architecture layer rules, that the layer checker and the CI skip conditions still work in both directions, workflow and shell linting, licences, and security advisories |
| **Test** | Linux, Windows, macOS | Build, clippy with `-D warnings`, and the test suite on every platform we ship |
| **Cross-build** | Linux → aarch64 | Raspberry Pi stays compilable on every commit |
| **Protobuf** | Linux | Contracts lint and stay backward compatible (skips until WP-M0.2) |
| **Desktop UI** | Linux | Dart formatting, analysis, and widget tests (skips until WP-M0.12) |
| **CI** | Linux | Aggregate gate — the single required status check |

Two deliberate choices:

**`fail-fast: false` on the matrix.** When something breaks on Windows, we want to know
whether Linux and macOS broke the same way. Cancelling siblings on first failure destroys
exactly the information that makes a cross-platform bug diagnosable.

**One aggregate `CI` check for branch protection.** Adding a platform to the matrix does not
require reconfiguring branch rules, and a skipped or cancelled job still fails the gate — a
common way for protection rules to silently stop protecting anything.

## Jobs that skip on purpose

The protobuf, desktop, fuzz, netns, fault-injection, soak, and uninstall-contract jobs check
for their inputs and skip cleanly when those do not exist yet. This is intentional: the
schedule, structure, and notification path get proven long before they carry anything
valuable. A nightly workflow first written the week you need it is a nightly workflow that
fails the week you need it.

Each prints which work package fills it in.

## Branch protection

Run once, after the remote exists:

```bash
gh auth login
tools/setup-branch-protection.sh          # solo: 0 required reviews
REQUIRED_APPROVALS=1 tools/setup-branch-protection.sh   # once a second maintainer exists
```

The script is the source of truth for merge policy. Change it and re-run rather than editing
settings in the web UI, so the policy stays reviewable in git.

**Solo-developer caveat:** GitHub does not let you approve your own pull request, so the
review requirement defaults to `0` and CI is the gate. `enforce_admins` is also `false` —
with nobody else holding admin, enabling it could lock you out of your own repository. Both
should change the day a second maintainer joins, along with `require_code_owner_reviews`;
until then the CODEOWNERS entries document intent rather than enforce it.

## Reproducing CI locally

CI runs exactly what `tools/setup-*.sh` tells you to run before pushing:

```bash
cargo fmt --all --check
cargo lint                      # clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-layers
tools/verify-layer-enforcement.sh
cargo deny check
```

If these pass locally and CI fails, the difference is platform-specific — which is the
failure CI exists to catch, and worth reading carefully rather than retrying.

## Runners and cost

Hosted runners bill **Windows at 2× and macOS at 10×** the per-minute rate of Linux. A
three-platform matrix on every push consumes minutes quickly.

### Self-hosted runners (recommended here)

You have Windows, Linux, and macOS machines available, which makes self-hosted runners a
strong fit:

- **Free** — no minute consumption
- **Faster** — warm caches and no VM provisioning per job
- **Real hardware** — from M2 onward, hotspot tests need actual Wi-Fi adapters, which hosted
  runners cannot provide at all

Set one up per machine from *Settings → Actions → Runners → New self-hosted runner*, then
change `runs-on` in the matrix from `ubuntu-latest` to a label such as `[self-hosted, linux]`.

Two cautions worth taking seriously:

1. **Self-hosted runners are not ephemeral.** State leaks between jobs, so a test that
   depends on leftover state passes when it should fail. The uninstall-contract and
   fault-injection suites specifically need a clean machine — keep those on hosted runners,
   or run them inside VM snapshots (WP-0.5).
2. **Never attach a self-hosted runner to a public repository.** Any fork can open a pull
   request, and the workflow runs on your machine. Keep the repository private, or use hosted
   runners once it is public.

A reasonable end state: self-hosted runners for the fast per-pull-request matrix, hosted
runners for nightly clean-machine work, and a dedicated hardware-lab runner for the M2 Wi-Fi
suite.

## Supply chain

Workflow actions are executable code in our build pipeline and are treated as dependencies:
Dependabot updates them weekly, and `cargo deny check advisories` runs nightly because the
advisory database moves even when our code does not.

Actions are currently pinned to version **tags**, which are mutable. **WP-M3.5 pins them to
commit digests** — before the pipeline ever holds a signing key. A compromised action in a
workflow that can sign releases is a compromised product.
