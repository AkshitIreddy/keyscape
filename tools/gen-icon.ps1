# Draws the Keyscape icon (glowing keycap grid on a dark rounded tile) with
# System.Drawing and emits PNGs at all needed sizes into ui/src-tauri/icons.
# Run from repo root:  powershell -File tools/gen-icon.ps1
# Then pack the .ico:  node tools/make-ico.mjs

Add-Type -AssemblyName System.Drawing

$outDir = "ui/src-tauri/icons"
New-Item -ItemType Directory -Force $outDir | Out-Null

$S = 256
$bmp = New-Object System.Drawing.Bitmap($S, $S)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

function RoundRect([System.Drawing.Graphics]$g, $brush, $x, $y, $w, $h, $r) {
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $g.FillPath($brush, $path)
    $path.Dispose()
}

# dark tile with a subtle diagonal sheen
$tileBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    (New-Object System.Drawing.Point(0, 0)), (New-Object System.Drawing.Point($S, $S)),
    [System.Drawing.Color]::FromArgb(255, 13, 17, 27), [System.Drawing.Color]::FromArgb(255, 7, 9, 15))
RoundRect $g $tileBrush 8 8 ($S - 16) ($S - 16) 52

# glow halo behind the bright key (radial)
$halo = New-Object System.Drawing.Drawing2D.GraphicsPath
$halo.AddEllipse(52, 34, 152, 152)
$haloBrush = New-Object System.Drawing.Drawing2D.PathGradientBrush($halo)
$haloBrush.CenterColor = [System.Drawing.Color]::FromArgb(150, 34, 211, 165)
$haloBrush.SurroundColors = @([System.Drawing.Color]::FromArgb(0, 34, 211, 165))
$g.FillPath($haloBrush, $halo)

# 3x2 keycap grid; per-key gradient colors teal -> violet
$keyW = 56; $keyH = 56; $gap = 14
$gridW = 3 * $keyW + 2 * $gap
$x0 = ($S - $gridW) / 2
$y0 = 62
$cols = @(
    @(34, 211, 165), @(64, 170, 220), @(124, 92, 255),
    @(45, 190, 200), @(94, 130, 240), @(150, 80, 255)
)
for ($i = 0; $i -lt 6; $i++) {
    $cx = $x0 + ($i % 3) * ($keyW + $gap)
    $cy = $y0 + [Math]::Floor($i / 3) * ($keyH + $gap)
    $c = $cols[$i]
    $alpha = if ($i -eq 1) { 255 } else { 200 }
    $kb = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.Point([int]$cx, [int]$cy)),
        (New-Object System.Drawing.Point([int]$cx, [int]($cy + $keyH))),
        [System.Drawing.Color]::FromArgb($alpha, $c[0], $c[1], $c[2]),
        [System.Drawing.Color]::FromArgb([Math]::Max(120, $alpha - 90), $c[0], $c[1], $c[2]))
    RoundRect $g $kb $cx $cy $keyW $keyH 12
    # keycap top bevel
    $bev = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(46, 255, 255, 255))
    RoundRect $g $bev $cx $cy $keyW ($keyH * 0.45) 12
    $bev.Dispose()
    $kb.Dispose()
}

# the "hero" key gets a hot core
$hot = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(110, 255, 255, 255))
RoundRect $g $hot ($x0 + $keyW + $gap + 10) ($y0 + 10) ($keyW - 20) ($keyH - 20) 8
$hot.Dispose()

$g.Dispose()
$bmp.Save("$outDir/icon-256.png", [System.Drawing.Imaging.ImageFormat]::Png)

# downscale set
foreach ($size in 16, 32, 48, 64, 128) {
    $small = New-Object System.Drawing.Bitmap($size, $size)
    $sg = [System.Drawing.Graphics]::FromImage($small)
    $sg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $sg.DrawImage($bmp, 0, 0, $size, $size)
    $sg.Dispose()
    $small.Save("$outDir/icon-$size.png", [System.Drawing.Imaging.ImageFormat]::Png)
    # sizes <= 64 also get raw BGRA dumps so make-ico.mjs can pack classic
    # BMP entries (parts of the shell refuse PNG entries below 256px)
    if ($size -le 64) {
        $rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
        $data = $small.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
            [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $bytes = New-Object byte[] ($size * $size * 4)
        [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
        $small.UnlockBits($data)
        [System.IO.File]::WriteAllBytes("$outDir/icon-$size.bgra", $bytes)
    }
    $small.Dispose()
}
$bmp.Dispose()

# tauri-conventional names
Copy-Item "$outDir/icon-32.png" "$outDir/32x32.png" -Force
Copy-Item "$outDir/icon-128.png" "$outDir/128x128.png" -Force
Copy-Item "$outDir/icon-256.png" "$outDir/128x128@2x.png" -Force
Copy-Item "$outDir/icon-256.png" "$outDir/icon.png" -Force
Write-Output "PNGs written to $outDir"
