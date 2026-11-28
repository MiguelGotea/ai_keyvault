//! Switch de cuenta real (Fase 13b): escribe en Credential Manager de Windows.
//! Sin cmake — usa la API de Windows directamente.
//!
//! ARQUITECTURA DEL SWITCH (agy):
//! 1. Leer email + credential_blob de la bóveda (descifrado en Rust)
//! 2. Escribir el blob en Credential Manager "gemini:antigravity" (Windows)
//! 3. Registrar el resultado en el VPS
//! 4. Avisar al usuario que reinicie las ventanas del agente
//!
//! ARQUITECTURA DEL SWITCH (codex):
//! 1. Leer tokens_json de la bóveda
//! 2. Escribir ~/.codex/auth.json atómicamente (con backup)
//! 3. Registrar en VPS

use crate::api_client::{http_client, PostSwitchPayload, SERVER_URL};
use crate::commands::vault::VaultState;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{command, State};
use tracing::{info, warn};

// ── Estado para Deshacer (Fase 26) ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PreviousSwitchBackup {
    pub provider: String,
    pub agy_blob: Option<Vec<u8>>,
    pub codex_content: Option<String>,
    pub previous_label: Option<String>,
}

#[derive(Default)]
pub struct SwitchUndoState {
    pub last: Mutex<Option<PreviousSwitchBackup>>,
}

// ── Tipos de respuesta ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchResult {
    pub success: bool,
    pub error: Option<String>,
    pub account_id: String,
    pub provider: String,
    /// Aviso que el usuario debe reiniciar ventanas del agente.
    pub restart_notice: Option<String>,
    /// Indica si es posible deshacer este cambio restaurando la credencial previa.
    #[serde(default)]
    pub can_undo: bool,
    /// Procesos de herramientas/agentes detectados abiertos (sin cerrarlos).
    #[serde(default)]
    pub running_processes: Vec<String>,
}

// ── Windows Credential Manager ───────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod credman {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Lee el blob crudo de una credencial del Credential Manager de Windows.
    pub fn read_credential_raw(target: &str) -> Option<Vec<u8>> {
        use windows_sys::Win32::Foundation::TRUE;
        use windows_sys::Win32::Security::Credentials::{
            CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
        };

        let target_w = to_wide(target);
        let mut pcred = std::ptr::null_mut::<CREDENTIALW>();
        let ok = unsafe { CredReadW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pcred) };
        if ok == TRUE && !pcred.is_null() {
            let cred = unsafe { &*pcred };
            let len = cred.CredentialBlobSize as usize;
            let slice = unsafe { std::slice::from_raw_parts(cred.CredentialBlob, len) };
            let data = slice.to_vec();
            unsafe { CredFree(pcred as *mut _) };
            Some(data)
        } else {
            None
        }
    }

    /// Escribe un blob en el Credential Manager de Windows.
    pub fn write_credential_raw(target: &str, username: &str, blob: &[u8]) -> Result<(), String> {
        use windows_sys::Win32::Foundation::{GetLastError, TRUE};
        use windows_sys::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        let target_w = to_wide(target);
        let username_w = to_wide(username);
        let comment_w = to_wide("AI KeyVault");

        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target_w.as_ptr() as *mut _,
            Comment: comment_w.as_ptr() as *mut _,
            LastWritten: unsafe { std::mem::zeroed() },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut _,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: username_w.as_ptr() as *mut _,
        };

        let ok = unsafe { CredWriteW(&cred, 0) };
        if ok == TRUE {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            Err(format!(
                "CredWriteW falló (código {err}). Verifica permisos."
            ))
        }
    }

    /// Elimina una credencial del Credential Manager de Windows.
    pub fn delete_credential(target: &str) {
        use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
        let target_w = to_wide(target);
        unsafe { CredDeleteW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0) };
    }
}

/// Respalda el blob crudo actual de `gemini:antigravity` y lo borra del Credential Manager
/// para forzar que `agy` inicie un nuevo flujo OAuth en el navegador.
pub fn backup_and_clear_agy_credential() -> Option<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        let backup = credman::read_credential_raw("gemini:antigravity");
        credman::delete_credential("gemini:antigravity");
        backup
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Restaura un blob crudo respaldado en `gemini:antigravity`.
pub fn restore_agy_credential(blob: &[u8]) {
    #[cfg(target_os = "windows")]
    {
        let _ = credman::write_credential_raw("gemini:antigravity", "antigravity", blob);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = blob;
    }
}

/// Lee la credencial agy activa del Credential Manager y la devuelve como JSON String.
/// Función pública para que vault.rs pueda usarla en import_current_agy_credential.
pub fn read_agy_credential() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        credman::read_credential_raw("gemini:antigravity")
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|s| serde_json::from_str::<serde_json::Value>(s).is_ok())
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn codex_auth_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".codex")
        .join("auth.json")
}

/// Versión pública para usar desde vault.rs en import_current_codex_credential.
pub fn codex_auth_path_pub() -> std::path::PathBuf {
    codex_auth_path()
}

// ── Comandos Tauri ───────────────────────────────────────────────────────────

/// Obtiene la cuenta actualmente activa desde el Credential Manager / auth.json.
#[command]
pub async fn get_active_account(provider: String) -> Result<Option<String>, String> {
    match provider.as_str() {
        "agy" => {
            #[cfg(target_os = "windows")]
            {
                if let Some(bytes) = credman::read_credential_raw("gemini:antigravity") {
                    if let Ok(text) = String::from_utf8(bytes) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            let email = json
                                .get("id_token_email")
                                .or_else(|| json.get("email"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            return Ok(email);
                        }
                    }
                }
                Ok(None)
            }
            #[cfg(not(target_os = "windows"))]
            Ok(None)
        }
        "codex" => {
            let path = codex_auth_path();
            if path.exists() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let email = json
                            .pointer("/user/email")
                            .or_else(|| json.get("account_id"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        return Ok(email);
                    }
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Parsea la salida CSV de `tasklist /FO CSV /NH` e identifica herramientas abiertas sin cerrarlas.
pub fn parse_running_agent_processes(tasklist_csv: &str) -> Vec<String> {
    let mut detected = Vec::new();
    let known = [
        ("antigravity.exe", "Antigravity"),
        ("code.exe", "VS Code"),
        ("codex.exe", "Codex CLI"),
        ("agy.exe", "Antigravity CLI (agy)"),
    ];
    for line in tasklist_csv.lines() {
        let lower = line.trim().to_ascii_lowercase();
        for (exe, label) in known {
            if lower.starts_with(&format!("\"{exe}\"")) || lower.starts_with(exe) {
                let label_str = label.to_string();
                if !detected.contains(&label_str) {
                    detected.push(label_str);
                }
            }
        }
    }
    detected
}

/// Detecta procesos activos de herramientas/agentes en el sistema sin cerrarlos (Fase 26).
pub fn detect_running_agent_processes() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let output = std::process::Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                parse_running_agent_processes(&text)
            }
            _ => Vec::new(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

fn build_restart_notice(provider: &str, running: &[String]) -> String {
    if !running.is_empty() {
        return format!(
            "Procesos abiertos detectados (no se cerraron): {}. Reinicia esas herramientas para aplicar el cambio.",
            running.join(", ")
        );
    }
    match provider {
        "agy" => {
            "Reinicia las ventanas de Antigravity (agy) para aplicar el cambio de cuenta.".into()
        }
        "codex" => "Cierra y vuelve a abrir Codex para usar la nueva cuenta.".into(),
        _ => "Reinicia el agente para aplicar el cambio.".into(),
    }
}

/// Devuelve las herramientas/agentes actualmente abiertos sin cerrarlos (Fase 26).
#[command]
pub async fn get_running_agent_processes() -> Result<Vec<String>, String> {
    Ok(detect_running_agent_processes())
}

/// Switch de cuenta real: lee credenciales de la bóveda y las escribe en el SO.
#[command]
pub async fn switch_account(
    app: tauri::AppHandle,
    vault_state: State<'_, VaultState>,
    undo_state: State<'_, SwitchUndoState>,
    account_id: String,
    provider: String,
    device_id: String,
) -> Result<SwitchResult, String> {
    // Parsear account_id como UUID (el id interno de la bóveda)
    // Si no es UUID, buscar por alias/email
    let (email, password, tokens) = {
        let guard = vault_state.vault.lock().unwrap();
        match &*guard {
            Some(vault) if vault.is_unlocked() => {
                // Buscar la cuenta: por UUID, por alias, o por email
                let accounts = &vault.state().accounts;
                let found_id = accounts
                    .iter()
                    .find(|a| a.id.to_string() == account_id || (a.provider == provider && a.alias == account_id))
                    .map(|a| a.id);

                match found_id {
                    Some(uid) => {
                        let email = vault
                            .decrypt_account_email(uid)
                            .map_err(|e| format!("Error descifrando email: {e}"))?;
                        let pass = vault.decrypt_account_password(uid).unwrap_or(None);
                        // tokens: descifrar el blob completo del Credential Manager
                        let tokens = vault.decrypt_account_tokens(uid).unwrap_or(None);
                        (email, pass, tokens)
                    }
                    None => {
                        return Ok(SwitchResult {
                            success: false,
                            error: Some(format!(
                                "Cuenta '{}' no encontrada en la bóveda. Agrégala primero en Ajustes → Mis Cuentas.",
                                account_id
                            )),
                            account_id,
                            provider,
                            restart_notice: None,
                            can_undo: false,
                            running_processes: Vec::new(),
                        });
                    }
                }
            }
            Some(_) => return Err("La bóveda está bloqueada. Desbloquea primero.".into()),
            None => return Err("No hay bóveda cargada.".into()),
        }
    };

    // Capturar credencial previa para permitir «Deshacer» (Fase 26)
    let previous_label = get_active_account(provider.clone()).await.ok().flatten();

    // Si la cuenta actualmente activa en esta PC está registrada en la bóveda,
    // ejecutar un poll manual antes de reemplazarla para guardar su cuota final.
    if let Some(ref current_email) = previous_label {
        let map_path = crate::commands::vault::identity_map_path(&app);
        let is_registered = ai_keyvault_core::identity_map::IdentityMap::load_encrypted(&map_path)
            .map(|m| m.resolve(&provider, current_email).is_some())
            .unwrap_or(false);
        if is_registered {
            info!(
                provider = %provider,
                current_email = %current_email,
                "⚡ Ejecutando poll manual antes de cambiar de cuenta..."
            );
            if let Err(e) = crate::commands::poller::run_poller_once(&app).await {
                warn!("⚠️ Poll previo al cambio de cuenta falló (se continúa): {e}");
            }
        }
    }
    let prev_agy_blob = if provider == "agy" {
        #[cfg(target_os = "windows")]
        {
            credman::read_credential_raw("gemini:antigravity")
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    } else {
        None
    };
    let prev_codex_content = if provider == "codex" {
        std::fs::read_to_string(codex_auth_path()).ok()
    } else {
        None
    };

    // Realizar switch según proveedor
    let result = match provider.as_str() {
        "agy" => switch_agy(&account_id, &email, tokens.as_deref()).await,
        "codex" => switch_codex(&account_id, tokens.as_deref(), password.as_deref()).await,
        _ => Err(format!("Proveedor '{}' no soportado aún", provider)),
    };

    let (success, error_msg) = match result {
        Ok(()) => (true, None),
        Err(e) => {
            warn!("Switch fallido: {e}");
            (false, Some(e))
        }
    };

    let can_undo = if success && (prev_agy_blob.is_some() || prev_codex_content.is_some()) {
        if let Ok(mut guard) = undo_state.last.lock() {
            *guard = Some(PreviousSwitchBackup {
                provider: provider.clone(),
                agy_blob: prev_agy_blob,
                codex_content: prev_codex_content,
                previous_label,
            });
            true
        } else {
            false
        }
    } else {
        false
    };

    // Registrar en VPS (best-effort) — usando los campos reales del VPS
    let _ = http_client()
        .post(&format!("{SERVER_URL}/api/switch"))
        .json(&PostSwitchPayload {
            to_account_id: account_id.clone(),
            from_account_id: None,
            provider: provider.clone(),
            device_id,
            success,
            reason: error_msg.clone(),
        })
        .send()
        .await;

    let running_processes = if success {
        info!("Switch OK: {} ({})", account_id, provider);
        detect_running_agent_processes()
    } else {
        Vec::new()
    };

    Ok(SwitchResult {
        success,
        error: error_msg,
        account_id,
        provider: provider.clone(),
        restart_notice: if success {
            Some(build_restart_notice(&provider, &running_processes))
        } else {
            None
        },
        can_undo,
        running_processes,
    })
}

/// Deshace el último switch restaurando la credencial previa en el SO (Fase 26).
#[command]
pub async fn undo_last_switch(
    undo_state: State<'_, SwitchUndoState>,
    device_id: String,
) -> Result<SwitchResult, String> {
    let backup = {
        let mut guard = undo_state.last.lock().unwrap();
        guard.take()
    };

    let Some(prev) = backup else {
        return Err("No hay un cambio reciente para deshacer.".into());
    };

    let restore_res: Result<(), String> = match prev.provider.as_str() {
        "agy" => {
            #[cfg(target_os = "windows")]
            {
                match &prev.agy_blob {
                    Some(blob) => {
                        let user = prev.previous_label.as_deref().unwrap_or("antigravity");
                        credman::write_credential_raw("gemini:antigravity", user, blob)
                    }
                    None => Err("No había credencial previa de Antigravity para restaurar.".into()),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("Deshacer agy solo disponible en Windows".into())
            }
        }
        "codex" => match &prev.codex_content {
            Some(content) => switch_codex("undo", Some(content), None).await,
            None => Err("No había auth.json previo de Codex para restaurar.".into()),
        },
        other => Err(format!("Proveedor '{}' no soportado", other)),
    };

    restore_res?;

    let target_label = prev
        .previous_label
        .unwrap_or_else(|| "cuenta-anterior".into());
    let _ = http_client()
        .post(&format!("{SERVER_URL}/api/switch"))
        .json(&PostSwitchPayload {
            to_account_id: target_label.clone(),
            from_account_id: None,
            provider: prev.provider.clone(),
            device_id,
            success: true,
            reason: Some("Deshacer cambio anterior".into()),
        })
        .send()
        .await;

    let running_processes = detect_running_agent_processes();
    let restart_notice = Some(build_restart_notice(&prev.provider, &running_processes));

    Ok(SwitchResult {
        success: true,
        error: None,
        account_id: target_label,
        provider: prev.provider,
        restart_notice,
        can_undo: false,
        running_processes,
    })
}

// ── Implementaciones por proveedor ───────────────────────────────────────────

async fn switch_agy(
    account_id: &str,
    email: &str,
    credential_json: Option<&str>,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let backup = credman::read_credential_raw("gemini:antigravity");

        // Si tenemos el blob real de la bóveda, lo usamos. Si no, error claro.
        let blob_bytes = match credential_json {
            Some(json) => json.as_bytes().to_vec(),
            None => {
                // Sin blob guardado — no podemos hacer un switch real que funcione en agy.
                // Escribimos un marcador para que el usuario sepa que debe reimportar la cuenta.
                return Err(format!(
                    "La cuenta '{}' no tiene credenciales completas guardadas en la bóveda. \
                     Ve a Mis Cuentas y agrega la cuenta de nuevo para capturar el blob completo.",
                    account_id
                ));
            }
        };

        credman::write_credential_raw("gemini:antigravity", email, &blob_bytes).map_err(|e| {
            // Restaurar si falla
            if let Some(blob) = &backup {
                let _ = credman::write_credential_raw("gemini:antigravity", "restore", blob);
            }
            e
        })?;

        info!("Credential Manager actualizado para agy: {email}");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (account_id, email, credential_json);
        Err("Switch de agy solo disponible en Windows".into())
    }
}

async fn switch_codex(
    account_id: &str,
    tokens_json: Option<&str>,
    _password: Option<&str>,
) -> Result<(), String> {
    let auth_path = codex_auth_path();
    let parent = auth_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear ~/.codex/: {e}"))?;

    // Respaldar auth.json actual
    if auth_path.exists() {
        let _ = std::fs::copy(&auth_path, parent.join("auth.json.akvault-backup"));
    }

    // Usar el blob completo descifrado de la bóveda (nunca JSON sintético)
    let new_content = match tokens_json {
        Some(tokens) => tokens.to_string(),
        None => {
            return Err(format!(
                "La cuenta '{}' no tiene credenciales de Codex guardadas en la bóveda. \
                 Ve a Mis Cuentas e importa auth.json de nuevo.",
                account_id
            ));
        }
    };

    // Escribir atómicamente (temp → rename)
    let temp_path = parent.join("auth.json.akvault-tmp");
    std::fs::write(&temp_path, new_content.as_bytes())
        .map_err(|e| format!("Error escribiendo auth.json temporal: {e}"))?;
    std::fs::rename(&temp_path, &auth_path)
        .map_err(|e| format!("Error renombrando auth.json: {e}"))?;

    info!("auth.json de Codex actualizado para {account_id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_running_agent_processes;

    #[test]
    fn detecta_procesos_sin_duplicados() {
        let csv = "\"Code.exe\",\"1234\",\"Console\",\"1\",\"100,000 K\"\n\
                   \"Code.exe\",\"5678\",\"Console\",\"1\",\"80,000 K\"\n\
                   \"Antigravity.exe\",\"9012\",\"Console\",\"1\",\"120,000 K\"\n\
                   \"notepad.exe\",\"1111\",\"Console\",\"1\",\"10,000 K\"\n";
        let found = parse_running_agent_processes(csv);
        assert_eq!(found, vec!["VS Code", "Antigravity"]);
    }
}
