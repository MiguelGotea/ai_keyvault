//! Poller de cuota para AI KeyVault.
//!
//! Consulta periódicamente la cuota de las cuentas de agy y Codex,
//! con lease anti-duplicado para que solo un PC consulte a la vez por cuenta.
//!
//! **Flujo por ciclo:**
//! ```text
//! scheduler tick (cada 10 min configurable)
//!   → lease.acquire(account_id, device_id)   — si falla, otro PC ya está consultando
//!   → collector.read_all()                   — llama a agy /usage y lee codex auth
//!   → uploader.send(readings)                — POST /api/quota/readings al servidor
//!   → (el lease expira automáticamente en 10 min si no se renueva)
//! ```

pub mod collector;
pub mod lease;
pub mod models;
pub mod scheduler;
pub mod uploader;

pub use models::{PollerConfig, QuotaSnapshot};
pub use scheduler::PollerHandle;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PollerError {
    #[error("Error al consultar cuota de agy: {0}")]
    AgyError(String),

    #[error("Error al consultar cuota de Codex: {0}")]
    CodexError(String),

    #[error("Error al adquirir lease: {0}")]
    LeaseError(String),

    #[error("Error al subir lectura al servidor: {0}")]
    UploadError(String),

    #[error("Error de red: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Error de serialización: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PollerError>;
