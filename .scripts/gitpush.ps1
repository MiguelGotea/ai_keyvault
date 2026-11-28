# PowerShell — Git Push rápido para AI KeyVault
# Uso: .\.scripts\gitpush.ps1 ["mensaje opcional"]

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Auto-navegar a la raíz del proyecto
Set-Location (Split-Path $PSScriptRoot -Parent)

$msg = $args[0]

# --- Versionado SemVer interactivo (MAJOR.MINOR.PATCH) ---
$tauriConfPath = "apps/desktop/src-tauri/tauri.conf.json"
$packageJsonPath = "apps/desktop/package.json"
$cargoTomlPath = "apps/desktop/src-tauri/Cargo.toml"

$tauriJson = Get-Content $tauriConfPath -Raw -Encoding UTF8 | ConvertFrom-Json
$currentVer = $tauriJson.version

if ($currentVer -notmatch '^(\d+)\.(\d+)\.(\d+)$') {
    Write-Host "[X] Versión actual '$currentVer' en $tauriConfPath no cumple formato X.Y.Z." -ForegroundColor Red
    exit 1
}

$major = [int]$Matches[1]
$minor = [int]$Matches[2]
$patch = [int]$Matches[3]

$nextPatchVer = "$major.$minor.$($patch + 1)"
$nextMinorVer = "$major.$($minor + 1).0"

Write-Host ""
Write-Host "[*] Versión actual: $currentVer" -ForegroundColor Cyan
$answer = Read-Host "¿Es un ajuste menor (PATCH -> $nextPatchVer)? [S/n] (Enter = Sí, N = subir MINOR a $nextMinorVer)"

if ([string]::IsNullOrWhiteSpace($answer) -or $answer -match '^(s|si|sí|y|yes)$') {
    $newVer = $nextPatchVer
    Write-Host "[*] Subiendo PATCH: $currentVer -> $newVer" -ForegroundColor Green
} elseif ($answer -match '^(n|no)$') {
    $newVer = $nextMinorVer
    Write-Host "[*] Subiendo MINOR: $currentVer -> $newVer" -ForegroundColor Green
} else {
    Write-Host "[X] Respuesta no válida '$answer'. Usa Enter/S para PATCH o N para MINOR." -ForegroundColor Red
    exit 1
}

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

# 1. Actualizar tauri.conf.json
$tauriRaw = [System.IO.File]::ReadAllText((Resolve-Path $tauriConfPath), $utf8NoBom)
$tauriUpdated = [regex]::Replace($tauriRaw, '("version"\s*:\s*")\d+\.\d+\.\d+(")', "`${1}$newVer`${2}", 1)
[System.IO.File]::WriteAllText((Resolve-Path $tauriConfPath), $tauriUpdated, $utf8NoBom)

# 2. Actualizar package.json
$pkgRaw = [System.IO.File]::ReadAllText((Resolve-Path $packageJsonPath), $utf8NoBom)
$pkgUpdated = [regex]::Replace($pkgRaw, '("version"\s*:\s*")\d+\.\d+\.\d+(")', "`${1}$newVer`${2}", 1)
[System.IO.File]::WriteAllText((Resolve-Path $packageJsonPath), $pkgUpdated, $utf8NoBom)

# 3. Actualizar Cargo.toml (solo la primera ocurrencia bajo [package])
$cargoRaw = [System.IO.File]::ReadAllText((Resolve-Path $cargoTomlPath), $utf8NoBom)
$cargoUpdated = [regex]::Replace($cargoRaw, '(?m)^(version\s*=\s*")\d+\.\d+\.\d+(")', "`${1}$newVer`${2}", 1)
[System.IO.File]::WriteAllText((Resolve-Path $cargoTomlPath), $cargoUpdated, $utf8NoBom)

Write-Host "[OK] Archivos actualizados a v$newVer." -ForegroundColor Green

if (-not $msg) { $msg = "AI KeyVault v$newVer ($(Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))" }

Write-Host "[*] Verificando que no hay secretos en el commit..." -ForegroundColor Cyan
# Si tienes gitleaks instalado localmente, descomenta:
# gitleaks detect --source . --no-banner
# if ($LASTEXITCODE -ne 0) { Write-Host "[X] gitleaks encontro secretos. Abortando." -ForegroundColor Red; exit 1 }

git add .
git commit -m "$msg" 2>$null

Write-Host "[>] Sincronizando y subiendo..." -ForegroundColor Cyan
git pull origin main --rebase

if ($LASTEXITCODE -ne 0) {
    Write-Host "[!] Conflicto detectado. Resolviendo..." -ForegroundColor Yellow
    git rebase --abort 2>$null
    git pull origin main --no-rebase -X ours
    git add .
    git commit -m "$msg (Conflict Resolved)" 2>$null
}

git push origin main

if ($LASTEXITCODE -eq 0) {
    Write-Host "[OK] Subida completada (v$newVer)! GitHub Actions ejecutara el CI." -ForegroundColor Green
}
else {
    Write-Host "[X] Error en push." -ForegroundColor Red
}
