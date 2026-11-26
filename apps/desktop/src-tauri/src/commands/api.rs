//! Comandos Tauri para toda comunicación con el backend VPS.
//! REGLA: Ningún fetch() en React. Todo pasa por estos comandos.
//! El frontend invoca: await invoke("get_quota", { accountId: "..." })
//! Este módulo hace la petición HTTP desde Rust y devuelve JSON serializado.

use crate::api_client::{
    http_client, AlertRecord, ChatSession, DeviceRegistration, HeatmapCell, PingNeedItem,
    PingRecord, PoolSummary, PostPingPayload, PostSwitchPayload, QuotaReading, SwitchRecord,
    SERVER_URL,
};
use tauri::command;

// ── Helper interno ───────────────────────────────────────────────────────────

/// Convierte un error de reqwest en String para que Tauri lo serialice al frontend.
fn req_err(e: reqwest::Error) -> String {
    format!("Error HTTP: {e}")
}

// ── Health ───────────────────────────────────────────────────────────────────

/// Verifica si el servidor VPS responde.
/// El command de health.rs también sirve, pero este usa el cliente singleton.
#[command]
pub async fn check_server_health() -> Result<bool, String> {
    let url = format!("{SERVER_URL}/health");
    let resp = http_client().get(&url).send().await.map_err(req_err)?;
    Ok(resp.status().is_success())
}

// ── Cuota ────────────────────────────────────────────────────────────────────

/// Obtiene las últimas lecturas de cuota de una cuenta.
/// Mapea: GET /api/quota/readings/:account_id
#[command]
pub async fn get_quota(account_id: String) -> Result<Vec<QuotaReading>, String> {
    let url = format!(
        "{SERVER_URL}/api/quota/readings/{}",
        urlencoding(&account_id)
    );
    let readings = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<QuotaReading>>()
        .await
        .map_err(req_err)?;
    Ok(readings)
}

/// Resumen de cuota con burn rate y proyección.
/// Mapea: GET /api/stats/summary/:account_id?days=N
#[command]
pub async fn get_stats_summary(account_id: String, days: u32) -> Result<Vec<PoolSummary>, String> {
    let url = format!(
        "{SERVER_URL}/api/stats/summary/{}?days={days}",
        urlencoding(&account_id)
    );
    let summary = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<PoolSummary>>()
        .await
        .map_err(req_err)?;
    Ok(summary)
}

/// Heatmap de uso (hora del día × día de la semana).
/// Mapea: GET /api/stats/heatmap/:account_id?days=N
#[command]
pub async fn get_stats_heatmap(account_id: String, days: u32) -> Result<Vec<HeatmapCell>, String> {
    let url = format!(
        "{SERVER_URL}/api/stats/heatmap/{}?days={days}",
        urlencoding(&account_id)
    );
    let cells = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<HeatmapCell>>()
        .await
        .map_err(req_err)?;
    Ok(cells)
}

/// Comparativa de cuota entre todas las cuentas.
/// Mapea: GET /api/stats/compare?days=N
#[command]
pub async fn get_stats_compare(days: u32) -> Result<Vec<PoolSummary>, String> {
    let url = format!("{SERVER_URL}/api/stats/compare?days={days}");
    let result = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<PoolSummary>>()
        .await
        .map_err(req_err)?;
    Ok(result)
}

/// Estadísticas semanales de una cuenta.
/// Mapea: GET /api/stats/weekly/:account_id?days=N
#[command]
pub async fn get_stats_weekly(account_id: String, days: u32) -> Result<serde_json::Value, String> {
    let url = format!(
        "{SERVER_URL}/api/stats/weekly/{}?days={days}",
        urlencoding(&account_id)
    );
    let result = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<serde_json::Value>()
        .await
        .map_err(req_err)?;
    Ok(result)
}

// ── Chats ────────────────────────────────────────────────────────────────────

/// Obtiene sesiones de chat de una cuenta.
/// Mapea: GET /api/chats/:account_id
#[command]
pub async fn get_chats(account_id: String) -> Result<Vec<ChatSession>, String> {
    let url = format!("{SERVER_URL}/api/chats/{}", urlencoding(&account_id));
    let chats = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<ChatSession>>()
        .await
        .map_err(req_err)?;
    Ok(chats)
}

// ── Alertas ──────────────────────────────────────────────────────────────────

/// Obtiene el log de alertas. Si account_id es None, devuelve todos.
/// Mapea: GET /api/alerts?account_id=...  (opcional)
#[command]
pub async fn get_alerts(account_id: Option<String>) -> Result<Vec<AlertRecord>, String> {
    let url = match account_id {
        Some(ref id) => format!("{SERVER_URL}/api/alerts?account_id={}", urlencoding(id)),
        None => format!("{SERVER_URL}/api/alerts"),
    };
    let alerts = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<AlertRecord>>()
        .await
        .map_err(req_err)?;
    Ok(alerts)
}

// ── Switch de cuenta ─────────────────────────────────────────────────────────

/// Obtiene el historial de switches de cuenta.
/// Mapea: GET /api/switch/history?limit=N
/// El VPS devuelve {id, to_account_id, from_account_id, provider, device_id, success, reason, switched_at}
#[command]
pub async fn get_switch_history(limit: u32) -> Result<Vec<SwitchRecord>, String> {
    let url = format!("{SERVER_URL}/api/switch/history?limit={limit}");
    let history = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<SwitchRecord>>()
        .await
        .map_err(req_err)?;
    Ok(history)
}

/// Registra un switch de cuenta en el log del servidor.
/// Mapea: POST /api/switch
/// El VPS espera `to_account_id`, `reason` — NO `account_id`/`reverted`.
#[command]
pub async fn post_switch(
    to_account_id: String,
    from_account_id: Option<String>,
    provider: String,
    device_id: String,
    success: bool,
    reason: Option<String>,
) -> Result<(), String> {
    let payload = PostSwitchPayload {
        to_account_id,
        from_account_id,
        provider,
        device_id,
        success,
        reason,
    };
    http_client()
        .post(&format!("{SERVER_URL}/api/switch"))
        .json(&payload)
        .send()
        .await
        .map_err(req_err)?;
    Ok(())
}

// ── Pings ────────────────────────────────────────────────────────────────────

/// Obtiene las cuentas que necesitan ping (cuota baja o sin uso).
/// Mapea: GET /api/pings/needs?max_pct=N
/// El VPS devuelve un array de objetos {account_id, provider, pool, percent_used, needs_ping, ...}
#[command]
pub async fn get_pings_needs(max_pct: f64) -> Result<Vec<PingNeedItem>, String> {
    let url = format!("{SERVER_URL}/api/pings/needs?max_pct={max_pct}");
    let items = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<PingNeedItem>>()
        .await
        .map_err(req_err)?;
    Ok(items)
}

/// Obtiene el historial de pings de una cuenta.
/// Mapea: GET /api/pings/:account_id
#[command]
pub async fn get_pings(account_id: String) -> Result<Vec<PingRecord>, String> {
    let url = format!("{SERVER_URL}/api/pings/{}", urlencoding(&account_id));
    let pings = http_client()
        .get(&url)
        .send()
        .await
        .map_err(req_err)?
        .json::<Vec<PingRecord>>()
        .await
        .map_err(req_err)?;
    Ok(pings)
}

/// Registra un ping manual en el servidor.
/// Mapea: POST /api/pings
#[command]
pub async fn post_ping(
    account_id: String,
    provider: String,
    device_id: String,
    success: bool,
    error: Option<String>,
    duration_ms: Option<i64>,
    is_manual: bool,
) -> Result<(), String> {
    let payload = PostPingPayload {
        account_id,
        provider,
        device_id,
        success,
        error,
        duration_ms,
        is_manual,
    };
    http_client()
        .post(&format!("{SERVER_URL}/api/pings"))
        .json(&payload)
        .send()
        .await
        .map_err(req_err)?;
    Ok(())
}

// ── Dispositivos ─────────────────────────────────────────────────────────────

/// Registra el dispositivo en el servidor VPS.
/// Mapea: POST /api/devices/register
#[command]
pub async fn register_device(
    device_id: String,
    device_name: String,
    public_key_hex: String,
) -> Result<(), String> {
    let payload = DeviceRegistration {
        device_id,
        device_name,
        public_key_hex,
    };
    http_client()
        .post(&format!("{SERVER_URL}/api/devices/register"))
        .json(&payload)
        .send()
        .await
        .map_err(req_err)?;
    Ok(())
}

// ── Helpers privados ─────────────────────────────────────────────────────────

/// Codifica un string para usarlo en segmentos de URL.
fn urlencoding(s: &str) -> String {
    // Reemplaza caracteres especiales comunes en account_ids
    // Para producción usar el crate `urlencoding` si se necesitan más casos
    s.replace(' ', "%20")
        .replace('@', "%40")
        .replace('#', "%23")
        .replace('&', "%26")
}
