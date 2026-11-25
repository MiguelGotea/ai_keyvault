//! Punto de entrada de la app de escritorio AI KeyVault (Tauri 2).
//! La lógica de UI vive en React (src/). Este módulo solo inicializa Tauri
//! y registra los comandos IPC que el frontend puede invocar.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod tray;

use tauri::Manager;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("🔐 Iniciando AI KeyVault Desktop v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Configurar el icono de la bandeja del sistema
            tray::setup_tray(app)?;

            #[cfg(debug_assertions)]
            {
                // En desarrollo: abrir DevTools automáticamente
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            info!("✅ AI KeyVault Desktop iniciado");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::ping_server,
            commands::vault::get_vault_status,
        ])
        .run(tauri::generate_context!())
        .expect("Error al arrancar la aplicación AI KeyVault");
}
