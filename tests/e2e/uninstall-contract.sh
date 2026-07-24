#!/usr/bin/env bash
#
# The uninstall contract (ADR-0014, §13.5): a machine after install → use →
# uninstall must diff clean against its pre-install snapshot. Run by the nightly
# `uninstall-contract` job on hosted runners (a clean-VM proxy) and by hand on
# real VM snapshots.
#
#   state-diff snapshot before.json
#   <install the package, use it, uninstall it>
#   state-diff snapshot after.json
#   state-diff diff before.json after.json   # exit 0 required
#
# Linux runs the full contract against the deb. The macOS/Windows package-install
# flows need on-target validation and are a WP-M0.13 follow-up; they skip cleanly
# so the job is honest about what it does and does not yet cover.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

NFPM_VERSION=2.47.0

ensure_nfpm() {
    command -v nfpm >/dev/null 2>&1 && return 0
    echo "==> installing nfpm ${NFPM_VERSION}"
    curl -fsSL \
        "https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VERSION}/nfpm_${NFPM_VERSION}_amd64.deb" \
        -o "$WORK/nfpm.deb"
    sudo dpkg -i "$WORK/nfpm.deb"
}

contract_linux() {
    ensure_nfpm
    packaging/linux/build.sh "$WORK/pkg"
    deb="$(find "$WORK/pkg" -name '*.deb' | head -1)"
    [ -n "$deb" ] || { echo "no .deb produced" >&2; exit 2; }

    echo "==> installing $deb"
    sudo dpkg -i "$deb"
    test -x /usr/libexec/networkhub/nh-daemon
    test -x /usr/bin/nh-cli

    echo "==> using it"
    nh-cli --help >/dev/null 2>&1 || true

    echo "==> uninstalling (purge)"
    sudo dpkg --purge networkhub
    test ! -e /usr/libexec/networkhub/nh-daemon
}

echo "==> building state-diff + binaries"
cargo build --release -p state-diff -p nh-daemon -p nh-cli
diff_bin="$ROOT/target/release/state-diff"

echo "==> snapshot (before)"
"$diff_bin" snapshot "$WORK/before.json"

case "$(uname -s)" in
    Linux) contract_linux ;;
    Darwin)
        echo "::warning::macOS DMG/SMAppService install test is a WP-M0.13 follow-up; skipping."
        exit 0
        ;;
    *)
        echo "::warning::Windows MSI install test is a WP-M0.13 follow-up; skipping."
        exit 0
        ;;
esac

echo "==> snapshot (after)"
"$diff_bin" snapshot "$WORK/after.json"

echo "==> diff (must be clean)"
"$diff_bin" diff "$WORK/before.json" "$WORK/after.json"
echo "==> uninstall contract holds: the machine diffs clean."
