//! Lectura y escritura atómica de `~/.codex/auth.json`.
//!
//! **Formato real verificado en campo (2026-11-22):**
//! ```json
//! {
//!   "tokens": {
//!     "access_token": "...",
//!     "id_token": "<JWT>",
//!     "refresh_token": "...",
//!     "token_type": "Bearer",
//!     "expiry_time": 1234567890
//!   }
//! }
//! ```
//!
//! El `id_token` es un JWT cuyo payload (base64url sin padding) contiene:
//! `email`, `exp` (Unix timestamp), y posiblemente `plan`/`subscription_end`.
//!
//! **Escritura atómica en NTFS:**
//! ```text
//! 1. Escribir auth.json.tmp con el nuevo contenido
//! 2. Renombrar .tmp → auth.json  (atómico en NTFS con MoveFileExW REPLACE_EXISTING)
//! ```
//! El archivo `.bak` se conserva para permitir revertir.

use crate::{CodexError, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Ruta por defecto de auth.json de Codex.
pub fn default_auth_path() -> PathBuf {
    dirs_path().join("auth.json")
}

fn dirs_path() -> PathBuf {
    // Windows: C:\Users\<usuario>\.codex\
    // Linux/Mac: ~/.codex/
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".codex")
}

// ─── Estructuras de datos ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexAuthFile {
    pub tokens: CodexTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTokens {
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    /// Timestamp Unix (segundos) del vencimiento.
    pub expiry_time: Option<i64>,
}

/// Credencial procesada lista para usar.
#[derive(Debug, Clone)]
pub struct CodexAuth {
    pub email: Option<String>,
    pub access_token: String,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
    /// Ruta del archivo del que se leyó.
    pub source_path: PathBuf,
}

impl CodexAuth {
    /// Devuelve `true` si el access_token sigue vigente (margen de 60 s).
    pub fn is_valid(&self) -> bool {
        match &self.expiry {
            None => true,
            Some(exp) => (*exp - Utc::now()).num_seconds() > 60,
        }
    }

    pub fn minutes_remaining(&self) -> Option<i64> {
        self.expiry
            .map(|exp| (exp - Utc::now()).num_seconds() / 60)
    }
}

// ─── Manager ─────────────────────────────────────────────────────────────────

pub struct CodexAuthManager {
    path: PathBuf,
}

impl CodexAuthManager {
    pub fn new() -> Self {
        Self { path: default_auth_path() }
    }

    /// Para tests: usar una ruta personalizada.
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Lee auth.json y lo parsea.
    pub fn read(&self) -> Result<CodexAuth> {
        if !self.path.exists() {
            return Err(CodexError::AuthFileNotFound(
                self.path.display().to_string(),
            ));
        }

        let content = std::fs::read_to_string(&self.path)?;
        let file: CodexAuthFile = serde_json::from_str(&content)
            .map_err(|e| CodexError::InvalidJson(e.to_string()))?;

        let expiry = file
            .tokens
            .expiry_time
            .map(|ts| Utc.timestamp_opt(ts, 0).single())
            .flatten();

        let email = file
            .tokens
            .id_token
            .as_deref()
            .and_then(|jwt| decode_jwt_email(jwt));

        Ok(CodexAuth {
            email,
            access_token: file.tokens.access_token,
            id_token: file.tokens.id_token,
            refresh_token: file.tokens.refresh_token,
            expiry,
            source_path: self.path.clone(),
        })
    }

    /// Escribe auth.json de forma atómica: .tmp → rename.
    /// Guarda el anterior como .bak para revertir.
    pub fn write_atomic(&self, auth: &CodexAuth) -> Result<()> {
        let content = serde_json::to_string_pretty(&CodexAuthFile {
            tokens: CodexTokens {
                access_token: auth.access_token.clone(),
                id_token: auth.id_token.clone(),
                refresh_token: auth.refresh_token.clone(),
                token_type: Some("Bearer".to_string()),
                expiry_time: auth.expiry.map(|e| e.timestamp()),
            },
        })?;

        let tmp_path = self.path.with_extension("json.tmp");
        let bak_path = self.path.with_extension("json.bak");

        // Paso 1: escribir a .tmp
        std::fs::write(&tmp_path, content.as_bytes())
            .map_err(|e| CodexError::AtomicWrite(format!("No se pudo escribir .tmp: {}", e)))?;

        // Paso 2: backup del actual → .bak (si existe)
        if self.path.exists() {
            std::fs::copy(&self.path, &bak_path)
                .map_err(|e| CodexError::AtomicWrite(format!("No se pudo crear .bak: {}", e)))?;
        }

        // Paso 3: rename atómico .tmp → auth.json
        std::fs::rename(&tmp_path, &self.path)
            .map_err(|e| CodexError::AtomicWrite(format!("Rename atómico falló: {}", e)))?;

        Ok(())
    }

    /// Restaura auth.json desde .bak (deshacer).
    pub fn revert(&self) -> Result<()> {
        let bak_path = self.path.with_extension("json.bak");
        if !bak_path.exists() {
            return Err(CodexError::AuthFileNotFound(
                bak_path.display().to_string(),
            ));
        }
        std::fs::rename(&bak_path, &self.path)
            .map_err(|e| CodexError::AtomicWrite(format!("Revert falló: {}", e)))?;
        Ok(())
    }
}

impl Default for CodexAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Decodifica el payload base64url del JWT (sin verificar firma) y extrae el email.
fn decode_jwt_email(jwt: &str) -> Option<String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    // Añadir padding base64 si falta
    let payload_b64 = parts[1];
    let padded = match payload_b64.len() % 4 {
        2 => format!("{}==", payload_b64),
        3 => format!("{}=", payload_b64),
        _ => payload_b64.to_string(),
    };
    // base64url → base64 estándar
    let standard = padded.replace('-', "+").replace('_', "/");
    let decoded = base64_decode(&standard).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    payload["email"].as_str().map(|s| s.to_string())
}

/// Decodificador base64 sin dependencia externa.
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    // Usamos el decodificador de la stdlib vía iterador manual.
    // Para no agregar la crate `base64`, implementamos decodificación básica.
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lut[c as usize] = i as u8;
    }

    let input = input.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;

    for &c in input {
        let v = lut[c as usize];
        if v == 255 {
            continue; // saltar caracteres inválidos
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}
