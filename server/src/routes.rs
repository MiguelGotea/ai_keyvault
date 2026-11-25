use crate::error::{Result, ServerError};
use axum::{
    extract::{Path, State},
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
    #[allow(dead_code)]
    pub server_master_key: String,
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
