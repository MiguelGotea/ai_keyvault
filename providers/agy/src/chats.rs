//! Lector de metadatos de chats de agy (Antigravity).
//!
//! Escanea `~/.gemini/antigravity/brain/` y extrae metadatos mínimos de cada
//! conversación a partir de su `transcript.jsonl`.
//!
//! # Qué se lee
//!
//! - `id`         → nombre del directorio (UUID)
//! - `title`      → primer `USER_INPUT` truncado a 120 caracteres
//! - `created_at` → `created_at` del step_index 0
//! - `updated_at` → `created_at` del último step en el archivo
//! - `archived`   → siempre `false` (agy no expone archivado en el FS)
//!
//! **NO** se lee el contenido de los mensajes — solo metadatos.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Modelos públicos ──────────────────────────────────────────────────────────

/// Metadatos mínimos de una conversación de agy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgyChat {
    /// UUID del chat (nombre del directorio en brain/).
    pub id: String,
    /// Primer mensaje del usuario, truncado a 120 chars. `None` si no hay msgs.
    pub title: Option<String>,
    /// Timestamp ISO 8601 UTC del primer step.
    pub created_at: String,
    /// Timestamp ISO 8601 UTC del último step registrado.
    pub updated_at: String,
    /// Siempre `false` — agy no expone archivado en el FS local.
    pub archived: bool,
}

// ── Tipos internos de deserialización ────────────────────────────────────────

/// Un step del transcript (solo los campos que necesitamos).
#[derive(Debug, Deserialize)]
struct TranscriptStep {
    #[serde(rename = "type")]
    step_type: Option<String>,
    created_at: Option<String>,
    content: Option<String>,
}

// ── API pública ───────────────────────────────────────────────────────────────

/// Ruta base del directorio `brain/` de agy.
///
/// En Windows: `%USERPROFILE%\.gemini\antigravity\brain`
/// En Linux:   `~/.gemini/antigravity/brain`
pub fn brain_dir() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".gemini").join("antigravity").join("brain"))
}

/// Escanea `brain_path` y devuelve metadatos de todos los chats encontrados.
///
/// Ignora silenciosamente los directorios sin `transcript.jsonl` o con
/// transcripts vacíos / corruptos.
pub fn scan_chats(brain_path: &Path) -> Vec<AgyChat> {
    let entries = match std::fs::read_dir(brain_path) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut chats = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let transcript_path = path
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");

        if let Some(chat) = parse_transcript(&id, &transcript_path) {
            chats.push(chat);
        }
    }

    // Más reciente primero
    chats.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    chats
}

// ── Implementación interna ────────────────────────────────────────────────────

fn parse_transcript(id: &str, path: &Path) -> Option<AgyChat> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut created_at: Option<String> = None;
    let mut updated_at: Option<String> = None;
    let mut title: Option<String> = None;

    for line in content.lines() {
        let step: TranscriptStep = match serde_json::from_str(line) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // created_at del primer step = fecha de creación del chat
        if created_at.is_none() {
            created_at = step.created_at.clone();
        }

        // Actualizar siempre — el último step válido gana
        if let Some(ts) = &step.created_at {
            updated_at = Some(ts.clone());
        }

        // Título = primer mensaje USER_INPUT
        if title.is_none() && step.step_type.as_deref() == Some("USER_INPUT") {
            if let Some(raw) = &step.content {
                title = Some(extract_title(raw));
            }
        }
    }

    // Necesitamos al menos las fechas para construir el metadato
    let created_at = created_at?;
    let updated_at = updated_at.unwrap_or_else(|| created_at.clone());

    Some(AgyChat {
        id: id.to_string(),
        title,
        created_at,
        updated_at,
        archived: false,
    })
}

/// Extrae el título del contenido del primer USER_INPUT.
///
/// El content puede contener tags XML como `<USER_REQUEST>...</USER_REQUEST>`.
/// Prioriza el contenido dentro de `<USER_REQUEST>` si existe, si no usa el
/// texto completo sin tags. Luego trunca a 120 caracteres.
fn extract_title(raw: &str) -> String {
    // Intentar extraer solo el contenido de <USER_REQUEST>...</USER_REQUEST>
    let text = if let Some(start) = raw.find("<USER_REQUEST>") {
        let after_open = &raw[start + "<USER_REQUEST>".len()..];
        if let Some(end) = after_open.find("</USER_REQUEST>") {
            after_open[..end].trim().to_string()
        } else {
            after_open.trim().to_string()
        }
    } else {
        // Sin tag USER_REQUEST — quitar cualquier otro tag XML
        strip_xml_tags(raw)
    };

    let trimmed = text.trim();

    if trimmed.chars().count() <= 120 {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(117).collect();
        format!("{}...", truncated.trim_end())
    }
}

fn strip_xml_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    // Colapsar espacios múltiples y newlines
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_transcript(dir: &Path, lines: &[&str]) {
        let log_dir = dir.join(".system_generated").join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let mut f = std::fs::File::create(log_dir.join("transcript.jsonl")).unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
    }

    #[test]
    fn test_extrae_titulo_y_fechas() {
        let tmp = TempDir::new().unwrap();
        let chat_dir = tmp.path().join("abc-123");
        std::fs::create_dir(&chat_dir).unwrap();

        write_transcript(
            &chat_dir,
            &[
                r#"{"step_index":0,"type":"USER_INPUT","created_at":"2026-11-01T10:00:00Z","content":"Hola, necesito ayuda"}"#,
                r#"{"step_index":1,"type":"PLANNER_RESPONSE","created_at":"2026-11-01T10:01:00Z","content":"Claro"}"#,
            ],
        );

        let chats = scan_chats(tmp.path());
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].id, "abc-123");
        assert_eq!(chats[0].title.as_deref(), Some("Hola, necesito ayuda"));
        assert_eq!(chats[0].created_at, "2026-11-01T10:00:00Z");
        assert_eq!(chats[0].updated_at, "2026-11-01T10:01:00Z");
        assert!(!chats[0].archived);
    }

    #[test]
    fn test_limpia_tags_xml() {
        let raw = "<USER_REQUEST>\nEjecuta este plan\n</USER_REQUEST>\n<METADATA>datos</METADATA>";
        let title = extract_title(raw);
        assert_eq!(title, "Ejecuta este plan");
    }

    #[test]
    fn test_trunca_titulo_largo() {
        let largo = "a".repeat(200);
        let title = extract_title(&largo);
        assert!(title.chars().count() <= 120);
        assert!(title.ends_with("..."));
    }

    #[test]
    fn test_ignora_dirs_sin_transcript() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("sin-transcript")).unwrap();
        let chats = scan_chats(tmp.path());
        assert!(chats.is_empty());
    }

    #[test]
    fn test_multiples_chats_orden_por_fecha() {
        let tmp = TempDir::new().unwrap();

        for (uuid, ts) in [
            ("chat-a", "2026-11-05T10:00:00Z"),
            ("chat-b", "2026-11-10T10:00:00Z"),
        ] {
            let d = tmp.path().join(uuid);
            std::fs::create_dir(&d).unwrap();
            write_transcript(
                &d,
                &[&format!(
                    r#"{{"step_index":0,"type":"USER_INPUT","created_at":"{ts}","content":"msg"}}"#
                )],
            );
        }

        let chats = scan_chats(tmp.path());
        assert_eq!(chats.len(), 2);
        // Más reciente primero
        assert_eq!(chats[0].id, "chat-b");
    }
}
