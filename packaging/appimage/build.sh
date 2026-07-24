#!/usr/bin/env bash
#
# Builds a NetworkHub AppImage — the desktop UI + CLI as a single portable Linux
# binary. It carries NO privileged daemon: the AppImage talks to a daemon
# installed separately (deb/rpm), or to daemon-sim for development. Unsigned/dev.
#
#   packaging/appimage/build.sh [OUTPUT_DIR]
#
# Prerequisites (on Linux):
#   (cd apps/desktop && flutter build linux --release)
#   cargo build --release -p nh-cli
#   appimagetool on PATH  (https://github.com/AppImage/AppImageKit)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT="${1:-$ROOT/packaging/out}"
mkdir -p "$OUT"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

VERSION="$(grep -A6 '^\[workspace.package\]' Cargo.toml | grep -m1 '^version' | cut -d'"' -f2)"

BUNDLE="$ROOT/apps/desktop/build/linux/x64/release/bundle"
[ -d "$BUNDLE" ] || { echo "missing $BUNDLE — run: (cd apps/desktop && flutter build linux --release)" >&2; exit 1; }
[ -f "$ROOT/target/release/nh-cli" ] || { echo "missing nh-cli — run: cargo build --release -p nh-cli" >&2; exit 1; }

APPDIR="$WORK/NetworkHub.AppDir"
mkdir -p "$APPDIR/usr/bin"
cp -r "$BUNDLE"/* "$APPDIR/usr/bin/"
cp "$ROOT/target/release/nh-cli" "$APPDIR/usr/bin/nh-cli"

cat >"$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/networkhub_desktop" "$@"
RUN
chmod +x "$APPDIR/AppRun"

cat >"$APPDIR/networkhub.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=NetworkHub
Exec=networkhub_desktop
Icon=networkhub
Categories=Network;System;
DESKTOP

# appimagetool requires an icon; use the Flutter-generated one if present,
# otherwise a 1x1 placeholder so the build still succeeds (dev).
icon="$APPDIR/networkhub.png"
if [ -f "$BUNDLE/data/flutter_assets/assets/icon.png" ]; then
    cp "$BUNDLE/data/flutter_assets/assets/icon.png" "$icon"
else
    printf '\211PNG\r\n\032\n' >"$icon"
fi

appimagetool "$APPDIR" "$OUT/NetworkHub-$VERSION-x86_64.AppImage"
echo "==> built $OUT/NetworkHub-$VERSION-x86_64.AppImage"
