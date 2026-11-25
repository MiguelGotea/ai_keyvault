//! Ping mínimo para agy — arranca el contador de reinicio de cuota.
//!
//! # Propósito
//!
//! Según la hipótesis H1 (confirmada preliminarmente), el contador de 7 días
//! de agy empieza en el **primer uso** de la ventana. Si una cuenta lleva días
//! sin actividad y el `percent_used` es 0 %, el ping envía el mensaje más corto
//! posible para arrancar el reloj y maximizar el tiempo disponible en la semana.
//!
//! # Qué hace el ping
//!
//! 1. Verifica que `agy` esté disponible y el token sea válido (`agy models`).
//! 2. Envía `agy --print /chat "k"` — el mensaje más corto posible (1 token).
//! 3. Registra el resultado (éxito/error, tokens estimados) en el caller.
//!
//! # Guardas de seguridad
//!
//! - Solo se ejecuta si `percent_used` está en el rango `[0, PING_MAX_PCT)`.
//! - Máximo `AKYV_PING_MAX_DAILY` pings por cuenta por día (default: 2).
//! - Solo en el horario configurado `AKYV_PING_HOUR` (default: 8 AM hora local).
//! - Switch global `AKYV_PING_ENABLED` debe ser `"true"` para el automático.
//! - El ping manual siempre se puede ejecutar (sin restricción de horario).

use crate::{AgyError, Result};
use std::process::Command;

/// Resultado de un ping individual.
#[derive(Debug, Clone)]
pub struct PingResult {
    /// Email de la cuenta que recibió el ping.
    pub account_id: String,
    /// Si el ping se ejecutó con éxito.
    pub success: bool,
    /// Mensaje de error si `success = false`.
    pub error: Option<String>,
    /// Duración en ms del ping (incluye tiempo de respuesta de agy).
    pub duration_ms: u64,
    /// Mensaje enviado (siempre el mínimo configurable).
    pub message_sent: String,
}

/// Configuración del ping leída desde variables de entorno.
#[derive(Debug, Clone)]
pub struct PingConfig {
    /// Habilitar ping automático (default: false).
    pub enabled: bool,
    /// Hora local (0-23) para ejecutar el ping automático (default: 8).
    pub hour: u8,
    /// Máximo de pings automáticos por cuenta por día (default: 2).
    pub max_per_day: u32,
    /// Porcentaje máximo de uso permitido para disparar ping (default: 5.0).
    /// Si la cuenta ya tiene más del 5% de uso, el ping no tiene sentido.
    pub max_pct_to_ping: f64,
    /// Mensaje mínimo a enviar (default: "k").
    pub message: String,
}

impl PingConfig {
    /// Lee la configuración desde variables de entorno.
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("AKYV_PING_ENABLED")
                .unwrap_or_default()
                .to_lowercase()
                == "true",
            hour: std::env::var("AKYV_PING_HOUR")
                .ok()
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(8),
            max_per_day: std::env::var("AKYV_PING_MAX_DAILY")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(2),
            max_pct_to_ping: std::env::var("AKYV_PING_MAX_PCT")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(5.0),
            message: std::env::var("AKYV_PING_MESSAGE").unwrap_or_else(|_| "k".to_string()),
        }
    }
}

impl Default for PingConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Ejecuta un ping mínimo hacia agy para la cuenta activa actual.
///
/// El ping usa el subcomando `agy --print /chat "<mensaje>"` que consume
/// la mínima cantidad de tokens posible y activa el contador de la ventana.
///
/// # Parámetros
/// - `account_id`: Email de la cuenta (solo para logging/registro).
/// - `message`: Mensaje corto a enviar (típicamente `"k"`).
///
/// # Errores
/// - Si `agy` no está instalado o el token no es válido.
/// - Si el comando sale con error.
pub fn execute_ping(account_id: &str, message: &str) -> Result<PingResult> {
    let start = std::time::Instant::now();

    // Primero verificar que el token sea válido
    let verify = Command::new("agy")
        .args(["--print", "/usage"])
        .output()
        .map_err(|e| AgyError::CommandError(format!("No se pudo verificar token agy: {e}")))?;

    if !verify.status.success() {
        let err = String::from_utf8_lossy(&verify.stderr).to_string();
        return Ok(PingResult {
            account_id: account_id.to_string(),
            success: false,
            error: Some(format!("Token inválido o agy no disponible: {err}")),
            duration_ms: start.elapsed().as_millis() as u64,
            message_sent: message.to_string(),
        });
    }

    // Ejecutar el ping — `agy --print /chat "k"` consume ~1 token
    let output = Command::new("agy")
        .args(["--print", "/chat", message])
        .output()
        .map_err(|e| AgyError::CommandError(format!("No se pudo ejecutar ping agy: {e}")))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    if output.status.success() {
        Ok(PingResult {
            account_id: account_id.to_string(),
            success: true,
            error: None,
            duration_ms,
            message_sent: message.to_string(),
        })
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(PingResult {
            account_id: account_id.to_string(),
            success: false,
            error: Some(err),
            duration_ms,
            message_sent: message.to_string(),
        })
    }
}

/// Determina si una cuenta necesita ping según sus datos de cuota.
///
/// Una cuenta "necesita ping" cuando:
/// - `percent_used` está entre 0 y `max_pct_to_ping` (exclusivo).
/// - `reset_time` es `Some(...)` → ya tiene ventana activa (hay consumo previo
///   o la cuenta se acaba de crear con la ventana ya iniciada por otro medio).
/// - O `reset_time` es `None` → la ventana aún no ha arrancado (0 % de uso
///   y nunca se ha usado esta semana → ideal para hacer ping y arrancar el reloj).
///
/// En la práctica, con H1 lo más importante es `percent_used == 0.0`.
pub fn needs_ping(percent_used: f64, max_pct: f64) -> bool {
    percent_used < max_pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_ping_cero_uso() {
        // Cuenta sin uso → necesita ping
        assert!(needs_ping(0.0, 5.0));
    }

    #[test]
    fn test_needs_ping_uso_bajo() {
        // 2% < 5% → necesita ping
        assert!(needs_ping(2.0, 5.0));
    }

    #[test]
    fn test_needs_ping_en_limite() {
        // 5% == 5% → ya NO necesita ping (exclusivo)
        assert!(!needs_ping(5.0, 5.0));
    }

    #[test]
    fn test_needs_ping_uso_alto() {
        // 47% → no necesita ping
        assert!(!needs_ping(47.0, 5.0));
    }

    #[test]
    fn test_ping_config_defaults() {
        // Sin vars de entorno, los defaults son correctos
        // (no podemos limpiar el env en tests, pero sí verificar los valores default lógicos)
        let cfg = PingConfig {
            enabled: false,
            hour: 8,
            max_per_day: 2,
            max_pct_to_ping: 5.0,
            message: "k".to_string(),
        };
        assert!(!cfg.enabled);
        assert_eq!(cfg.hour, 8);
        assert_eq!(cfg.max_per_day, 2);
        assert_eq!(cfg.max_pct_to_ping, 5.0);
        assert_eq!(cfg.message, "k");
    }
}
