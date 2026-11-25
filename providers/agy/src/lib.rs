//! Adaptador agy (Antigravity) para AI KeyVault.
//!
//! Responsabilidades:
//! - Leer / escribir credenciales desde Windows Credential Manager (`gemini:antigravity`).
//! - Verificar que el token sea válido ejecutando `agy models` (sin consumir cuota).
//! - Parsear la salida de `agy /usage` para leer cuota de los dos pools.
//! - Revertir un cambio de cuenta si la verificación falla.
//!
//! **Nota de seguridad:** este módulo nunca escribe tokens en disco ni en logs.
//! Las claves se mantienen en memoria y se pasan como referencias de corta duración.

pub mod chats;
pub mod credential;
pub mod quota;
pub mod switcher;

// Re-exportaciones públicas
pub use chats::{brain_dir, scan_chats, AgyChat};
pub use credential::{AgyCredential, AgyCredentialManager};
pub use quota::{parse_usage_output, AgyPool, AgyQuotaReading};
pub use switcher::{AccountSwitcher, SwitchResult};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgyError {
    #[error("Credencial no encontrada en Credential Manager: {0}")]
    CredentialNotFound(String),

    #[error("Error al leer Credential Manager: {0}")]
    CredentialRead(String),

    #[error("Error al escribir Credential Manager: {0}")]
    CredentialWrite(String),

    #[error("El blob de Credential Manager no es JSON válido: {0}")]
    InvalidBlob(String),

    #[error("Token vencido (expiry: {0})")]
    TokenExpired(String),

    #[error("Verificación fallida — agy models devolvió error: {0}")]
    VerificationFailed(String),

    #[error("Error al ejecutar comando: {0}")]
    CommandError(String),

    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de serialización: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AgyError>;
