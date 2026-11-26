use crate::error::{Result, ServerError};
use crate::stats::{
    build_heatmap, build_weekly_report, summarize_pools, to_csv, HeatmapCell, PoolSummary,
    ReadingSample, WeeklyReport,
};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub _server_master_key: String,
}

// ─────────────────────────────────────────────
// 1. Healthcheck
// ─────────────────────────────────────────────
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-keyvault-server",
        "version": "0.1.0",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

// ─────────────────────────────────────────────
// 2. Registro de Dispositivos (Ed25519)
// ─────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub device_id: String,
    pub device_name: String,
    pub public_key_hex: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterDeviceResponse {
    pub device_id: String,
    pub registered: bool,
    pub message: String,
}

pub async fn register_device(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<RegisterDeviceResponse>> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO devices (id, name, public_key_hex, is_active, created_at, last_seen_at)
        VALUES (?, ?, ?, 1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            last_seen_at = excluded.last_seen_at
        "#,
    )
    .bind(&payload.device_id)
    .bind(&payload.device_name)
    .bind(&payload.public_key_hex)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(RegisterDeviceResponse {
        device_id: payload.device_id,
        registered: true,
        message: "Dispositivo registrado exitosamente".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct DeviceItem {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeDeviceRequest {
    pub device_id: String,
}

/// `GET /api/devices` — lista los dispositivos registrados (Fase 25).
pub async fn list_devices(State(state): State<Arc<AppState>>) -> Result<Json<Vec<DeviceItem>>> {
    let rows = sqlx::query(
        "SELECT id, name, is_active, created_at, last_seen_at FROM devices ORDER BY last_seen_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let devices = rows
        .into_iter()
        .map(|r| DeviceItem {
            id: r.get("id"),
            name: r.get("name"),
            is_active: r.get::<i64, _>("is_active") != 0,
            created_at: r.get("created_at"),
            last_seen_at: r.get("last_seen_at"),
        })
        .collect();

    Ok(Json(devices))
}

/// `POST /api/devices/revoke` — revoca el acceso de un dispositivo (Fase 25).
pub async fn revoke_device(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RevokeDeviceRequest>,
) -> Result<Json<serde_json::Value>> {
    sqlx::query("UPDATE devices SET is_active = 0 WHERE id = ?")
        .bind(&payload.device_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "device_id": payload.device_id,
        "revoked": true
    })))
}

// ─────────────────────────────────────────────
// 3. Sincronización de Bóveda Cifrada (Conocimiento Cero)
// ─────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct SyncUploadRequest {
    pub device_id: String,
    pub encrypted_payload: String, // JSON cifrado en cliente
}

#[derive(Debug, Serialize)]
pub struct SyncUploadResponse {
    pub version: i64,
    pub checksum_sha256: String,
    pub updated_at: String,
}

pub async fn upload_vault(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SyncUploadRequest>,
) -> Result<Json<SyncUploadResponse>> {
    let mut hasher = Sha256::new();
    hasher.update(payload.encrypted_payload.as_bytes());
    let checksum = hex::encode(hasher.finalize());
    let now = Utc::now().to_rfc3339();

    // Obtener la última versión
    let row = sqlx::query("SELECT MAX(version) as max_v FROM vault_snapshots")
        .fetch_one(&state.db)
        .await?;

    let current_version: Option<i64> = row.try_get("max_v").unwrap_or(None);
    let next_version = current_version.unwrap_or(0) + 1;

    sqlx::query(
        r#"
        INSERT INTO vault_snapshots (version, device_id, encrypted_payload, checksum_sha256, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(next_version)
    .bind(&payload.device_id)
    .bind(&payload.encrypted_payload)
    .bind(&checksum)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(SyncUploadResponse {
        version: next_version,
        checksum_sha256: checksum,
        updated_at: now,
    }))
}

#[derive(Debug, Serialize)]
pub struct SyncDownloadResponse {
    pub version: i64,
    pub device_id: String,
    pub encrypted_payload: String,
    pub checksum_sha256: String,
    pub updated_at: String,
}

pub async fn download_vault(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SyncDownloadResponse>> {
    let row = sqlx::query(
        r#"
        SELECT version, device_id, encrypted_payload, checksum_sha256, updated_at
        FROM vault_snapshots
        ORDER BY version DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = row {
        Ok(Json(SyncDownloadResponse {
            version: r.get("version"),
            device_id: r.get("device_id"),
            encrypted_payload: r.get("encrypted_payload"),
            checksum_sha256: r.get("checksum_sha256"),
            updated_at: r.get("updated_at"),
        }))
    } else {
        Err(ServerError::BadRequest(
            "No existe ninguna bóveda sincronizada aún".into(),
        ))
    }
}

// ─────────────────────────────────────────────
// 4. Gestión de Cuotas y Leases Anti-Duplicados
// ─────────────────────────────────────────────
#[derive(Debug, Deserialize)]
pub struct AcquireLeaseRequest {
    pub account_id: String,
    pub device_id: String,
    pub lease_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LeaseResponse {
    pub account_id: String,
    pub acquired: bool,
    pub locked_by: String,
    pub locked_until: String,
}

pub async fn acquire_lease(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AcquireLeaseRequest>,
) -> Result<Json<LeaseResponse>> {
    let now = Utc::now();
    let duration = payload.lease_seconds.unwrap_or(600); // 10 min default
    let new_locked_until = (now + Duration::seconds(duration)).to_rfc3339();

    // Verificar si ya existe un lease activo
    let row =
        sqlx::query("SELECT locked_by_device, locked_until FROM quota_leases WHERE account_id = ?")
            .bind(&payload.account_id)
            .fetch_optional(&state.db)
            .await?;

    if let Some(r) = row {
        let current_lock_str: String = r.get("locked_until");
        let current_locked_by: String = r.get("locked_by_device");

        if let Ok(current_locked_until) = DateTime::parse_from_rfc3339(&current_lock_str) {
            if current_locked_until.with_timezone(&Utc) > now
                && current_locked_by != payload.device_id
            {
                return Ok(Json(LeaseResponse {
                    account_id: payload.account_id,
                    acquired: false,
                    locked_by: current_locked_by,
                    locked_until: current_lock_str,
                }));
            }
        }
    }

    // Adquirir o renovar el lease
    sqlx::query(
        r#"
        INSERT INTO quota_leases (account_id, locked_by_device, locked_until)
        VALUES (?, ?, ?)
        ON CONFLICT(account_id) DO UPDATE SET
            locked_by_device = excluded.locked_by_device,
            locked_until = excluded.locked_until
        "#,
    )
    .bind(&payload.account_id)
    .bind(&payload.device_id)
    .bind(&new_locked_until)
    .execute(&state.db)
    .await?;

    Ok(Json(LeaseResponse {
        account_id: payload.account_id,
        acquired: true,
        locked_by: payload.device_id,
        locked_until: new_locked_until,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PostQuotaReadingRequest {
    pub account_id: String,
    pub provider: String,
    pub pool: String,
    pub tokens_used: i64,
    pub tokens_limit: i64,
    pub percent_used: f64,
    pub reset_time: Option<String>,
    pub source: String,
}

pub async fn post_quota_reading(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PostQuotaReadingRequest>,
) -> Result<Json<serde_json::Value>> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO quota_readings (
            account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source, read_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&payload.account_id)
    .bind(&payload.provider)
    .bind(&payload.pool)
    .bind(payload.tokens_used)
    .bind(payload.tokens_limit)
    .bind(payload.percent_used)
    .bind(&payload.reset_time)
    .bind(&payload.source)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // Actualizar last_read_at en el lease
    let _ = sqlx::query("UPDATE quota_leases SET last_read_at = ? WHERE account_id = ?")
        .bind(&now)
        .bind(&payload.account_id)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "status": "recorded",
        "account_id": payload.account_id,
        "recorded_at": now
    })))
}

#[derive(Debug, Serialize)]
pub struct QuotaReadingItem {
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

pub async fn get_quota_readings(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
) -> Result<Json<Vec<QuotaReadingItem>>> {
    let rows = sqlx::query(
        r#"
        SELECT account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source, read_at
        FROM quota_readings
        WHERE account_id = ?
        ORDER BY id DESC
        LIMIT 20
        "#,
    )
    .bind(&account_id)
    .fetch_all(&state.db)
    .await?;

    let readings = rows
        .into_iter()
        .map(|r| QuotaReadingItem {
            account_id: r.get("account_id"),
            provider: r.get("provider"),
            pool: r.get("pool"),
            tokens_used: r.get("tokens_used"),
            tokens_limit: r.get("tokens_limit"),
            percent_used: r.get("percent_used"),
            reset_time: r.get("reset_time"),
            source: r.get("source"),
            read_at: r.get("read_at"),
        })
        .collect();

    Ok(Json(readings))
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Chat Sessions (Fase 6 — agy chats)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PostChatSessionRequest {
    pub account_id: String,
    pub device_id: String,
    pub chats: Vec<ChatSessionItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatSessionItem {
    pub id: String,
    pub provider: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived: bool,
}

/// `POST /api/chats` — el poller sube metadatos de chats locales.
pub async fn post_chat_sessions(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PostChatSessionRequest>,
) -> Result<Json<serde_json::Value>> {
    let now = Utc::now().to_rfc3339();
    let mut upserted = 0u32;

    for chat in &payload.chats {
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (id, account_id, provider, title, created_at, updated_at, device_id, archived, synced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id, account_id) DO UPDATE SET
                title      = excluded.title,
                updated_at = excluded.updated_at,
                archived   = excluded.archived,
                device_id  = excluded.device_id,
                synced_at  = excluded.synced_at
            "#,
        )
        .bind(&chat.id)
        .bind(&payload.account_id)
        .bind(&chat.provider)
        .bind(&chat.title)
        .bind(&chat.created_at)
        .bind(&chat.updated_at)
        .bind(&payload.device_id)
        .bind(chat.archived as i32)
        .bind(&now)
        .execute(&state.db)
        .await?;
        upserted += 1;
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "account_id": payload.account_id,
        "upserted": upserted,
        "synced_at": now
    })))
}

/// `GET /api/chats/:account_id` — devuelve los chats más recientes de una cuenta.
pub async fn get_chat_sessions(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
) -> Result<Json<Vec<ChatSessionItem>>> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider, title, created_at, updated_at, archived
        FROM chat_sessions
        WHERE account_id = ? AND archived = 0
        ORDER BY updated_at DESC
        LIMIT 50
        "#,
    )
    .bind(&account_id)
    .fetch_all(&state.db)
    .await?;

    let chats = rows
        .into_iter()
        .map(|r| ChatSessionItem {
            id: r.get("id"),
            provider: r.get("provider"),
            title: r.get("title"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            archived: r.get::<i32, _>("archived") != 0,
        })
        .collect();

    Ok(Json(chats))
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Estadísticas y Exportación (Fase 7)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub days: Option<i64>,
    pub format: Option<String>,
}

async fn fetch_readings(
    db: &SqlitePool,
    account_id: Option<&str>,
    days: i64,
) -> Result<Vec<ReadingSample>> {
    let cutoff = (Utc::now() - Duration::days(days.clamp(1, 365))).to_rfc3339();

    let rows = match account_id {
        Some(acc) => {
            sqlx::query(
                r#"
                SELECT account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source, read_at
                FROM quota_readings
                WHERE account_id = ? AND read_at >= ?
                ORDER BY read_at ASC
                LIMIT 5000
                "#,
            )
            .bind(acc)
            .bind(&cutoff)
            .fetch_all(db)
            .await?
        }
        None => {
            sqlx::query(
                r#"
                SELECT account_id, provider, pool, tokens_used, tokens_limit, percent_used, reset_time, source, read_at
                FROM quota_readings
                WHERE read_at >= ?
                ORDER BY read_at ASC
                LIMIT 10000
                "#,
            )
            .bind(&cutoff)
            .fetch_all(db)
            .await?
        }
    };

    Ok(rows
        .into_iter()
        .map(|r| ReadingSample {
            account_id: r.get("account_id"),
            provider: r.get("provider"),
            pool: r.get("pool"),
            tokens_used: r.get("tokens_used"),
            tokens_limit: r.get("tokens_limit"),
            percent_used: r.get("percent_used"),
            reset_time: r.get("reset_time"),
            source: r.get("source"),
            read_at: r.get("read_at"),
        })
        .collect())
}

/// `GET /api/stats/summary/:account_id` — Resumen por pool con burn rate y proyección.
pub async fn get_stats_summary(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<Vec<PoolSummary>>> {
    let samples = fetch_readings(&state.db, Some(&account_id), q.days.unwrap_or(7)).await?;
    Ok(Json(summarize_pools(&samples)))
}

/// `GET /api/stats/heatmap/:account_id` — Mapa de calor por día de semana y hora UTC.
pub async fn get_stats_heatmap(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<Vec<HeatmapCell>>> {
    let samples = fetch_readings(&state.db, Some(&account_id), q.days.unwrap_or(30)).await?;
    Ok(Json(build_heatmap(&samples)))
}

/// `GET /api/stats/compare` — Comparativa entre todas las cuentas registradas.
pub async fn get_stats_compare(
    State(state): State<Arc<AppState>>,
    Query(q): Query<StatsQuery>,
) -> Result<Json<Vec<PoolSummary>>> {
    let samples = fetch_readings(&state.db, None, q.days.unwrap_or(7)).await?;
    Ok(Json(summarize_pools(&samples)))
}

/// `GET /api/stats/weekly/:account_id` — Informe semanal agrupado por día y pool.
pub async fn get_stats_weekly(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
) -> Result<Json<WeeklyReport>> {
    let samples = fetch_readings(&state.db, Some(&account_id), 7).await?;
    Ok(Json(build_weekly_report(&account_id, &samples)))
}

/// `GET /api/stats/export/:account_id?format=csv|json&days=30` — Exportación de serie temporal.
pub async fn get_stats_export(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Result<Response> {
    let samples = fetch_readings(&state.db, Some(&account_id), q.days.unwrap_or(30)).await?;
    let format = q.format.as_deref().unwrap_or("json").to_lowercase();

    match format.as_str() {
        "csv" => {
            let csv_body = to_csv(&samples);
            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"keyvault-stats.csv\"",
                    ),
                ],
                csv_body,
            )
                .into_response())
        }
        _ => Ok(Json(samples).into_response()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Ping (Fase 8)
// ─────────────────────────────────────────────────────────────────────────────

/// Cuerpo de `POST /api/pings` — registro de ping enviado por el poller o manualmente.
#[derive(Debug, Deserialize)]
pub struct PostPingBody {
    pub account_id: String,
    pub provider: Option<String>,
    pub device_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: Option<i64>,
    pub message_sent: Option<String>,
    pub is_manual: Option<bool>,
    pub pinged_at: Option<String>,
}

/// Registro de ping devuelto por el servidor.
#[derive(Debug, Serialize)]
pub struct PingEntry {
    pub id: i64,
    pub account_id: String,
    pub provider: String,
    pub device_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub message_sent: String,
    pub is_manual: bool,
    pub pinged_at: String,
}

/// `POST /api/pings` — registra un ping (manual o automático).
pub async fn post_ping(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PostPingBody>,
) -> Result<Json<serde_json::Value>> {
    let provider = body.provider.unwrap_or_else(|| "agy".to_string());
    let duration_ms = body.duration_ms.unwrap_or(0);
    let message_sent = body.message_sent.unwrap_or_else(|| "k".to_string());
    let is_manual = body.is_manual.unwrap_or(true);
    let pinged_at = body.pinged_at.unwrap_or_else(|| Utc::now().to_rfc3339());

    sqlx::query(
        r#"
        INSERT INTO ping_log
            (account_id, provider, device_id, success, error, duration_ms,
             message_sent, is_manual, pinged_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&body.account_id)
    .bind(&provider)
    .bind(&body.device_id)
    .bind(body.success as i64)
    .bind(&body.error)
    .bind(duration_ms)
    .bind(&message_sent)
    .bind(is_manual as i64)
    .bind(&pinged_at)
    .execute(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "account_id": body.account_id,
        "pinged_at": pinged_at
    })))
}

/// `GET /api/pings/:account_id?limit=20` — historial de pings de una cuenta.
pub async fn get_pings(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<PingEntry>>> {
    let limit: i64 = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    let rows = sqlx::query(
        r#"
        SELECT id, account_id, provider, device_id, success, error,
               duration_ms, message_sent, is_manual, pinged_at
        FROM ping_log
        WHERE account_id = ?
        ORDER BY pinged_at DESC
        LIMIT ?
        "#,
    )
    .bind(&account_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    let entries: Vec<PingEntry> = rows
        .iter()
        .map(|r| PingEntry {
            id: r.get::<i64, _>("id"),
            account_id: r.get::<String, _>("account_id"),
            provider: r.get::<String, _>("provider"),
            device_id: r.get::<String, _>("device_id"),
            success: r.get::<i64, _>("success") != 0,
            error: r.get::<Option<String>, _>("error"),
            duration_ms: r.get::<i64, _>("duration_ms"),
            message_sent: r.get::<String, _>("message_sent"),
            is_manual: r.get::<i64, _>("is_manual") != 0,
            pinged_at: r.get::<String, _>("pinged_at"),
        })
        .collect();

    Ok(Json(entries))
}

/// `GET /api/pings/needs` — cuentas que necesitan ping (percent_used < umbral).
///
/// Devuelve la última lectura por cuenta/pool donde el uso está por debajo
/// del umbral configurable (default 5%). Query param: `?max_pct=5`.
pub async fn get_pings_needs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let max_pct: f64 = q.get("max_pct").and_then(|v| v.parse().ok()).unwrap_or(5.0);

    // Última lectura por (account_id, provider, pool) con percent_used < umbral
    let rows = sqlx::query(
        r#"
        SELECT account_id, provider, pool,
               percent_used, reset_time, MAX(read_at) AS last_read_at
        FROM quota_readings
        WHERE account_id NOT LIKE 'agy-unknown%'
          AND percent_used < ?
        GROUP BY account_id, provider, pool
        ORDER BY account_id, pool
        "#,
    )
    .bind(max_pct)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "account_id":   r.get::<String, _>("account_id"),
                "provider":     r.get::<String, _>("provider"),
                "pool":         r.get::<String, _>("pool"),
                "percent_used": r.get::<f64, _>("percent_used"),
                "reset_time":   r.get::<Option<String>, _>("reset_time"),
                "last_read_at": r.get::<String, _>("last_read_at"),
                "needs_ping":   true
            })
        })
        .collect();

    Ok(Json(result))
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Account Switcher — agy (Fase E)
// ─────────────────────────────────────────────────────────────────────────────
// NOTA: El switcher de agy opera en la PC local donde está el Credential Manager.
// El servidor solo registra el historial de switches (quién cambió de cuenta y cuándo).
// La ejecución real de `AgyCredentialManager::write()` ocurre en el poller/CLI local,
// no en el VPS.

/// Registro de un cambio de cuenta (guardado en el servidor para historial).
#[derive(Debug, Deserialize)]
pub struct PostSwitchBody {
    /// Email de la cuenta a la que se cambió.
    pub to_account_id: String,
    /// Email de la cuenta anterior (antes del switch).
    pub from_account_id: Option<String>,
    pub provider: Option<String>,
    pub device_id: String,
    /// Si el switch fue exitoso.
    pub success: bool,
    /// Razón de fallo si `success = false`.
    pub reason: Option<String>,
    pub switched_at: Option<String>,
}

/// `POST /api/switch` — registra un cambio de cuenta en el historial del servidor.
pub async fn post_switch(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PostSwitchBody>,
) -> Result<Json<serde_json::Value>> {
    let provider = body.provider.unwrap_or_else(|| "agy".to_string());
    let switched_at = body.switched_at.unwrap_or_else(|| Utc::now().to_rfc3339());

    sqlx::query(
        r#"
        INSERT INTO switch_log
            (to_account_id, from_account_id, provider, device_id, success, reason, switched_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&body.to_account_id)
    .bind(&body.from_account_id)
    .bind(&provider)
    .bind(&body.device_id)
    .bind(body.success as i64)
    .bind(&body.reason)
    .bind(&switched_at)
    .execute(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "to_account_id": body.to_account_id,
        "switched_at": switched_at
    })))
}

/// `GET /api/switch/history?limit=20` — historial de switches de cuenta.
pub async fn get_switch_history(
    State(state): State<Arc<AppState>>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let limit: i64 = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);

    let rows = sqlx::query(
        r#"
        SELECT id, to_account_id, from_account_id, provider, device_id,
               success, reason, switched_at
        FROM switch_log
        ORDER BY switched_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":              r.get::<i64, _>("id"),
                "to_account_id":   r.get::<String, _>("to_account_id"),
                "from_account_id": r.get::<Option<String>, _>("from_account_id"),
                "provider":        r.get::<String, _>("provider"),
                "device_id":       r.get::<String, _>("device_id"),
                "success":         r.get::<i64, _>("success") != 0,
                "reason":          r.get::<Option<String>, _>("reason"),
                "switched_at":     r.get::<String, _>("switched_at"),
            })
        })
        .collect();

    Ok(Json(result))
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Alert Log — historial de alertas disparadas (Fase D)
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/alerts?account_id=...&limit=20` — historial de alertas del alerter.
pub async fn get_alert_log(
    State(state): State<Arc<AppState>>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<serde_json::Value>>> {
    let limit: i64 = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20);
    let account_filter = q.get("account_id").cloned();

    let rows = if let Some(ref account_id) = account_filter {
        sqlx::query(
            r#"
            SELECT id, account_id, provider, pool, level, percent_used, reset_time,
                   notified_toast, notified_ntfy, detected_at
            FROM alert_log
            WHERE account_id = ?
            ORDER BY detected_at DESC
            LIMIT ?
            "#,
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
    } else {
        sqlx::query(
            r#"
            SELECT id, account_id, provider, pool, level, percent_used, reset_time,
                   notified_toast, notified_ntfy, detected_at
            FROM alert_log
            ORDER BY detected_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))?
    };

    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id":             r.get::<i64, _>("id"),
                "account_id":     r.get::<String, _>("account_id"),
                "provider":       r.get::<String, _>("provider"),
                "pool":           r.get::<String, _>("pool"),
                "level":          r.get::<String, _>("level"),
                "percent_used":   r.get::<f64, _>("percent_used"),
                "reset_time":     r.get::<Option<String>, _>("reset_time"),
                "notified_toast": r.get::<i64, _>("notified_toast") != 0,
                "notified_ntfy":  r.get::<i64, _>("notified_ntfy") != 0,
                "detected_at":    r.get::<String, _>("detected_at"),
            })
        })
        .collect();

    Ok(Json(result))
}

/// `POST /api/alerts` — el poller registra una alerta disparada.
#[derive(Debug, Deserialize)]
pub struct PostAlertBody {
    pub account_id: String,
    pub provider: Option<String>,
    pub pool: String,
    pub level: String,
    pub percent_used: f64,
    pub reset_time: Option<String>,
    pub notified_toast: Option<bool>,
    pub notified_ntfy: Option<bool>,
    pub detected_at: Option<String>,
}

pub async fn post_alert(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PostAlertBody>,
) -> Result<Json<serde_json::Value>> {
    let provider = body.provider.unwrap_or_else(|| "agy".to_string());
    let detected_at = body.detected_at.unwrap_or_else(|| Utc::now().to_rfc3339());

    sqlx::query(
        r#"
        INSERT INTO alert_log
            (account_id, provider, pool, level, percent_used, reset_time,
             notified_toast, notified_ntfy, detected_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&body.account_id)
    .bind(&provider)
    .bind(&body.pool)
    .bind(&body.level)
    .bind(body.percent_used)
    .bind(&body.reset_time)
    .bind(body.notified_toast.unwrap_or(false) as i64)
    .bind(body.notified_ntfy.unwrap_or(false) as i64)
    .bind(&detected_at)
    .execute(&state.db)
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "account_id": body.account_id,
        "level": body.level,
        "detected_at": detected_at
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. Autenticación de usuario y bóveda por usuario (Fase 19.1)
// ─────────────────────────────────────────────────────────────────────────────

fn hash_auth_key(auth_key_hex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(auth_key_hex.trim().to_lowercase().as_bytes());
    hex::encode(hasher.finalize())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[derive(Debug, Deserialize)]
pub struct AuthRegisterRequest {
    pub username: String,
    pub display_name: String,
    pub kdf_salt_hex: String,
    pub kdf_memory_kib: i64,
    pub kdf_iterations: i64,
    pub kdf_parallelism: i64,
    pub auth_key_hex: String,
    pub device_id: String,
    pub device_name: String,
    pub encrypted_payload: String,
}

#[derive(Debug, Serialize)]
pub struct AuthRegisterResponse {
    pub registered: bool,
    pub username: String,
    pub version: i64,
}

/// `POST /api/auth/register` — crea usuario único con AuthKey separada y bóveda inicial.
pub async fn auth_register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthRegisterRequest>,
) -> Result<Json<AuthRegisterResponse>> {
    let username = body.username.trim().to_lowercase();
    if username.is_empty() || body.display_name.trim().is_empty() {
        return Err(ServerError::BadRequest(
            "Usuario y nombre son obligatorios".into(),
        ));
    }
    if body.auth_key_hex.trim().len() != 64 || body.kdf_salt_hex.trim().len() < 32 {
        return Err(ServerError::BadRequest("Parámetros KDF inválidos".into()));
    }

    let existing = sqlx::query("SELECT username FROM users WHERE username = ?")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(ServerError::Conflict(
            "Ese nombre de usuario ya está registrado".into(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let auth_hash = hash_auth_key(&body.auth_key_hex);
    let mut hasher = Sha256::new();
    hasher.update(body.encrypted_payload.as_bytes());
    let checksum = hex::encode(hasher.finalize());

    sqlx::query(
        r#"
        INSERT INTO devices (id, name, public_key_hex, is_active, created_at, last_seen_at)
        VALUES (?, ?, ?, 1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET name = excluded.name, last_seen_at = excluded.last_seen_at
        "#,
    )
    .bind(&body.device_id)
    .bind(&body.device_name)
    .bind("0".repeat(64))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users (
            username, display_name, kdf_salt_hex, kdf_memory_kib,
            kdf_iterations, kdf_parallelism, auth_key_hash, failed_attempts,
            locked_until, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL, ?)
        "#,
    )
    .bind(&username)
    .bind(body.display_name.trim())
    .bind(body.kdf_salt_hex.trim())
    .bind(body.kdf_memory_kib)
    .bind(body.kdf_iterations)
    .bind(body.kdf_parallelism)
    .bind(&auth_hash)
    .bind(&now)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_vaults (username, version, device_id, encrypted_payload, checksum_sha256, updated_at)
        VALUES (?, 1, ?, ?, ?, ?)
        "#,
    )
    .bind(&username)
    .bind(&body.device_id)
    .bind(&body.encrypted_payload)
    .bind(&checksum)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(AuthRegisterResponse {
        registered: true,
        username,
        version: 1,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AuthParamsRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct AuthParamsResponse {
    pub kdf_salt_hex: String,
    pub kdf_memory_kib: i64,
    pub kdf_iterations: i64,
    pub kdf_parallelism: i64,
}

/// `POST /api/auth/params` — devuelve parámetros KDF sin revelar si el usuario existe.
pub async fn auth_params(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthParamsRequest>,
) -> Result<Json<AuthParamsResponse>> {
    let username = body.username.trim().to_lowercase();
    let row = sqlx::query(
        "SELECT kdf_salt_hex, kdf_memory_kib, kdf_iterations, kdf_parallelism FROM users WHERE username = ?",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await?;

    if let Some(r) = row {
        Ok(Json(AuthParamsResponse {
            kdf_salt_hex: r.get("kdf_salt_hex"),
            kdf_memory_kib: r.get("kdf_memory_kib"),
            kdf_iterations: r.get("kdf_iterations"),
            kdf_parallelism: r.get("kdf_parallelism"),
        }))
    } else {
        // Salt determinista falso para no filtrar si el usuario existe
        let mut hasher = Sha256::new();
        hasher.update(format!("akv-fake-salt:{username}:{}", state._server_master_key).as_bytes());
        Ok(Json(AuthParamsResponse {
            kdf_salt_hex: hex::encode(hasher.finalize()),
            kdf_memory_kib: 64 * 1024,
            kdf_iterations: 3,
            kdf_parallelism: 4,
        }))
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthLoginRequest {
    pub username: String,
    pub auth_key_hex: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Serialize)]
pub struct AuthLoginResponse {
    pub username: String,
    pub display_name: String,
    pub version: i64,
    pub encrypted_payload: String,
    pub checksum_sha256: String,
    pub updated_at: String,
}

async fn verify_user_auth(state: &AppState, username: &str, auth_key_hex: &str) -> Result<String> {
    let row = sqlx::query(
        "SELECT display_name, auth_key_hash, failed_attempts, locked_until FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(&state.db)
    .await?;

    let Some(r) = row else {
        return Err(ServerError::Unauthorized("Credenciales inválidas".into()));
    };

    let now = Utc::now();
    if let Some(locked_str) = r.get::<Option<String>, _>("locked_until") {
        if let Ok(locked_dt) = DateTime::parse_from_rfc3339(&locked_str) {
            if locked_dt.with_timezone(&Utc) > now {
                return Err(ServerError::Unauthorized(
                    "Demasiados intentos fallidos. Espera antes de volver a intentar.".into(),
                ));
            }
        }
    }

    let expected_hash: String = r.get("auth_key_hash");
    let provided_hash = hash_auth_key(auth_key_hex);
    if !constant_time_eq(expected_hash.as_bytes(), provided_hash.as_bytes()) {
        let attempts = r.get::<i64, _>("failed_attempts") + 1;
        let locked_until = if attempts >= 5 {
            let exp = (attempts - 5).min(5) as u32;
            let secs = (30i64 * (1i64 << exp)).min(900);
            Some((now + Duration::seconds(secs)).to_rfc3339())
        } else {
            None
        };
        let _ = sqlx::query(
            "UPDATE users SET failed_attempts = ?, locked_until = ? WHERE username = ?",
        )
        .bind(attempts)
        .bind(&locked_until)
        .bind(username)
        .execute(&state.db)
        .await;

        return Err(ServerError::Unauthorized("Credenciales inválidas".into()));
    }

    let _ =
        sqlx::query("UPDATE users SET failed_attempts = 0, locked_until = NULL WHERE username = ?")
            .bind(username)
            .execute(&state.db)
            .await;

    Ok(r.get("display_name"))
}

/// `POST /api/auth/login` — autentica con AuthKey, registra dispositivo y devuelve bóveda cifrada.
pub async fn auth_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthLoginRequest>,
) -> Result<Json<AuthLoginResponse>> {
    let username = body.username.trim().to_lowercase();
    let display_name = verify_user_auth(&state, &username, &body.auth_key_hex).await?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO devices (id, name, public_key_hex, is_active, created_at, last_seen_at)
        VALUES (?, ?, ?, 1, ?, ?)
        ON CONFLICT(id) DO UPDATE SET name = excluded.name, last_seen_at = excluded.last_seen_at
        "#,
    )
    .bind(&body.device_id)
    .bind(&body.device_name)
    .bind("0".repeat(64))
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let vault = sqlx::query(
        "SELECT version, encrypted_payload, checksum_sha256, updated_at FROM user_vaults WHERE username = ?",
    )
    .bind(&username)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(AuthLoginResponse {
        username,
        display_name,
        version: vault.get("version"),
        encrypted_payload: vault.get("encrypted_payload"),
        checksum_sha256: vault.get("checksum_sha256"),
        updated_at: vault.get("updated_at"),
    }))
}

#[derive(Debug, Deserialize)]
pub struct AuthUploadVaultRequest {
    pub username: String,
    pub auth_key_hex: String,
    pub device_id: String,
    pub encrypted_payload: String,
}

/// `POST /api/auth/vault` — sube una nueva versión cifrada de la bóveda del usuario autenticado.
pub async fn auth_upload_vault(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AuthUploadVaultRequest>,
) -> Result<Json<SyncUploadResponse>> {
    let username = body.username.trim().to_lowercase();
    verify_user_auth(&state, &username, &body.auth_key_hex).await?;

    let now = Utc::now().to_rfc3339();
    let mut hasher = Sha256::new();
    hasher.update(body.encrypted_payload.as_bytes());
    let checksum = hex::encode(hasher.finalize());

    let row = sqlx::query("SELECT version FROM user_vaults WHERE username = ?")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?;
    let next_version = row.map_or(1, |r| r.get::<i64, _>("version") + 1);

    sqlx::query(
        r#"
        INSERT INTO user_vaults (username, version, device_id, encrypted_payload, checksum_sha256, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(username) DO UPDATE SET
            version = excluded.version,
            device_id = excluded.device_id,
            encrypted_payload = excluded.encrypted_payload,
            checksum_sha256 = excluded.checksum_sha256,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&username)
    .bind(next_version)
    .bind(&body.device_id)
    .bind(&body.encrypted_payload)
    .bind(&checksum)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(SyncUploadResponse {
        version: next_version,
        checksum_sha256: checksum,
        updated_at: now,
    }))
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_state() -> Arc<AppState> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory");
        crate::db::init_db(&pool).await.expect("init_db");
        Arc::new(AppState {
            db: pool,
            _server_master_key: "fake-test-server-key".into(),
        })
    }

    #[tokio::test]
    async fn registro_login_bloqueo_y_sync_usuario() {
        let state = test_state().await;
        let fake_auth_key = "a".repeat(64);
        let fake_salt = "b".repeat(64);

        let reg = auth_register(
            State(state.clone()),
            Json(AuthRegisterRequest {
                username: "test-fake-user".into(),
                display_name: "Usuario Falso".into(),
                kdf_salt_hex: fake_salt.clone(),
                kdf_memory_kib: 65536,
                kdf_iterations: 3,
                kdf_parallelism: 4,
                auth_key_hex: fake_auth_key.clone(),
                device_id: "test-fake-pc-a".into(),
                device_name: "PC-A".into(),
                encrypted_payload: "{\"fake\":true}".into(),
            }),
        )
        .await
        .expect("register");
        assert_eq!(reg.0.version, 1);

        // Usuario inexistente devuelve params falsos (no revela existencia)
        let missing_params = auth_params(
            State(state.clone()),
            Json(AuthParamsRequest {
                username: "no-existe-fake".into(),
            }),
        )
        .await
        .expect("missing params");
        assert_eq!(missing_params.0.kdf_salt_hex.len(), 64);

        // Login correcto
        let login_ok = auth_login(
            State(state.clone()),
            Json(AuthLoginRequest {
                username: "test-fake-user".into(),
                auth_key_hex: fake_auth_key.clone(),
                device_id: "test-fake-pc-b".into(),
                device_name: "PC-B".into(),
            }),
        )
        .await
        .expect("login ok");
        assert_eq!(login_ok.0.display_name, "Usuario Falso");

        // 5 intentos fallidos activan bloqueo progresivo
        let wrong_key = "f".repeat(64);
        for _ in 0..5 {
            let err = auth_login(
                State(state.clone()),
                Json(AuthLoginRequest {
                    username: "test-fake-user".into(),
                    auth_key_hex: wrong_key.clone(),
                    device_id: "test-fake-pc-b".into(),
                    device_name: "PC-B".into(),
                }),
            )
            .await
            .expect_err("debe fallar");
            assert!(matches!(err, ServerError::Unauthorized(_)));
        }

        // Incluso con la clave correcta queda bloqueado temporalmente
        let locked = auth_login(
            State(state.clone()),
            Json(AuthLoginRequest {
                username: "test-fake-user".into(),
                auth_key_hex: fake_auth_key,
                device_id: "test-fake-pc-b".into(),
                device_name: "PC-B".into(),
            }),
        )
        .await
        .expect_err("debe estar bloqueado");
        assert!(matches!(locked, ServerError::Unauthorized(_)));
    }
}
