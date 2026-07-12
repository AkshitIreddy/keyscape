# Keyscape installer / updater.
#
#   powershell -ExecutionPolicy Bypass -File tools/install.ps1            # build + install
#   powershell -ExecutionPolicy Bypass -File tools/install.ps1 -NoBuild   # reuse existing target/release
#
# Builds in-place, then hands PLACEMENT (copy to %LOCALAPPDATA%\Keyscape\bin,
# Start Menu shortcut, login autostart, core restart) to
# install-finalize.ps1 spawned through WMI — that child always runs
# unvirtualized, so the install lands in the real user profile even when
# this script runs inside an MSIX-packaged host whose AppData/HKCU writes
# are sandboxed.

param([switch]$NoBuild)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

if (-not $NoBuild) {
    Push-Location "$root\ui"
    npm run build
    if ($LASTEXITCODE -ne 0) { Pop-Location; throw "frontend build failed" }
    Pop-Location
    Push-Location $root
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed" }
    Pop-Location
}

$log = Join-Path $PSScriptRoot "install-finalize.log"
Remove-Item $log -Force -ErrorAction SilentlyContinue

$fin = Join-Path $PSScriptRoot "install-finalize.ps1"
$cmd = "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$fin`""
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = $cmd }
if ($r.ReturnValue -ne 0) { throw "failed to spawn finalizer (rc $($r.ReturnValue))" }

for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep -Milliseconds 500
    if (Test-Path $log) { break }
}
$result = Get-Content $log -ErrorAction SilentlyContinue
if (-not $result) { throw "finalizer produced no result" }
if ($result -like "ERR*") { throw "finalizer failed: $result" }
Write-Output "installed: $($result -replace '^OK ', '') (Start Menu shortcut + login autostart, real profile)"
