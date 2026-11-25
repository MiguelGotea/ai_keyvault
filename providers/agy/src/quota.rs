//! Parser de la salida de `agy --print /usage` para leer cuota de los pools.
//!
//! La salida real verificada (2026-11-25) tiene este formato:
//!
//! ```text
//! Quota:
//! Gemini Models          Weekly Limit Remaining     100%  2026-12-01 19:19 CST
//! Claude and GPT models  Weekly Limit Remaining     49%   2026-12-01 18:57 CST
//! Claude and GPT models  Five Hour Limit Remaining  49%   2026-11-24 23:57 CST
//! ```
//!
//! Cada línea contiene (separado por espacios múltiples):
//!   - Nombre del pool (`Gemini Models` o `Claude and GPT models`)
//!   - Tipo de ventana (`Weekly Limit Remaining` o `Five Hour Limit Remaining`)
//!   - Porcentaje restante (`100%`, `49%`, etc.)
//!   - Timestamp del próximo reinicio (ISO 8601 o local `YYYY-MM-DD HH:MM TZ`)

use crate::{AgyError, Result};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Identificador de pool de agy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgyPool {
    /// Pool de modelos Gemini (ventana semanal).
    Gemini,
    /// Pool de modelos Claude + GPT (ventana semanal).
    ClaudeGpt,
}

impl AgyPool {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgyPool::Gemini => "gemini-weekly",
            AgyPool::ClaudeGpt => "3p-weekly",
        }
    }
}

/// Lectura de cuota de un pool de agy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgyQuotaReading {
    pub pool: AgyPool,
    /// Ventana de tiempo: `"weekly"` o `"5h"`.
    pub window: String,
    /// Porcentaje RESTANTE (0.0 – 100.0). `100.0` = cuota disponible completa.
    pub percent_remaining: f64,
    /// Porcentaje CONSUMIDO (0.0 – 100.0). Derivado de `percent_remaining`.
    pub percent_used: f64,
    /// Timestamp del próximo reinicio (ISO 8601 o string local). `None` si no se pudo parsear.
    pub reset_at: Option<String>,
    /// Timestamp en que se tomó esta lectura.
    pub read_at: String,
    /// Correo de la cuenta activa al momento de la lectura.
    pub account_email: Option<String>,
}

/// Ejecuta `agy /usage` y devuelve las lecturas de los dos pools.
///
/// # Errores
/// - Si `agy` no está instalado o falla.
/// - Si la salida no tiene el formato esperado.
pub fn read_quota() -> Result<Vec<AgyQuotaReading>> {
    let output = Command::new("agy")
        .args(["--print", "/usage"])
        .output()
        .map_err(|e| AgyError::CommandError(format!("No se pudo ejecutar 'agy': {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AgyError::CommandError(format!(
            "agy /usage salió con error: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut readings = parse_usage_output(&stdout)?;

    // Enriquecer con email de la cuenta activa
    let email = read_account_email();
    for r in &mut readings {
        r.account_email = email.clone();
    }

    Ok(readings)
}

/// Intenta obtener el email de la cuenta agy activa.
///
/// Estrategia (en orden):
/// 1. Windows Credential Manager (`gemini:antigravity` → `id_token_email`)
/// 2. `agy --print /account` como fallback subprocess
///
/// Devuelve `None` si ninguna estrategia funciona.
pub fn read_account_email() -> Option<String> {
    // 1. Credential Manager — más confiable, no lanza subprocess
    {
        use crate::credential::AgyCredentialManager;
        if let Ok(cred) = AgyCredentialManager::read() {
            if let Some(email) = cred.email {
                return Some(email);
            }
        }
    }

    // 2. Fallback: `agy --print /account`
    if let Ok(output) = Command::new("agy").args(["--print", "/account"]).output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let email_re = Regex::new(r"[\w.+%-]+@[\w-]+\.[\w.]+").unwrap();
            if let Some(m) = email_re.find(&text) {
                return Some(m.as_str().to_string());
            }
        }
    }

    None
}

/// Parsea la salida de texto de `agy --print /usage` en lecturas estructuradas.
///
/// Formato soportado (una línea por pool, campos separados por espacios múltiples):
/// ```text
/// Quota:
/// Gemini Models          Weekly Limit Remaining     100%  2026-12-01 19:19 CST
/// Claude and GPT models  Weekly Limit Remaining     49%   2026-12-01 18:57 CST
/// Claude and GPT models  Five Hour Limit Remaining  49%   2026-11-24 23:57 CST
/// ```
/// También soporta el formato ISO 8601 anterior.
///
/// Esta función es pública para permitir tests unitarios con salida fake.
pub fn parse_usage_output(text: &str) -> Result<Vec<AgyQuotaReading>> {
    let now = Utc::now();
    let read_at = now.to_rfc3339();

    // Porcentaje: "100%", "62%", "97.48%"
    let pct_re = Regex::new(r"(\d{1,3}(?:\.\d+)?)%").unwrap();
    // Timestamp: ISO 8601 "2026-12-02T00:32:25Z" O local "2026-12-01 19:19 CST"
    let ts_re = Regex::new(
        r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z|\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}\s+[A-Z]{2,4})",
    )
    .unwrap();

    let mut readings = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detectar pool por el inicio de la línea (case-insensitive)
        let lower = trimmed.to_lowercase();
        let pool = if lower.starts_with("gemini") {
            AgyPool::Gemini
        } else if lower.starts_with("claude") {
            AgyPool::ClaudeGpt
        } else {
            continue; // línea no relevante (ej. "Quota:", cabeceras)
        };

        // Detectar ventana de tiempo: "Five Hour" → "5h", cualquier otro → "weekly"
        let window = if lower.contains("five hour") {
            "5h".to_string()
        } else {
            "weekly".to_string()
        };

        let percent_remaining = pct_re
            .captures(trimmed)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(100.0);

        // El timestamp al final indica cuando se reinicia el límite
        let reset_at = ts_re
            .captures(trimmed)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        readings.push(AgyQuotaReading {
            pool,
            window,
            percent_remaining,
            percent_used: 100.0 - percent_remaining,
            reset_at,
            read_at: read_at.clone(),
            account_email: None, // Se setea por el caller (read_quota / collect_agy)
        });
    }

    if readings.is_empty() {
        return Err(AgyError::CommandError(
            "No se pudieron parsear lecturas de cuota de la salida de 'agy /usage'".to_string(),
        ));
    }

    Ok(readings)
}

/// Verifica que el token de agy es válido ejecutando `agy models`.
/// No consume cuota de mensajes.
pub fn verify_token() -> Result<Vec<String>> {
    let output = Command::new("agy")
        .arg("models")
        .output()
        .map_err(|e| AgyError::CommandError(format!("No se pudo ejecutar 'agy models': {}", e)))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AgyError::VerificationFailed(err));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models: Vec<String> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().next().unwrap_or(l).to_string())
        .collect();

    Ok(models)
}
