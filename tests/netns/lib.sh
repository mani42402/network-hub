#!/usr/bin/env bash
#
# Shared helpers for the network-namespace test harness.
#
# This harness is the crown jewel of the test strategy (ARCHITECTURE.md §14.1):
# with network namespaces plus mac80211_hwsim virtual radios, the full hotspot →
# DHCP → DNS → device-control path runs in CI with no hardware. Scenarios that
# use it land from WP-M2.11; this file is the reusable foundation (WP-0.5).
#
# Everything here needs root and Linux. It is never pointed at a developer's live
# configuration — all state lives in namespaces that are destroyed on teardown.

set -uo pipefail

# --- guards ----------------------------------------------------------------

netns_require_root() {
    [ "$(id -u)" -eq 0 ] || { echo "netns harness needs root"; return 1; }
}

netns_require_linux() {
    [ "$(uname -s)" = "Linux" ] || { echo "netns harness is Linux-only"; return 1; }
}

# Tools the harness relies on. Reported all at once (not fail-on-first), the same
# preflight philosophy the hotspot module uses (E-6 style).
netns_require_tools() {
    local missing=()
    for t in ip nft; do
        command -v "$t" >/dev/null 2>&1 || missing+=("$t")
    done
    if [ "${#missing[@]}" -ne 0 ]; then
        echo "missing required tools: ${missing[*]}"
        return 1
    fi
}

# --- namespace lifecycle ---------------------------------------------------

# All namespaces created by a run are prefixed so teardown is unambiguous and a
# crashed run leaves an obvious, greppable mess rather than silent orphans.
NETNS_PREFIX="nhtest"

netns_create() {
    local name="$1"
    ip netns add "${NETNS_PREFIX}-${name}"
    ip -n "${NETNS_PREFIX}-${name}" link set lo up
}

netns_exec() {
    local name="$1"; shift
    ip netns exec "${NETNS_PREFIX}-${name}" "$@"
}

# Connect two namespaces with a veth pair and address each end.
# usage: netns_link <ns_a> <addr_a/cidr> <ns_b> <addr_b/cidr>
netns_link() {
    local a="$1" addr_a="$2" b="$3" addr_b="$4"
    local va="v-${a}-${b}" vb="v-${b}-${a}"
    ip link add "$va" type veth peer name "$vb"
    ip link set "$va" netns "${NETNS_PREFIX}-${a}"
    ip link set "$vb" netns "${NETNS_PREFIX}-${b}"
    netns_exec "$a" ip addr add "$addr_a" dev "$va"
    netns_exec "$b" ip addr add "$addr_b" dev "$vb"
    netns_exec "$a" ip link set "$va" up
    netns_exec "$b" ip link set "$vb" up
}

# Remove every namespace this harness created. Safe to call on a partial run.
netns_teardown_all() {
    local ns
    for ns in $(ip netns list 2>/dev/null | awk '{print $1}' | grep "^${NETNS_PREFIX}-" || true); do
        ip netns del "$ns" 2>/dev/null || true
    done
}

# --- virtual Wi-Fi (mac80211_hwsim) ----------------------------------------
#
# hwsim gives us virtual radios that hostapd and wpa_supplicant drive exactly
# like real ones, so the hotspot AP path is testable without a Wi-Fi card. This
# is what keeps the highest-variance milestone (M2) developable in CI.

netns_hwsim_available() {
    modinfo mac80211_hwsim >/dev/null 2>&1
}

# Loads N virtual radios. Returns nonzero (and explains) when hwsim is absent,
# so a scenario can skip the Wi-Fi parts cleanly on a runner without the module.
netns_hwsim_load() {
    local radios="${1:-2}"
    if ! netns_hwsim_available; then
        echo "mac80211_hwsim not available; Wi-Fi scenarios will be skipped"
        return 1
    fi
    modprobe mac80211_hwsim radios="$radios"
    echo "loaded mac80211_hwsim with $radios virtual radios"
}

netns_hwsim_unload() {
    rmmod mac80211_hwsim 2>/dev/null || true
}
