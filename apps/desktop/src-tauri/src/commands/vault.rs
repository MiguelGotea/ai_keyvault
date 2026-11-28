//! Comandos Tauri para la bóveda local cifrada (Fase 12).
//! Gestiona el ciclo de vida: crear → desbloquear → autobloquear → recuperar.
//!
//! La bóveda se guarda en el directorio de datos de la app:
//!   %APPDATA%\com.miguelgotea.ai-keyvault\vault.json
//!
//! SEGURIDAD:
//! - La VaultKey (32 bytes) solo existe en memoria mientras la bóveda está desbloqueada.
//! - Al bloquearse: zeroize() borra los bytes de la clave de RAM.
//! - La contraseña maestra NUNCA se guarda — se deriva con Argon2id en cada desbloqueo.

use ai_keyvault_core::kdf::{derive_keys, KdfParams};
use ai_keyvault_core::vault::{EncryptedVaultState, Vault};
use chrono;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{command, AppHandle, Manager, State};
use zeroize::Zeroize;

// ── Estado global ────────────────────────────────────────────────────────────

/// Estado de la bóveda compartido entre threads via Mutex.
/// Vive en Tauri AppState — se accede con `app.state::<VaultState>()`.
pub struct VaultState {
    pub vault: Mutex<Option<Vault>>,
}

impl Default for VaultState {
    fn default() -> Self {
        Self {
            vault: Mutex::new(None),
        }
    }
}

/// Credenciales de sesión autenticada en RAM (se borran con zeroize al bloquear o cerrar sesión).
#[derive(Default)]
pub struct SessionAuth {
    pub server_url: String,
    pub username: String,
    pub auth_key_hex: String,
}

impl Drop for SessionAuth {
    fn drop(&mut self) {
        self.auth_key_hex.zeroize();
    }
}

pub struct SessionAuthState {
    pub auth: Mutex<Option<SessionAuth>>,
}

impl Default for SessionAuthState {
    fn default() -> Self {
        Self {
            auth: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionFile {
    pub server_url: String,
    pub username: String,
    pub display_name: String,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfoResponse {
    pub has_session: bool,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub server_url: String,
}

// ── Tipos de respuesta al frontend ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultStatusResponse {
    pub initialized: bool, // ¿existe vault.json en disco?
    pub locked: bool,      // ¿tiene VaultKey en memoria?
    pub version: String,   // versión del binario
    pub accounts_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnlockResult {
    pub success: bool,
    pub error: Option<String>,
    pub accounts_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateVaultResult {
    pub success: bool,
    pub recovery_key: Option<String>, // AKVR-XXXX-... mostrar una sola vez
    pub error: Option<String>,
}

// ── Ruta del archivo de bóveda ───────────────────────────────────────────────

pub fn vault_path(app: &AppHandle) -> PathBuf {
    // %APPDATA%\com.miguelgotea.ai-keyvault\vault.json
    app.path()
        .app_data_dir()
        .expect("No se pudo obtener AppData dir")
        .join("vault.json")
}

/// Ruta del mapa local cifrado correo → UUID (D-02) usado por el poller.
pub fn identity_map_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("No se pudo obtener AppData dir")
        .join("identity-map.enc")
}

/// Ruta del archivo de sesión local (`session.json`).
pub fn session_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("No se pudo obtener AppData dir")
        .join("session.json")
}

pub fn load_session_file(app: &AppHandle) -> Option<SessionFile> {
    let path = session_path(app);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<SessionFile>(&raw).ok()
}

fn save_session_file(app: &AppHandle, session: &SessionFile) -> Result<(), String> {
    let path = session_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear directorio de sesión: {e}"))?;
    }
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("Error serializando sesión: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Error guardando session.json: {e}"))?;
    Ok(())
}

fn current_device_id() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "desktop-win".to_string())
}

/// Regenera el mapa local cifrado correo → UUID a partir de la bóveda desbloqueada.
pub fn sync_identity_map(app: &AppHandle, vault: &Vault) -> Result<(), String> {
    use ai_keyvault_core::IdentityMap;

    if !vault.is_unlocked() {
        return Ok(());
    }
    let mut map = IdentityMap::new();
    for acc in &vault.state().accounts {
        let email = vault
            .decrypt_account_email(acc.id)
            .map_err(|e| format!("Error descifrando correo para mapa de identidad: {e}"))?;
        map.insert(&acc.provider, &email, &acc.id.to_string());
    }
    map.save_encrypted(&identity_map_path(app))
        .map_err(|e| format!("Error guardando mapa de identidad local: {e}"))
}

fn vault_exists(app: &AppHandle) -> bool {
    vault_path(app).exists()
}

fn save_vault_state(app: &AppHandle, state: &EncryptedVaultState) -> Result<(), String> {
    let path = vault_path(app);
    // Crear directorio si no existe
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear directorio de bóveda: {e}"))?;
    }
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Error serializando bóveda: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Error guardando bóveda en {}: {e}", path.display()))?;
    Ok(())
}

fn load_vault_state(app: &AppHandle) -> Result<EncryptedVaultState, String> {
    let path = vault_path(app);
    let json =
        std::fs::read_to_string(&path).map_err(|e| format!("No se pudo leer vault.json: {e}"))?;
    serde_json::from_str::<EncryptedVaultState>(&json)
        .map_err(|e| format!("vault.json corrupto o formato inválido: {e}"))
}

// ── Comandos Tauri ────────────────────────────────────────────────────────────

/// Devuelve el estado actual de la bóveda.
#[command]
pub async fn get_vault_status(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
) -> Result<VaultStatusResponse, String> {
    let initialized = vault_exists(&app);
    let guard = vault_state.vault.lock().unwrap();
    let (locked, accounts_count) = match &*guard {
        Some(v) => (!v.is_unlocked(), v.state().accounts.len()),
        None => (true, 0),
    };
    Ok(VaultStatusResponse {
        initialized,
        locked,
        version: env!("CARGO_PKG_VERSION").to_string(),
        accounts_count,
    })
}

/// Crea una bóveda nueva con la contraseña maestra proporcionada.
/// Devuelve la clave de recuperación (AKVR-...) — mostrar SOLO UNA VEZ al usuario.
/// Si ya existe una bóveda, devuelve error.
#[command]
pub async fn create_vault(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    password: String,
) -> Result<CreateVaultResult, String> {
    if vault_exists(&app) {
        return Ok(CreateVaultResult {
            success: false,
            recovery_key: None,
            error: Some(
                "Ya existe una bóveda en este dispositivo. Para resetear, elimina vault.json."
                    .into(),
            ),
        });
    }

    match Vault::create_new(&password, None) {
        Ok((vault, recovery)) => {
            // Persistir el estado cifrado en disco
            save_vault_state(&app, vault.state())?;
            sync_identity_map(&app, &vault)?;

            // Guardar la bóveda desbloqueada en memoria
            {
                let mut guard = vault_state.vault.lock().unwrap();
                *guard = Some(vault);
            }

            // Sincronizar con el VPS (best-effort)
            let _ = crate::commands::security::sync_vault_to_vps(&app).await;

            Ok(CreateVaultResult {
                success: true,
                recovery_key: Some(recovery.to_formatted_string()),
                error: None,
            })
        }
        Err(e) => Ok(CreateVaultResult {
            success: false,
            recovery_key: None,
            error: Some(format!("Error creando bóveda: {e}")),
        }),
    }
}

/// Desbloquea la bóveda existente con la contraseña maestra.
#[command]
pub async fn unlock_vault(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    session_auth: State<'_, SessionAuthState>,
    password: String,
) -> Result<UnlockResult, String> {
    if !vault_exists(&app) {
        return Ok(UnlockResult {
            success: false,
            error: Some("No existe ninguna bóveda en este dispositivo. Crea una primero.".into()),
            accounts_count: 0,
        });
    }

    let state = load_vault_state(&app)?;
    let mut vault = Vault::load_locked(state);

    match vault.unlock_with_password(&password) {
        Ok(_) => {
            sync_identity_map(&app, &vault)?;
            if let Some(sess) = load_session_file(&app) {
                if let Ok((auth_key, _)) =
                    derive_keys(&password, &vault.state().salt, &vault.state().kdf_params)
                {
                    let mut auth_guard = session_auth.auth.lock().unwrap();
                    *auth_guard = Some(SessionAuth {
                        server_url: sess.server_url,
                        username: sess.username,
                        auth_key_hex: hex::encode(auth_key.0),
                    });
                }
            }
            let count = vault.state().accounts.len();
            let mut guard = vault_state.vault.lock().unwrap();
            *guard = Some(vault);
            Ok(UnlockResult {
                success: true,
                error: None,
                accounts_count: count,
            })
        }
        Err(e) => Ok(UnlockResult {
            success: false,
            error: Some(format!("Clave maestra incorrecta: {e}")),
            accounts_count: 0,
        }),
    }
}

/// Bloquea la bóveda — hace zeroize de la VaultKey en memoria.
/// El poller sigue corriendo (lee del SO, no de la bóveda).
#[command]
pub async fn lock_vault(
    vault_state: State<'_, VaultState>,
    session_auth: State<'_, SessionAuthState>,
) -> Result<(), String> {
    {
        let mut guard = vault_state.vault.lock().unwrap();
        if let Some(vault) = &mut *guard {
            vault.lock();
        }
    }
    let mut auth_guard = session_auth.auth.lock().unwrap();
    *auth_guard = None;
    Ok(())
}

/// Información de la sesión local para primer arranque vs arranques siguientes (Fase 19.2).
#[command]
pub async fn get_session_info(app: AppHandle) -> Result<SessionInfoResponse, String> {
    let default_url = crate::api_client::SERVER_URL.to_string();
    if let Some(sess) = load_session_file(&app) {
        Ok(SessionInfoResponse {
            has_session: vault_exists(&app),
            username: Some(sess.username),
            display_name: Some(sess.display_name),
            server_url: sess.server_url,
        })
    } else {
        Ok(SessionInfoResponse {
            has_session: false,
            username: None,
            display_name: None,
            server_url: default_url,
        })
    }
}

/// Crea un usuario nuevo + bóveda cifrada y la registra en el VPS (Fase 19.2).
#[command]
pub async fn register_user_and_vault(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    session_auth: State<'_, SessionAuthState>,
    server_url: String,
    display_name: String,
    username: String,
    password: String,
) -> Result<CreateVaultResult, String> {
    use crate::api_client::http_client;

    let clean_url = server_url.trim().trim_end_matches('/').to_string();
    let clean_user = username.trim().to_lowercase();
    let clean_name = display_name.trim().to_string();
    if clean_user.is_empty() || clean_name.is_empty() {
        return Ok(CreateVaultResult {
            success: false,
            recovery_key: None,
            error: Some("Ingresa tu nombre y nombre de usuario.".into()),
        });
    }
    if password.len() < 12 {
        return Ok(CreateVaultResult {
            success: false,
            recovery_key: None,
            error: Some("La clave maestra debe tener al menos 12 caracteres.".into()),
        });
    }

    let (vault, recovery) =
        Vault::create_new(&password, None).map_err(|e| format!("Error creando bóveda: {e}"))?;
    let (auth_key, _) = derive_keys(&password, &vault.state().salt, &vault.state().kdf_params)
        .map_err(|e| format!("Error derivando clave de autenticación: {e}"))?;

    let auth_key_hex = hex::encode(auth_key.0);
    let device_id = current_device_id();
    let encrypted_payload = serde_json::to_string_pretty(vault.state())
        .map_err(|e| format!("Error serializando bóveda: {e}"))?;

    let reg_body = serde_json::json!({
        "username": clean_user,
        "display_name": clean_name,
        "kdf_salt_hex": hex::encode(&vault.state().salt),
        "kdf_memory_kib": vault.state().kdf_params.memory_kib,
        "kdf_iterations": vault.state().kdf_params.iterations,
        "kdf_parallelism": vault.state().kdf_params.parallelism,
        "auth_key_hex": auth_key_hex,
        "device_id": device_id,
        "device_name": device_id,
        "encrypted_payload": encrypted_payload,
    });

    let resp = http_client()
        .post(&format!("{clean_url}/api/auth/register"))
        .json(&reg_body)
        .send()
        .await
        .map_err(|e| format!("No se pudo conectar al servidor: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        return Ok(CreateVaultResult {
            success: false,
            recovery_key: None,
            error: Some(format!(
                "No se pudo registrar usuario ({status}): {err_body}"
            )),
        });
    }

    save_vault_state(&app, vault.state())?;
    sync_identity_map(&app, &vault)?;
    save_session_file(
        &app,
        &SessionFile {
            server_url: clean_url.clone(),
            username: clean_user.clone(),
            display_name: clean_name,
            device_id,
        },
    )?;

    {
        let mut auth_guard = session_auth.auth.lock().unwrap();
        *auth_guard = Some(SessionAuth {
            server_url: clean_url,
            username: clean_user,
            auth_key_hex,
        });
    }
    {
        let mut guard = vault_state.vault.lock().unwrap();
        *guard = Some(vault);
    }

    Ok(CreateVaultResult {
        success: true,
        recovery_key: Some(recovery.to_formatted_string()),
        error: None,
    })
}

/// Inicia sesión en una bóveda existente del VPS (estilo Bitwarden) y la descifra en local.
#[command]
pub async fn login_user_and_sync(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    session_auth: State<'_, SessionAuthState>,
    server_url: String,
    username: String,
    password: String,
) -> Result<UnlockResult, String> {
    use crate::api_client::http_client;

    let clean_url = server_url.trim().trim_end_matches('/').to_string();
    let clean_user = username.trim().to_lowercase();
    if clean_user.is_empty() || password.is_empty() {
        return Ok(UnlockResult {
            success: false,
            error: Some("Ingresa usuario y clave maestra.".into()),
            accounts_count: 0,
        });
    }

    let params_resp = http_client()
        .post(&format!("{clean_url}/api/auth/params"))
        .json(&serde_json::json!({ "username": clean_user }))
        .send()
        .await
        .map_err(|e| format!("Error conectando al servidor: {e}"))?;

    if !params_resp.status().is_success() {
        return Ok(UnlockResult {
            success: false,
            error: Some(format!(
                "Error consultando servidor: HTTP {}",
                params_resp.status()
            )),
            accounts_count: 0,
        });
    }

    let params_json: serde_json::Value = params_resp
        .json()
        .await
        .map_err(|e| format!("Respuesta inválida del servidor: {e}"))?;
    let salt_hex = params_json
        .get("kdf_salt_hex")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Falta kdf_salt_hex en respuesta".to_string())?;
    let salt = hex::decode(salt_hex).map_err(|e| format!("Salt inválido: {e}"))?;
    let kdf_params = KdfParams {
        memory_kib: params_json
            .get("kdf_memory_kib")
            .and_then(|v| v.as_u64())
            .unwrap_or(65536) as u32,
        iterations: params_json
            .get("kdf_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32,
        parallelism: params_json
            .get("kdf_parallelism")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as u32,
    };

    let (auth_key, _) = derive_keys(&password, &salt, &kdf_params)
        .map_err(|e| format!("Error derivando clave de autenticación: {e}"))?;
    let auth_key_hex = hex::encode(auth_key.0);
    let device_id = current_device_id();

    let login_resp = http_client()
        .post(&format!("{clean_url}/api/auth/login"))
        .json(&serde_json::json!({
            "username": clean_user,
            "auth_key_hex": auth_key_hex,
            "device_id": device_id,
            "device_name": device_id,
        }))
        .send()
        .await
        .map_err(|e| format!("Error conectando al servidor: {e}"))?;

    if !login_resp.status().is_success() {
        let body: serde_json::Value = login_resp.json().await.unwrap_or_default();
        let msg = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Credenciales inválidas")
            .to_string();
        return Ok(UnlockResult {
            success: false,
            error: Some(msg),
            accounts_count: 0,
        });
    }

    let login_json: serde_json::Value = login_resp
        .json()
        .await
        .map_err(|e| format!("Error leyendo respuesta de login: {e}"))?;
    let display_name = login_json
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&clean_user)
        .to_string();
    let encrypted_payload = login_json
        .get("encrypted_payload")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "La respuesta no contiene la bóveda cifrada".to_string())?;

    let enc_state: EncryptedVaultState = serde_json::from_str(encrypted_payload)
        .map_err(|e| format!("Bóveda descargada inválida: {e}"))?;
    let mut vault = Vault::load_locked(enc_state);
    vault
        .unlock_with_password(&password)
        .map_err(|e| format!("No se pudo descifrar la bóveda con esa clave maestra: {e}"))?;

    save_vault_state(&app, vault.state())?;
    sync_identity_map(&app, &vault)?;
    save_session_file(
        &app,
        &SessionFile {
            server_url: clean_url.clone(),
            username: clean_user.clone(),
            display_name,
            device_id,
        },
    )?;

    let count = vault.state().accounts.len();
    {
        let mut auth_guard = session_auth.auth.lock().unwrap();
        *auth_guard = Some(SessionAuth {
            server_url: clean_url,
            username: clean_user,
            auth_key_hex,
        });
    }
    {
        let mut guard = vault_state.vault.lock().unwrap();
        *guard = Some(vault);
    }

    Ok(UnlockResult {
        success: true,
        error: None,
        accounts_count: count,
    })
}

/// Cierre de sesión global en esta PC (D-12):
/// Sube bóveda pendiente (salvo force_without_sync), detiene poller y borra datos locales de sesión.
/// Nunca borra nada en el VPS ni toca logins de agy/Codex en el SO.
#[command]
pub async fn global_logout(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    session_auth: State<'_, SessionAuthState>,
    poller: State<'_, crate::commands::poller::PollerManager>,
    force_without_sync: bool,
) -> Result<(), String> {
    if !force_without_sync {
        crate::commands::security::sync_vault_to_vps(&app).await?;
    }

    crate::commands::poller::kill_poller_process(&poller);
    let _ = crate::commands::security::cancel_autolock_timer().await;

    {
        let mut guard = vault_state.vault.lock().unwrap();
        if let Some(vault) = &mut *guard {
            vault.lock();
        }
        *guard = None;
    }
    {
        let mut auth_guard = session_auth.auth.lock().unwrap();
        *auth_guard = None;
    }

    let id_map = identity_map_path(&app);
    let paths = [
        vault_path(&app),
        session_path(&app),
        id_map.clone(),
        id_map.with_extension("key"),
        app.path()
            .app_data_dir()
            .expect("No se pudo obtener AppData dir")
            .join("dpapi.cache"),
    ];
    for p in paths {
        if p.exists() {
            std::fs::remove_file(&p)
                .map_err(|e| format!("No se pudo eliminar {}: {e}", p.display()))?;
        }
    }

    Ok(())
}

/// Información de la sesión local para primer arranque vs arranques siguientes (Fase 19.2).
/// Desbloquea la bóveda usando la clave de recuperación y establece nueva contraseña.
#[command]
pub async fn recover_vault(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    recovery_key_str: String,
    new_password: String,
) -> Result<UnlockResult, String> {
    use ai_keyvault_core::recovery::RecoveryKey;

    if !vault_exists(&app) {
        return Ok(UnlockResult {
            success: false,
            error: Some("No existe ninguna bóveda en este dispositivo.".into()),
            accounts_count: 0,
        });
    }

    let state = load_vault_state(&app)?;
    let mut vault = Vault::load_locked(state);

    let recovery_key = RecoveryKey::from_formatted_string(&recovery_key_str)
        .map_err(|e| format!("Clave de recuperación inválida: {e}"))?;

    match vault.recover_with_key(&recovery_key, &new_password) {
        Ok(()) => {
            // Guardar bóveda re-cifrada con nueva contraseña
            save_vault_state(&app, vault.state())?;
            sync_identity_map(&app, &vault)?;
            let count = vault.state().accounts.len();
            {
                let mut guard = vault_state.vault.lock().unwrap();
                *guard = Some(vault);
            }
            let _ = crate::commands::security::sync_vault_to_vps(&app).await;
            Ok(UnlockResult {
                success: true,
                error: None,
                accounts_count: count,
            })
        }
        Err(e) => Ok(UnlockResult {
            success: false,
            error: Some(format!("Recuperación fallida: {e}")),
            accounts_count: 0,
        }),
    }
}

// ── Credenciales y limpieza de portapapeles (Fase 20, D-07) ─────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub id: String,
    pub alias: String,
    pub email: String,
    pub password: Option<String>,
    pub notes: Option<String>,
    pub details: Option<String>,
    pub subscription_ends_at: Option<String>,
}

/// Descifra credenciales, notas y detalles de una cuenta (solo si bóveda desbloqueada).
#[command]
pub async fn get_account_credentials(
    vault_state: State<'_, VaultState>,
    account_uuid: String,
) -> Result<AccountCredentials, String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let guard = vault_state.vault.lock().unwrap();
    match &*guard {
        Some(vault) if vault.is_unlocked() => {
            let acc_ref = vault
                .state()
                .accounts
                .iter()
                .find(|a| a.id == uid)
                .ok_or_else(|| "Cuenta no encontrada en la bóveda".to_string())?;
            let alias = acc_ref.alias.clone();
            let subscription_ends_at = acc_ref.subscription_ends_at.clone();
            let email = vault
                .decrypt_account_email(uid)
                .map_err(|e| e.to_string())?;
            let password = vault
                .decrypt_account_password(uid)
                .map_err(|e| e.to_string())?;
            let notes = vault
                .decrypt_account_notes(uid)
                .map_err(|e| e.to_string())?;
            let details = vault
                .decrypt_account_details(uid)
                .map_err(|e| e.to_string())?;
            Ok(AccountCredentials {
                id: account_uuid,
                alias,
                email,
                password,
                notes,
                details,
                subscription_ends_at,
            })
        }
        _ => Err("Bóveda bloqueada o no cargada".into()),
    }
}

/// Actualiza alias, correo, contraseña, notas y detalles cifrados de una cuenta (Fase 20).
#[command]
pub async fn update_vault_account(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    account_uuid: String,
    alias: String,
    email: String,
    password: Option<String>,
    notes: Option<String>,
    details: Option<String>,
    subscription_ends_at: Option<String>,
) -> Result<(), String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let clean_alias = alias.trim().to_string();
    let clean_email = email.trim().to_string();
    if clean_alias.is_empty() || clean_email.is_empty() {
        return Err("El alias y el correo son obligatorios.".into());
    }

    {
        let mut guard = vault_state.vault.lock().unwrap();
        match &mut *guard {
            Some(vault) if vault.is_unlocked() => {
                let target_provider = vault
                    .state()
                    .accounts
                    .iter()
                    .find(|a| a.id == uid)
                    .map(|a| a.provider.clone())
                    .unwrap_or_default();
                let duplicate = vault
                    .state()
                    .accounts
                    .iter()
                    .any(|a| a.id != uid && a.provider == target_provider && a.alias == clean_alias);
                if duplicate {
                    return Err(format!(
                        "Ya existe otra cuenta de este proveedor con el alias '{clean_alias}'."
                    ));
                }
                vault
                    .update_account(
                        uid,
                        &clean_alias,
                        &clean_email,
                        password.as_deref(),
                        notes.as_deref(),
                        details.as_deref(),
                        subscription_ends_at.as_deref(),
                    )
                    .map_err(|e| format!("Error actualizando cuenta: {e}"))?;
                save_vault_state(&app, vault.state())?;
                sync_identity_map(&app, vault)?;
            }
            _ => return Err("La bóveda está bloqueada".into()),
        }
    }

    crate::commands::security::sync_vault_to_vps(&app).await?;
    Ok(())
}

static CLIPBOARD_CLEAR_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Programa la limpieza automática del portapapeles de Windows tras `seconds` segundos (D-07).
#[command]
pub async fn schedule_clipboard_clear(seconds: u64) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    let secs = seconds.max(1).min(300);
    let my_gen = CLIPBOARD_CLEAR_GEN.fetch_add(1, Ordering::SeqCst) + 1;

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;
        if CLIPBOARD_CLEAR_GEN.load(Ordering::SeqCst) == my_gen {
            #[cfg(target_os = "windows")]
            {
                use windows_sys::Win32::System::DataExchange::{
                    CloseClipboard, EmptyClipboard, OpenClipboard,
                };
                unsafe {
                    if OpenClipboard(0) != 0 {
                        let _ = EmptyClipboard();
                        let _ = CloseClipboard();
                    }
                }
            }
        }
    });

    Ok(())
}

// ── Credenciales y limpieza de portapapeles (Fase 20, D-07) ─────────────────
#[derive(Debug, Serialize, Deserialize)]
pub struct AddAccountResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// Agrega una cuenta a la bóveda local cifrada y sincroniza al VPS.
#[command]
pub async fn add_vault_account(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    provider: String,
    alias: String,
    email: String,
    password: Option<String>,
    tokens_json: Option<String>,
    notes: Option<String>,
) -> Result<AddAccountResult, String> {
    let res: AddAccountResult = {
        let mut guard = vault_state.vault.lock().unwrap();
        match &mut *guard {
            Some(vault) if vault.is_unlocked() => {
                match vault.add_account(
                    &provider,
                    &alias,
                    &email,
                    password.as_deref(),
                    tokens_json.as_deref(),
                    notes.as_deref(),
                ) {
                    Ok(uuid) => {
                        save_vault_state(&app, vault.state())?;
                        sync_identity_map(&app, vault)?;
                        Ok(AddAccountResult {
                            success: true,
                            id: Some(uuid.to_string()),
                            error: None,
                        })
                    }
                    Err(e) => Ok(AddAccountResult {
                        success: false,
                        id: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
            Some(_) => Err("La bóveda está bloqueada".to_string()),
            None => Err("No hay bóveda cargada".to_string()),
        }
    }?;

    if res.success {
        let _ = crate::commands::security::sync_vault_to_vps(&app).await;
    }
    Ok(res)
}

/// Elimina una cuenta de la bóveda por su UUID y sincroniza al VPS.
#[command]
pub async fn delete_vault_account(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    account_uuid: String,
) -> Result<bool, String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let removed: bool = {
        let mut guard = vault_state.vault.lock().unwrap();
        match &mut *guard {
            Some(vault) if vault.is_unlocked() => {
                let before = vault.state().accounts.len();
                vault.state_mut().accounts.retain(|a| a.id != uid);
                vault.state_mut().updated_at = chrono::Utc::now();
                let removed = vault.state().accounts.len() < before;
                if removed {
                    save_vault_state(&app, vault.state())?;
                    sync_identity_map(&app, vault)?;
                }
                Ok(removed)
            }
            Some(_) => Err("La bóveda está bloqueada".to_string()),
            None => Err("No hay bóveda cargada".to_string()),
        }
    }?;

    if removed {
        let _ = crate::commands::security::sync_vault_to_vps(&app).await;
    }
    Ok(removed)
}

/// Descifra y devuelve el email de una cuenta (para mostrar en UI).
#[command]
pub async fn get_account_email(
    vault_state: State<'_, VaultState>,
    account_uuid: String,
) -> Result<String, String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let guard = vault_state.vault.lock().unwrap();
    match &*guard {
        Some(vault) if vault.is_unlocked() => {
            vault.decrypt_account_email(uid).map_err(|e| e.to_string())
        }
        _ => Err("Bóveda bloqueada o no cargada".into()),
    }
}

/// Devuelve la lista de cuentas descifradas (alias + proveedor, SIN credenciales).
/// Solo funciona si la bóveda está desbloqueada.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub provider: String,
    pub alias: String,
    pub email: Option<String>,
    pub subscription_ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[command]
pub async fn list_vault_accounts(
    vault_state: State<'_, VaultState>,
) -> Result<Vec<AccountInfo>, String> {
    let guard = vault_state.vault.lock().unwrap();
    match &*guard {
        Some(vault) if vault.is_unlocked() => {
            let accounts = vault
                .state()
                .accounts
                .iter()
                .map(|a| AccountInfo {
                    id: a.id.to_string(),
                    provider: a.provider.clone(),
                    alias: a.alias.clone(),
                    email: vault.decrypt_account_email(a.id).ok(),
                    subscription_ends_at: a.subscription_ends_at.clone(),
                    created_at: a.created_at.to_rfc3339(),
                    updated_at: a.updated_at.to_rfc3339(),
                })
                .collect();
            Ok(accounts)
        }
        Some(_) => Err("La bóveda está bloqueada".into()),
        None => Err("No hay bóveda cargada".into()),
    }
}

/// Descifra y devuelve el JSON de tokens de una cuenta (para hacer switch real).
/// SOLO para uso interno del switch — nunca exponer en la UI directamente.
#[command]
pub async fn decrypt_account_tokens(
    vault_state: State<'_, VaultState>,
    account_uuid: String,
) -> Result<Option<String>, String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let guard = vault_state.vault.lock().unwrap();
    match &*guard {
        Some(vault) if vault.is_unlocked() => {
            vault.decrypt_account_tokens(uid).map_err(|e| e.to_string())
        }
        _ => Err("Bóveda bloqueada o no cargada".into()),
    }
}

static OAUTH_CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Cancela la espera del login OAuth en curso.
#[command]
pub async fn cancel_agy_oauth() -> Result<(), String> {
    OAUTH_CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Busca el ejecutable de `agy` en PATH o en `%LOCALAPPDATA%\agy\bin\agy.exe`.
fn find_agy_binary() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let candidate = std::path::PathBuf::from(local_app)
                .join("agy")
                .join("bin")
                .join("agy.exe");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    Some("agy".to_string())
}

/// Inicia el login OAuth de Antigravity (`agy`):
///   1. Respalda y limpia temporalmente `gemini:antigravity` para forzar un login nuevo.
///   2. Abre `agy` en una ventana de consola visible (`cmd /c start`) para que dispare
///      el login de Google en el navegador.
///   3. Hace polling cada 1.5 s (hasta 120 s o hasta que el usuario cancele).
///   4. Al detectar la nueva credencial, la cifra en la bóveda y sincroniza al VPS.
///   5. Si se cancela o expira, restaura la credencial previa.
#[command]
pub async fn add_agy_account_oauth(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    alias: String,
) -> Result<AddAccountResult, String> {
    use crate::commands::switch_cmd::{
        backup_and_clear_agy_credential, read_agy_credential, restore_agy_credential,
    };
    use std::sync::atomic::Ordering;
    use tokio::time::{sleep, Duration};

    OAUTH_CANCELLED.store(false, Ordering::Relaxed);

    let agy_bin =
        find_agy_binary().ok_or_else(|| "No se encontró agy.exe en el sistema.".to_string())?;

    // 1. Respaldar credencial actual y limpiar Credential Manager para forzar login OAuth
    let backup = backup_and_clear_agy_credential();

    // 2. Lanzar agy en una ventana visible para que dispare el flujo OAuth en el navegador
    #[cfg(target_os = "windows")]
    let spawn_res = std::process::Command::new("cmd")
        .args(["/c", "start", "Login Antigravity (AI KeyVault)", &agy_bin])
        .spawn();
    #[cfg(not(target_os = "windows"))]
    let spawn_res = std::process::Command::new(&agy_bin).spawn();

    if let Err(e) = spawn_res {
        if let Some(blob) = backup {
            restore_agy_credential(&blob);
        }
        return Ok(AddAccountResult {
            success: false,
            id: None,
            error: Some(format!("No se pudo abrir agy ({agy_bin}): {e}")),
        });
    }

    // 3. Poll hasta 120 s esperando la nueva credencial o cancelación
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let new_cred_json = loop {
        sleep(Duration::from_millis(1500)).await;

        if OAUTH_CANCELLED.load(Ordering::Relaxed) {
            if let Some(blob) = backup {
                restore_agy_credential(&blob);
            }
            return Ok(AddAccountResult {
                success: false,
                id: None,
                error: Some("Login cancelado por el usuario.".into()),
            });
        }

        if std::time::Instant::now() > deadline {
            if let Some(blob) = backup {
                restore_agy_credential(&blob);
            }
            return Ok(AddAccountResult {
                success: false,
                id: None,
                error: Some("Tiempo de espera agotado (120 s). Completa el login en el navegador y vuelve a intentarlo.".into()),
            });
        }

        if let Some(json) = read_agy_credential() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                if val.get("id_token_email").and_then(|v| v.as_str()).is_some() {
                    break json;
                }
            }
        }
    };

    // 4. Extraer email del nuevo blob
    let new_email = {
        let val: serde_json::Value = serde_json::from_str(&new_cred_json)
            .map_err(|e| format!("Blob de credencial inválido: {e}"))?;
        val.get("id_token_email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown@agy")
            .to_string()
    };

    let alias_final = if alias.trim().is_empty() {
        new_email.clone()
    } else {
        alias.trim().to_string()
    };

    // 5. Guardar en bóveda
    let res: AddAccountResult = {
        let mut guard = vault_state.vault.lock().unwrap();
        match &mut *guard {
            Some(vault) if vault.is_unlocked() => {
                let already_exists = vault
                    .state()
                    .accounts
                    .iter()
                    .any(|a| a.provider == "agy" && a.alias == alias_final);
                if already_exists {
                    Ok(AddAccountResult {
                        success: false,
                        id: None,
                        error: Some(format!(
                            "Ya existe una cuenta de Antigravity con el alias '{alias_final}'. Usa otro alias."
                        )),
                    })
                } else {
                    match vault.add_account(
                        "agy",
                        &alias_final,
                        &new_email,
                        None,
                        Some(&new_cred_json),
                        None,
                    ) {
                        Ok(uuid) => {
                            save_vault_state(&app, vault.state())?;
                            sync_identity_map(&app, vault)?;
                            Ok(AddAccountResult {
                                success: true,
                                id: Some(uuid.to_string()),
                                error: None,
                            })
                        }
                        Err(e) => Ok(AddAccountResult {
                            success: false,
                            id: None,
                            error: Some(e.to_string()),
                        }),
                    }
                }
            }
            Some(_) => Err("La bóveda está bloqueada".to_string()),
            None => Err("No hay bóveda cargada".to_string()),
        }
    }?;

    // 6. Sincronizar con el VPS (best-effort)
    if res.success {
        let _ = crate::commands::security::sync_vault_to_vps(&app).await;
    }

    Ok(res)
}

/// Importa la cuenta Codex actualmente activa en ~/.codex/auth.json
/// y la guarda cifrada en la bóveda.
///
/// FLUJO:
///   1. Lee ~/.codex/auth.json (escrito por Codex al hacer login)
///   2. Extrae el email/account_id del JSON
///   3. Cifra el JSON completo en la bóveda como encrypted_tokens
///   4. Sincroniza al VPS (best-effort)
#[command]
pub async fn import_current_codex_credential(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    alias: String,
) -> Result<AddAccountResult, String> {
    use crate::commands::switch_cmd::codex_auth_path_pub;

    let auth_path = codex_auth_path_pub();

    if !auth_path.exists() {
        return Ok(AddAccountResult {
            success: false,
            id: None,
            error: Some(
                "No hay ninguna cuenta Codex activa (~/.codex/auth.json no encontrado). \
                 Inicia sesión en Codex primero y vuelve a intentarlo."
                    .into(),
            ),
        });
    }

    let cred_json = std::fs::read_to_string(&auth_path)
        .map_err(|e| format!("No se pudo leer auth.json: {e}"))?;

    // Validar que es JSON válido
    let val: serde_json::Value = serde_json::from_str(&cred_json)
        .map_err(|e| format!("auth.json no es JSON válido: {e}"))?;

    // Extraer email/account_id (prioridad: JWT tokens.id_token igual que providers/codex)
    let email = val
        .pointer("/tokens/id_token")
        .and_then(|v| v.as_str())
        .and_then(decode_jwt_email)
        .or_else(|| {
            val.pointer("/user/email")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .or_else(|| {
            val.get("email")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .or_else(|| {
            val.get("account_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown@codex".to_string());

    let alias_final = if alias.trim().is_empty() {
        email.clone()
    } else {
        alias.trim().to_string()
    };

    let res: AddAccountResult = {
        let mut guard = vault_state.vault.lock().unwrap();
        match &mut *guard {
            Some(vault) if vault.is_unlocked() => {
                // Verificar que no exista ya un alias igual para Codex
                let already_exists = vault
                    .state()
                    .accounts
                    .iter()
                    .any(|a| a.provider == "codex" && a.alias == alias_final);

                if already_exists {
                    Ok(AddAccountResult {
                        success: false,
                        id: None,
                        error: Some(format!(
                            "Ya existe una cuenta de Codex con el alias '{}' en la bóveda. \
                             Usa otro alias o elimina la existente.",
                            alias_final
                        )),
                    })
                } else {
                    match vault.add_account(
                        "codex",
                        &alias_final,
                        &email,
                        None,
                        Some(&cred_json), // JSON completo cifrado como tokens
                        None,
                    ) {
                        Ok(uuid) => {
                            save_vault_state(&app, vault.state())?;
                            sync_identity_map(&app, vault)?;
                            Ok(AddAccountResult {
                                success: true,
                                id: Some(uuid.to_string()),
                                error: None,
                            })
                        }
                        Err(e) => Ok(AddAccountResult {
                            success: false,
                            id: None,
                            error: Some(e.to_string()),
                        }),
                    }
                }
            }
            Some(_) => Err("La bóveda está bloqueada".to_string()),
            None => Err("No hay bóveda cargada".to_string()),
        }
    }?;

    // Sincronizar con el VPS (best-effort)
    if res.success {
        let _ = crate::commands::security::sync_vault_to_vps(&app).await;
    }

    Ok(res)
}

/// Decodifica el payload base64url del JWT (sin verificar firma) y extrae el campo `email`.
fn decode_jwt_email(jwt: &str) -> Option<String> {
    let payload_b64 = jwt.split('.').nth(1)?;
    let standard = payload_b64.replace('-', "+").replace('_', "/");
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lut[c as usize] = i as u8;
    }
    let input = standard.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &c in input {
        let v = lut[c as usize];
        if v == 255 {
            continue;
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    let payload: serde_json::Value = serde_json::from_slice(&out).ok()?;
    payload
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}
