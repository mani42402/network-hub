#!/usr/bin/env bash
#
# Developer environment setup for macOS.
# Idempotent: safe to re-run after pulling changes.
set -euo pipefail

BOLD=$'\033[1m'; RESET=$'\033[0m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'
info()  { echo "${BOLD}==>${RESET} $*"; }
ok()    { echo "  ${GREEN}✓${RESET} $*"; }
warn()  { echo "  ${YELLOW}!${RESET} $*"; }

info "NetworkHub — macOS developer setup"

# --- Xcode command line tools ---------------------------------------------
if xcode-select -p >/dev/null 2>&1; then
    ok "Xcode command line tools present"
else
    info "Installing Xcode command line tools (a GUI prompt will appear)"
    xcode-select --install || true
    echo "  Re-run this script once the installation finishes."
    exit 1
fi

# --- Homebrew --------------------------------------------------------------
if command -v brew >/dev/null 2>&1; then
    ok "Homebrew present"
else
    warn "Homebrew not found. Install it from https://brew.sh and re-run."
    exit 1
fi

# --- Rust ------------------------------------------------------------------
if command -v rustup >/dev/null 2>&1; then
    ok "rustup present"
else
    info "Installing rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
fi

# rust-toolchain.toml pins the compiler; this materialises it up front so the
# first build is not a surprise download.
info "Installing the pinned Rust toolchain (see rust-toolchain.toml)"
rustup show active-toolchain >/dev/null
ok "toolchain: $(rustc --version)"

# --- Build-time tools ------------------------------------------------------
brew_ensure() {
    if brew list "$1" >/dev/null 2>&1; then
        ok "$1 present"
    else
        info "Installing $1"
        brew install "$1"
    fi
}

info "Installing build and lint tooling"
brew_ensure protobuf     # contract generation — WP-M0.2
brew_ensure buf          # protobuf breaking-change checks — WP-M0.2
# Workflow and shell files are code. These two catch expression and quoting
# bugs that would otherwise appear as a failing CI job at an inconvenient moment.
brew_ensure actionlint
brew_ensure shellcheck

# --- Cargo tooling ---------------------------------------------------------
info "Installing cargo tooling"
if command -v cargo-deny >/dev/null 2>&1; then
    ok "cargo-deny present"
else
    cargo install cargo-deny --locked
fi

# --- Flutter ---------------------------------------------------------------
# Needed from WP-M0.12 onward. Not a hard requirement for engine work, so this
# is a warning rather than a failure.
if command -v flutter >/dev/null 2>&1; then
    ok "flutter present: $(flutter --version 2>/dev/null | head -1)"
else
    warn "flutter not found — required from WP-M0.12 (desktop UI)."
    warn "Install with: brew install --cask flutter"
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
warn "Note: macOS hotspot support is assisted-manual by design (ADR-0012)."
warn "Full hotspot development and testing happens on Linux."
