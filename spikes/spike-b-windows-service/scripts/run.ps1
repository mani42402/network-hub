# SPIKE-B full lifecycle on Windows. Requires an ELEVATED PowerShell prompt
# (installing a service and creating a pipe with a system ACL are privileged).
#
#   powershell -ExecutionPolicy Bypass -File scripts\run.ps1
#
# Exercises: build -> install (with recovery) -> start -> probe the pipe ->
# confirm the service is Running -> uninstall -> confirm it is gone.

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $here

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $p.IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)) {
        Write-Error "Run this from an elevated PowerShell prompt."
        exit 1
    }
}
Assert-Admin

$exe = Join-Path $here 'service\target\release\spike-b-service.exe'
$svc = 'NetworkHubSpikeB'
$fail = 0

Write-Host "==> building"
Push-Location service; cargo build --release; Pop-Location
if (-not (Test-Path $exe)) { Write-Error "build produced no exe"; exit 1 }

Write-Host "==> install (with restart-on-failure recovery)"
& $exe install

Write-Host "==> start"
Start-Service $svc
Start-Sleep -Seconds 1
$state = (Get-Service $svc).Status
Write-Host "    service state: $state"
if ($state -ne 'Running') { Write-Warning "not Running"; $fail = 1 }

Write-Host "==> probe the pipe"
try {
    & $exe probe
} catch {
    Write-Warning "probe failed: $_"; $fail = 1
}

Write-Host "==> uninstall"
& $exe uninstall
Start-Sleep -Seconds 1
if (Get-Service $svc -ErrorAction SilentlyContinue) {
    Write-Warning "service still present after uninstall"; $fail = 1
} else {
    Write-Host "    service removed"
}

Write-Host ""
if ($fail -eq 0) { Write-Host "SPIKE-B: PASS" } else { Write-Host "SPIKE-B: FAIL" }
exit $fail
