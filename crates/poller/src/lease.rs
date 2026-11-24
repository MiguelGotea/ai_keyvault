//! Cliente de lease anti-duplicado.
//!
//! Antes de consultar cuota, el poller adquiere un lease de 10 min en el servidor.
//! Si otro PC ya tiene el lease activo para esa cuenta, se salta la consulta.
//! El lease expira automáticamente — no hace falta liberarlo explícitamente.

use crate::{models::LeaseResponse, PollerConfig, PollerError, Result};
use tracing::{debug, info, warn};

/// Intenta adquirir el lease para una cuenta.
///
/// Retorna `Ok(true)` si lo adquirió, `Ok(false)` si otro PC ya lo tiene.
pub async fn acquire(
    client: &reqwest::Client,
    config: &PollerConfig,
    account_id: &str,
) -> Result<bool> {
    let url = format!("{}/api/quota/lease", config.server_url);

    let body = serde_json::json!({
        "account_id": account_id,
        "device_id": config.device_id,
        "lease_seconds": config.lease_duration_secs,
    });

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| PollerError::LeaseError(format!("Error de red al adquirir lease: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(PollerError::LeaseError(format!(
            "Servidor rechazó la solicitud de lease: HTTP {status}"
        )));
    }

    let lease: LeaseResponse = response
        .json()
        .await
        .map_err(|e| PollerError::LeaseError(format!("Respuesta de lease inválida: {e}")))?;

    if lease.acquired {
        info!(
            account_id = %account_id,
            device = %config.device_id,
            locked_until = %lease.locked_until,
            "🔒 Lease adquirido"
        );
    } else {
        debug!(
            account_id = %account_id,
            locked_by = %lease.locked_by,
            locked_until = %lease.locked_until,
            "⏭️ Lease activo en otro dispositivo — se omite consulta"
        );
    }

    Ok(lease.acquired)
}

/// Verifica si el servidor es alcanzable antes de intentar lease.
/// Útil para evitar errores ruidosos cuando no hay red.
pub async fn check_server(client: &reqwest::Client, config: &PollerConfig) -> bool {
    let url = format!("{}/health", config.server_url);
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => true,
        Ok(r) => {
            warn!("Servidor respondió HTTP {} en /health", r.status());
            false
        }
        Err(e) => {
            warn!("Servidor no alcanzable: {e}");
            false
        }
    }
}
