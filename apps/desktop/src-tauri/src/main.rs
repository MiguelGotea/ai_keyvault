//! Punto de entrada de la app de escritorio AI KeyVault (Tauri 2).
//! La lógica de UI vive en React (src/). Este módulo solo inicializa Tauri
//! y registra los comandos IPC que el frontend puede invocar.
//!
//! ARQUITECTURA: Todo HTTP al VPS va por comandos Rust (commands/api.rs).
//! La CSP en tauri.conf.json bloquea fetch() externo desde React.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api_client;
mod commands;
mod tray;

use commands::vault::VaultState;
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
        // Estado global de la bóveda (Mutex<Option<Vault>>)
        .manage(VaultState::default())
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
            // Health / conectividad
            commands::health::ping_server,
            commands::api::check_server_health,
            // Bóveda local (Fase 12)
            commands::vault::get_vault_status,
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::recover_vault,
            commands::vault::list_vault_accounts,
            // Cuota y estadísticas (VPS)
            commands::api::get_quota,
            commands::api::get_stats_summary,
            commands::api::get_stats_heatmap,
            commands::api::get_stats_compare,
            commands::api::get_stats_weekly,
            // Chats (VPS)
            commands::api::get_chats,
            // Alertas (VPS)
            commands::api::get_alerts,
            // Switch de cuenta (VPS)
            commands::api::get_switch_history,
            commands::api::post_switch,
            // Pings (VPS)
            commands::api::get_pings_needs,
            commands::api::get_pings,
            commands::api::post_ping,
            // Dispositivos (VPS)
            commands::api::register_device,
        ])
        .run(tauri::generate_context!())
        .expect("Error al arrancar la aplicación AI KeyVault");
}
