# Build the README demo animation (docs/assets/keyscape.webp).
#
# Pipeline:
#   1. Capture frames of the LIVE UI in headless Chromium (capture.js).
#   2. Build a seamless loop by cross-fading each frame with its half-period
#      counterpart, so motion stays continuously forward and the loop seam is
#      invisible (no ping-pong reversal, no hard cut).
#   3. Encode an animated WebP (full colour -> no GIF dithering grain, small).
#
# Prereqs (all local, nothing published):
#   - Dev server running:     cd ../../ui;  npm run dev        (serves :5173)
#   - Lighting core running:  keyscape-core run                (feeds the preview)
#   - Puppeteer installed:    npm install                      (in this folder)
#   - ImageMagick on PATH (magick) with WebP support.
#
# Usage:  powershell -ExecutionPolicy Bypass -File tools/demo/make-demo.ps1

param(
  [string]$Effect = "magnetic_poles",
  [string]$Params = '{"speed":2.4,"palette":"aurora","flip_rate":0}',
  [int]$Frames = 120,
  [int]$Cadence = 60,
  [int]$Width = 1040,
  [int]$Quality = 92
)
$ErrorActionPreference = "Stop"
$here = $PSScriptRoot
$repo = Split-Path (Split-Path $here)
$frameDir = Join-Path $here "frames"
$loopDir  = Join-Path $here "loop"

$magick = (Get-Command magick -ErrorAction SilentlyContinue).Source
if (-not $magick) { $magick = "$env:LOCALAPPDATA\Microsoft\WindowsApps\magick.exe" }
if (-not (Test-Path $magick)) { throw "ImageMagick (magick) not found on PATH" }

# 1. Capture frames of the live UI.
node (Join-Path $here "capture.js") $frameDir $Effect $Params $Frames $Cadence
if ($LASTEXITCODE -ne 0) { throw "capture failed" }

# 2. Seamless self-crossfade loop.
#    out[i] = w[i]*frame[i] + (1-w[i])*frame[(i + N/2) mod N]
#    with the raised-cosine weight  w[i] = 0.5*(1 - cos(2*pi*i/N)).
#    w is 0 at the seam (i = 0) and 1 at the midpoint, so near the wrap the
#    frame is dominated by the half-shifted stream — which is continuous across
#    i = N-1 -> 0 — and the loop has no visible jump. The whole clip is a blend
#    of the sequence with a copy of itself offset by half a period, so it is
#    mathematically periodic in N frames.
New-Item -ItemType Directory -Force $loopDir | Out-Null
$half = [int]($Frames / 2)
for ($i = 0; $i -lt $Frames; $i++) {
  $w = 0.5 * (1 - [math]::Cos(2 * [math]::PI * $i / $Frames))
  $p = [math]::Round($w * 100)
  $A = "{0}\f{1:D3}.png" -f $frameDir, $i
  $B = "{0}\f{1:D3}.png" -f $frameDir, (($i + $half) % $Frames)
  & $magick $B $A -compose blend -define compose:args=$p -composite ("{0}\l{1:D3}.png" -f $loopDir, $i)
}

# 3. Encode animated WebP straight into the README's assets folder.
$dest = Join-Path $repo "docs\assets\keyscape.webp"
& $magick -delay 6 -loop 0 "$loopDir\l*.png" -resize "$($Width)x" -quality $Quality -define webp:method=6 $dest
$mb = [math]::Round((Get-Item $dest).Length / 1MB, 2)
Write-Output "wrote $dest ($mb MB, $Frames frames)"
