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

# Creates a file ONLY if it does not already exist, tracking it for cleanup.
# A file that is a real committed fixture (e.g. the netns harness, once it
# lands) must never be created-then-deleted by this test — that would delete
# the fixture. So permanent fixtures are asserted present separately below.
touch_temp() {
    if [ ! -e "$1" ]; then
        touch "$1"
        CREATED+=("$1")
    fi
}

# Harnesses that already exist as committed fixtures. As each work package lands
# its harness, move it from the "not yet present" group to here.
echo "Permanent fixtures — their jobs should RUN:"
check "netns harness" true netns_present
check "proto"         true proto_present

echo
echo "Not-yet-present harnesses — their jobs should SKIP today:"
check "fuzz targets"      false fuzz_present
check "flutter app"       false flutter_present
check "fault harness"     false fault_present
check "soak harness"      false soak_present
check "uninstall harness" false uninst_present

echo
echo "With inputs present — every job should RUN:"
mkdir -p apps/desktop tests/fault tests/e2e crates/nh-dns/fuzz/fuzz_targets
touch_temp apps/desktop/pubspec.yaml
touch_temp tests/fault/run.sh
touch_temp tests/e2e/soak.sh
touch_temp tests/e2e/uninstall-contract.sh

check "fuzz targets"      true fuzz_present
check "flutter app"       true flutter_present
check "fault harness"     true fault_present
check "soak harness"      true soak_present
check "uninstall harness" true uninst_present

cleanup
rmdir crates/nh-dns/fuzz/fuzz_targets crates/nh-dns/fuzz 2>/dev/null || true

echo
echo "Back to clean — should SKIP again:"
check "fuzz targets"  false fuzz_present
check "fault harness" false fault_present

echo
echo "skip logic: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
