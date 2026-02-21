//! System tray event handling for Tauri 2

use tauri::menu::MenuEvent;
use tauri::tray::{TrayIcon, TrayIconEvent};
use tauri::{AppHandle, Manager};

/// Handle tray menu item clicks
pub fn handle_menu_event(app: &AppHandle, event: &MenuEvent) {
    match event.id().as_ref() {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "hide" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        "start-server" => {
            // Emit event to frontend to start server
            if let Some(window) = app.get_webview_window("main") {
                window.emit("tray-start-server", ()).ok();
            }
        }
        "stop-server" => {
            // Emit event to frontend to stop server
            if let Some(window) = app.get_webview_window("main") {
                window.emit("tray-stop-server", ()).ok();
            }
        }
        "settings" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                // Navigate to settings page via JS
                // Note: eval() is used here intentionally for Tauri webview navigation
                let _ = window.eval("window.location.hash = '#/settings'");
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Handle tray icon events (e.g., left click on the icon itself)
pub fn handle_tray_icon_event(tray: &TrayIcon, event: &TrayIconEvent) {
    if let TrayIconEvent::Click { button, .. } = event {
        if *button == tauri::tray::MouseButton::Left {
            let app = tray.app_handle();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}
