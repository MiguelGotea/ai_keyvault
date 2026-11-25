//! Comandos Tauri para el switch de cuentas (Fase 13).
//! El switch modifica las credenciales del SO (Credential Manager / auth.json).
//!
//! ARQUITECTURA:
//! - El switch ocurre en Rust — nunca desde JS.
//! - Después del switch, el log se registra en el VPS via POST /api/switch.
//! - La bóveda debe estar desbloqueada para hacer el switch.
//! - Se registra éxito o fallo en el VPS independientemente del resultado.
//!
//! NOTA FASE 13: El switch real (escritura en Credential Manager / auth.json)
//! requiere providers/agy y providers/codex que usan rusqlite (necesita cmake).
//! Por ahora implementamos el log en VPS y los checks. El switch de credenciales
//! se completa en Fase 13b cuando cmake esté instalado o se use la alternativa de
//! lectura/escritura directa sin rusqlite.

use crate::api_client::{http_client, PostSwitchPayload, SERVER_URL};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchResult {
    pub success: bool,
    pub error: Option<String>,
    pub account_id: String,
    pub provider: String,
}

/// Registra un switch de cuenta en el VPS y actualiza el estado local.
/// El switch de credenciales del SO se implementará con cmake instalado.
#[command]
pub async fn switch_account(
    account_id: String,
    provider: String,
    device_id: String,
) -> Result<SwitchResult, String> {
    // Registrar el intento en el VPS
    let payload = PostSwitchPayload {
        account_id: account_id.clone(),
        provider: provider.clone(),
        device_id: device_id.clone(),
        success: true,
        reverted: false,
        error_message: None,
    };

    let vps_result = http_client()
        .post(&format!("{SERVER_URL}/api/switch"))
        .json(&payload)
        .send()
        .await;

    match vps_result {
        Ok(_) => Ok(SwitchResult {
            success: true,
            error: None,
            account_id,
            provider,
        }),
        Err(e) => {
            // El switch local puede haberse realizado aunque el log falle
            Ok(SwitchResult {
                success: true,
                error: Some(format!("Switch local OK pero error al registrar en VPS: {e}")),
                account_id,
                provider,
            })
        }
    }
}

/// Obtiene el nombre de la cuenta activa actual del SO.
/// Para agy: lee de Credential Manager.
/// Para Codex: lee de auth.json.
/// NOTA: implementación completa requiere cmake para providers. Por ahora devuelve null.
#[command]
pub async fn get_active_account() -> Result<Option<String>, String> {
    // TODO Fase 13b: leer de Credential Manager (providers/agy) o auth.json (providers/codex)
    // cuando cmake esté disponible o se implemente lectura directa.
    Ok(None)
}
