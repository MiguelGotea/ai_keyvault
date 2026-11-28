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

use crate::api_client::http_client;
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

    // Sin sesión de usuario — la app siempre requiere cuenta registrada
    Err("No hay sesión activa. Inicia sesión para sincronizar la bóveda.".into())
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

/// Descarga la bóveda cifrada del VPS, la guarda en disco y recarga el estado en memoria
/// (hot-reload sin pedir contraseña) para que la UI refleje los cambios de inmediato.
/// Si hay sesión autenticada usa POST /api/auth/vault/download (tabla user_vaults).
/// En fallback usa GET /api/vault/sync (legado, sin sesión).
#[command]
pub async fn download_vault_sync(app: AppHandle) -> Result<VaultSyncResult, String> {
    use crate::commands::vault::{vault_path, SessionAuthState, VaultState};
    use ai_keyvault_core::vault::EncryptedVaultState;

    // ── Helper: aplica hot-reload en memoria y emite evento vault-reloaded ──
    fn apply_hot_reload(
        app: &AppHandle,
        new_state: EncryptedVaultState,
        path: &std::path::Path,
        payload_bytes: &[u8],
    ) -> Result<(), String> {
        // 1. Guardar en disco
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("No se pudo crear directorio: {e}"))?;
        }
        std::fs::write(path, payload_bytes)
            .map_err(|e| format!("Error guardando bóveda descargada: {e}"))?;

        // 2. Hot-reload en RAM: si la bóveda está desbloqueada, reemplazamos el
        //    EncryptedVaultState en memoria (cuentas, etc.) sin tocar unlocked_key.
        //    state_mut() es &mut al campo state, el unlocked_key queda intacto.
        {
            let vault_state = app.state::<VaultState>();
            let mut guard = vault_state.vault.lock().unwrap();
            if let Some(vault) = &mut *guard {
                if vault.is_unlocked() {
                    *vault.state_mut() = new_state;
                    info!("🔄 Bóveda recargada en memoria sin re-bloquear.");
                }
            }
        }

        // 3. Notificar al frontend para que haga refetch
        let _ = app.emit("vault-reloaded", ());
        Ok(())
    }

    // Si hay sesión de usuario registrado, descargar desde /api/auth/vault/download
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
        let device_id = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "desktop-win".to_string());

        let resp = http_client()
            .post(&format!("{server_url}/api/auth/vault/download"))
            .json(&serde_json::json!({
                "username": username,
                "auth_key_hex": auth_key_hex,
                "device_id": device_id,
            }))
            .send()
            .await
            .map_err(|e| format!("Error al descargar bóveda de usuario: {e}"))?;

        let status = resp.status();
        if status.is_success() {
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Error leyendo respuesta del servidor: {e}"))?;

            let encrypted_payload = body
                .get("encrypted_payload")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    "La respuesta del servidor no contiene encrypted_payload".to_string()
                })?;

            let new_state = serde_json::from_str::<EncryptedVaultState>(encrypted_payload)
                .map_err(|e| format!("La bóveda descargada tiene formato inválido: {e}"))?;

            let path = vault_path(&app);
            apply_hot_reload(&app, new_state, &path, encrypted_payload.as_bytes())?;

            info!(
                "⬇️ Bóveda de usuario descargada del VPS ({} bytes)",
                encrypted_payload.len()
            );
            return Ok(VaultSyncResult {
                success: true,
                direction: "download".into(),
                error: None,
            });
        } else if status.as_u16() == 400 || status.as_u16() == 404 {
            return Ok(VaultSyncResult {
                success: true,
                direction: "none".into(),
                error: None,
            });
        } else {
            return Ok(VaultSyncResult {
                success: false,
                direction: "download".into(),
                error: Some(format!(
                    "Error del servidor al descargar bóveda: {}",
                    status
                )),
            });
        }
    }

    // Sin sesión de usuario — la app siempre requiere cuenta registrada
    Ok(VaultSyncResult {
        success: false,
        direction: "download".into(),
        error: Some("No hay sesión activa. Inicia sesión para sincronizar la bóveda.".into()),
    })
}

/// Lanza el timer de auto-sincronización periódica de la bóveda en background.
/// Se llama al desbloquear la bóveda. Cada `interval_minutes` minutos sube la bóveda al VPS.
/// Si ya hay uno corriendo, el nuevo reemplaza al anterior (via generación atómica).
static VAULT_SYNC_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[command]
pub async fn start_vault_autosync(app: AppHandle, interval_minutes: u32) -> Result<(), String> {
    use crate::commands::vault::VaultState;
    use std::sync::atomic::Ordering;

    let interval_minutes = interval_minutes.max(5).min(60);
    let my_gen = VAULT_SYNC_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    info!("🔄 Auto-sync de bóveda iniciado: cada {interval_minutes} min (gen {my_gen})");

    let app_handle = app.clone();
    let duration = tokio::time::Duration::from_secs((interval_minutes as u64) * 60);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(duration);
        ticker.tick().await; // Consumir el primer tick inmediato
        loop {
            ticker.tick().await;
            if VAULT_SYNC_GEN.load(Ordering::SeqCst) != my_gen {
                break; // Esta generación fue reemplazada
            }
            // Solo subir si la bóveda está desbloqueada
            let is_unlocked = {
                let vault_state = app_handle.state::<VaultState>();
                let guard = vault_state.vault.lock().unwrap();
                guard.as_ref().map_or(false, |v| v.is_unlocked())
            };
            if is_unlocked {
                match sync_vault_to_vps(&app_handle).await {
                    Ok(()) => info!("⏱ Auto-sync: bóveda subida al VPS correctamente"),
                    Err(e) => tracing::warn!("⚠️ Auto-sync: error al subir bóveda: {e}"),
                }
            }
        }
        info!("⏹ Auto-sync de bóveda detenido (gen {my_gen})");
    });

    Ok(())
}

/// Cancela el timer de auto-sincronización (al cerrar sesión o bloquear la bóveda).
#[command]
pub async fn cancel_vault_autosync() -> Result<(), String> {
    use std::sync::atomic::Ordering;
    VAULT_SYNC_GEN.fetch_add(1, Ordering::SeqCst);
    info!("⏹ Auto-sync de bóveda cancelado");
    Ok(())
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

/// Ruta del archivo de silencio temporal de alertas (Fase 22).
pub fn alerts_mute_file_path(app: &AppHandle) -> std::path::PathBuf {
    app.path()
        .app_data_dir()
        .expect("No se pudo obtener AppData dir")
        .join("alerts-muted-until.txt")
}

/// Silencia notificaciones de alerta por `hours` horas (0 = reactivar de inmediato).
#[command]
pub async fn set_alerts_muted_hours(app: AppHandle, hours: u32) -> Result<Option<String>, String> {
    let path = alerts_mute_file_path(&app);
    if hours == 0 {
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("No se pudo reactivar alertas: {e}"))?;
        }
        return Ok(None);
    }

    let clamped = hours.min(168);
    let until = (chrono::Utc::now() + chrono::Duration::hours(clamped as i64)).to_rfc3339();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear directorio de datos: {e}"))?;
    }
    std::fs::write(&path, &until)
        .map_err(|e| format!("No se pudo guardar silencio de alertas: {e}"))?;
    Ok(Some(until))
}

/// Devuelve el timestamp RFC 3339 hasta el cual las alertas están silenciadas, o None.
#[command]
pub async fn get_alerts_muted_until(app: AppHandle) -> Result<Option<String>, String> {
    let path = alerts_mute_file_path(&app);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer estado de silencio: {e}"))?;
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw.trim()) {
        if dt.with_timezone(&chrono::Utc) > chrono::Utc::now() {
            return Ok(Some(raw.trim().to_string()));
        }
    }
    let _ = std::fs::remove_file(&path);
    Ok(None)
}
