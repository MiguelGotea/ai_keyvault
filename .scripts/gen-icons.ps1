# gen-icons.ps1 — Genera los íconos requeridos por Tauri desde las imágenes de branding.
# Ejecutar desde la raíz del repositorio.
# 
# USO: .\.scripts\gen-icons.ps1
#
# Genera en apps/desktop/src-tauri/icons/:
#   32x32.png, 128x128.png, 128x128@2x.png, icon.png, icon.ico, tray-icon.png

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$Repo = "C:\Users\User\Desktop\Sistema\PitayaWeb\ai_keyvault"
$src = "$Repo\assets\branding\AI_KeyVault_Color.png"
$mono = "$Repo\assets\branding\AI_KeyVault_MonoColor.png"
$iconsDir = "$Repo\apps\desktop\src-tauri\icons"

New-Item -ItemType Directory -Force -Path $iconsDir | Out-Null

function Save-ResizedPng {
    param($sourcePath, $destPath, $width, $height)
    $img = [System.Drawing.Image]::FromFile($sourcePath)
    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($img, 0, 0, $width, $height)
    $g.Dispose(); $img.Dispose()
    $bmp.Save($destPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "  Creado: $(Split-Path $destPath -Leaf) ($width x $height)"
}

function Create-Ico {
    param($sourcePath, $destPath)
    $sizes = @(16, 32, 48, 64, 128, 256)
    $bitmaps = @()
    $srcImg = [System.Drawing.Image]::FromFile($sourcePath)
    foreach ($size in $sizes) {
        $bmp = New-Object System.Drawing.Bitmap($size, $size)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.DrawImage($srcImg, 0, 0, $size, $size)
        $g.Dispose(); $bitmaps += $bmp
    }
    $srcImg.Dispose()
    $pngStreams = @()
    foreach ($bmp in $bitmaps) {
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngStreams += $ms; $bmp.Dispose()
    }
    $stream = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter($stream)
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
    $offset = 6 + (16 * $sizes.Count)
    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $sz = [byte]($sizes[$i] -band 0xFF)
        $writer.Write($sz); $writer.Write($sz)
        $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$pngStreams[$i].Length)
        $writer.Write([uint32]$offset)
        $offset += $pngStreams[$i].Length
    }
    foreach ($ms in $pngStreams) { $writer.Write($ms.ToArray()); $ms.Dispose() }
    [System.IO.File]::WriteAllBytes($destPath, $stream.ToArray())
    $writer.Dispose(); $stream.Dispose()
    Write-Host "  Creado: $(Split-Path $destPath -Leaf) (ICO multi-tamaño)"
}

Write-Host "Generando íconos desde branding..."
Save-ResizedPng $src "$iconsDir\32x32.png" 32 32
Save-ResizedPng $src "$iconsDir\128x128.png" 128 128
Save-ResizedPng $src "$iconsDir\128x128@2x.png" 256 256
Save-ResizedPng $src "$iconsDir\icon.png" 512 512
Save-ResizedPng $mono "$iconsDir\tray-icon.png" 32 32
Create-Ico $src "$iconsDir\icon.ico"
Write-Host ""
Write-Host "Iconos generados en: $iconsDir"
Get-ChildItem $iconsDir | Select-Object Name, @{N='Tamaño';E={[math]::Round($_.Length/1KB,1).ToString() + " KB"}}
