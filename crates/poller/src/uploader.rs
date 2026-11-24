//! Uploader — envía las lecturas de cuota al servidor VPS.

use crate::{models::QuotaSnapshot, PollerConfig, PollerError, Result};
use tracing::{info, warn};

/// Envía una lista de snapshots de cuota al servidor.
///
/// Cada snapshot se envía como POST individual a `/api/quota/readings`.
/// Los fallos individuales se loguean pero no detienen el resto.
pub async fn upload_snapshots(
    client: &reqwest::Client,
    config: &PollerConfig,
    snapshots: &[QuotaSnapshot],
) -> Result<()> {
    if snapshots.is_empty() {
        return Ok(());
    }

    let url = format!("{}/api/quota/readings", config.server_url);
    let mut uploaded = 0usize;

    for snapshot in snapshots {
        let body = serde_json::json!({
            "account_id": snapshot.account_id,
            "provider": snapshot.provider,
            "pool": snapshot.pool,
            "tokens_used": snapshot.tokens_used,
            "tokens_limit": snapshot.tokens_limit,
            "percent_used": snapshot.percent_used,
            "reset_time": snapshot.reset_time,
            "source": snapshot.source,
        });

        match client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                uploaded += 1;
                info!(
                    account_id = %snapshot.account_id,
                    provider = %snapshot.provider,
                    pool = %snapshot.pool,
                    percent_used = snapshot.percent_used,
                    "📤 Lectura subida"
                );
            }
            Ok(resp) => {
                warn!(
                    account_id = %snapshot.account_id,
                    status = %resp.status(),
                    "⚠️ Servidor rechazó lectura"
                );
            }
            Err(e) => {
                warn!(
                    account_id = %snapshot.account_id,
                    error = %e,
                    "⚠️ Error de red al subir lectura"
                );
            }
        }
    }

    info!(
        uploaded = uploaded,
        total = snapshots.len(),
        "✅ Ciclo de upload completado"
    );

    if uploaded == 0 && !snapshots.is_empty() {
        return Err(PollerError::UploadError(
            "No se pudo subir ninguna lectura al servidor".to_string(),
        ));
    }

    Ok(())
}
