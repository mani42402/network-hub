# SPIKE-A on Windows: builds the Rust named-pipe server, starts it, runs the
# Dart FFI client against it, and tears down.
#
# Usage (from this directory, no elevation needed):
#   powershell -ExecutionPolicy Bypass -File run-windows.ps1
#
# Optional: -TryAfUnix records whether Dart can open an AF_UNIX socket on
# Windows at all (a data point for ADR-0016, not the shipping path).

param([switch]$TryAfUnix)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

Write-Host "==> building Rust named-pipe server"
Push-Location rust-server
cargo build --quiet
if ($LASTEXITCODE -ne 0) { Write-Error "server build failed"; exit 1 }
Pop-Location

Write-Host "==> fetching Dart dependencies"
Push-Location dart-client
dart pub get | Out-Null
Pop-Location

Write-Host "==> starting server"
$serverLog = Join-Path $env:TEMP "spike-a-server.log"
$server = Start-Process -FilePath ".\rust-server\target\debug\spike-a-server.exe" `
    -RedirectStandardOutput $serverLog -RedirectStandardError "$serverLog.err" `
    -PassThru -NoNewWindow

try {
    # Wait for the pipe to exist rather than sleeping a fixed interval.
    $pipe = "networkhub-spike-a"
    $up = $false
    foreach ($i in 1..50) {
        if ([System.IO.Directory]::GetFiles("\\.\pipe\") -match $pipe) { $up = $true; break }
        Start-Sleep -Milliseconds 100
    }
    if (-not $up) {
        Write-Warning "pipe not detected via enumeration; attempting the client anyway"
    }

    Write-Host ""
    Push-Location dart-client
    if ($TryAfUnix) {
        dart run bin/client.dart --try-afunix
    } else {
        dart run bin/client.dart
    }
    $result = $LASTEXITCODE
    Pop-Location

    Write-Host ""
    Write-Host "==> server log"
    if (Test-Path $serverLog) { Get-Content $serverLog | Select-Object -First 20 | ForEach-Object { "    $_" } }
}
finally {
    if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force }
}

exit $result
