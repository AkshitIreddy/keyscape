# Build the distributable NSIS installer:
#   powershell -ExecutionPolicy Bypass -File tools/bundle-installer.ps1
#
# Output: target\release\bundle\nsis\Keyscape_<version>_x64-setup.exe
# The installer carries Keyscape.exe, keyscape-core.exe and the example JS
# effects; on first launch the app registers the core for login autostart.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

Push-Location "$root\ui"
npm run build
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "frontend build failed" }
Pop-Location

Push-Location $root
cargo build --release -p keyscape-core
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "core build failed" }
Pop-Location

# stage bundle resources
$bindir = "$root\ui\src-tauri\binaries"
New-Item -ItemType Directory -Force "$bindir\effects" | Out-Null
Copy-Item "$root\target\release\keyscape-core.exe" $bindir -Force
Copy-Item "$root\examples\js-effects\*.js" "$bindir\effects" -Force

Push-Location "$root\ui"
npx tauri build --bundles nsis
$rc = $LASTEXITCODE
Pop-Location
if ($rc -ne 0) { throw "tauri bundle failed" }

Get-ChildItem "$root\target\release\bundle\nsis\*.exe" | ForEach-Object {
    Write-Output "installer: $($_.FullName) ($([math]::Round($_.Length / 1MB, 1)) MB)"
}
