#!/usr/bin/env bash
#
# Installs the NetworkHub daemon as a launchd LaunchDaemon on macOS. Requires
# sudo. The manual install path until the Developer ID cert (PRE-2) enables the
# SMAppService registration (ADR-0012).
#
#   sudo packaging/macos/install.sh [path/to/nh-daemon]
set -euo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_SRC="${1:-$HERE/../../target/release/nh-daemon}"
LIBEXEC=/usr/local/libexec/networkhub
DAEMONS=/Library/LaunchDaemons

[ -f "$BIN_SRC" ] || { echo "daemon binary not found at $BIN_SRC (build with: cargo build --release -p nh-daemon)"; exit 1; }

echo "==> installing the daemon binary to $LIBEXEC"
install -d -m 0755 "$LIBEXEC"
install -m 0755 "$BIN_SRC" "$LIBEXEC/nh-daemon"

# Ad-hoc code signature; a real Developer ID + notarization replaces this (PRE-2).
echo "==> ad-hoc signing"
codesign --force --sign - "$LIBEXEC/nh-daemon" 2>/dev/null || true

echo "==> installing LaunchDaemon plists"
install -m 0644 "$HERE/com.networkhub.cleanup.plist" "$DAEMONS/"
install -m 0644 "$HERE/com.networkhub.daemon.plist" "$DAEMONS/"
chown root:wheel "$DAEMONS/com.networkhub.cleanup.plist" "$DAEMONS/com.networkhub.daemon.plist"

echo "==> loading with launchd"
launchctl bootstrap system "$DAEMONS/com.networkhub.cleanup.plist" 2>/dev/null || true
launchctl bootstrap system "$DAEMONS/com.networkhub.daemon.plist"
launchctl enable system/com.networkhub.daemon

echo "==> done. Check with: sudo launchctl print system/com.networkhub.daemon"
