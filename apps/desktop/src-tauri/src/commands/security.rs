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

use crate::api_client::{http_client, SERVER_URL};
use crate::commands::vault::VaultState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{command, AppHandle, Emitter, State};
use tokio::time::{sleep, Duration};
use tracing::info;

// ── Autobloqueo ──────────────────────────────────────────────────────────────

static AUTOLOCK_RUNNING: AtomicBool = AtomicBool::new(false);

/// Inicia el timer de autobloqueo en background.
/// Se llama al desbloquear la bóveda.
/// Si ya hay un timer corriendo, lo reemplaza (cancela el anterior vía flag).
#[command]
pub async fn start_autolock_timer(
    app: AppHandle,
    minutes: u32,
) -> Result<(), String> {
    let minutes = minutes.max(1).min(120);
    info!("⏱ Iniciando timer de autobloqueo: {minutes} min");

    // Señalizar al timer anterior que se cancele
    AUTOLOCK_RUNNING.store(false, Ordering::Relaxed);
    tokio::time::sleep(Duration::from_millis(100)).await;
    AUTOLOCK_RUNNING.store(true, Ordering::Relaxed);

    let app_handle = app.clone();
    let duration = Duration::from_secs((minutes * 60) as u64);

    tokio::spawn(async move {
        sleep(duration).await;
        if AUTOLOCK_RUNNING.load(Ordering::Relaxed) {
            info!("🔒 Timer de autobloqueo expirado — emitiendo evento autolock");
            AUTOLOCK_RUNNING.store(false, Ordering::Relaxed);
            // Emitir evento al frontend
            let _ = app_handle.emit("autolock", ());
        }
    });

    Ok(())
}

/// Reinicia el timer de autobloqueo (llamar en cada interacción del usuario).
/// El frontend llama esto en onMouseMove / onKeyDown del Tauri window.
#[command]
pub async fn reset_autolock_timer(
    app: AppHandle,
    minutes: u32,
) -> Result<(), String> {
    // Reiniciar simplemente vuelve a iniciar — cancela el anterior automáticamente
    start_autolock_timer(app, minutes).await
}

/// Cancela el timer de autobloqueo (al bloquear manualmente o al cerrar).
#[command]
pub async fn cancel_autolock_timer() -> Result<(), String> {
    AUTOLOCK_RUNNING.store(false, Ordering::Relaxed);
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
    use crate::commands::vault::vault_path;

    let path = vault_path(app);
    if !path.exists() {
        return Err("No hay bóveda local para subir".into());
    }

    let vault_str = std::fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer vault.json: {e}"))?;

    let device_id = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "desktop-win".to_string());
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
        Err(format!("Error del servidor al sincronizar: {}", resp.status()))
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

        info!("⬇️ Bóveda descargada del VPS ({} bytes)", encrypted_payload.len());
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


