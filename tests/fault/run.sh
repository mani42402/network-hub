#!/usr/bin/env bash
#
# Fault-injection suite (WP-M0.14): the fail-open invariant (AD-5, §11) as an
# automated test — crash the daemon inside a mutating flow and assert it recovers
# to the desired state or the pre-NetworkHub state, never a third thing.
#
# At the mock level the suite is ordinary cargo tests, also run by the CI Test
# job on every push. This nightly entry point runs them and is where the heavier
# process-kill / netns / VM scenarios plug in as real mutating reconcilers land
# (firewall, hotspot, VPN — M1+): each ships its scenario here.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "==> mock-level fault suite (AD-5 invariant + failure catalog)"
cargo test -p nh-state --test fault

echo "==> fault suite passed"
