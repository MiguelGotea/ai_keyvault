//! Modelos de datos del poller.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuración del poller.
#[derive(Debug, Clone)]
pub struct PollerConfig {
    /// URL base del servidor VPS, ej. `http://198.211.97.243:3010`
    pub server_url: String,

    /// Identificador único de esta PC, ej. nombre de host.
    pub device_id: String,

    /// Intervalo de consulta en segundos (default: 600 = 10 min).
    pub poll_interval_secs: u64,

    /// Duración del lease en segundos (default: 600 = 10 min).
    pub lease_duration_secs: i64,
}

impl PollerConfig {
    pub fn new(server_url: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            device_id: device_id.into(),
            poll_interval_secs: 600,
            lease_duration_secs: 600,
        }
    }

    /// Sobrescribir el intervalo de consulta (útil para tests o configuración del usuario).
    pub fn with_interval(mut self, secs: u64) -> Self {
        self.poll_interval_secs = secs;
        self
    }
}

/// Una lectura de cuota de un pool específico en un momento dado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    /// ID de cuenta (email o identificador único de la cuenta del proveedor).
    pub account_id: String,

    /// Proveedor: "agy" o "codex".
    pub provider: String,

    /// Pool: "gemini", "claude_gpt", "default".
    pub pool: String,

    /// Tokens usados (aproximado — para agy se calcula como porcentaje × límite estimado).
    pub tokens_used: i64,

    /// Límite total de tokens (estimado si no es exacto).
    pub tokens_limit: i64,

    /// Porcentaje de cuota usado (0.0 a 100.0).
    pub percent_used: f64,

    /// Porcentaje de cuota restante (0.0 a 100.0).
    pub percent_remaining: f64,

    /// Cuándo se reinicia la cuota (RFC 3339), si se conoce.
    pub reset_time: Option<String>,

    /// PC que realizó la lectura (hostname o device_id).
    pub source: String,

    /// Cuándo se tomó la lectura.
    pub read_at: DateTime<Utc>,
}

/// Respuesta del servidor al adquirir un lease.
#[derive(Debug, Deserialize)]
pub struct LeaseResponse {
    pub account_id: String,
    pub acquired: bool,
    pub locked_by: String,
    pub locked_until: String,
}
