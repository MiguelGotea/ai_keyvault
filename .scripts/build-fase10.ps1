# ============================================================================
# build-fase10.ps1 — Script completo para el primer build de Tauri (Fase 10)
# Ejecutar desde la raíz del repositorio como administrador (o normal)
# ============================================================================
# USO:
#   cd C:\Users\User\Desktop\Sistema\PitayaWeb\ai_keyvault
#   .\.scripts\build-fase10.ps1
# ============================================================================

$ErrorActionPreference = "Stop"
$Repo = "C:\Users\User\Desktop\Sistema\PitayaWeb\ai_keyvault"
$DesktopDir = "$Repo\apps\desktop"
$SrcTauriDir = "$DesktopDir\src-tauri"

Write-Host "============================================================"
Write-Host "  AI KeyVault - Build Fase 10 (Primer build Tauri)"
Write-Host "============================================================"
Write-Host ""

# --- PASO 1: Verificar MSVC toolchain ---
Write-Host "PASO 1: Verificando toolchain MSVC..."
$msvcTarget = rustup target list --installed --toolchain "stable-x86_64-pc-windows-msvc" 2>$null | Select-String "x86_64-pc-windows-msvc"
if (-not $msvcTarget) {
    Write-Host "  Instalando target MSVC..."
    rustup target add x86_64-pc-windows-msvc --toolchain stable-x86_64-pc-windows-msvc
}
Write-Host "  OK - MSVC target disponible"

# --- PASO 2: Verificar/instalar dependencias npm ---
Write-Host ""
Write-Host "PASO 2: Verificando dependencias pnpm..."
if (-not (Test-Path "$DesktopDir\node_modules\.bin\tauri")) {
    Write-Host "  Ejecutando pnpm install..."
    Set-Location $DesktopDir
    pnpm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Si falla por esbuild, ejecuta: pnpm approve-builds"
        pnpm approve-builds
        pnpm install
    }
} else {
    Write-Host "  OK - node_modules ya instalado"
}

# --- PASO 3: TypeScript check ---
Write-Host ""
Write-Host "PASO 3: Verificando TypeScript..."
Set-Location $DesktopDir
pnpm typecheck
if ($LASTEXITCODE -ne 0) {
    Write-Error "ERROR: TypeScript tiene errores. Corrígelos antes de continuar."
}
Write-Host "  OK - TypeScript compila sin errores"

# --- PASO 4: Compilar poller para MSVC ---
Write-Host ""
Write-Host "PASO 4: Compilando ai-keyvault-poller para MSVC..."
$PollerBin = "$Repo\target\x86_64-pc-windows-msvc\release\ai-keyvault-poller.exe"
if (Test-Path $PollerBin) {
    Write-Host "  OK - poller ya compilado"
} else {
    Write-Host "  Compilando (puede tardar varios minutos en la primera vez)..."
    Set-Location $Repo
    cargo +stable-x86_64-pc-windows-msvc build --release --package ai-keyvault-poller --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) {
        Write-Error "ERROR: Falló la compilación del poller MSVC."
    }
}

# --- PASO 5: Copiar sidecar ---
Write-Host ""
Write-Host "PASO 5: Copiando sidecar del poller..."
$SidecarDest = "$SrcTauriDir\binaries\ai-keyvault-poller-x86_64-pc-windows-msvc.exe"
New-Item -ItemType Directory -Force -Path "$SrcTauriDir\binaries" | Out-Null
Copy-Item -Path $PollerBin -Destination $SidecarDest -Force
$Size = [math]::Round((Get-Item $SidecarDest).Length / 1MB, 2)
Write-Host "  OK - Sidecar copiado ($Size MB)"

# --- PASO 6: Verificar íconos ---
Write-Host ""
Write-Host "PASO 6: Verificando íconos..."
$IconsOk = (Test-Path "$SrcTauriDir\icons\icon.ico") -and
           (Test-Path "$SrcTauriDir\icons\32x32.png") -and
           (Test-Path "$SrcTauriDir\icons\128x128.png")
if (-not $IconsOk) {
    Write-Error "ERROR: Faltan íconos en src-tauri/icons/. Ejecuta .scripts\gen-icons.ps1 primero."
}
Write-Host "  OK - Íconos presentes"

# --- PASO 7: cargo tauri build ---
Write-Host ""
Write-Host "PASO 7: Ejecutando cargo tauri build..."
Write-Host "  (Este paso tarda 5-15 minutos en la primera compilación)"
Set-Location $DesktopDir
pnpm tauri build
if ($LASTEXITCODE -ne 0) {
    Write-Error "ERROR: `pnpm tauri build` falló."
}

# --- RESULTADO ---
Write-Host ""
Write-Host "============================================================"
Write-Host "  BUILD EXITOSO"
Write-Host "============================================================"
$MsiPath = Get-ChildItem "$SrcTauriDir\target\release\bundle\msi\*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($MsiPath) {
    Write-Host "  Instalador MSI: $($MsiPath.FullName)"
    Write-Host "  Tamaño: $([math]::Round($MsiPath.Length / 1MB, 1)) MB"
} else {
    Write-Host "  Busca el instalador en: $SrcTauriDir\target\release\bundle\"
}
Write-Host ""
Write-Host "Siguiente paso: instalar el .msi y verificar que el poller"
Write-Host "quede en la carpeta de instalación con el nombre correcto."
