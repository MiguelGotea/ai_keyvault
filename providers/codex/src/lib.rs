//! Adaptador Codex (OpenAI) para AI KeyVault.
//!
//! Responsabilidades:
//! - Leer / escribir `~/.codex/auth.json` de forma atómica.
//! - Decodificar el `id_token` (JWT) para extraer email y vencimiento.
//! - Verificar que el token funciona con la API de OpenAI.
//! - Revertir un cambio de cuenta si la verificación falla.

pub mod auth;
pub mod switcher;

pub use auth::{CodexAuth, CodexAuthManager};
pub use switcher::{CodexSwitcher, CodexSwitchResult};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodexError {
    #[error("auth.json no encontrado en: {0}")]
    AuthFileNotFound(String),

    #[error("auth.json no es JSON válido: {0}")]
    InvalidJson(String),

    #[error("Token vencido (exp: {0})")]
    TokenExpired(String),

    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error de serialización: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Error al verificar token: {0}")]
    VerificationFailed(String),

    #[error("Error de escritura atómica: {0}")]
    AtomicWrite(String),
}

pub type Result<T> = std::result::Result<T, CodexError>;
