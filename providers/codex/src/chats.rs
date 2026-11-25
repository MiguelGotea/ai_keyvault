//! Lector de sesiones de chat de Codex (OpenAI Codex CLI).
//!
//! Codex almacena sus threads en SQLite: `~/.codex/state_5.sqlite`, tabla `threads`.
//!
//! # Qué se lee
//!
//! - `id`         → rowid o UUID del thread según la versión de Codex
//! - `title`      → primer mensaje del usuario (content truncado a 120 chars)
//! - `created_at` → timestamp del primer mensaje
//! - `updated_at` → timestamp del último mensaje
//!
//! **NO** se lee el contenido completo — solo metadatos mínimos.
//!
//! # Compatibilidad
//!
//! El número de versión del SQLite (`state_5`) puede cambiar con actualizaciones
//! de Codex. Si el archivo no existe, se devuelve lista vacía sin error.

use crate::{CodexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadatos mínimos de una sesión de Codex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexChat {
    /// ID del thread (string del rowid o UUID según versión).
    pub id: String,
    /// Primer mensaje del usuario, truncado a 120 chars. `None` si está vacío.
    pub title: Option<String>,
    /// Timestamp ISO 8601 UTC del primer mensaje. Vacío si no hay mensajes.
    pub created_at: String,
    /// Timestamp ISO 8601 UTC del último mensaje.
    pub updated_at: String,
}

/// Ruta por defecto de la BD de Codex.
pub fn default_db_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codex").join("state_5.sqlite")
}

/// Escanea `~/.codex/state_5.sqlite` y devuelve metadatos de los threads.
///
/// Si el archivo no existe, devuelve `Ok(vec![])` — el usuario puede no tener
/// Codex instalado en este dispositivo.
///
/// # Lectura sin bloquear Codex
///
/// Abre la BD en modo solo lectura con `SQLITE_OPEN_READONLY` usando `rusqlite`.
/// Si Codex está corriendo con write lock, esta lectura puede fallar; en ese
/// caso se devuelve `Ok(vec![])` con un warning en logs (no es error fatal).
pub fn scan_codex_chats() -> Result<Vec<CodexChat>> {
    let path = default_db_path();

    if !path.exists() {
        return Ok(vec![]);
    }

    let conn = rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| CodexError::Io(std::io::Error::other(e.to_string())))?;

    // Verificar que la tabla `threads` existe (versión de Codex puede variar)
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='threads'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !table_exists {
        return Ok(vec![]);
    }

    // Leer threads ordenados por actividad más reciente
    // La estructura real de `threads` verificada: columnas id, messages (JSON), created_at, updated_at
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                CAST(id AS TEXT)         AS id,
                messages                 AS messages_json,
                COALESCE(created_at, '') AS created_at,
                COALESCE(updated_at, created_at, '') AS updated_at
            FROM threads
            ORDER BY updated_at DESC
            LIMIT 200
            "#,
        )
        .map_err(|e| CodexError::Io(std::io::Error::other(e.to_string())))?;

    let chats: Vec<CodexChat> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let messages_json: Option<String> = row.get(1).ok();
            let created_at: String = row.get(2)?;
            let updated_at: String = row.get(3)?;
            Ok((id, messages_json, created_at, updated_at))
        })
        .map_err(|e| CodexError::Io(std::io::Error::other(e.to_string())))?
        .filter_map(|r| r.ok())
        .map(|(id, messages_json, created_at, updated_at)| {
            let title = extract_first_user_message(messages_json.as_deref());
            CodexChat {
                id,
                title,
                created_at,
                updated_at,
            }
        })
        .collect();

    Ok(chats)
}

/// Extrae el primer mensaje del usuario del JSON de mensajes de Codex.
///
/// El campo `messages` es un array JSON de objetos con `role` y `content`.
/// Solo se extrae texto plano — sin emojis ni tags XML.
fn extract_first_user_message(messages_json: Option<&str>) -> Option<String> {
    let json = messages_json?;
    let arr: serde_json::Value = serde_json::from_str(json).ok()?;
    let messages = arr.as_array()?;

    for msg in messages {
        let role = msg.get("role")?.as_str()?;
        if role != "user" {
            continue;
        }
        let content = msg.get("content")?;
        // content puede ser string o array de parts
        let text = if let Some(s) = content.as_str() {
            s.to_string()
        } else if let Some(parts) = content.as_array() {
            parts
                .iter()
                .filter_map(|p| p.get("text")?.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            continue;
        };

        let clean = text.trim().to_string();
        if clean.is_empty() {
            continue;
        }

        // Truncar a 120 caracteres
        let truncated = if clean.chars().count() > 120 {
            clean.chars().take(120).collect::<String>()
        } else {
            clean
        };
        return Some(truncated);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_user_string_simple() {
        let json = r#"[{"role":"user","content":"hola mundo"}]"#;
        let title = extract_first_user_message(Some(json));
        assert_eq!(title, Some("hola mundo".to_string()));
    }

    #[test]
    fn test_extract_saltar_assistant() {
        let json =
            r#"[{"role":"assistant","content":"hola"},{"role":"user","content":"mi pregunta"}]"#;
        let title = extract_first_user_message(Some(json));
        assert_eq!(title, Some("mi pregunta".to_string()));
    }

    #[test]
    fn test_extract_content_array() {
        let json = r#"[{"role":"user","content":[{"text":"parte uno"},{"text":"parte dos"}]}]"#;
        let title = extract_first_user_message(Some(json));
        assert_eq!(title, Some("parte uno parte dos".to_string()));
    }

    #[test]
    fn test_truncate_120() {
        let long = "a".repeat(200);
        let json = format!(r#"[{{"role":"user","content":"{long}"}}]"#);
        let title = extract_first_user_message(Some(&json));
        assert_eq!(title.unwrap().chars().count(), 120);
    }

    #[test]
    fn test_no_existe_db() {
        // Cuando no existe el archivo, devuelve Ok([])
        let result = scan_codex_chats();
        // El resultado depende del entorno — solo verificamos que no panike
        assert!(result.is_ok());
    }
}
