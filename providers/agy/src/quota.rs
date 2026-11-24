//! Parser de la salida de `agy /usage` para leer cuota de los dos pools.
//!
//! La salida real verificada (2026-11-22) tiene este formato de texto:
//!
//! ```text
//! /usage
//! └ Models & Quota
//!
//!   Account: usuario@gmail.com
//!
//! GEMINI MODELS
//!   Models within this group: Gemini 3.8 Flash
//!
//!   Weekly Limit Remaining
//!     [████████████████████████░] 98.41%
//!     Refreshes in 167h 55m
//!
//! CLAUDE AND GPT MODELS
//!   Models within this group: Claude Sonnet 4.6, Claude Opus 4.6, GPT-OSS 120B
//!
//!   Weekly Limit Remaining
//!     [██████████████████████████] 97.48%
//!     Refreshes in 167h 59m
//! ```
//!
//! Cuando la cuota está al 100%, la línea de refresh dice "Quota available" (sin tiempo).

use crate::{AgyError, Result};
use chrono::{Duration, Utc};
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
    /// Porcentaje RESTANTE (0.0 – 100.0). `100.0` = cuota disponible completa.
    pub percent_remaining: f64,
    /// Porcentaje CONSUMIDO (0.0 – 100.0). Derivado de `percent_remaining`.
    pub percent_used: f64,
    /// ISO 8601 estimado del próximo reinicio. `None` si la cuota está al 100%.
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
        .arg("/usage")
        .output()
        .map_err(|e| AgyError::CommandError(format!("No se pudo ejecutar 'agy': {}", e)))?;

    // agy /usage escribe en stdout (paginado). Combinamos stdout + stderr.
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    parse_usage_output(&combined)
}

/// Parsea la salida de texto de `agy /usage` en lecturas estructuradas.
/// Esta función es pública para permitir tests unitarios con salida fake.
pub fn parse_usage_output(text: &str) -> Result<Vec<AgyQuotaReading>> {
    let now = Utc::now();
    let read_at = now.to_rfc3339();

    // Extraer el email de la cuenta
    let email_re = Regex::new(r"Account:\s*(\S+@\S+)").unwrap();
    let account_email = email_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    // Regex para detectar porcentaje: "98.41%" o "100.00%"
    let pct_re = Regex::new(r"(\d{1,3}\.\d+)%").unwrap();
    // Regex para "Refreshes in 167h 55m" o "Refreshes in 55m" o "Refreshes in 2h"
    let refresh_re = Regex::new(r"Refreshes in\s+(?:(\d+)h\s*)?(?:(\d+)m)?").unwrap();

    let mut readings = Vec::new();

    // Dividir por sección (GEMINI / CLAUDE AND GPT)
    for (pool, section_text) in extract_sections(text) {
        let percent_remaining = pct_re
            .captures(section_text)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(100.0);

        let reset_at = if section_text.contains("Quota available") {
            None
        } else {
            refresh_re.captures(section_text).and_then(|c| {
                let hours: i64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let minutes: i64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let total_minutes = hours * 60 + minutes;
                if total_minutes == 0 {
                    None
                } else {
                    Some((now + Duration::minutes(total_minutes)).to_rfc3339())
                }
            })
        };

        readings.push(AgyQuotaReading {
            pool,
            percent_remaining,
            percent_used: 100.0 - percent_remaining,
            reset_at,
            read_at: read_at.clone(),
            account_email: account_email.clone(),
        });
    }

    if readings.is_empty() {
        return Err(AgyError::CommandError(
            "No se pudieron parsear lecturas de cuota de la salida de 'agy /usage'".to_string(),
        ));
    }

    Ok(readings)
}

/// Extrae las secciones GEMINI y CLAUDE AND GPT del texto.
fn extract_sections(text: &str) -> Vec<(AgyPool, &str)> {
    let mut sections = Vec::new();

    // Buscar posición de cada encabezado de sección
    if let Some(gemini_start) = text.find("GEMINI MODELS") {
        let gemini_end = text[gemini_start..]
            .find("CLAUDE AND GPT MODELS")
            .map(|p| gemini_start + p)
            .unwrap_or(text.len());
        sections.push((AgyPool::Gemini, &text[gemini_start..gemini_end]));
    }

    if let Some(claude_start) = text.find("CLAUDE AND GPT MODELS") {
        sections.push((AgyPool::ClaudeGpt, &text[claude_start..]));
    }

    sections
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
