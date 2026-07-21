#!/usr/bin/env bash
# Verifies the CI skip-detection conditionals in BOTH directions.
# A conditional stuck on "skip" would silently disable a job forever — exactly
# the kind of rot that makes a pipeline look green while testing nothing.
set -uo pipefail
# Note: `set -e` is deliberately NOT used — a failing assertion must be counted
# and reported, not abort the run. That makes an unchecked `cd` genuinely
# dangerous here, since the script creates and removes files.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || { echo "cannot enter repository root: $ROOT" >&2; exit 1; }

PASS=0; FAIL=0
CREATED=()
cleanup() { for f in "${CREATED[@]:-}"; do [ -n "$f" ] && rm -f "$f"; done; }
trap cleanup EXIT

check() { # <label> <expected true|false> <command>
    local label="$1" expected="$2"; shift 2
    if "$@" >/dev/null 2>&1; then actual=true; else actual=false; fi
    if [ "$actual" = "$expected" ]; then
        echo "  ✓ $label → $actual"
        PASS=$((PASS+1))
    else
        echo "  ✗ $label → $actual (expected $expected)"
        FAIL=$((FAIL+1))
    fi
}

proto_present()   { find proto -name '*.proto' -print -quit 2>/dev/null | grep -q .; }
fuzz_present()    { find . -path ./target -prune -o -name 'fuzz_targets' -type d -print -quit 2>/dev/null | grep -q .; }
flutter_present() { [ -f apps/desktop/pubspec.yaml ]; }
netns_present()   { [ -f tests/netns/run.sh ]; }
fault_present()   { [ -f tests/fault/run.sh ]; }
soak_present()    { [ -f tests/e2e/soak.sh ]; }
uninst_present()  { [ -f tests/e2e/uninstall-contract.sh ]; }

echo "State today — every optional job should SKIP:"
check "proto"             false proto_present
check "fuzz targets"      false fuzz_present
check "flutter app"       false flutter_present
check "netns harness"     false netns_present
check "fault harness"     false fault_present
check "soak harness"      false soak_present
check "uninstall harness" false uninst_present

echo
echo "With inputs present — every job should RUN:"
mkdir -p proto/networkhub/v1 apps/desktop tests/netns tests/fault tests/e2e crates/nh-dns/fuzz/fuzz_targets
for f in proto/networkhub/v1/_probe.proto apps/desktop/pubspec.yaml tests/netns/run.sh \
         tests/fault/run.sh tests/e2e/soak.sh tests/e2e/uninstall-contract.sh; do
    touch "$f"; CREATED+=("$f")
done

check "proto"             true proto_present
check "fuzz targets"      true fuzz_present
check "flutter app"       true flutter_present
check "netns harness"     true netns_present
check "fault harness"     true fault_present
check "soak harness"      true soak_present
check "uninstall harness" true uninst_present

cleanup
rmdir crates/nh-dns/fuzz/fuzz_targets crates/nh-dns/fuzz 2>/dev/null || true
rmdir proto/networkhub/v1 proto/networkhub 2>/dev/null || true

echo
echo "Back to clean — should SKIP again:"
check "proto"        false proto_present
check "fuzz targets" false fuzz_present

echo
echo "skip logic: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
