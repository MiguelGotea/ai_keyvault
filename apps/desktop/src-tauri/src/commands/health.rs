//! Comandos de salud y conectividad con el VPS.
//! Toda llamada HTTP al VPS sale desde Rust — nunca desde fetch() en React.

use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerHealth {
    pub online: bool,
    pub status: String,
    pub server_url: String,
}

/// Verifica si el servidor VPS responde.
/// El frontend llama a invoke("ping_server") — no hace fetch() directamente.
#[command]
pub async fn ping_server(server_url: String) -> Result<ServerHealth, String> {
    let url = format!("{}/health", server_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {e}"))?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => Ok(ServerHealth {
            online: true,
            status: "healthy".to_string(),
            server_url: server_url,
        }),
        Ok(resp) => Ok(ServerHealth {
            online: false,
            status: format!("HTTP {}", resp.status()),
            server_url: server_url,
        }),
        Err(e) => Ok(ServerHealth {
            online: false,
            status: format!("Sin conexión: {e}"),
            server_url: server_url,
        }),
    }
}
