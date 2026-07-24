#!/usr/bin/env bash
#
# Builds an unsigned NetworkHub .dmg (dev). Real Developer ID signing and
# notarization are M3 (gated on PRE-2); until then this is for local/dev use.
#
#   packaging/macos/build-dmg.sh [OUTPUT_DIR]
#
# Build the pieces first:
#   cargo build --release -p nh-daemon -p nh-cli
#   (cd apps/desktop && flutter build macos --release)   # optional GUI
# then run this. Set MAC_BINDIR to override where the binaries are found.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT="${1:-$ROOT/packaging/out}"
mkdir -p "$OUT"

VERSION="$(grep -A6 '^\[workspace.package\]' Cargo.toml | grep -m1 '^version' | cut -d'"' -f2)"
[ -n "$VERSION" ] || { echo "could not read workspace version" >&2; exit 1; }

BINDIR="${MAC_BINDIR:-$ROOT/target/release}"
for bin in nh-daemon nh-cli; do
    [ -f "$BINDIR/$bin" ] || {
        echo "missing $BINDIR/$bin — run: cargo build --release -p nh-daemon -p nh-cli" >&2
        exit 1
    }
done

STAGE="$(mktemp -d)/NetworkHub"
cleanup() { rm -rf "$(dirname "$STAGE")"; }
trap cleanup EXIT
mkdir -p "$STAGE"

# The daemon and CLI, plus the launchd plists and install/uninstall scripts.
install -m 0755 "$BINDIR/nh-daemon" "$STAGE/nh-daemon"
install -m 0755 "$BINDIR/nh-cli" "$STAGE/nh-cli"
install -m 0644 packaging/macos/com.networkhub.daemon.plist "$STAGE/"
install -m 0644 packaging/macos/com.networkhub.cleanup.plist "$STAGE/"
install -m 0755 packaging/macos/install.sh "$STAGE/install.sh"
install -m 0755 packaging/macos/uninstall.sh "$STAGE/uninstall.sh"

# The desktop GUI, if it has been built (needs Xcode + CocoaPods).
APP="apps/desktop/build/macos/Build/Products/Release/networkhub_desktop.app"
if [ -d "$APP" ]; then
    cp -R "$APP" "$STAGE/NetworkHub.app"
    ln -s /Applications "$STAGE/Applications"
else
    echo "note: $APP not built — DMG will ship the daemon + CLI only." >&2
fi

cat >"$STAGE/README.txt" <<'TXT'
NetworkHub (developer build, unsigned)

1. Drag NetworkHub.app to Applications (if present).
2. Install the privileged daemon:  sudo ./install.sh
   Uninstall:                       sudo ./uninstall.sh
The daemon serves the app and the CLI (nh-cli) over a local socket.
TXT

DMG="$OUT/NetworkHub-$VERSION.dmg"
rm -f "$DMG"
hdiutil create -volname "NetworkHub" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
echo "==> built $DMG"
