// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::AppState;
use crate::server::MockServerManager;
use std::sync::Arc;
use tauri::{Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};
use tokio::sync::RwLock;

mod app;
mod commands;
mod notifications;
mod server;
mod shortcuts;
mod system_tray;
mod updater;

use notifications::show_notification;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    // Create system tray menu
    let tray_menu = SystemTrayMenu::new()
        .add_item(tauri::CustomMenuItem::new("show", "Show MockForge"))
        .add_item(tauri::CustomMenuItem::new("hide", "Hide"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(tauri::CustomMenuItem::new("start-server", "Start Server"))
        .add_item(tauri::CustomMenuItem::new("stop-server", "Stop Server"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(tauri::CustomMenuItem::new("settings", "Settings"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(tauri::CustomMenuItem::new("quit", "Quit"));

    let system_tray = SystemTray::new().with_menu(tray_menu);

    // Create app state
    let app_state = Arc::new(RwLock::new(AppState::new()));

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| {
            system_tray::handle_system_tray_event(app, event);
        })
        .setup(move |app| {
            // Initialize app state
            let state = app_state.clone();
            app.manage(state);

            // Initialize server manager
            let server_manager = Arc::new(RwLock::new(MockServerManager::new()));
            app.manage(server_manager);

            // Note: Tauri 1.5 has built-in dialog and fs APIs, no plugins needed

            // Set up window event handlers
            if let Some(window) = app.get_window("main") {
                // Handle window close - minimize to tray instead
                let app_handle = app.handle().clone();
                window.listen("tauri://close-requested", move |_event| {
                    if let Some(window) = app_handle.get_window("main") {
                        let _ = window.hide();
                    }
                });
            }

            // Handle file drop events
            let app_handle = app.handle().clone();
            if let Some(window) = app.get_window("main") {
                window.listen("tauri://file-drop", move |event| {
                    if let Some(paths) = event.payload() {
                        if let Ok(paths) = serde_json::from_str::<Vec<String>>(paths) {
                            if let Some(path) = paths.first() {
                                // Emit event to frontend to handle file
                                if let Some(window) = app_handle.get_window("main") {
                                    let _ = window.emit("file-dropped", path);
                                }
                            }
                        }
                    }
                });
            }

            // Handle file open events (when app is opened with a file)
            // Note: For Tauri 1.5, we handle file associations via window events
            // Single instance handling would require a different approach or plugin
            // For now, file associations will work, but single instance needs manual implementation

            // Register keyboard shortcuts
            if let Err(e) = shortcuts::register_shortcuts(&app.handle()) {
                tracing::warn!("Failed to register keyboard shortcuts: {}", e);
            }

            // Show notification on startup
            show_notification(&app.handle(), "MockForge", "MockForge Desktop is running");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_server,
            commands::stop_server,
            commands::get_server_status,
            commands::open_config_file,
            commands::save_config_file,
            commands::get_app_version,
            commands::handle_file_open,
            commands::check_for_updates,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
