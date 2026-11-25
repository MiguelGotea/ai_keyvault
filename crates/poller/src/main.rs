//! Binario ejecutable del poller de cuota.
//!
//! Lee configuración de variables de entorno o de `/opt/ai-keyvault/.env` (en VPS)
//! o de `~/.config/ai-keyvault/poller.env` (en Windows).
//!
//! **Variables de entorno:**
//! - `AKYV_SERVER_URL`      — URL del servidor VPS (default: http://198.211.97.243:3010)
//! - `AKYV_DEVICE_ID`       — Identificador de esta PC (default: hostname)
//! - `AKYV_POLL_INTERVAL`   — Intervalo en segundos (default: 600)
//!
//! **Uso:**
//! ```powershell
//! # Correr una vez manualmente
//! cargo run --package ai-keyvault-poller
//!
//! # Con variables personalizadas
//! $env:AKYV_POLL_INTERVAL = "60"
//! cargo run --package ai-keyvault-poller
//! ```

use ai_keyvault_poller::{PollerConfig, PollerHandle};
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() {
    // Inicializar logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ai_keyvault_poller=info,warn")),
        )
        .with_target(false)
        .init();

    // Leer configuración de entorno
    let server_url = std::env::var("AKYV_SERVER_URL")
        .unwrap_or_else(|_| "http://198.211.97.243:3010".to_string());

    let device_id = std::env::var("AKYV_DEVICE_ID").unwrap_or_else(|_| hostname());

    let poll_interval: u64 = std::env::var("AKYV_POLL_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(600);

    info!("╔══════════════════════════════════════╗");
    info!("║      AI KeyVault — Poller v0.1       ║");
    info!("╚══════════════════════════════════════╝");
    info!("  Servidor : {}", server_url);
    info!("  Dispositivo: {}", device_id);
    info!(
        "  Intervalo: {}s ({:.0}min)",
        poll_interval,
        poll_interval as f64 / 60.0
    );

    let config = PollerConfig::new(server_url, device_id).with_interval(poll_interval);

    // Iniciar el poller en background
    let _handle = PollerHandle::start(config);

    // Mantener el proceso vivo (Ctrl+C para detener)
    info!("⏳ Poller corriendo — presiona Ctrl+C para detener");
    tokio::signal::ctrl_c()
        .await
        .expect("Error al esperar Ctrl+C");

    info!("🛑 Poller detenido");
}

/// Obtiene el nombre del host como identificador de dispositivo.
fn hostname() -> String {
    std::env::var("COMPUTERNAME") // Windows
        .or_else(|_| std::env::var("HOSTNAME")) // Linux/Mac
        .unwrap_or_else(|_| "pc-desconocida".to_string())
}
