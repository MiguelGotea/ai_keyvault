# test_poller_manual.ps1
# Corre el poller una sola vez con intervalo de 30s para verificar el flujo completo.
#
# USO:
#   .\crates\poller\scripts\test_poller_manual.ps1
#   .\crates\poller\scripts\test_poller_manual.ps1 -Interval 60 -Verbose

param(
    [string]$ServerUrl  = "http://198.211.97.243:3010",
    [string]$DeviceId   = $env:COMPUTERNAME,
    [int]   $Interval   = 30
)

$env:PATH = "C:\msys64\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:AKYV_SERVER_URL    = $ServerUrl
$env:AKYV_DEVICE_ID     = $DeviceId
$env:AKYV_POLL_INTERVAL = $Interval
$env:RUST_LOG           = "ai_keyvault_poller=info"

Write-Host "🔄 Iniciando poller de prueba..." -ForegroundColor Cyan
Write-Host "   Servidor : $ServerUrl"
Write-Host "   Dispositivo: $DeviceId"
Write-Host "   Intervalo: ${Interval}s"
Write-Host ""
Write-Host "Presiona Ctrl+C para detener despues del primer ciclo" -ForegroundColor Yellow
Write-Host ""

cargo run --package ai-keyvault-poller
