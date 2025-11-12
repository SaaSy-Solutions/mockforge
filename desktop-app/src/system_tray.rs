//! System tray event handling

use tauri::{AppHandle, SystemTrayEvent, Window};

/// Handle system tray events
pub fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick {
            position: _,
            size: _,
            ..
        } => {
            // Show window on left click
            if let Some(window) = app.get_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.hide();
                    }
                }
                "start-server" => {
                    // Emit event to frontend to start server
                    if let Some(window) = app.get_window("main") {
                        window.emit("tray-start-server", ()).ok();
                    }
                }
                "stop-server" => {
                    // Emit event to frontend to stop server
                    if let Some(window) = app.get_window("main") {
                        window.emit("tray-stop-server", ()).ok();
                    }
                }
                "settings" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Navigate to settings (if implemented)
                        let _ = window.eval("window.location.hash = '#/settings'");
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
