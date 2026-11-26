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

use ai_keyvault_core::vault::{EncryptedVaultState, Vault};
use chrono;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{command, AppHandle, Manager, State};

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
pub async fn lock_vault(vault_state: State<'_, VaultState>) -> Result<(), String> {
    let mut guard = vault_state.vault.lock().unwrap();
    if let Some(vault) = &mut *guard {
        vault.lock();
    }
    Ok(())
}

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

// ── Nuevos comandos: agregar / eliminar / leer cuentas de la bóveda ─────────

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
                    .any(|a| a.alias == alias_final);
                if already_exists {
                    Ok(AddAccountResult {
                        success: false,
                        id: None,
                        error: Some(format!(
                            "Ya existe una cuenta con el alias '{alias_final}'. Usa otro alias."
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
                // Verificar que no exista ya un alias igual
                let already_exists = vault
                    .state()
                    .accounts
                    .iter()
                    .any(|a| a.alias == alias_final);

                if already_exists {
                    Ok(AddAccountResult {
                        success: false,
                        id: None,
                        error: Some(format!(
                            "Ya existe una cuenta con el alias '{}' en la bóveda. \
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
