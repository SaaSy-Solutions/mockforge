//! Keyboard shortcuts for MockForge Desktop

use tauri::{AppHandle, Manager, Window};

/// Register global keyboard shortcuts
pub fn register_shortcuts(app: &AppHandle) -> Result<(), String> {
    let _window = app.get_window("main").ok_or("Main window not found")?;

    // Register shortcuts using Tauri's global shortcut API
    // Note: Tauri 1.5 uses global-shortcut feature

    // Ctrl/Cmd + Shift + S: Start server
    // Note: Tauri 1.5 global shortcuts API uses on_global_shortcut
    // For now, we'll skip global shortcuts as they require a different setup
    // Global shortcuts in Tauri 1.5 need to be registered differently
    // This functionality can be implemented later or moved to frontend
    tracing::warn!(
        "Global shortcuts registration skipped - requires Tauri 1.5 specific implementation"
    );

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
