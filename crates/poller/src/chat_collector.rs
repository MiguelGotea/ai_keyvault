//! Recolector de metadatos de chats de agy.
//!
//! Se invoca desde el scheduler tras cada ciclo de cuota. Escanea
//! `~/.gemini/antigravity/brain/` y sube los metadatos al servidor.
//!
//! **Privacidad:** solo se suben id, título (primer mensaje truncado),
//! fechas y bandera `archived`. El contenido de los mensajes NO se sube.

use crate::PollerConfig;
use ai_keyvault_provider_agy::{brain_dir, scan_chats, AgyChat};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Tipos de request al servidor ─────────────────────────────────────────────

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

/// Recolecta metadatos de chats de agy y los sube al servidor.
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
        debug!(path = %brain.display(), "brain/ no existe — se omite recolección de chats");
        return 0;
    }

    let chats = scan_chats(&brain);
    if chats.is_empty() {
        debug!("No se encontraron chats de agy");
        return 0;
    }

    let count = chats.len() as u32;
    debug!(count, "Chats agy recolectados");

    let payload: Vec<ChatSessionPayload> = chats.into_iter().map(chat_to_payload).collect();

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
                count,
                "📤 Metadatos de chats agy sincronizados"
            );
            count
        }
        Ok(resp) => {
            warn!(
                status = %resp.status(),
                "⚠️ El servidor rechazó los metadatos de chats"
            );
            0
        }
        Err(e) => {
            warn!(error = %e, "⚠️ Error al subir metadatos de chats");
            0
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn chat_to_payload(chat: AgyChat) -> ChatSessionPayload {
    ChatSessionPayload {
        id: chat.id,
        provider: "agy".to_string(),
        title: chat.title,
        created_at: chat.created_at,
        updated_at: chat.updated_at,
        archived: chat.archived,
    }
}
