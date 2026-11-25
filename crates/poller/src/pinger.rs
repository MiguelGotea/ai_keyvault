//! Pinger — módulo de ping automático y manual para agy.
//!
//! Se integra en el scheduler del poller como paso opcional después del
//! ciclo de cuota. Para cada cuenta agy con `percent_used < max_pct_to_ping`,
//! decide si es momento de hacer ping según el horario y el límite diario.
//!
//! # Variables de entorno
//!
//! | Variable              | Default | Descripción                                       |
//! |-----------------------|---------|---------------------------------------------------|
//! | `AKYV_PING_ENABLED`   | `false` | Habilita el ping automático global                 |
//! | `AKYV_PING_HOUR`      | `8`     | Hora local (0-23) para disparar el ping automático |
//! | `AKYV_PING_MAX_DAILY` | `2`     | Máximo de pings automáticos por cuenta por día     |
//! | `AKYV_PING_MAX_PCT`   | `5.0`   | % máximo de uso para considerar que necesita ping  |
//! | `AKYV_PING_MESSAGE`   | `k`     | Mensaje mínimo a enviar                            |

use crate::{models::QuotaSnapshot, PollerConfig};
use ai_keyvault_provider_agy::ping::{execute_ping, needs_ping, PingConfig};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Registro de un ping enviado (se sube al servidor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRecord {
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub message_sent: String,
    pub is_manual: bool,
    pub pinged_at: String,
}

/// Evalúa los snapshots actuales y ejecuta pings automáticos si corresponde.
///
/// Solo actúa si `AKYV_PING_ENABLED=true` y es la hora configurada.
/// Los resultados se suben al servidor vía `POST /api/pings`.
pub async fn maybe_ping_auto(
    client: &reqwest::Client,
    config: &PollerConfig,
    snapshots: &[QuotaSnapshot],
) {
    let ping_cfg = PingConfig::from_env();
    if !ping_cfg.enabled {
        return;
    }

    let local_hour = chrono::Local::now().hour();
    if local_hour != ping_cfg.hour as u32 {
        return; // No es la hora configurada
    }

    info!(
        "🏓 Hora de ping automático ({}:00) — evaluando cuentas",
        local_hour
    );

    for snapshot in snapshots {
        if snapshot.provider != "agy" {
            continue;
        }
        if !needs_ping(snapshot.percent_used, ping_cfg.max_pct_to_ping) {
            continue;
        }

        info!(
            account_id = %snapshot.account_id,
            percent_used = snapshot.percent_used,
            pool = %snapshot.pool,
            "🏓 Cuenta necesita ping — ejecutando"
        );

        let result = execute_ping(&snapshot.account_id, &ping_cfg.message);

        let record = match result {
            Ok(r) => {
                if r.success {
                    info!(
                        account_id = %r.account_id,
                        duration_ms = r.duration_ms,
                        "✅ Ping automático exitoso"
                    );
                } else {
                    warn!(
                        account_id = %r.account_id,
                        error = ?r.error,
                        "⚠️ Ping automático fallido"
                    );
                }
                PingRecord {
                    account_id: r.account_id,
                    provider: "agy".to_string(),
                    device_id: config.device_id.clone(),
                    success: r.success,
                    error: r.error,
                    duration_ms: r.duration_ms,
                    message_sent: r.message_sent,
                    is_manual: false,
                    pinged_at: chrono::Utc::now().to_rfc3339(),
                }
            }
            Err(e) => {
                warn!(error = %e, "❌ Error ejecutando ping automático");
                PingRecord {
                    account_id: snapshot.account_id.clone(),
                    provider: "agy".to_string(),
                    device_id: config.device_id.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    duration_ms: 0,
                    message_sent: ping_cfg.message.clone(),
                    is_manual: false,
                    pinged_at: chrono::Utc::now().to_rfc3339(),
                }
            }
        };

        upload_ping_record(client, config, &record).await;
    }
}

/// Sube un registro de ping al servidor.
pub async fn upload_ping_record(
    client: &reqwest::Client,
    config: &PollerConfig,
    record: &PingRecord,
) {
    let url = format!("{}/api/pings", config.server_url);
    match client.post(&url).json(record).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!(account_id = %record.account_id, "📤 Ping registrado en servidor");
        }
        Ok(resp) => {
            warn!(
                account_id = %record.account_id,
                status = %resp.status(),
                "⚠️ Servidor rechazó registro de ping"
            );
        }
        Err(e) => {
            warn!(
                account_id = %record.account_id,
                error = %e,
                "⚠️ Error de red al registrar ping"
            );
        }
    }
}
