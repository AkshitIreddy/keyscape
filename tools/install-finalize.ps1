# Placement half of the installer. ALWAYS run via a WMI-spawned process
# (install.ps1 does this) so it executes UNVIRTUALIZED: when the build runs
# inside an MSIX-packaged host (e.g. an AI coding session), plain
# %LOCALAPPDATA% and HKCU writes get silently redirected into the package's
# LocalCache sandbox — invisible to Explorer, the Start Menu and logon.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$log = Join-Path $PSScriptRoot "install-finalize.log"

try {
    $bin = "$env:LOCALAPPDATA\Keyscape\bin"
    New-Item -ItemType Directory -Force $bin | Out-Null

    try { Stop-Process -Name keyscape-core, Keyscape -Force -ErrorAction Stop -Confirm:$false } catch {}
    Start-Sleep -Milliseconds 700

    Copy-Item "$root\target\release\keyscape-core.exe" $bin -Force
    Copy-Item "$root\target\release\Keyscape.exe" $bin -Force
    Copy-Item "$root\ui\src-tauri\icons\icon.ico" "$bin\keyscape.ico" -Force

    # Start Menu shortcut
    $shell = New-Object -ComObject WScript.Shell
    $lnk = $shell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Keyscape.lnk")
    $lnk.TargetPath = "$bin\Keyscape.exe"
    $lnk.WorkingDirectory = $bin
    $lnk.IconLocation = "$bin\keyscape.ico,0"
    $lnk.Description = "Per-key RGB keyboard lighting"
    $lnk.Save()

    # lighting core at login
    Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
        -Name "Keyscape" -Value "`"$bin\keyscape-core.exe`" run"

    # seed the custom-effects folder with the JS examples on first install
    $fxDir = "$env:APPDATA\Keyscape\effects"
    if (-not (Test-Path $fxDir)) {
        New-Item -ItemType Directory -Force $fxDir | Out-Null
        Copy-Item "$root\examples\js-effects\*.js" $fxDir -Force -ErrorAction SilentlyContinue
    }

    # clean up any sandbox-ghost install from a virtualized run
    $ghost = "$env:LOCALAPPDATA\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\Keyscape"
    if (Test-Path $ghost) { Remove-Item -Recurse -Force $ghost -ErrorAction SilentlyContinue }

    Start-Process -FilePath "$bin\keyscape-core.exe" -ArgumentList "run" -WorkingDirectory $bin -WindowStyle Hidden
    Start-Process ie4uinit.exe -ArgumentList "-show" -WindowStyle Hidden -ErrorAction SilentlyContinue

    $ver = & "$bin\keyscape-core.exe" --version
    "OK $ver -> $bin" | Set-Content $log
} catch {
    "ERR $_" | Set-Content $log
}
