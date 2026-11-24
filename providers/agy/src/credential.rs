//! Lectura y escritura de credenciales de agy desde Windows Credential Manager.
//!
//! **Formato real verificado en campo (2026-11-22):**
//! - Target:  `gemini:antigravity`
//! - Tipo:    Generic (LegacyGeneric)
//! - Usuario: `antigravity`
//! - Blob:    JSON de ~653 bytes en UTF-8
//!
//! Estructura del blob:
//! ```json
//! {
//!   "token": {
//!     "access_token": "ya29...",
//!     "token_type": "Bearer",
//!     "refresh_token": "...",
//!     "expiry": "2026-11-22T23:21:00+00:00",
//!     "expires_in": 3599
//!   },
//!   "auth_method": "...",
//!   "id_token_email": "usuario@gmail.com",
//!   "writer_id": "..."
//! }
//! ```

use crate::{AgyError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const CREDENTIAL_TARGET: &str = "gemini:antigravity";
pub const CREDENTIAL_USERNAME: &str = "antigravity";

/// Blob completo del Credential Manager de agy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgyCredentialBlob {
    pub token: AgyTokenInner,
    pub auth_method: Option<String>,
    pub id_token_email: Option<String>,
    pub writer_id: Option<String>,
}

/// Token interno dentro del blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgyTokenInner {
    pub access_token: String,
    pub token_type: Option<String>,
    pub refresh_token: Option<String>,
    pub expiry: Option<String>,
    pub expires_in: Option<i64>,
}

/// Credencial procesada lista para usar.
#[derive(Debug, Clone)]
pub struct AgyCredential {
    pub email: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
}

impl AgyCredential {
    /// Devuelve `true` si el token sigue vigente (con margen de 60 s).
    pub fn is_valid(&self) -> bool {
        match &self.expiry {
            None => true, // sin fecha de vencimiento asumimos válido
            Some(exp) => (*exp - Utc::now()).num_seconds() > 60,
        }
    }

    /// Minutos restantes de vida del token. `None` si no hay fecha de vencimiento.
    pub fn minutes_remaining(&self) -> Option<i64> {
        self.expiry.map(|exp| (exp - Utc::now()).num_seconds() / 60)
    }
}

/// Interfaz de alto nivel sobre Credential Manager.
pub struct AgyCredentialManager;

impl AgyCredentialManager {
    /// Lee la credencial activa de agy desde el Credential Manager.
    pub fn read() -> Result<AgyCredential> {
        let raw = platform::read_raw_blob(CREDENTIAL_TARGET)?;
        Self::parse_blob(&raw)
    }

    /// Escribe una nueva credencial en el Credential Manager.
    /// La credencial anterior se sobreescribe atómicamente (CredWrite es atómico en Windows).
    pub fn write(credential: &AgyCredential) -> Result<()> {
        // Reconstruir el blob JSON a partir de la credencial
        let blob = AgyCredentialBlob {
            token: AgyTokenInner {
                access_token: credential.access_token.clone(),
                token_type: Some("Bearer".to_string()),
                refresh_token: credential.refresh_token.clone(),
                expiry: credential.expiry.map(|e| e.to_rfc3339()),
                expires_in: credential
                    .expiry
                    .map(|e| (e - Utc::now()).num_seconds().max(0)),
            },
            auth_method: None,
            id_token_email: credential.email.clone(),
            writer_id: None,
        };
        let json = serde_json::to_string(&blob)?;
        platform::write_raw_blob(CREDENTIAL_TARGET, CREDENTIAL_USERNAME, json.as_bytes())
    }

    /// Parsea el blob raw (bytes) en un `AgyCredential`.
    fn parse_blob(raw: &[u8]) -> Result<AgyCredential> {
        // El blob puede venir en UTF-8 o UTF-16-LE (Windows guarda en UTF-16).
        let text = decode_blob(raw)?;
        let blob: AgyCredentialBlob =
            serde_json::from_str(&text).map_err(|e| AgyError::InvalidBlob(e.to_string()))?;

        let expiry = blob
            .token
            .expiry
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(AgyCredential {
            email: blob.id_token_email,
            access_token: blob.token.access_token,
            refresh_token: blob.token.refresh_token,
            expiry,
        })
    }

    /// Helper para tests: parsea un blob de bytes directamente (sin Credential Manager real).
    pub fn parse_blob_for_test(raw: &[u8]) -> Result<AgyCredential> {
        Self::parse_blob(raw)
    }
}

/// Intenta decodificar el blob como UTF-8 primero, luego como UTF-16-LE.
fn decode_blob(raw: &[u8]) -> Result<String> {
    // UTF-8
    if let Ok(s) = std::str::from_utf8(raw) {
        return Ok(s.to_string());
    }
    // UTF-16-LE (Windows nativo)
    if raw.len() % 2 == 0 {
        let u16_data: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&u16_data) {
            return Ok(s);
        }
    }
    Err(AgyError::InvalidBlob(
        "El blob no es UTF-8 ni UTF-16-LE válido".to_string(),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Implementación real para Windows
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn read_raw_blob(target: &str) -> Result<Vec<u8>> {
        let target_w = to_wide(target);
        let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();

        let ok = unsafe { CredReadW(target_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pcred) };
        if ok == 0 {
            return Err(AgyError::CredentialNotFound(target.to_string()));
        }

        let cred = unsafe { &*pcred };
        let size = cred.CredentialBlobSize as usize;
        let mut blob = vec![0u8; size];
        unsafe {
            std::ptr::copy_nonoverlapping(cred.CredentialBlob, blob.as_mut_ptr(), size);
            CredFree(pcred as *mut _);
        }
        Ok(blob)
    }

    pub fn write_raw_blob(target: &str, username: &str, blob: &[u8]) -> Result<()> {
        let target_w = to_wide(target);
        let username_w = to_wide(username);

        let mut cred: CREDENTIALW = unsafe { std::mem::zeroed() };
        cred.Type = CRED_TYPE_GENERIC;
        cred.TargetName = target_w.as_ptr() as _;
        cred.UserName = username_w.as_ptr() as _;
        cred.CredentialBlob = blob.as_ptr() as _;
        cred.CredentialBlobSize = blob.len() as u32;
        cred.Persist = CRED_PERSIST_LOCAL_MACHINE;

        let ok = unsafe { CredWriteW(&cred, 0) };
        if ok == 0 {
            return Err(AgyError::CredentialWrite("CredWriteW falló".to_string()));
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stub para plataformas no-Windows (compilación en CI Linux)
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;

    pub fn read_raw_blob(_target: &str) -> Result<Vec<u8>> {
        // En Linux/CI se usa la implementación fake del test
        Err(AgyError::CredentialNotFound(
            "Windows Credential Manager no disponible en esta plataforma".to_string(),
        ))
    }

    pub fn write_raw_blob(_target: &str, _username: &str, _blob: &[u8]) -> Result<()> {
        Err(AgyError::CredentialWrite(
            "Windows Credential Manager no disponible en esta plataforma".to_string(),
        ))
    }
}
