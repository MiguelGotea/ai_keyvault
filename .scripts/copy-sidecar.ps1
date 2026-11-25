# Script para copiar el sidecar del poller al directorio correcto de Tauri.
# Ejecutar desde la raíz del repositorio después de compilar el poller para MSVC.
#
# Uso:
#   .\.scripts\copy-sidecar.ps1
#
# Requerido antes de: cargo tauri build (desde apps/desktop/)

$ErrorActionPreference = "Stop"
$Repo = $PSScriptRoot | Split-Path -Parent

$Origen = "$Repo\target\x86_64-pc-windows-msvc\release\ai-keyvault-poller.exe"
$Destino = "$Repo\apps\desktop\src-tauri\binaries\ai-keyvault-poller-x86_64-pc-windows-msvc.exe"

Write-Host "[INFO] Copiando sidecar del poller..."
Write-Host "       Origen:  $Origen"
Write-Host "       Destino: $Destino"

if (-not (Test-Path $Origen)) {
    Write-Error "[ERROR] No se encontro el binario del poller en: $Origen"
    Write-Host "        Compila primero con:"
    Write-Host "        cargo build --release --package ai-keyvault-poller --target x86_64-pc-windows-msvc"
    exit 1
}

Copy-Item -Path $Origen -Destination $Destino -Force
$Size = (Get-Item $Destino).Length / 1MB
$SizeRounded = [math]::Round($Size, 2)
Write-Host "[OK] Sidecar copiado correctamente ($SizeRounded MB)"
Write-Host ""
Write-Host "Ahora puedes ejecutar: cd apps\desktop; pnpm tauri build"
