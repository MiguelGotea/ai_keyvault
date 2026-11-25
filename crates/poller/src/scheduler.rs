//! Scheduler — ciclo periódico de consulta de cuota.
//!
//! Lanza un task de tokio que cada `poll_interval_secs` segundos:
//! 1. Verifica que el servidor esté disponible.
//! 2. Recolecta cuota de todos los providers locales.
//! 3. Agrupa por account_id y para cada cuenta intenta adquirir el lease.
//! 4. Si adquiere el lease, sube las lecturas.
//! 5. Evalúa umbrales y dispara alertas (toast + ntfy.sh) si corresponde.

use crate::{collector, lease, uploader, PollerConfig};
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
    for s in snapshots {
        by_account.entry(s.account_id.clone()).or_default().push(s);
    }

    // 5. Para cada cuenta: adquirir lease y subir
    for (account_id, account_snapshots) in &by_account {
        match lease::acquire(client, config, account_id).await {
            Ok(true) => {
                // Tenemos el lease — subir lecturas
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

    info!("✅ Ciclo completado");
}
