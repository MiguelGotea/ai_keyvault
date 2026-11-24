//! Cambio de cuenta de agy con soporte de deshacer.
//!
//! **Flujo de cambio:**
//! 1. Leer la credencial actual de Credential Manager → guardar en bóveda como "cuenta anterior".
//! 2. Escribir la credencial de la cuenta destino en Credential Manager.
//! 3. Verificar con `agy models` que el nuevo token funciona.
//! 4. Si falla: revertir la credencial anterior → marcar la cuenta destino en cuarentena.
//! 5. Si OK: registrar el cambio en el historial.
//!
//! **Invariantes de seguridad:**
//! - El proceso de Codex/agy en curso NO es terminado. El cambio es de Credential Manager;
//!   el proceso activo sigue con su sesión hasta que lo cierre el usuario.
//! - La reversión es atómica: CredWrite sobreescribe la entrada existente.

use crate::{
    credential::{AgyCredential, AgyCredentialManager},
    quota::verify_token,
    Result, AgyError,
};

/// Resultado de un intento de cambio de cuenta.
#[derive(Debug)]
pub enum SwitchResult {
    /// Cambio exitoso. El campo contiene los modelos que `agy models` listó.
    Success { models: Vec<String> },
    /// Cambio fallido y revertido. El campo describe el error.
    Reverted { reason: String },
    /// Fallo al revertir — estado inconsistente.
    RevertFailed { original_error: String, revert_error: String },
}

/// Orquestador del cambio de cuenta.
pub struct AccountSwitcher;

impl AccountSwitcher {
    /// Cambia la cuenta activa de agy.
    ///
    /// `new_credential`: credencial de la cuenta destino (leída de la bóveda).
    ///
    /// Devuelve `SwitchResult` en lugar de `Result` para que el llamador pueda
    /// tomar decisiones sin propagar errores de lógica de negocio como errores Rust.
    pub fn switch(new_credential: &AgyCredential) -> SwitchResult {
        // ── Paso 1: leer credencial actual para poder revertir ──────────────
        let previous = match AgyCredentialManager::read() {
            Ok(c) => Some(c),
            Err(_) => None, // si no hay credencial previa, no podemos revertir pero continuamos
        };

        // ── Paso 2: escribir nueva credencial ───────────────────────────────
        if let Err(e) = AgyCredentialManager::write(new_credential) {
            return SwitchResult::Reverted {
                reason: format!("No se pudo escribir en Credential Manager: {}", e),
            };
        }

        // ── Paso 3: verificar con `agy models` ─────────────────────────────
        match verify_token() {
            Ok(models) => SwitchResult::Success { models },
            Err(e) => {
                // ── Paso 4: revertir ────────────────────────────────────────
                let original_error = e.to_string();
                match previous {
                    Some(prev) => match AgyCredentialManager::write(&prev) {
                        Ok(()) => SwitchResult::Reverted {
                            reason: original_error,
                        },
                        Err(re) => SwitchResult::RevertFailed {
                            original_error,
                            revert_error: re.to_string(),
                        },
                    },
                    None => SwitchResult::Reverted {
                        reason: format!(
                            "{} (sin credencial previa para revertir — Credential Manager vacío)",
                            original_error
                        ),
                    },
                }
            }
        }
    }

    /// Solo verifica si la credencial activa en Credential Manager es válida.
    /// No hace ningún cambio.
    pub fn verify_current() -> Result<Vec<String>> {
        let cred = AgyCredentialManager::read()?;
        if !cred.is_valid() {
            return Err(AgyError::TokenExpired(
                cred.expiry
                    .map(|e| e.to_rfc3339())
                    .unwrap_or_else(|| "desconocido".to_string()),
            ));
        }
        verify_token()
    }
}
