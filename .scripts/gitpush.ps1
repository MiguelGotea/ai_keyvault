# PowerShell — Git Push rápido para AI KeyVault
# Uso: .\.scripts\gitpush.ps1 ["mensaje opcional"]

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# Auto-navegar a la raíz del proyecto
Set-Location (Split-Path $PSScriptRoot -Parent)

$msg = $args[0]
if (-not $msg) { $msg = "AI KeyVault Update $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" }

Write-Host "🔒 Verificando que no hay secretos en el commit..." -ForegroundColor Cyan
# Si tienes gitleaks instalado localmente, descomenta:
# gitleaks detect --source . --no-banner
# if ($LASTEXITCODE -ne 0) { Write-Host "❌ gitleaks encontró secretos. Abortando." -ForegroundColor Red; exit 1 }

git add .
git commit -m "$msg" 2>$null

Write-Host "🚀 Sincronizando y subiendo..." -ForegroundColor Cyan
git pull origin main --rebase

if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️ Conflicto detectado. Resolviendo..." -ForegroundColor Yellow
    git rebase --abort 2>$null
    git pull origin main --no-rebase -X ours
    git add .
    git commit -m "$msg (Conflict Resolved)" 2>$null
}

git push origin main

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ ¡Subida completada! GitHub Actions ejecutará el CI." -ForegroundColor Green
} else {
    Write-Host "❌ Error en push." -ForegroundColor Red
}
