# Instalador firmado y edición portable (Fase 28)

## Edición portable (`.zip`)
En cada ejecución de `.github/workflows/ci.yml` en `main`, GitHub Actions genera y publica en GitHub Releases el archivo:
- `AI-KeyVault-portable-v<versión>-win-x64.zip`

Contenido del paquete portable:
- `AI KeyVault.exe` (aplicación desktop compilada para Windows x64 MSVC)
- `ai-keyvault-poller.exe` (sidecar del poller)
- `LEEME-PORTABLE.txt`

Para usarlo: descomprime la carpeta en cualquier ruta y ejecuta `AI KeyVault.exe` manteniendo `ai-keyvault-poller.exe` junto a él.

---

## Firma de código Authenticode en Windows (opcional)
El workflow `.github/workflows/ci.yml` firma automáticamente los instaladores `.exe` (NSIS) y `.msi` con `signtool.exe` (SHA-256 + timestamp DigiCert) si configuras estos **Repository Secrets** en GitHub (`Settings → Secrets and variables → Actions`):

1. `WINDOWS_CERT_PFX_BASE64`: archivo `.pfx` codificado en Base64.
   En PowerShell:
   ```powershell
   [Convert]::ToBase64String([IO.File]::ReadAllBytes("ruta\certificado.pfx")) | Set-Clipboard
   ```
2. `WINDOWS_CERT_PASSWORD`: contraseña del certificado `.pfx`.
3. `TAURI_SIGNING_PRIVATE_KEY` *(opcional)*: clave privada generada con `pnpm tauri signer generate` si deseas firmar paquetes de auto-actualización de Tauri.

Si los secrets no están presentes, el CI continúa normalmente sin fallar y publica los binarios sin firma.
