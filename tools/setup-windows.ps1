# Developer environment setup for Windows.
# Idempotent: safe to re-run after pulling changes.
#
# Usage (from an elevated PowerShell prompt, repo root):
#   powershell -ExecutionPolicy Bypass -File tools\setup-windows.ps1

#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

function Write-Info { param($m) Write-Host "==> $m" -ForegroundColor White }
function Write-Ok   { param($m) Write-Host "  [ok] $m" -ForegroundColor Green }
function Write-Warn { param($m) Write-Host "  [!]  $m" -ForegroundColor Yellow }

Write-Info "NetworkHub - Windows developer setup"

# --- winget ----------------------------------------------------------------
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Warn "winget not found. Install 'App Installer' from the Microsoft Store and re-run."
    exit 1
}
Write-Ok "winget present"

function Install-IfMissing {
    param(
        [string]$Command,
        [string]$WingetId,
        [string]$Label
    )
    if (Get-Command $Command -ErrorAction SilentlyContinue) {
        Write-Ok "$Label present"
    } else {
        Write-Info "Installing $Label"
        winget install --id $WingetId --silent --accept-package-agreements --accept-source-agreements
    }
}

# --- Core toolchain --------------------------------------------------------
Install-IfMissing -Command 'git'    -WingetId 'Git.Git'                     -Label 'git'
Install-IfMissing -Command 'rustup' -WingetId 'Rustlang.Rustup'             -Label 'rustup'
Install-IfMissing -Command 'protoc' -WingetId 'Google.Protobuf'             -Label 'protoc'

# The MSVC build tools are required: the Rust toolchain on Windows targets the
# MSVC ABI, and the PAL backend links against Windows SDK libraries (WFP,
# IP Helper, WinRT).
if (Test-Path "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe") {
    Write-Ok "Visual Studio Build Tools present"
} else {
    Write-Info "Installing Visual Studio Build Tools (C++ workload + Windows SDK)"
    winget install --id Microsoft.VisualStudio.2022.BuildTools --silent `
        --accept-package-agreements --accept-source-agreements `
        --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows11SDK.22621"
}

# --- Refresh PATH for this session ----------------------------------------
$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' +
            [System.Environment]::GetEnvironmentVariable('Path','User')

# --- Rust toolchain --------------------------------------------------------
# rust-toolchain.toml pins the compiler; materialise it now so the first build
# is not a surprise download.
Write-Info "Installing the pinned Rust toolchain (see rust-toolchain.toml)"
rustup show active-toolchain | Out-Null
Write-Ok "toolchain: $(rustc --version)"

# --- buf -------------------------------------------------------------------
if (Get-Command buf -ErrorAction SilentlyContinue) {
    Write-Ok "buf present"
} else {
    Write-Warn "buf not found - needed from WP-M0.2 for protobuf breaking-change checks."
    Write-Warn "Install from https://github.com/bufbuild/buf/releases"
}

# --- actionlint ------------------------------------------------------------
# Workflow files are code; actionlint catches expression and shell bugs that
# would otherwise appear as a failing CI job at an inconvenient moment.
if (Get-Command actionlint -ErrorAction SilentlyContinue) {
    Write-Ok "actionlint present"
} elseif (Get-Command go -ErrorAction SilentlyContinue) {
    Write-Info "Installing actionlint"
    go install github.com/rhysd/actionlint/cmd/actionlint@latest
} else {
    Write-Warn "actionlint not found - install Go, or download a release from"
    Write-Warn "https://github.com/rhysd/actionlint/releases"
}

# --- Cargo tooling ---------------------------------------------------------
Write-Info "Installing cargo tooling"
if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    Write-Ok "cargo-deny present"
} else {
    cargo install cargo-deny --locked
}

# --- WiX v4 ----------------------------------------------------------------
# Needed from WP-M0.13 to build the MSI (ADR-0014).
if (Get-Command wix -ErrorAction SilentlyContinue) {
    Write-Ok "WiX present"
} else {
    Write-Warn "WiX v4 not found - required from WP-M0.13 (MSI packaging)."
    Write-Warn "Install with: dotnet tool install --global wix"
}

# --- Flutter ---------------------------------------------------------------
if (Get-Command flutter -ErrorAction SilentlyContinue) {
    Write-Ok "flutter present"
} else {
    Write-Warn "flutter not found - required from WP-M0.12 (desktop UI)."
    Write-Warn "Install with: winget install --id Google.Flutter"
}

# --- Verify ----------------------------------------------------------------
Write-Info "Verifying the workspace builds"
cargo build --workspace --quiet
Write-Ok "workspace builds"

cargo xtask check-layers

Write-Host ""
Write-Info "Setup complete. Before pushing, run:"
Write-Host "    cargo fmt --all; cargo lint; cargo test --workspace"
Write-Host "    cargo xtask check-layers; cargo deny check"
Write-Host ""
Write-Warn "Service and WFP work (WP-M0.7, WP-M1.4) requires an elevated shell."
Write-Warn "Test service install/uninstall in a VM snapshot, not on your working machine."
