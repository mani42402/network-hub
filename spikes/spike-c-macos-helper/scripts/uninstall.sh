#!/usr/bin/env bash
#
# SPIKE-C uninstall — the clean-removal contract (ADR-0014, ARCHITECTURE.md
# §13.5) in miniature. After this, the machine must show no trace of the spike.
#
#   sudo scripts/uninstall.sh
set -uo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

LABEL="com.networkhub.spike-c"
PLIST_DST="/Library/LaunchDaemons/$LABEL.plist"
BIN_DST="/usr/local/libexec/networkhub-spike-c-helper"
SOCKET="/var/run/networkhub-spike-c.sock"
LOG="/var/log/networkhub-spike-c.log"

echo "==> unloading from launchd"
launchctl bootout "system/$LABEL" 2>/dev/null || echo "    (was not loaded)"

echo "==> removing files"
for f in "$PLIST_DST" "$BIN_DST" "$SOCKET" "$LOG"; do
    if [ -e "$f" ]; then rm -f "$f"; echo "    removed $f"; fi
done

echo "==> confirming clean"
LEFT=0
for f in "$PLIST_DST" "$BIN_DST" "$SOCKET"; do
    [ -e "$f" ] && { echo "    STILL PRESENT: $f"; LEFT=1; }
done
launchctl print "system/$LABEL" >/dev/null 2>&1 && { echo "    STILL LOADED in launchd"; LEFT=1; }

[ "$LEFT" -eq 0 ] && echo "==> clean." || { echo "==> NOT clean — investigate."; exit 1; }
