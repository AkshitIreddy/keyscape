# Keyscape installer / updater.
#
#   powershell -ExecutionPolicy Bypass -File tools/install.ps1            # build + install
#   powershell -ExecutionPolicy Bypass -File tools/install.ps1 -NoBuild   # reuse existing target/release
#
# What it does:
#   1. builds the frontend and both release binaries (unless -NoBuild)
#   2. copies them to %LOCALAPPDATA%\Keyscape\bin (the "installed" location,
#      independent of this repo folder)
#   3. creates a Start Menu shortcut ("Keyscape")
#   4. registers the lighting core to start at login (HKCU Run key)
#   5. restarts the running core so the new build takes over seamlessly

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

$bin = "$env:LOCALAPPDATA\Keyscape\bin"
New-Item -ItemType Directory -Force $bin | Out-Null

# a running core holds its exe; stop it, remember to restart
$wasRunning = $null -ne (Get-Process keyscape-core -ErrorAction SilentlyContinue)
Stop-Process -Name keyscape-core, Keyscape -Force -Confirm:$false -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 600

Copy-Item "$root\target\release\keyscape-core.exe" $bin -Force
Copy-Item "$root\target\release\Keyscape.exe" $bin -Force
Copy-Item "$root\ui\src-tauri\icons\icon.ico" "$bin\keyscape.ico" -Force

# Start Menu shortcut (icon from the standalone .ico — immune to exe/icon
# cache weirdness)
$shell = New-Object -ComObject WScript.Shell
$lnk = $shell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Keyscape.lnk")
$lnk.TargetPath = "$bin\Keyscape.exe"
$lnk.WorkingDirectory = $bin
$lnk.IconLocation = "$bin\keyscape.ico,0"
$lnk.Description = "Per-key RGB lighting for the ROG Strix SCAR 16"
$lnk.Save()

# nudge Explorer to rebuild its icon cache so the shortcut shows immediately
Start-Process ie4uinit.exe -ArgumentList "-show" -WindowStyle Hidden -ErrorAction SilentlyContinue

# lighting core at login (remove with: Remove-ItemProperty HKCU:\...\Run -Name Keyscape)
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
    -Name "Keyscape" -Value "`"$bin\keyscape-core.exe`" run"

if ($wasRunning -or $true) {
    Start-Process -FilePath "$bin\keyscape-core.exe" -ArgumentList "run" -WorkingDirectory $bin -WindowStyle Hidden
}

$ver = & "$bin\keyscape-core.exe" --version
Write-Output "installed: $ver -> $bin (Start Menu shortcut + login autostart created)"
