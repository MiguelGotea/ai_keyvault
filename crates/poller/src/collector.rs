//! Recolector de cuota — llama a providers agy y Codex.

use crate::{models::QuotaSnapshot, PollerError, Result};
use ai_keyvault_provider_agy::quota::{parse_usage_output, read_account_email, AgyPool};
use chrono::Utc;
use tracing::{info, warn};

/// Límite estimado de tokens por semana para normalizar el porcentaje de agy.
/// agy reporta porcentaje, no tokens absolutos — usamos 1_000_000 como denominador.
const AGY_ESTIMATED_LIMIT: i64 = 1_000_000;

/// Recolecta las lecturas de cuota de agy ejecutando `agy /usage`.
///
/// Devuelve un `QuotaSnapshot` por pool activo.
/// Si falla (agy no instalado, no autenticado, etc.), devuelve error.
pub fn collect_agy(device_id: &str) -> Result<Vec<QuotaSnapshot>> {
    let output = std::process::Command::new("agy")
        .args(["--print", "/usage"])
        .output()
        .map_err(|e| PollerError::AgyError(format!("No se pudo ejecutar agy: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PollerError::AgyError(format!(
            "agy /usage salió con error: {stderr}"
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let readings = parse_usage_output(&text)
        .map_err(|e| PollerError::AgyError(format!("Error al parsear agy /usage: {e}")))?;

    if readings.is_empty() {
        warn!("agy /usage no devolvió lecturas de cuota");
        return Ok(vec![]);
    }

    let now = Utc::now();
    // Enriquecer con email de la cuenta — parse_usage_output no lo incluye en el nuevo formato,
    // lo obtenemos via read_account_email (agy --print /account) con fallback a "agy-unknown".
    let account_id = read_account_email().unwrap_or_else(|| "agy-unknown".to_string());

    info!(
        account_id = %account_id,
        pools = readings.len(),
        "✅ Cuota agy recolectada"
    );

    let snapshots = readings
        .into_iter()
        .map(|r| {
            let pool = match r.pool {
                AgyPool::Gemini => "gemini",
                AgyPool::ClaudeGpt => "claude_gpt",
            };
            let tokens_used = ((r.percent_used / 100.0) * AGY_ESTIMATED_LIMIT as f64) as i64;
            QuotaSnapshot {
                account_id: account_id.clone(),
                provider: "agy".to_string(),
                pool: pool.to_string(),
                tokens_used,
                tokens_limit: AGY_ESTIMATED_LIMIT,
                percent_used: r.percent_used,
                percent_remaining: r.percent_remaining,
                reset_time: r.reset_at.clone(),
                source: device_id.to_string(),
                read_at: now,
            }
        })
        .collect();

    Ok(snapshots)
}

/// Recolecta la lectura de cuota de Codex leyendo `~/.codex/auth.json`.
///
/// Codex no expone cuota via CLI — solo verificamos si el token sigue vigente
/// y registramos eso como snapshot de "disponibilidad" del token.
pub fn collect_codex(device_id: &str) -> Result<Vec<QuotaSnapshot>> {
    use ai_keyvault_provider_codex::auth::CodexAuthManager;

    let manager = CodexAuthManager::new();
    let auth = manager
        .read()
        .map_err(|e| PollerError::CodexError(format!("No se pudo leer auth.json: {e}")))?;

    let now = Utc::now();
    let account_id = auth
        .email
        .clone()
        .unwrap_or_else(|| "codex-unknown".to_string());

    let (percent_remaining, percent_used) = if auth.is_valid() {
        (100.0_f64, 0.0_f64)
    } else {
        (0.0_f64, 100.0_f64)
    };

    info!(
        account_id = %account_id,
        valid = auth.is_valid(),
        mins_remaining = auth.minutes_remaining().unwrap_or(0),
        "✅ Estado Codex recolectado"
    );

    Ok(vec![QuotaSnapshot {
        account_id,
        provider: "codex".to_string(),
        pool: "default".to_string(),
        tokens_used: 0,    // Codex no expone tokens usados via auth.json
        tokens_limit: 100, // Representativo — 100% = token vigente
        percent_used,
        percent_remaining,
        reset_time: auth.expiry.map(|e| e.to_rfc3339()),
        source: device_id.to_string(),
        read_at: now,
    }])
}

/// Recolecta cuota de todos los providers disponibles en esta PC.
pub fn collect_all(device_id: &str) -> Vec<QuotaSnapshot> {
    let mut snapshots = Vec::new();

    match collect_agy(device_id) {
        Ok(s) => snapshots.extend(s),
        Err(e) => warn!("Error al recolectar cuota de agy: {e}"),
    }

    match collect_codex(device_id) {
        Ok(s) => snapshots.extend(s),
        Err(e) => warn!("Error al recolectar cuota de Codex: {e}"),
    }

    snapshots
}
