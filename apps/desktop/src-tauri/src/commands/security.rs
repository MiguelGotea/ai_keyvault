//! Comandos Tauri para autobloqueo y sincronización de bóveda (Fase 14/15).
//!
//! AUTOBLOQUEO:
//! - El frontend llama `reset_autolock_timer()` en cada interacción del usuario.
//! - Cuando el timer expira, Tauri emite el evento `autolock` al frontend.
//! - El frontend navega a /unlock y llama `lock_vault()`.
//! - El poller NO se pausa — sigue corriendo con credenciales del SO.
//!
//! VAULT SYNC:
//! - La bóveda se sincroniza con el VPS via POST /api/vault/sync (cifrada, conocimiento cero).
//! - Descarga al abrir y sube al guardar cambios.

use crate::api_client::{http_client, DeviceRegistration, SERVER_URL};
use crate::commands::vault::VaultState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{command, AppHandle, Emitter, Manager, State};
use tokio::time::{sleep, Duration};
use tracing::info;

// ── Autobloqueo ──────────────────────────────────────────────────────────────

static AUTOLOCK_GEN: AtomicU64 = AtomicU64::new(0);

/// Inicia el timer de autobloqueo en background.
/// Se llama al desbloquear la bóveda.
/// Si ya hay un timer corriendo, lo invalida incrementando la generación.
#[command]
pub async fn start_autolock_timer(app: AppHandle, minutes: u32) -> Result<(), String> {
    let minutes = minutes.max(1).min(120);
    let my_gen = AUTOLOCK_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    info!("⏱ Iniciando timer de autobloqueo: {minutes} min (gen {my_gen})");

    let app_handle = app.clone();
    let duration = Duration::from_secs((minutes * 60) as u64);

    tokio::spawn(async move {
        sleep(duration).await;
        if AUTOLOCK_GEN.load(Ordering::SeqCst) == my_gen {
            info!(
                "🔒 Timer de autobloqueo expirado — bloqueando bóveda y emitiendo evento autolock"
            );
            let vault_state = app_handle.state::<VaultState>();
            if let Ok(mut guard) = vault_state.vault.lock() {
                if let Some(vault) = &mut *guard {
                    vault.lock();
                }
            }
            let _ = app_handle.emit("autolock", ());
        }
    });

    Ok(())
}

/// Reinicia el timer de autobloqueo (llamar en cada interacción del usuario).
/// El frontend llama esto en onMouseMove / onKeyDown del Tauri window.
#[command]
pub async fn reset_autolock_timer(app: AppHandle, minutes: u32) -> Result<(), String> {
    // Reiniciar simplemente vuelve a iniciar — invalida la generación anterior
    start_autolock_timer(app, minutes).await
}

/// Cancela el timer de autobloqueo (al bloquear manualmente o al cerrar).
#[command]
pub async fn cancel_autolock_timer() -> Result<(), String> {
    AUTOLOCK_GEN.fetch_add(1, Ordering::SeqCst);
    info!("⏹ Timer de autobloqueo cancelado");
    Ok(())
}

// ── Vault Sync con VPS ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSyncResult {
    pub success: bool,
    pub direction: String, // "upload" | "download" | "none"
    pub error: Option<String>,
}

/// Sube la bóveda cifrada al VPS (conocimiento cero — el servidor no puede descifrarla).
/// Mapea: POST /api/vault/sync con `{ device_id, encrypted_payload }`.
pub async fn sync_vault_to_vps(app: &AppHandle) -> Result<(), String> {
    use crate::commands::vault::{vault_path, SessionAuthState};

    let path = vault_path(app);
    if !path.exists() {
        return Err("No hay bóveda local para subir".into());
    }

    let vault_str =
        std::fs::read_to_string(&path).map_err(|e| format!("No se pudo leer vault.json: {e}"))?;

    let device_id = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "desktop-win".to_string());

    // Si hay sesión de usuario autenticada (Fase 19), subir a /api/auth/vault
    let session_creds = {
        let state = app.state::<SessionAuthState>();
        let guard = state.auth.lock().unwrap();
        guard.as_ref().map(|a| {
            (
                a.server_url.clone(),
                a.username.clone(),
                a.auth_key_hex.clone(),
            )
        })
    };

    if let Some((server_url, username, auth_key_hex)) = session_creds {
        let resp = http_client()
            .post(&format!("{server_url}/api/auth/vault"))
            .json(&serde_json::json!({
                "username": username,
                "auth_key_hex": auth_key_hex,
                "device_id": device_id,
                "encrypted_payload": vault_str,
            }))
            .send()
            .await
            .map_err(|e| format!("Error al subir bóveda de usuario: {e}"))?;

        if resp.status().is_success() {
            info!("☁️ Bóveda de usuario sincronizada con VPS");
            return Ok(());
        } else {
            return Err(format!(
                "Error del servidor al sincronizar bóveda: {}",
                resp.status()
            ));
        }
    }

    // Fallback legado sin usuario registrado
    let reg_payload = DeviceRegistration {
        device_id: device_id.clone(),
        device_name: device_id.clone(),
        public_key_hex: "0".repeat(64),
    };
    let reg_resp = http_client()
        .post(&format!("{SERVER_URL}/api/devices/register"))
        .json(&reg_payload)
        .send()
        .await
        .map_err(|e| format!("Error al registrar dispositivo: {e}"))?;
    if !reg_resp.status().is_success() {
        return Err(format!(
            "Error del servidor al registrar dispositivo: {}",
            reg_resp.status()
        ));
    }

    let payload = serde_json::json!({
        "device_id": device_id,
        "encrypted_payload": vault_str,
    });

    let resp = http_client()
        .post(&format!("{SERVER_URL}/api/vault/sync"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Error al subir bóveda: {e}"))?;

    if resp.status().is_success() {
        info!("☁️ Bóveda sincronizada con VPS");
        Ok(())
    } else {
        Err(format!(
            "Error del servidor al sincronizar: {}",
            resp.status()
        ))
    }
}

/// Sube la bóveda cifrada al VPS (conocimiento cero — el servidor no puede descifrarla).
/// Mapea: POST /api/vault/sync
#[command]
pub async fn upload_vault_sync(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
) -> Result<VaultSyncResult, String> {
    // Verificar que la bóveda está desbloqueada (solo subir si el usuario está activo)
    {
        let guard = vault_state.vault.lock().unwrap();
        if guard.as_ref().map_or(true, |v| !v.is_unlocked()) {
            return Ok(VaultSyncResult {
                success: false,
                direction: "upload".into(),
                error: Some("La bóveda debe estar desbloqueada para sincronizar".into()),
            });
        }
    }

    match sync_vault_to_vps(&app).await {
        Ok(()) => Ok(VaultSyncResult {
            success: true,
            direction: "upload".into(),
            error: None,
        }),
        Err(e) => Ok(VaultSyncResult {
            success: false,
            direction: "upload".into(),
            error: Some(e),
        }),
    }
}

/// Descarga la bóveda cifrada del VPS y la guarda localmente.
/// Mapea: GET /api/vault/sync -> `{ version, device_id, encrypted_payload, checksum_sha256, updated_at }`
#[command]
pub async fn download_vault_sync(app: AppHandle) -> Result<VaultSyncResult, String> {
    use crate::commands::vault::vault_path;
    use ai_keyvault_core::vault::EncryptedVaultState;

    let resp = http_client()
        .get(&format!("{SERVER_URL}/api/vault/sync"))
        .send()
        .await
        .map_err(|e| format!("Error al descargar bóveda: {e}"))?;

    let status = resp.status();
    if status.is_success() {
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Error leyendo respuesta del servidor: {e}"))?;

        let encrypted_payload = body
            .get("encrypted_payload")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "La respuesta del servidor no contiene encrypted_payload".to_string())?;

        // Validar que es un EncryptedVaultState válido antes de escribir a disco
        serde_json::from_str::<EncryptedVaultState>(encrypted_payload)
            .map_err(|e| format!("La bóveda descargada tiene formato inválido: {e}"))?;

        let path = vault_path(&app);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear directorio: {e}"))?;
        }

        std::fs::write(&path, encrypted_payload.as_bytes())
            .map_err(|e| format!("Error guardando bóveda descargada: {e}"))?;

        info!(
            "⬇️ Bóveda descargada del VPS ({} bytes)",
            encrypted_payload.len()
        );
        Ok(VaultSyncResult {
            success: true,
            direction: "download".into(),
            error: None,
        })
    } else if status.as_u16() == 400 || status.as_u16() == 404 {
        // El servidor devuelve 400 ("No existe ninguna bóveda sincronizada aún") cuando está vacío
        Ok(VaultSyncResult {
            success: true,
            direction: "none".into(),
            error: None,
        })
    } else {
        Ok(VaultSyncResult {
            success: false,
            direction: "download".into(),
            error: Some(format!("Error del servidor: {}", status)),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupInfo {
    pub available: bool,
    pub last_backup_at: Option<String>,
    pub message: String,
}

/// Consulta la fecha del último respaldo en `MiguelGotea/ai_keyvault_data` (D-04).
#[command]
pub async fn get_last_backup_info() -> Result<BackupInfo, String> {
    let url = "https://api.github.com/repos/MiguelGotea/ai_keyvault_data/commits?per_page=1";
    let resp = http_client()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("No se pudo consultar GitHub: {e}"))?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let message = if code == 404 {
            "Repo privado (respaldo automático cifrado activo en GitHub Actions).".to_string()
        } else {
            format!("No disponible en este momento (HTTP {code}).")
        };
        return Ok(BackupInfo {
            available: false,
            last_backup_at: None,
            message,
        });
    }

    let commits: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida de GitHub: {e}"))?;

    if let Some(first) = commits.first() {
        let date = first
            .pointer("/commit/committer/date")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let msg = first
            .pointer("/commit/message")
            .and_then(|v| v.as_str())
            .unwrap_or("Respaldo")
            .to_string();
        return Ok(BackupInfo {
            available: date.is_some(),
            last_backup_at: date,
            message: msg,
        });
    }

    Ok(BackupInfo {
        available: false,
        last_backup_at: None,
        message: "Sin commits de respaldo aún en ai_keyvault_data.".into(),
    })
}

const AUTOSTART_REG_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const AUTOSTART_VALUE_NAME: &str = "AI KeyVault";

/// Consulta si «Iniciar con Windows» está habilitado en el registro del usuario (D-03).
#[command]
pub async fn get_autostart_enabled() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = std::process::Command::new("reg")
            .args(["query", AUTOSTART_REG_KEY, "/v", AUTOSTART_VALUE_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("No se pudo consultar inicio automático: {e}"))?;
        Ok(out.status.success())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

/// Activa o desactiva «Iniciar con Windows (en bandeja)» (D-03).
#[command]
pub async fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| format!("No se pudo obtener la ruta del ejecutable: {e}"))?;
            let value = format!("\"{}\" --minimized", exe.display());
            let out = std::process::Command::new("reg")
                .args([
                    "add",
                    AUTOSTART_REG_KEY,
                    "/v",
                    AUTOSTART_VALUE_NAME,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &value,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("No se pudo registrar inicio automático: {e}"))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                return Err(format!("Error al guardar inicio automático: {err}"));
            }
        } else {
            let out = std::process::Command::new("reg")
                .args([
                    "delete",
                    AUTOSTART_REG_KEY,
                    "/v",
                    AUTOSTART_VALUE_NAME,
                    "/f",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("No se pudo quitar inicio automático: {e}"))?;
            // Si no existía la entrada, no es error al desactivar
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !out.status.success() && !stderr.contains("unable to find") {
                return Err(format!("Error al quitar inicio automático: {stderr}"));
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Err("Iniciar con Windows solo está disponible en Windows.".into())
    }
}
