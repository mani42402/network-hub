#!/usr/bin/env bash
#
# Builds NetworkHub deb + rpm packages from the release binaries via nfpm.
#
#   packaging/linux/build.sh [OUTPUT_DIR]
#
# Build the binaries first (on Linux):
#   cargo build --release -p nh-daemon -p nh-cli
# then run this. Set NFPM_BINDIR to override where the binaries are found.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT="${1:-$ROOT/packaging/out}"
mkdir -p "$OUT"

# Version from the workspace manifest (the single source of truth).
VERSION="$(grep -A6 '^\[workspace.package\]' Cargo.toml | grep -m1 '^version' | cut -d'"' -f2)"
[ -n "$VERSION" ] || { echo "could not read workspace version" >&2; exit 1; }

case "$(uname -m)" in
    x86_64 | amd64) ARCH=amd64 ;;
    aarch64 | arm64) ARCH=arm64 ;;
    *) ARCH="$(uname -m)" ;;
esac

NFPM_BINDIR="${NFPM_BINDIR:-$ROOT/target/release}"
for bin in nh-daemon nh-cli; do
    [ -f "$NFPM_BINDIR/$bin" ] || {
        echo "missing $NFPM_BINDIR/$bin — run: cargo build --release -p nh-daemon -p nh-cli" >&2
        exit 1
    }
done

export NFPM_ARCH="$ARCH" NFPM_VERSION="$VERSION" NFPM_BINDIR NFPM_ROOT="$ROOT"

# nfpm does not expand env vars inside content `src` globs, so pre-expand the
# config ourselves (only the four NFPM_* vars) and make every path absolute.
CONFIG="$(mktemp)"
trap 'rm -f "$CONFIG"' EXIT
# The single-quoted list is envsubst's allowlist of variables to expand — the
# shell must NOT expand it, so single quotes are correct here.
# shellcheck disable=SC2016
envsubst '${NFPM_ARCH} ${NFPM_VERSION} ${NFPM_BINDIR} ${NFPM_ROOT}' \
    <packaging/linux/nfpm.yaml >"$CONFIG"

for fmt in deb rpm; do
    echo "==> building $fmt"
    nfpm package --config "$CONFIG" --packager "$fmt" --target "$OUT/"
done

echo "==> packages in $OUT:"
ls -1 "$OUT"
