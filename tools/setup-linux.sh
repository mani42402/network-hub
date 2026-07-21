#!/usr/bin/env bash
#
# Developer environment setup for Linux.
# Idempotent: safe to re-run after pulling changes.
#
# Linux is the reference platform: it is the only one where the full feature set
# (hotspot, DHCP, DNS, per-device shaping, captive portal) is implementable, so
# this script installs the networking tooling the integration tests need.
set -euo pipefail

BOLD=$'\033[1m'; RESET=$'\033[0m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
info()  { echo "${BOLD}==>${RESET} $*"; }
ok()    { echo "  ${GREEN}✓${RESET} $*"; }
warn()  { echo "  ${YELLOW}!${RESET} $*"; }

info "NetworkHub — Linux developer setup"

# --- Package manager detection --------------------------------------------
if   command -v apt-get >/dev/null 2>&1; then PM=apt
elif command -v dnf     >/dev/null 2>&1; then PM=dnf
elif command -v pacman  >/dev/null 2>&1; then PM=pacman
else
    warn "Unsupported package manager. Install the equivalents of the apt list below by hand."
    PM=none
fi
[ "$PM" != none ] && ok "package manager: $PM"

SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"

install_packages() {
    case "$PM" in
        apt)
            $SUDO apt-get update -qq
            $SUDO apt-get install -y \
                build-essential pkg-config curl git \
                protobuf-compiler \
                libnl-3-dev libnl-genl-3-dev libnl-route-3-dev \
                nftables iproute2 iw \
                hostapd \
                iperf3 tcpdump
            ;;
        dnf)
            $SUDO dnf install -y \
                @development-tools pkgconf-pkg-config curl git \
                protobuf-compiler \
                libnl3-devel \
                nftables iproute iw \
                hostapd \
                iperf3 tcpdump
            ;;
        pacman)
            $SUDO pacman -Sy --needed --noconfirm \
                base-devel pkgconf curl git \
                protobuf \
                libnl \
                nftables iproute2 iw \
                hostapd \
                iperf3 tcpdump
            ;;
        none) warn "Skipping system packages." ;;
    esac
}

info "Installing system packages (networking stack + build tools)"
install_packages
ok "system packages installed"

# --- Rust ------------------------------------------------------------------
if command -v rustup >/dev/null 2>&1; then
    ok "rustup present"
else
    info "Installing rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

info "Installing the pinned Rust toolchain (see rust-toolchain.toml)"
rustup show active-toolchain >/dev/null
ok "toolchain: $(rustc --version)"

# --- buf -------------------------------------------------------------------
if command -v buf >/dev/null 2>&1; then
    ok "buf present"
else
    warn "buf not found — needed from WP-M0.2 for protobuf breaking-change checks."
    warn "Install from https://github.com/bufbuild/buf/releases"
fi

# --- Cargo tooling ---------------------------------------------------------
info "Installing cargo tooling"
if command -v cargo-deny >/dev/null 2>&1; then
    ok "cargo-deny present"
else
    cargo install cargo-deny --locked
fi

# --- Flutter ---------------------------------------------------------------
if command -v flutter >/dev/null 2>&1; then
    ok "flutter present: $(flutter --version 2>/dev/null | head -1)"
else
    warn "flutter not found — required from WP-M0.12 (desktop UI)."
    warn "Install from https://docs.flutter.dev/get-started/install/linux"
fi

# --- Kernel modules used by the test harness ------------------------------
# mac80211_hwsim provides virtual Wi-Fi radios, which is what lets the hotspot
# end-to-end suite run in CI without hardware (WP-0.5, WP-M2.11).
info "Checking test-harness kernel support"
if modinfo mac80211_hwsim >/dev/null 2>&1; then
    ok "mac80211_hwsim available (virtual Wi-Fi for hotspot E2E tests)"
else
    warn "mac80211_hwsim not available — hotspot E2E tests will need real hardware."
    warn "On Debian/Ubuntu this lives in linux-modules-extra-\$(uname -r)."
fi

# --- Verify ----------------------------------------------------------------
info "Verifying the workspace builds"
cargo build --workspace --quiet
ok "workspace builds"

cargo xtask check-layers

echo
info "Setup complete. Before pushing, run:"
echo "    cargo fmt --all && cargo lint && cargo test --workspace"
echo "    cargo xtask check-layers && cargo deny check"
echo
warn "Integration tests that touch nftables, hostapd, or network namespaces need root."
warn "They run inside namespaces or VMs — never point them at your working machine's live config."
