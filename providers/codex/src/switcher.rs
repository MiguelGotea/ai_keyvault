//! Cambio de cuenta de Codex con escritura atómica y soporte de deshacer.
//!
//! **Flujo completo:**
//! 1. Leer auth.json actual → actualizar en bóveda (tokens pueden haber rotado).
//! 2. Escribir auth.json de cuenta destino (.tmp → rename, guarda .bak).
//! 3. Verificar que el token es aceptado (el adaptador llama a la API).
//! 4. Si falla: revertir .bak → auth.json y marcar cuenta en cuarentena.
//! 5. Si OK: registrar en historial y avisar al usuario que reinicie Codex.

use crate::auth::{CodexAuth, CodexAuthManager};
use std::path::PathBuf;

/// Resultado de un cambio de cuenta.
#[derive(Debug)]
pub enum CodexSwitchResult {
    /// Cambio exitoso.
    Success,
    /// Cambio fallido y revertido correctamente.
    Reverted { reason: String },
    /// Fallo al revertir — estado inconsistente, requiere intervención manual.
    RevertFailed {
        original_error: String,
        revert_error: String,
    },
}

pub struct CodexSwitcher {
    manager: CodexAuthManager,
}

impl CodexSwitcher {
    pub fn new() -> Self {
        Self {
            manager: CodexAuthManager::new(),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self {
            manager: CodexAuthManager::with_path(path),
        }
    }

    /// Cambia la cuenta activa de Codex.
    ///
    /// `new_auth`: credencial de la cuenta destino (leída de la bóveda).
    /// `verify_fn`: función de verificación inyectable (facilita tests con `fake_codex`).
    pub fn switch<F>(&self, new_auth: &CodexAuth, verify_fn: F) -> CodexSwitchResult
    where
        F: Fn(&str) -> std::result::Result<(), String>,
    {
        // ── Paso 1: leer estado actual para hacer backup implícito ──────────
        // (write_atomic ya guarda .bak automáticamente)

        // ── Paso 2: escritura atómica ───────────────────────────────────────
        if let Err(e) = self.manager.write_atomic(new_auth) {
            return CodexSwitchResult::Reverted {
                reason: format!("Escritura atómica falló: {}", e),
            };
        }

        // ── Paso 3: verificar ───────────────────────────────────────────────
        match verify_fn(&new_auth.access_token) {
            Ok(()) => CodexSwitchResult::Success,
            Err(err) => {
                // ── Paso 4: revertir ────────────────────────────────────────
                match self.manager.revert() {
                    Ok(()) => CodexSwitchResult::Reverted { reason: err },
                    Err(re) => CodexSwitchResult::RevertFailed {
                        original_error: err,
                        revert_error: re.to_string(),
                    },
                }
            }
        }
    }
}

impl Default for CodexSwitcher {
    fn default() -> Self {
        Self::new()
    }
}
