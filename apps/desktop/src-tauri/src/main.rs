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

use commands::poller::PollerManager;
use commands::vault::{SessionAuthState, VaultState};
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

    info!(
        "🔐 Iniciando AI KeyVault Desktop v{}",
        env!("CARGO_PKG_VERSION")
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        // Estado global de la bóveda (Mutex<Option<Vault>>)
        .manage(VaultState::default())
        // Estado de sesión autenticada en RAM (AuthKey con zeroize)
        .manage(SessionAuthState::default())
        // Estado global del poller (Mutex<PollerStatus>)
        .manage(PollerManager::default())
        .setup(|app| {
            // Configurar el icono de la bandeja del sistema
            tray::setup_tray(app)?;

            // D-03: si arranca con Windows (--minimized), iniciar oculto en bandeja
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

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
            // Bóveda local y sesión (Fase 12 / 19)
            commands::vault::get_vault_status,
            commands::vault::get_session_info,
            commands::vault::register_user_and_vault,
            commands::vault::login_user_and_sync,
            commands::vault::global_logout,
            commands::vault::create_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::recover_vault,
            commands::vault::list_vault_accounts,
            // Gestión de cuentas en la bóveda (Fase 13b)
            commands::vault::add_vault_account,
            commands::vault::delete_vault_account,
            commands::vault::get_account_email,
            commands::vault::get_account_credentials,
            commands::vault::update_vault_account,
            commands::vault::schedule_clipboard_clear,
            commands::vault::decrypt_account_tokens,
            commands::vault::add_agy_account_oauth,
            commands::vault::cancel_agy_oauth,
            commands::vault::import_current_codex_credential,
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
            commands::api::get_devices,
            commands::api::revoke_device,
            // Poller IPC (Fase 13)
            commands::poller::get_poller_status,
            commands::poller::start_poller,
            commands::poller::pause_poller,
            commands::poller::resume_poller,
            commands::poller::stop_poller,
            // Switch de cuenta (Fase 13)
            commands::switch_cmd::switch_account,
            commands::switch_cmd::get_active_account,
            // Autobloqueo y sincronización vault (Fase 14/15)
            commands::security::start_autolock_timer,
            commands::security::reset_autolock_timer,
            commands::security::cancel_autolock_timer,
            commands::security::upload_vault_sync,
            commands::security::download_vault_sync,
            commands::security::get_last_backup_info,
            commands::security::get_autostart_enabled,
            commands::security::set_autostart_enabled,
            commands::security::set_alerts_muted_hours,
            commands::security::get_alerts_muted_until,
        ])
        .build(tauri::generate_context!())
        .expect("Error al construir la aplicación AI KeyVault")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let poller = app_handle.state::<PollerManager>();
                commands::poller::kill_poller_process(&poller);
            }
        });
}
