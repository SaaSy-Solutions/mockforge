//! Keyboard shortcuts for MockForge Desktop

use tauri::{AppHandle, Manager, Window};

/// Register global keyboard shortcuts
pub fn register_shortcuts(app: &AppHandle) -> Result<(), String> {
    let window = app.get_window("main").ok_or("Main window not found")?;

    // Register shortcuts using Tauri's global shortcut API
    // Note: Tauri 1.5 uses global-shortcut feature

    // Ctrl/Cmd + Shift + S: Start server
    {
        use tauri::api::global_shortcut::GlobalShortcutManager;

        let mut shortcut_manager = app.global_shortcut_manager();
        let app_handle = app.clone();

        // Start server shortcut
        shortcut_manager.register("CommandOrControl+Shift+S", move || {
            if let Some(window) = app_handle.get_window("main") {
                window.emit("shortcut-start-server", ()).ok();
            }
        })?;

        let app_handle = app.clone();
        // Stop server shortcut
        shortcut_manager.register("CommandOrControl+Shift+X", move || {
            if let Some(window) = app_handle.get_window("main") {
                window.emit("shortcut-stop-server", ()).ok();
            }
        })?;

        let app_handle = app.clone();
        // Show/hide window shortcut
        shortcut_manager.register("CommandOrControl+Shift+H", move || {
            if let Some(window) = app_handle.get_window("main") {
                if window.is_visible().unwrap_or(false) {
                    window.hide().ok();
                } else {
                    window.show().ok();
                    window.set_focus().ok();
                }
            }
        })?;

        let app_handle = app.clone();
        // Open config file shortcut
        shortcut_manager.register("CommandOrControl+O", move || {
            if let Some(window) = app_handle.get_window("main") {
                window.emit("shortcut-open-config", ()).ok();
            }
        })?;
    }

    Ok(())
}

/// Default keyboard shortcuts
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl/Cmd + Shift + S", "Start Server"),
    ("Ctrl/Cmd + Shift + X", "Stop Server"),
    ("Ctrl/Cmd + Shift + H", "Show/Hide Window"),
    ("Ctrl/Cmd + O", "Open Config File"),
    ("Ctrl/Cmd + W", "Close Window (minimize to tray)"),
    ("F11", "Toggle Fullscreen"),
    ("Ctrl/Cmd + ,", "Open Settings"),
];
