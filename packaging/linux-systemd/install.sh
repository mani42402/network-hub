#!/usr/bin/env bash
#
# Installs the NetworkHub daemon as a systemd service on Linux. Requires root.
#
#   sudo packaging/linux-systemd/install.sh [path/to/nh-daemon]
#
# This is the manual/dev install path; the deb/rpm packages (WP-M0.13) do the
# same steps via package scriptlets. Everything it creates is removed by
# uninstall.sh — the uninstall contract (ADR-0014, §13.5).
set -euo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC="${1:-$HERE/../../target/release/nh-daemon}"
LIBEXEC=/usr/libexec/networkhub
UNIT_DIR=/etc/systemd/system

[ -f "$BIN_SRC" ] || { echo "daemon binary not found at $BIN_SRC (build with: cargo build --release -p nh-daemon)"; exit 1; }

echo "==> creating the networkhub group"
# The socket is group-accessible; members of this group may administer the
# daemon without being root. (ADR-0002; group->role mapping is refined later.)
getent group networkhub >/dev/null || groupadd --system networkhub

echo "==> installing the daemon binary to $LIBEXEC"
install -d -m 0755 "$LIBEXEC"
install -m 0755 "$BIN_SRC" "$LIBEXEC/nh-daemon"

echo "==> installing systemd units"
install -m 0644 "$HERE/networkhub.service" "$UNIT_DIR/"
install -m 0644 "$HERE/networkhub-cleanup.service" "$UNIT_DIR/"

echo "==> enabling and starting"
systemctl daemon-reload
systemctl enable --now networkhub-cleanup.service
systemctl enable --now networkhub.service

echo "==> done. Check status with: systemctl status networkhub.service"
