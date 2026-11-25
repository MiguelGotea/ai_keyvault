//! Recolector de metadatos de chats — agy y Codex.
//!
//! Se invoca desde el scheduler tras cada ciclo de cuota. Escanea los
//! directorios/BDs locales y sube los metadatos al servidor.
//!
//! **Privacidad:** solo se suben id, título (primer mensaje truncado),
//! fechas y bandera `archived`. El contenido de los mensajes NO se sube.
//!
//! # Proveedores soportados
//! - **agy** — `~/.gemini/antigravity/brain/<uuid>/transcript.jsonl`
//! - **codex** — `~/.codex/state_5.sqlite` tabla `threads`

use crate::PollerConfig;
use ai_keyvault_provider_agy::{brain_dir, scan_chats, AgyChat};
use ai_keyvault_provider_codex::scan_codex_chats;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Tipos de request al servidor ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct PostChatSessionsRequest<'a> {
    account_id: &'a str,
    device_id: &'a str,
    chats: Vec<ChatSessionPayload>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatSessionPayload {
    id: String,
    provider: String,
    title: Option<String>,
    created_at: String,
    updated_at: String,
    archived: bool,
}

// ── API pública ───────────────────────────────────────────────────────────────

/// Recolecta metadatos de chats de **agy** y los sube al servidor.
///
/// Devuelve el número de chats procesados, o 0 si no hay nada que subir.
pub async fn collect_and_upload(
    client: &reqwest::Client,
    config: &PollerConfig,
    account_id: &str,
) -> u32 {
    let brain = match brain_dir() {
        Some(p) => p,
        None => {
            warn!("No se pudo determinar el directorio brain/ de agy");
            return 0;
        }
    };

    if !brain.exists() {
        debug!(path = %brain.display(), "brain/ no existe — se omite recolección de chats agy");
        return 0;
    }

    let chats = scan_chats(&brain);
    if chats.is_empty() {
        debug!("No se encontraron chats de agy");
        return 0;
    }

    let count = chats.len() as u32;
    debug!(count, "Chats agy recolectados");

    let payload: Vec<ChatSessionPayload> = chats.into_iter().map(agy_to_payload).collect();

    upload_chats(client, config, account_id, payload).await;
    count
}

/// Recolecta metadatos de chats de **Codex** y los sube al servidor.
///
/// Usa `account_id` del `auth.json` de Codex. Si el archivo no existe
/// (Codex no instalado), retorna 0 sin error.
pub async fn collect_and_upload_codex(
    client: &reqwest::Client,
    config: &PollerConfig,
    account_id: &str,
) -> u32 {
    match scan_codex_chats() {
        Ok(chats) if chats.is_empty() => {
            debug!("No se encontraron chats de Codex (BD vacía o no existe)");
            0
        }
        Ok(chats) => {
            let count = chats.len() as u32;
            debug!(count, "Chats Codex recolectados");

            let payload: Vec<ChatSessionPayload> =
                chats.into_iter().map(codex_to_payload).collect();

            upload_chats(client, config, account_id, payload).await;
            count
        }
        Err(e) => {
            warn!(error = %e, "⚠️ Error al recolectar chats de Codex");
            0
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn upload_chats(
    client: &reqwest::Client,
    config: &PollerConfig,
    account_id: &str,
    payload: Vec<ChatSessionPayload>,
) {
    let url = format!("{}/api/chats", config.server_url.trim_end_matches('/'));
    let body = PostChatSessionsRequest {
        account_id,
        device_id: &config.device_id,
        chats: payload,
    };

    match client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            info!(
                account_id = %account_id,
                "📤 Metadatos de chats sincronizados"
            );
        }
        Ok(resp) => {
            warn!(
                status = %resp.status(),
                "⚠️ El servidor rechazó los metadatos de chats"
            );
        }
        Err(e) => {
            warn!(error = %e, "⚠️ Error al subir metadatos de chats");
        }
    }
}

fn agy_to_payload(chat: AgyChat) -> ChatSessionPayload {
    ChatSessionPayload {
        id: chat.id,
        provider: "agy".to_string(),
        title: chat.title,
        created_at: chat.created_at,
        updated_at: chat.updated_at,
        archived: chat.archived,
    }
}

fn codex_to_payload(chat: ai_keyvault_provider_codex::CodexChat) -> ChatSessionPayload {
    ChatSessionPayload {
        id: chat.id,
        provider: "codex".to_string(),
        title: chat.title,
        created_at: chat.created_at,
        updated_at: chat.updated_at,
        archived: false,
    }
}
