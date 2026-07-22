#!/usr/bin/env bash
#
# Removes the NetworkHub daemon and everything install.sh created, leaving the
# machine as it was (ADR-0014, §13.5). Requires root.
#
#   sudo packaging/linux-systemd/uninstall.sh [--purge]
#
# --purge also removes config.db and state. Without it, state is kept.
set -uo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1

LIBEXEC=/usr/libexec/networkhub
UNIT_DIR=/etc/systemd/system

echo "==> stopping and disabling services"
systemctl disable --now networkhub.service 2>/dev/null || true
systemctl disable --now networkhub-cleanup.service 2>/dev/null || true

# Run the cleanup one last time so any NetworkHub-owned OS state (from M1:
# firewall tables, sysctls) is reclaimed before the binary is removed.
if [ -x "$LIBEXEC/nh-daemon" ]; then
    echo "==> final cleanup pass"
    "$LIBEXEC/nh-daemon" cleanup --dir /run/networkhub 2>/dev/null || true
fi

echo "==> removing units and binary"
rm -f "$UNIT_DIR/networkhub.service" "$UNIT_DIR/networkhub-cleanup.service"
rm -f "$LIBEXEC/nh-daemon"
rmdir "$LIBEXEC" 2>/dev/null || true
systemctl daemon-reload

echo "==> removing runtime state"
rm -rf /run/networkhub

if [ "$PURGE" -eq 1 ]; then
    echo "==> purging config and state (--purge)"
    rm -rf /var/lib/networkhub
    getent group networkhub >/dev/null && groupdel networkhub 2>/dev/null || true
else
    echo "==> keeping /var/lib/networkhub (config); pass --purge to remove it"
fi

echo "==> confirming clean"
LEFT=$(find /etc/systemd/system /usr/libexec -name '*networkhub*' 2>/dev/null)
if [ -z "$LEFT" ]; then
    echo "==> clean."
else
    echo "==> leftover: $LEFT"
    exit 1
fi
