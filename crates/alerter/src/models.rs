//! Modelos del alerter: configuración, niveles y eventos de alerta.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Lectura de cuota simplificada que el poller pasa al alerter.
///
/// El alerter no depende del poller — el poller convierte su `QuotaSnapshot`
/// a este tipo antes de llamar a `AlertManager::check_and_notify`.
#[derive(Debug, Clone)]
pub struct AlertReading {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub percent_used: f64,
    pub reset_time: Option<String>,
}

/// Nivel de alerta según el porcentaje consumido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    /// 80–94% consumido — aviso preventivo.
    Warn,
    /// ≥ 95% consumido — alerta crítica.
    Critical,
}

impl AlertLevel {
    pub fn emoji(&self) -> &'static str {
        match self {
            AlertLevel::Warn => "⚠️",
            AlertLevel::Critical => "🚨",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AlertLevel::Warn => "AVISO",
            AlertLevel::Critical => "CRÍTICO",
        }
    }
}

/// Un evento de alerta generado por el detector.
#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub level: AlertLevel,
    pub percent_used: f64,
    pub reset_time: Option<String>,
    pub detected_at: DateTime<Utc>,
}

impl AlertEvent {
    /// Título para la notificación.
    pub fn title(&self) -> String {
        format!(
            "{} AI KeyVault — Cuota {}",
            self.level.emoji(),
            self.level.label()
        )
    }

    /// Cuerpo de la notificación.
    pub fn body(&self) -> String {
        let reset = self.reset_time.as_deref().unwrap_or("desconocido");
        format!(
            "{} · {} · {:.1}% consumido\nReinicia: {}",
            self.provider, self.pool, self.percent_used, reset
        )
    }
}

/// Configuración del alerter, leída desde variables de entorno.
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Umbral de aviso (%). Default: 80.0
    pub warn_threshold: f64,
    /// Umbral crítico (%). Default: 95.0
    pub crit_threshold: f64,
    /// Minutos de cooldown entre alertas del mismo tipo. Default: 60.
    pub cooldown_minutes: i64,
    /// Tópico de ntfy.sh. `None` → no enviar a ntfy.
    pub ntfy_topic: Option<String>,
    /// URL base de ntfy.sh. Default: "https://ntfy.sh"
    pub ntfy_url: String,
    /// Habilitar toast de Windows. Default: true.
    pub toast_enabled: bool,
    /// Ruta del archivo de estado de cooldown.
    pub state_path: PathBuf,
}

impl AlertConfig {
    /// Lee la configuración desde variables de entorno.
    ///
    /// Variables:
    /// - `AKYV_WARN_THRESHOLD`   — porcentaje de aviso (default 80)
    /// - `AKYV_CRIT_THRESHOLD`   — porcentaje crítico (default 95)
    /// - `AKYV_COOLDOWN_MINUTES` — cooldown en minutos (default 60)
    /// - `AKYV_NTFY_TOPIC`       — tópico de ntfy.sh (opcional)
    /// - `AKYV_NTFY_URL`         — URL de ntfy.sh (default https://ntfy.sh)
    /// - `AKYV_TOAST_ENABLED`    — "false" para deshabilitar toast (default true)
    /// - `AKYV_ALERT_STATE_PATH` — ruta del archivo de estado
    pub fn from_env() -> Self {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_state = home.join(".ai-keyvault").join("alert-state.json");

        Self {
            warn_threshold: std::env::var("AKYV_WARN_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(80.0),
            crit_threshold: std::env::var("AKYV_CRIT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(95.0),
            cooldown_minutes: std::env::var("AKYV_COOLDOWN_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            ntfy_topic: std::env::var("AKYV_NTFY_TOPIC").ok(),
            ntfy_url: std::env::var("AKYV_NTFY_URL")
                .unwrap_or_else(|_| "https://ntfy.sh".to_string()),
            toast_enabled: std::env::var("AKYV_TOAST_ENABLED")
                .map(|v| v != "false")
                .unwrap_or(true),
            state_path: std::env::var("AKYV_ALERT_STATE_PATH")
                .map(PathBuf::from)
                .unwrap_or(default_state),
        }
    }

    /// Constructor para tests con valores fijos.
    #[cfg(test)]
    pub fn for_test() -> Self {
        Self {
            warn_threshold: 80.0,
            crit_threshold: 95.0,
            cooldown_minutes: 60,
            ntfy_topic: None,
            ntfy_url: "https://ntfy.sh".to_string(),
            toast_enabled: false,
            state_path: PathBuf::from("/tmp/akyv-test-alert-state.json"),
        }
    }
}
