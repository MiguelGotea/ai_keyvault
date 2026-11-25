//! ai-keyvault-alerter — Detección de umbrales de cuota y envío de alertas.
//!
//! # Responsabilidades
//!
//! - Detectar cuando un pool supera el 80% (WARN) o 95% (CRIT) de consumo.
//! - Enviar notificaciones Windows toast y/o HTTP a ntfy.sh.
//! - Suprimir alertas duplicadas mediante cooldown persistido en disco.
//!
//! # Uso mínimo desde el poller
//!
//! ```rust,ignore
//! use ai_keyvault_alerter::{AlertConfig, AlertManager};
//!
//! let config = AlertConfig::from_env();
//! let mut manager = AlertManager::new(config);
//! manager.check_and_notify(&snapshots);
//! ```

pub mod cooldown;
pub mod detector;
pub mod models;
pub mod notifier;

pub use models::{AlertConfig, AlertEvent, AlertLevel, AlertReading};

use crate::{cooldown::CooldownStore, detector::detect_alerts, notifier::Notifier};
use tracing::{debug, info, warn};

/// Punto de entrada principal del alerter.
/// Evalúa lecturas, filtra cooldowns y envía notificaciones pendientes.
pub struct AlertManager {
    config: AlertConfig,
    cooldown: CooldownStore,
    notifier: Notifier,
}

impl AlertManager {
    pub fn new(config: AlertConfig) -> Self {
        let cooldown = CooldownStore::load(&config.state_path);
        let notifier = Notifier::new(config.ntfy_topic.clone(), config.toast_enabled);
        Self {
            config,
            cooldown,
            notifier,
        }
    }

    /// Evalúa las lecturas y dispara alertas si corresponde.
    pub fn check_and_notify(&mut self, readings: &[AlertReading]) {
        let events = detect_alerts(readings, &self.config);

        for event in &events {
            let key = format!("{}:{}:{:?}", event.account_id, event.pool, event.level);

            if self
                .cooldown
                .is_cooling_down(&key, self.config.cooldown_minutes)
            {
                debug!(
                    key = %key,
                    "⏳ Alerta suprimida por cooldown"
                );
                continue;
            }

            info!(
                account = %event.account_id,
                pool    = %event.pool,
                level   = ?event.level,
                pct     = event.percent_used,
                "🔔 Disparando alerta de cuota"
            );

            match self.notifier.send(event) {
                Ok(_) => {
                    self.cooldown.mark_alerted(&key);
                    self.cooldown.save(&self.config.state_path);
                }
                Err(e) => {
                    warn!("Error enviando alerta: {e}");
                }
            }
        }
    }
}
