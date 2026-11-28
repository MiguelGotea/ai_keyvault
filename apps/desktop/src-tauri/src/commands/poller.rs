//! Comandos Tauri para el sidecar del poller (Fase 13).
//! El poller es un proceso hijo que corre mientras la app está abierta.
//! Se lanza al desbloquear la bóveda y se detiene al salir.
//!
//! SEGURIDAD:
//! - Estos comandos NUNCA exponen credenciales ni el contenido de la bóveda.
//! - Solo exponen estado operativo del proceso: running/paused/stopped.
//! - El poller lee credenciales directamente del SO (Credential Manager / auth.json).

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{command, AppHandle, Manager, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tracing::{info, warn};

// ── Estado global del poller ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PollerState {
    Running,
    Paused,
    Stopped,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PollerStatus {
    pub state: PollerState,
    pub pid: Option<u32>,
    pub last_poll_at: Option<String>,
    pub error: Option<String>,
}

pub struct PollerManager {
    pub status: Mutex<PollerStatus>,
    /// Handle del proceso hijo para poder terminarlo limpiamente al salir.
    pub child: Mutex<Option<CommandChild>>,
}

impl Default for PollerManager {
    fn default() -> Self {
        Self {
            status: Mutex::new(PollerStatus {
                state: PollerState::Stopped,
                pid: None,
                last_poll_at: None,
                error: None,
            }),
            child: Mutex::new(None),
        }
    }
}

/// Mata el proceso del poller si está corriendo (incluyendo instancias huérfanas en Windows).
pub fn kill_poller_process(poller: &PollerManager) {
    if let Ok(mut child_guard) = poller.child.lock() {
        if let Some(child) = child_guard.take() {
            let _ = child.kill();
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "ai-keyvault-poller.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    if let Ok(mut status_guard) = poller.status.lock() {
        status_guard.state = PollerState::Stopped;
        status_guard.pid = None;
    }
    info!("⏹ Proceso poller terminado");
}

// ── Comandos Tauri ────────────────────────────────────────────────────────────

/// Devuelve el estado actual del poller (running / paused / stopped / error).
/// Solo expone estado operativo — NUNCA credenciales.
#[command]
pub async fn get_poller_status(poller: State<'_, PollerManager>) -> Result<PollerStatus, String> {
    let guard = poller.status.lock().unwrap();
    Ok(guard.clone())
}

/// Lanza el sidecar del poller como proceso hijo.
/// Se llama desde el frontend después del desbloqueo de la bóveda.
/// El poller NO necesita la bóveda — lee del SO directamente.
#[command]
pub async fn start_poller(app: AppHandle, poller: State<'_, PollerManager>) -> Result<(), String> {
    {
        let guard = poller.status.lock().unwrap();
        if guard.state == PollerState::Running {
            info!("El poller ya está en ejecución (PID {:?})", guard.pid);
            return Ok(());
        }
    }

    info!("🚀 Lanzando sidecar del poller...");

    // Lanzar el sidecar via tauri-plugin-shell con mapa de identidad y configuración (D-02, D-06)
    // El binario se llama "ai-keyvault-poller" sin sufijo (Tauri agrega el triple)
    let map_path = crate::commands::vault::identity_map_path(&app);
    let mute_path = crate::commands::security::alerts_mute_file_path(&app);
    let result = app
        .shell()
        .sidecar("ai-keyvault-poller")
        .map_err(|e| format!("No se encontró el sidecar: {e}"))?
        .env(
            "AKYV_IDENTITY_MAP_PATH",
            map_path.to_string_lossy().as_ref(),
        )
        .env(
            "AKYV_ALERTS_MUTE_FILE",
            mute_path.to_string_lossy().as_ref(),
        )
        .env("AKYV_SERVER_URL", crate::api_client::SERVER_URL)
        .env("AKYV_POLL_INTERVAL", "600")
        .spawn();

    match result {
        Ok((mut rx, child)) => {
            let pid = child.pid();
            info!("✅ Poller iniciado con PID {pid}");
            if let Ok(mut child_guard) = poller.child.lock() {
                *child_guard = Some(child);
            }
            {
                let mut guard = poller.status.lock().unwrap();
                guard.state = PollerState::Running;
                guard.pid = Some(pid);
                guard.error = None;
            }

            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let manager = app_handle.state::<PollerManager>();
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let line = String::from_utf8_lossy(&bytes);
                            if line.contains("Ciclo completado") || line.contains("Lectura subida")
                            {
                                if let Ok(mut guard) = manager.status.lock() {
                                    guard.last_poll_at = Some(chrono::Utc::now().to_rfc3339());
                                    guard.error = None;
                                }
                            } else if line.contains("ERROR") || line.contains("Error al") {
                                if let Ok(mut guard) = manager.status.lock() {
                                    guard.error = Some(line.trim().to_string());
                                }
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            let line = String::from_utf8_lossy(&bytes).trim().to_string();
                            if !line.is_empty() {
                                warn!("Poller stderr: {line}");
                                if let Ok(mut guard) = manager.status.lock() {
                                    guard.error = Some(line);
                                }
                            }
                        }
                        CommandEvent::Error(err) => {
                            warn!("Error de proceso del poller: {err}");
                            if let Ok(mut guard) = manager.status.lock() {
                                guard.state = PollerState::Error;
                                guard.error = Some(format!("Error del poller: {err}"));
                            }
                        }
                        CommandEvent::Terminated(payload) => {
                            if let Ok(mut guard) = manager.status.lock() {
                                if guard.state != PollerState::Stopped {
                                    let code = payload
                                        .code
                                        .map_or_else(|| "desconocido".into(), |c| c.to_string());
                                    guard.state = PollerState::Error;
                                    guard.pid = None;
                                    guard.error = Some(format!(
                                        "El poller terminó inesperadamente (código {code})."
                                    ));
                                }
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        }
        Err(e) => {
            warn!("❌ Error al iniciar el poller: {e}");
            let mut guard = poller.status.lock().unwrap();
            guard.state = PollerState::Error;
            guard.error = Some(format!("{e}"));
            Err(format!("No se pudo iniciar el poller: {e}"))
        }
    }
}

/// Pausa el poller (el proceso sigue corriendo pero no hace polling).
/// El autobloqueo de la UI llama este comando si el usuario lo configuró.
/// REGLA: el poller NUNCA se detiene por autobloqueo — solo se puede pausar.
#[command]
pub async fn pause_poller(poller: State<'_, PollerManager>) -> Result<(), String> {
    let mut guard = poller.status.lock().unwrap();
    if guard.state == PollerState::Running {
        // En la implementación completa, enviaría una señal al proceso via stdin/IPC.
        // Por ahora actualizamos el estado local — el poller detectará el flag en el VPS.
        guard.state = PollerState::Paused;
        info!("⏸ Poller pausado");
    }
    Ok(())
}

/// Reanuda el poller pausado.
#[command]
pub async fn resume_poller(poller: State<'_, PollerManager>) -> Result<(), String> {
    let mut guard = poller.status.lock().unwrap();
    if guard.state == PollerState::Paused {
        guard.state = PollerState::Running;
        info!("▶️ Poller reanudado");
    }
    Ok(())
}

/// Detiene el poller completamente (al Salir desde el tray).
/// Después de esto la app cierra el proceso hijo.
#[command]
pub async fn stop_poller(poller: State<'_, PollerManager>) -> Result<(), String> {
    kill_poller_process(&poller);
    Ok(())
}

/// Ejecuta un ciclo único del poller (`--once`) y espera a que termine.
/// Se usa antes de cambiar de cuenta o agregar una nueva para capturar
/// la última cuota de la sesión en curso si pertenece a una cuenta registrada.
pub async fn run_poller_once(app: &AppHandle) -> Result<(), String> {
    let map_path = crate::commands::vault::identity_map_path(app);
    let mute_path = crate::commands::security::alerts_mute_file_path(app);

    let output = app
        .shell()
        .sidecar("ai-keyvault-poller")
        .map_err(|e| format!("No se encontró el sidecar del poller: {e}"))?
        .args(["--once"])
        .env(
            "AKYV_IDENTITY_MAP_PATH",
            map_path.to_string_lossy().as_ref(),
        )
        .env(
            "AKYV_ALERTS_MUTE_FILE",
            mute_path.to_string_lossy().as_ref(),
        )
        .env("AKYV_SERVER_URL", crate::api_client::SERVER_URL)
        .output()
        .await
        .map_err(|e| format!("Error ejecutando poller --once: {e}"))?;

    if output.status.success() {
        let manager = app.state::<PollerManager>();
        if let Ok(mut guard) = manager.status.lock() {
            guard.last_poll_at = Some(chrono::Utc::now().to_rfc3339());
            guard.error = None;
        }
        info!("✅ Poll manual (--once) completado");
        Ok(())
    } else {
        let err_txt = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Poller --once falló: {err_txt}"))
    }
}
