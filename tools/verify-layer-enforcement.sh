#!/usr/bin/env bash
#
# Tests the test: proves `cargo xtask check-layers` actually FAILS on each
# architecture violation it claims to catch.
#
# A guard rail nobody has watched fail is not a guard rail. This script
# temporarily introduces each forbidden dependency edge, asserts the checker
# rejects it, and restores the manifests. CI runs it (WP-0.2) so the enforcement
# cannot silently rot into a no-op.
#
# Usage: tools/verify-layer-enforcement.sh
set -uo pipefail

# Note: `set -e` is deliberately NOT used — a failing assertion must be counted
# and reported, not abort the run. That makes an unchecked `cd` genuinely
# dangerous here, since the script would carry on in the wrong directory.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || { echo "cannot enter repository root: $ROOT" >&2; exit 1; }

PASS=0
FAIL=0
BACKUPS=()

restore() {
    for f in "${BACKUPS[@]:-}"; do
        [ -n "$f" ] && [ -f "$f.layerbak" ] && mv "$f.layerbak" "$f"
    done
}
trap restore EXIT INT TERM

backup() {
    cp "$1" "$1.layerbak"
    BACKUPS+=("$1")
}

# assert_rejected <description> <manifest> <dependency-line>
assert_rejected() {
    local desc="$1" manifest="$2" dep_line="$3"

    backup "$manifest"
    printf '\n%s\n' "$dep_line" >> "$manifest"

    if cargo xtask check-layers >/dev/null 2>&1; then
        echo "  ✗ FAIL: $desc — checker ACCEPTED a forbidden edge"
        FAIL=$((FAIL + 1))
    else
        echo "  ✓ pass: $desc"
        PASS=$((PASS + 1))
    fi

    mv "$manifest.layerbak" "$manifest"
}

echo "Verifying that check-layers rejects forbidden dependency edges..."
echo

# Rule 1 — the foundation crate must stay a leaf.
assert_rejected \
    "Rule 1: nh-types may not depend on another internal crate" \
    "crates/nh-types/Cargo.toml" \
    'nh-pal = { path = "../nh-pal" }'

# Rule 2 — feature crates must not bind to a concrete PAL backend (AD-7).
assert_rejected \
    "Rule 2: nh-firewall may not depend on the concrete pal-linux backend" \
    "crates/nh-firewall/Cargo.toml" \
    'pal-linux = { path = "../nh-pal/pal-linux" }'

# Rule 4 — the daemon is the composition root and must remain a leaf.
assert_rejected \
    "Rule 4: nh-vpn may not depend on nh-daemon" \
    "crates/nh-vpn/Cargo.toml" \
    'nh-daemon = { path = "../nh-daemon" }'

# Baseline — with the manifests restored, the workspace must be clean again.
echo
if cargo xtask check-layers >/dev/null 2>&1; then
    echo "  ✓ pass: clean workspace still passes after restore"
    PASS=$((PASS + 1))
else
    echo "  ✗ FAIL: workspace did not return to a clean state — check for stray .layerbak files"
    FAIL=$((FAIL + 1))
fi

echo
echo "layer enforcement: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
