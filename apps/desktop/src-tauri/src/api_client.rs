//! Cliente HTTP centralizado para comunicarse con el backend VPS.
//! REGLA DE ARQUITECTURA: Todo el tráfico HTTP al VPS sale desde este módulo.
//! El frontend React NUNCA hace fetch() directo — siempre usa invoke() → comandos Tauri.
//!
//! URL base: http://198.211.97.243:3010 (sin TLS hasta conseguir subdominio)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;

/// URL base del servidor VPS. Cuando se obtenga el subdominio,
/// cambiar aquí y actualizar el Caddyfile.
pub const SERVER_URL: &str = "http://198.211.97.243:3010";

/// Timeout estándar para peticiones al VPS (5 segundos).
const TIMEOUT_SECS: u64 = 5;

/// Singleton del cliente HTTP — reutiliza conexiones TCP.
static CLIENT: OnceLock<Client> = OnceLock::new();

/// Devuelve el cliente HTTP compartido (lazy init).
pub fn http_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .user_agent("AI-KeyVault-Desktop/0.1.0")
            .build()
            .expect("No se pudo construir el cliente HTTP")
    })
}

// ── Tipos de respuesta del VPS ──────────────────────────────────────────────
// Deben estar sincronizados con las structs en server/src/routes.rs
// y con src/types/api.ts en el frontend.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuotaReading {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub percent_used: f64,
    pub percent_remaining: f64,
    pub reset_time: Option<String>,
    pub source: String,
    pub read_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolSummary {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub current_percent: f64,
    pub burn_rate_per_hour: Option<f64>,
    pub estimated_hours_to_100: Option<f64>,
    pub estimated_exhaustion_at: Option<String>,
    pub reset_time: Option<String>,
    pub last_read_at: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeatmapCell {
    pub weekday: u8,  // 0=lunes … 6=domingo
    pub hour_utc: u8, // 0-23
    pub total_delta: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub account_id: String,
    pub provider: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub device_id: String,
    pub archived: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertRecord {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub level: String, // "Warn" | "Crit"
    pub percent_used: f64,
    pub message: Option<String>,
    pub device_id: String,
    pub alerted_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwitchRecord {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub reverted: bool,
    pub error_message: Option<String>,
    pub switched_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingRecord {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub is_manual: bool,
    pub pinged_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceRegistration {
    pub device_id: String,
    pub public_key_hex: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingNeedsResponse {
    pub accounts_needing_ping: Vec<String>,
}

/// Payloads de peticiones POST
#[derive(Debug, Serialize)]
pub struct PostPingPayload {
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub is_manual: bool,
}

#[derive(Debug, Serialize)]
pub struct PostSwitchPayload {
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub reverted: bool,
    pub error_message: Option<String>,
}
