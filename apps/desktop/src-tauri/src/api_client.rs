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

/// Timeout estándar para peticiones al VPS (15 segundos).
const TIMEOUT_SECS: u64 = 15;

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
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub tokens_used: i64,
    pub tokens_limit: i64,
    pub percent_used: f64,
    pub reset_time: Option<String>,
    pub source: String,
    pub read_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolSummary {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    /// Porcentaje de uso actual (0-100). Nombre exacto del VPS: current_percent_used.
    pub current_percent_used: f64,
    /// Tokens consumidos (bruto).
    pub tokens_used: i64,
    /// Límite de tokens del pool.
    pub tokens_limit: i64,
    /// Tasa de quema en %/hora calculada sobre deltas positivos recientes.
    pub burn_rate_pct_per_hour: f64,
    /// Tiempo estimado hasta llegar al 100% (horas). Calculado en el desktop.
    pub estimated_hours_to_100: Option<f64>,
    /// Momento estimado de agotamiento (calculado en el desktop).
    pub estimated_exhaustion_at: Option<String>,
    pub reset_time: Option<String>,
    pub last_read_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeatmapCell {
    pub weekday: u8,          // 0=lunes … 6=domingo
    pub weekday_name: String, // "Lun", "Mar", ...
    pub hour: u8,             // 0-23 UTC
    pub delta_percent: f64,
    pub samples: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    #[serde(default)]
    pub account_id: String,
    pub provider: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub archived: bool,
}

/// Deserializa booleans que el VPS puede enviar como `true/false` o `0/1` (SQLite).
fn de_bool_or_int<'de, D>(d: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct BoolOrInt;
    impl<'de> Visitor<'de> for BoolOrInt {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "bool or integer 0/1")
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> Result<bool, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<bool, E> {
            Ok(v != 0)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<bool, E> {
            Ok(v != 0)
        }
    }
    d.deserialize_any(BoolOrInt)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertRecord {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub level: String, // "warn" | "crit" (minúscula en el VPS)
    pub percent_used: f64,
    pub reset_time: Option<String>,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub notified_toast: bool,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub notified_ntfy: bool,
    pub detected_at: String, // nombre real del VPS (no alerted_at)
}

// SwitchRecord movido a PostSwitchPayload — el historial se devuelve como serde_json::Value desde el VPS.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingRecord {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    #[serde(deserialize_with = "de_bool_or_int")]
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
    pub device_name: String,
    pub public_key_hex: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceItem {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub is_active: bool,
    pub created_at: String,
    pub last_seen_at: String,
}

/// El VPS devuelve un array de objetos con todos los campos del ping.
/// NO envuelve en { accounts_needing_ping: [...] }.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingNeedItem {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub percent_used: f64,
    pub needs_ping: bool,
    pub last_read_at: String,
    pub reset_time: Option<String>,
}

/// Payloads de peticiones POST — deben coincidir con los structs en server/src/routes.rs
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

/// El VPS espera `to_account_id`, `reason` — NO `account_id`/`reverted`/`error_message`.
#[derive(Debug, Serialize)]
pub struct PostSwitchPayload {
    pub to_account_id: String,
    pub from_account_id: Option<String>,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub reason: Option<String>,
}

/// El VPS devuelve `to_account_id`, `from_account_id`, `reason`, `switched_at`.
/// NO tiene `account_id`, `reverted`, ni `error_message`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwitchRecord {
    pub id: i64,
    pub to_account_id: String,
    pub from_account_id: Option<String>,
    pub provider: String,
    pub device_id: String,
    #[serde(deserialize_with = "de_bool_or_int")]
    pub success: bool,
    pub reason: Option<String>,
    pub switched_at: String,
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn deserializa_fixtures_contrato_vps() {
        let quota: Vec<QuotaReading> =
            serde_json::from_str(include_str!("../tests/fixtures/quota_readings.json"))
                .expect("quota_readings.json");
        assert_eq!(quota.len(), 1);
        assert_eq!(quota[0].tokens_limit, 1_000_000);

        let summary: Vec<PoolSummary> =
            serde_json::from_str(include_str!("../tests/fixtures/pool_summary.json"))
                .expect("pool_summary.json");
        assert_eq!(summary.len(), 1);

        let heatmap: Vec<HeatmapCell> =
            serde_json::from_str(include_str!("../tests/fixtures/heatmap.json"))
                .expect("heatmap.json");
        assert_eq!(heatmap[0].hour, 18);

        let chats: Vec<ChatSession> =
            serde_json::from_str(include_str!("../tests/fixtures/chats.json")).expect("chats.json");
        assert!(!chats[0].archived);
        assert!(chats[1].archived);

        let alerts: Vec<AlertRecord> =
            serde_json::from_str(include_str!("../tests/fixtures/alerts.json"))
                .expect("alerts.json");
        assert!(alerts[0].notified_toast);
        assert!(!alerts[0].notified_ntfy);
        assert!(alerts[1].notified_toast);

        let switches: Vec<SwitchRecord> =
            serde_json::from_str(include_str!("../tests/fixtures/switch_history.json"))
                .expect("switch_history.json");
        assert!(switches[0].success);

        let needs: Vec<PingNeedItem> =
            serde_json::from_str(include_str!("../tests/fixtures/pings_needs.json"))
                .expect("pings_needs.json");
        assert!(needs[0].needs_ping);

        let pings: Vec<PingRecord> =
            serde_json::from_str(include_str!("../tests/fixtures/pings.json")).expect("pings.json");
        assert!(pings[0].success);
        assert!(pings[0].is_manual);

        let devices: Vec<DeviceItem> =
            serde_json::from_str(include_str!("../tests/fixtures/devices.json"))
                .expect("devices.json");
        assert_eq!(devices.len(), 2);
        assert!(devices[0].is_active);
        assert!(!devices[1].is_active);
    }
}
