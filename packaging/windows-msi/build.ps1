# Builds the NetworkHub MSI with WiX Toolset v4.
#
#   packaging\windows-msi\build.ps1 [-OutDir <dir>]
#
# Prerequisites (on Windows):
#   cargo build --release -p nh-daemon -p nh-cli
#   dotnet tool install --global wix   # WiX v4 CLI
#
# Unsigned/dev; Authenticode signing is M3 (PRE-1).
param(
    [string]$OutDir = "packaging\out",
    [string]$BinDir = "target\release"
)
$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
Set-Location $root

# Version from the workspace manifest.
$version = (Select-String -Path "Cargo.toml" -Pattern '^\s*version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches.Groups[1].Value
if (-not $version) { throw "could not read workspace version from Cargo.toml" }

foreach ($bin in @("nh-daemon.exe", "nh-cli.exe")) {
    if (-not (Test-Path (Join-Path $BinDir $bin))) {
        throw "missing $BinDir\$bin — run: cargo build --release -p nh-daemon -p nh-cli"
    }
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$msi = Join-Path $OutDir "NetworkHub-$version.msi"

wix build packaging\windows-msi\networkhub.wxs `
    -d BinDir=$BinDir `
    -d Version=$version `
    -o $msi

Write-Host "==> built $msi"
