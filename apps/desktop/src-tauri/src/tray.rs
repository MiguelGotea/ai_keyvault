//! Configuración del System Tray (bandeja del sistema de Windows).
//! La app se minimiza a la bandeja al cerrar la ventana principal.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager, Runtime,
};
use tracing::info;

/// Configura el icono y menú del System Tray.
pub fn setup_tray<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", "Abrir AI KeyVault", true, None::<&str>)?;
    let separador = tauri::menu::PredefinedMenuItem::separator(app)?;
    let salir = MenuItem::with_id(app, "salir", "Salir", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&abrir, &separador, &salir])?;

    // Usar el ícono monocolor para la bandeja
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .menu_on_left_click(false)
        .tooltip("AI KeyVault")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "abrir" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "salir" => {
                info!("👋 Cerrando AI KeyVault desde el tray");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Doble clic en el ícono del tray → abrir ventana
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
