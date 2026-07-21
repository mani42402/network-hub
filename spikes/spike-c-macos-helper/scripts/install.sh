#!/usr/bin/env bash
#
# SPIKE-C manual install — the LaunchDaemon path that SMAppService automates for
# a signed app. Requires sudo (installing a root daemon is a privileged act).
#
#   sudo scripts/install.sh
#
# This is the fallback/verification path used before the Developer ID cert
# (PRE-2) makes the SMAppService registration available.
set -euo pipefail

[ "$(id -u)" -eq 0 ] || { echo "run with sudo: sudo $0"; exit 1; }

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LABEL="com.networkhub.spike-c"
BIN_SRC="$HERE/helper/target/release/spike-c-helper"
BIN_DST="/usr/local/libexec/networkhub-spike-c-helper"
PLIST_SRC="$HERE/$LABEL.plist"
PLIST_DST="/Library/LaunchDaemons/$LABEL.plist"

[ -f "$BIN_SRC" ] || { echo "build first: (cd helper && cargo build --release)"; exit 1; }

echo "==> installing helper binary to $BIN_DST"
install -d -m 0755 /usr/local/libexec
install -m 0755 "$BIN_SRC" "$BIN_DST"

# Ad-hoc code signature. With a real Developer ID (PRE-2) this becomes a proper
# signature + notarization; ad-hoc is enough to prove the mechanism and to
# satisfy Gatekeeper for a locally built binary.
echo "==> ad-hoc signing"
codesign --force --sign - "$BIN_DST"

echo "==> installing LaunchDaemon plist to $PLIST_DST"
install -m 0644 "$PLIST_SRC" "$PLIST_DST"
chown root:wheel "$PLIST_DST"

echo "==> loading with launchd"
# bootstrap is the modern replacement for `launchctl load`.
launchctl bootstrap system "$PLIST_DST"
launchctl enable "system/$LABEL"

echo "==> done. Verify with: scripts/verify.sh"
