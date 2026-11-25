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
    pub initialized: bool,  // ¿existe vault.json en disco?
    pub locked: bool,       // ¿tiene VaultKey en memoria?
    pub version: String,    // versión del binario
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
    pub recovery_key: Option<String>,  // AKVR-XXXX-... mostrar una sola vez
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
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer vault.json: {e}"))?;
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
            error: Some("Ya existe una bóveda en este dispositivo. Para resetear, elimina vault.json.".into()),
        });
    }

    match Vault::create_new(&password, None) {
        Ok((vault, recovery)) => {
            // Persistir el estado cifrado en disco
            save_vault_state(&app, vault.state())?;

            // Guardar la bóveda desbloqueada en memoria
            let mut guard = vault_state.vault.lock().unwrap();
            *guard = Some(vault);

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

/// Agrega una cuenta a la bóveda local cifrada.
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
                    Ok(AddAccountResult { success: true, id: Some(uuid.to_string()), error: None })
                }
                Err(e) => Ok(AddAccountResult { success: false, id: None, error: Some(e.to_string()) }),
            }
        }
        Some(_) => Err("La bóveda está bloqueada".into()),
        None => Err("No hay bóveda cargada".into()),
    }
}

/// Elimina una cuenta de la bóveda por su UUID.
#[command]
pub async fn delete_vault_account(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    account_uuid: String,
) -> Result<bool, String> {
    use uuid::Uuid;
    let uid = Uuid::parse_str(&account_uuid).map_err(|_| "UUID inválido".to_string())?;
    let mut guard = vault_state.vault.lock().unwrap();
    match &mut *guard {
        Some(vault) if vault.is_unlocked() => {
            let before = vault.state().accounts.len();
            vault.state_mut().accounts.retain(|a| a.id != uid);
            vault.state_mut().updated_at = chrono::Utc::now();
            let removed = vault.state().accounts.len() < before;
            if removed {
                save_vault_state(&app, vault.state())?;
            }
            Ok(removed)
        }
        Some(_) => Err("La bóveda está bloqueada".into()),
        None => Err("No hay bóveda cargada".into()),
    }
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
        Some(vault) if vault.is_unlocked() => vault
            .decrypt_account_email(uid)
            .map_err(|e| e.to_string()),
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

/// Lanza `agy login` para abrir el flujo OAuth en el navegador.
/// Espera hasta 120 segundos a que aparezca una credencial NUEVA en el
/// Credential Manager (distinta al email previo). Luego la cifra en la bóveda.
///
/// Flujo:
///   1. Lee la credencial actual (si existe) para detectar el cambio.
///   2. Lanza `agy login` que abre el navegador de Google OAuth.
///   3. Poll cada 2 s hasta que `gemini:antigravity` tenga una credencial
///      con un `id_token_email` distinto (o nueva si no había ninguna antes).
///   4. Guarda el blob completo cifrado en la bóveda + sincroniza al VPS.
#[command]
pub async fn add_agy_account_oauth(
    app: AppHandle,
    vault_state: State<'_, VaultState>,
    alias: String,
) -> Result<AddAccountResult, String> {
    use crate::commands::switch_cmd::read_agy_credential;
    use tauri_plugin_shell::ShellExt;
    use tokio::time::{sleep, Duration};

    // 1. Capturar email previo (para detectar cambio)
    let prev_email: Option<String> = read_agy_credential().and_then(|json| {
        serde_json::from_str::<serde_json::Value>(&json).ok()?
            .get("id_token_email")
            .or_else(|| {
                // re-parse needed
                None
            })
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    });

    // 2. Lanzar `agy login` en background (no bloqueante)
    app.shell()
        .command("agy")
        .args(["login"])
        .spawn()
        .map_err(|e| format!("No se pudo lanzar 'agy login': {e}. Asegúrate de que agy esté instalado."))?;

    // 3. Poll hasta 120 s esperando credencial nueva
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let new_cred_json = loop {
        sleep(Duration::from_secs(2)).await;
        if std::time::Instant::now() > deadline {
            return Ok(AddAccountResult {
                success: false,
                id: None,
                error: Some("Tiempo de espera agotado (120 s). Completa el login en el navegador y vuelve a intentarlo.".into()),
            });
        }
        if let Some(json) = read_agy_credential() {
            // Parsear email actual
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                let cur_email = val.get("id_token_email")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                // Si no había credencial antes → cualquier nueva vale
                // Si había una → esperamos que cambie el email
                let is_new = match (&prev_email, &cur_email) {
                    (None, Some(_)) => true,
                    (Some(prev), Some(cur)) => prev != cur,
                    _ => false,
                };
                if is_new {
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

    let alias_final = if alias.trim().is_empty() { new_email.clone() } else { alias.trim().to_string() };

    // 5. Guardar en bóveda
    let mut guard = vault_state.vault.lock().unwrap();
    match &mut *guard {
        Some(vault) if vault.is_unlocked() => {
            let already_exists = vault.state().accounts.iter().any(|a| a.alias == alias_final);
            if already_exists {
                return Ok(AddAccountResult {
                    success: false,
                    id: None,
                    error: Some(format!("Ya existe una cuenta con el alias '{alias_final}'. Usa otro alias.")),
                });
            }
            match vault.add_account("agy", &alias_final, &new_email, None, Some(&new_cred_json), None) {
                Ok(uuid) => {
                    save_vault_state(&app, vault.state())?;
                    Ok(AddAccountResult { success: true, id: Some(uuid.to_string()), error: None })
                }
                Err(e) => Ok(AddAccountResult { success: false, id: None, error: Some(e.to_string()) }),
            }
        }
        Some(_) => Err("La bóveda está bloqueada".into()),
        None => Err("No hay bóveda cargada".into()),
    }
}
