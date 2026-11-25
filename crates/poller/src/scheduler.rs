//! Scheduler — ciclo periódico de consulta de cuota.
//!
//! Lanza un task de tokio que cada `poll_interval_secs` segundos:
//! 1. Verifica que el servidor esté disponible.
//! 2. Recolecta cuota de todos los providers locales.
//! 3. Agrupa por account_id y para cada cuenta intenta adquirir el lease.
//! 4. Si adquiere el lease, sube las lecturas.
//! 5. Evalúa umbrales y dispara alertas (toast + ntfy.sh) si corresponde.

use crate::{chat_collector, collector, lease, pinger, uploader, PollerConfig};
use ai_keyvault_alerter::{AlertConfig, AlertManager, AlertReading};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Handle al poller en ejecución. Al soltar (drop), el poller se detiene.
pub struct PollerHandle {
    _task: JoinHandle<()>,
}

impl PollerHandle {
    /// Inicia el poller en background y devuelve el handle.
    pub fn start(config: PollerConfig) -> Self {
        let config = Arc::new(config);
        let task = tokio::spawn(run_loop(config));
        Self { _task: task }
    }
}

async fn run_loop(config: Arc<PollerConfig>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("No se pudo crear el cliente HTTP");

    let mut ticker = interval(Duration::from_secs(config.poll_interval_secs));
    // La primera iteración se dispara de inmediato
    ticker.tick().await;

    // AlertManager se crea una vez y mantiene estado de cooldown
    let alert_config = AlertConfig::from_env();
    let mut alert_manager = AlertManager::new(alert_config);

    info!(
        device_id = %config.device_id,
        server = %config.server_url,
        interval_secs = config.poll_interval_secs,
        "🚀 Poller de cuota iniciado"
    );

    loop {
        ticker.tick().await;
        run_cycle(&client, &config, &mut alert_manager).await;
    }
}

async fn run_cycle(
    client: &reqwest::Client,
    config: &PollerConfig,
    alert_manager: &mut AlertManager,
) {
    info!("🔄 Iniciando ciclo de consulta de cuota");

    // 1. Verificar servidor
    if !lease::check_server(client, config).await {
        warn!("⚠️ Servidor no disponible — se omite ciclo");
        return;
    }

    // 2. Recolectar snapshots locales
    let snapshots = collector::collect_all(&config.device_id);
    if snapshots.is_empty() {
        warn!("⚠️ No se obtuvieron lecturas de cuota — se omite ciclo");
        return;
    }

    // 3. Evaluar alertas (antes de subir — así alertamos aunque falle el upload)
    let alert_readings: Vec<AlertReading> = snapshots
        .iter()
        .map(|s| AlertReading {
            account_id: s.account_id.clone(),
            provider: s.provider.clone(),
            pool: s.pool.clone(),
            percent_used: s.percent_used,
            reset_time: s.reset_time.clone(),
        })
        .collect();
    alert_manager.check_and_notify(&alert_readings);

    // 4. Agrupar por account_id
    let mut by_account: HashMap<String, Vec<_>> = HashMap::new();
    for s in &snapshots {
        by_account
            .entry(s.account_id.clone())
            .or_default()
            .push(s.clone());
    }

    // 5. Para cada cuenta: adquirir lease y subir cuota
    for (account_id, account_snapshots) in &by_account {
        match lease::acquire(client, config, account_id).await {
            Ok(true) => {
                // Tenemos el lease — subir lecturas de cuota
                if let Err(e) = uploader::upload_snapshots(client, config, account_snapshots).await
                {
                    error!(account_id = %account_id, error = %e, "❌ Error al subir lecturas");
                }
            }
            Ok(false) => {
                // Otro dispositivo tiene el lease — omitir
            }
            Err(e) => {
                error!(account_id = %account_id, error = %e, "❌ Error al adquirir lease");
            }
        }
    }

    // 6. Sincronizar metadatos de chats agy (una vez por ciclo, siempre)
    //    No depende del lease — el servidor hace upsert idempotente.
    //    Usa el account_id del snapshot agy, no del codex.
    if let Some(agy_account) = snapshots
        .iter()
        .find(|s| s.provider == "agy")
        .map(|s| s.account_id.clone())
    {
        chat_collector::collect_and_upload(client, config, &agy_account).await;
    }

    // 6b. Sincronizar metadatos de chats Codex (Fase A)
    if let Some(codex_account) = snapshots
        .iter()
        .find(|s| s.provider == "codex")
        .map(|s| s.account_id.clone())
    {
        chat_collector::collect_and_upload_codex(client, config, &codex_account).await;
    }

    // 7. Upload de alertas disparadas al servidor (Fase D)
    //    Re-evalúa las readings para saber cuáles superaron el umbral
    //    y las sube como registro histórico.
    upload_alerts_to_server(client, config, &alert_readings).await;

    // 8. Ping automático (Fase 8) — solo si AKYV_PING_ENABLED=true y es la hora configurada
    pinger::maybe_ping_auto(client, config, &snapshots).await;

    info!("✅ Ciclo completado");
}

/// Sube al servidor las alertas que superaron el umbral en este ciclo.
/// Solo registra las que tienen nivel WARN o CRIT (percent_used >= warn_threshold).
async fn upload_alerts_to_server(
    client: &reqwest::Client,
    config: &PollerConfig,
    readings: &[ai_keyvault_alerter::AlertReading],
) {
    use ai_keyvault_alerter::AlertConfig;

    let alert_cfg = AlertConfig::from_env();

    for r in readings {
        let level = if r.percent_used >= alert_cfg.crit_threshold {
            "critical"
        } else if r.percent_used >= alert_cfg.warn_threshold {
            "warn"
        } else {
            continue;
        };

        let url = format!("{}/api/alerts", config.server_url);
        let body = serde_json::json!({
            "account_id":  r.account_id,
            "provider":    r.provider,
            "pool":        r.pool,
            "level":       level,
            "percent_used": r.percent_used,
            "reset_time":  r.reset_time,
            "notified_toast": false,
            "notified_ntfy":  false,
        });

        if let Err(e) = client.post(&url).json(&body).send().await {
            warn!(error = %e, "⚠️ Error al registrar alerta en servidor");
        }
    }
}
