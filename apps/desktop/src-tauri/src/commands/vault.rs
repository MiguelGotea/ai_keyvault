//! Comandos de bóveda local.
//! En esta fase (Fase 10) solo expone el estado básico.
//! La lógica completa de desbloqueo va en Fase 12.

use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStatus {
    pub initialized: bool,
    pub locked: bool,
    pub version: String,
}

/// Devuelve el estado actual de la bóveda local.
/// Si no existe aún (primer uso), `initialized = false`.
#[command]
pub async fn get_vault_status() -> Result<VaultStatus, String> {
    // En Fase 10 devolvemos estado estático — la lógica real va en Fase 12.
    // Cuando se implemente: leer el vault state del disco y verificar si existe.
    Ok(VaultStatus {
        initialized: false,
        locked: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
