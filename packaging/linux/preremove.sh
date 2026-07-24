#!/bin/sh
# Runs before the package's files are removed. On deb, $1 is "remove"/"upgrade";
# on rpm, $1 is a count (0 = final removal). Stop the services either way; do a
# final cleanup pass only on a real removal, not an upgrade.
set -e

is_upgrade=false
[ "$1" = "upgrade" ] && is_upgrade=true      # deb
[ "$1" = "1" ] && is_upgrade=true            # rpm upgrade leaves 1 package

systemctl disable --now networkhub.service 2>/dev/null || true
systemctl disable --now networkhub-cleanup.service 2>/dev/null || true

if [ "$is_upgrade" = false ] && [ -x /usr/libexec/networkhub/nh-daemon ]; then
    # Reclaim any NetworkHub-owned OS state before the binary goes away (§13.5).
    /usr/libexec/networkhub/nh-daemon cleanup --dir /run/networkhub 2>/dev/null || true
fi

exit 0
