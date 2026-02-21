//! Keyboard shortcuts for MockForge Desktop

use tauri::{AppHandle, Manager};

/// Register global keyboard shortcuts
pub fn register_shortcuts(app: &AppHandle) -> Result<(), String> {
    let _window = app.get_webview_window("main").ok_or("Main window not found")?;

    // Global shortcuts can be registered via tauri-plugin-global-shortcut
    // For now, shortcuts are handled via the frontend
    tracing::info!("Global shortcuts available via tauri-plugin-global-shortcut");

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
