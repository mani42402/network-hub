#!/usr/bin/env bash
#
# Removes the NetworkHub daemon and everything install.sh created (ADR-0014).
# Requires sudo.
#
#   sudo packaging/macos/uninstall.sh [--purge]
set -uo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1

LIBEXEC=/usr/local/libexec/networkhub
DAEMONS=/Library/LaunchDaemons

echo "==> unloading LaunchDaemons"
launchctl bootout system/com.networkhub.daemon 2>/dev/null || true
launchctl bootout system/com.networkhub.cleanup 2>/dev/null || true

# Final cleanup pass to reclaim any NetworkHub-owned OS state (from M1).
if [ -x "$LIBEXEC/nh-daemon" ]; then
    echo "==> final cleanup pass"
    "$LIBEXEC/nh-daemon" cleanup --dir /var/run/networkhub 2>/dev/null || true
fi

echo "==> removing plists and binary"
rm -f "$DAEMONS/com.networkhub.daemon.plist" "$DAEMONS/com.networkhub.cleanup.plist"
rm -f "$LIBEXEC/nh-daemon"
rmdir "$LIBEXEC" 2>/dev/null || true

echo "==> removing runtime state and logs"
rm -rf /var/run/networkhub
rm -f /var/log/networkhub.log /var/log/networkhub-cleanup.log

if [ "$PURGE" -eq 1 ]; then
    echo "==> purging config (--purge)"
    rm -rf "/Library/Application Support/NetworkHub"
else
    echo "==> keeping config; pass --purge to remove it"
fi

echo "==> confirming clean"
LEFT=$(find "$DAEMONS" /usr/local/libexec -name '*networkhub*' 2>/dev/null)
if [ -z "$LEFT" ]; then
    echo "==> clean."
else
    echo "==> leftover: $LEFT"
    exit 1
fi
