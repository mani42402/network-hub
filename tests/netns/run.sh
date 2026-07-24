#!/usr/bin/env bash
#
# netns harness entry point. CI (nightly.yml) invokes this; it also runs by hand
# on a Linux box with `sudo tests/netns/run.sh`.
#
# WP-0.5 delivers the harness foundation and a smoke test that proves it works:
# two namespaces, a veth link, connectivity, and (where available) hwsim virtual
# radios. Real hotspot/DHCP/DNS scenarios are added from WP-M2.11 as separate
# scripts under scenarios/, each sourcing lib.sh.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=tests/netns/lib.sh
. "$HERE/lib.sh"

FAIL=0
# shellcheck disable=SC2329,SC2317  # defined + reached only via the trap below
cleanup() {
    netns_teardown_all
    netns_hwsim_unload
}
trap cleanup EXIT INT TERM

echo "==> preflight"
netns_require_linux || exit 1
netns_require_root  || exit 1
netns_require_tools || exit 1
echo "    ok"

echo "==> smoke test: two namespaces linked by veth can talk"
netns_teardown_all   # clear any orphans from a crashed prior run
netns_create router
netns_create client
netns_link router 10.99.0.1/24 client 10.99.0.2/24

if netns_exec client ping -c1 -W1 10.99.0.1 >/dev/null 2>&1; then
    echo "    ok: client reached router across the veth"
else
    echo "    FAIL: no connectivity between namespaces"
    FAIL=1
fi

echo "==> nftables works inside a namespace"
if netns_exec router nft add table inet nhtest_probe 2>/dev/null \
   && netns_exec router nft list table inet nhtest_probe >/dev/null 2>&1; then
    echo "    ok: created and read a namespaced nftables table"
    netns_exec router nft delete table inet nhtest_probe 2>/dev/null || true
else
    echo "    FAIL: nftables not usable in the namespace"
    FAIL=1
fi

echo "==> virtual Wi-Fi (mac80211_hwsim)"
if netns_hwsim_load 2; then
    # A real AP scenario (hostapd on one radio, wpa_supplicant on another) lands
    # in WP-M2.11. Here we only confirm the radios appeared.
    if command -v iw >/dev/null 2>&1 && [ "$(iw dev 2>/dev/null | grep -c Interface)" -ge 2 ]; then
        echo "    ok: virtual radios present"
    else
        echo "    note: radios loaded but 'iw' did not enumerate them (non-fatal)"
    fi
else
    echo "    skipped: hwsim unavailable on this host (non-fatal for the smoke test)"
fi

echo
if [ "$FAIL" -eq 0 ]; then
    echo "netns harness: PASS"
else
    echo "netns harness: FAIL"
fi
exit $FAIL
