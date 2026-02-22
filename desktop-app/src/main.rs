// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::app::AppState;
use crate::server::MockServerManager;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tokio::sync::RwLock;

mod app;
mod commands;
mod notifications;
mod server;
mod shortcuts;
mod system_tray;
mod theme;
mod updater;

use notifications::show_notification;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    // Create app state
    let app_state = Arc::new(RwLock::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            // Build the tray menu
            let show_i = MenuItem::with_id(app, "show", "Show MockForge", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let start_i =
                MenuItem::with_id(app, "start-server", "Start Server", true, None::<&str>)?;
            let stop_i = MenuItem::with_id(app, "stop-server", "Stop Server", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep3 = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &hide_i,
                    &sep1,
                    &start_i,
                    &stop_i,
                    &sep2,
                    &settings_i,
                    &sep3,
                    &quit_i,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .icon_as_template(true)
                .menu(&menu)
                .menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    system_tray::handle_menu_event(app, &event);
                })
                .on_tray_icon_event(|tray, event| {
                    system_tray::handle_tray_icon_event(tray, &event);
                })
                .build(app)?;

            // Initialize app state
            let state = app_state.clone();
            app.manage(state);

            // Initialize server manager
            let server_manager = Arc::new(RwLock::new(MockServerManager::new()));
            app.manage(server_manager);

            // Set up window event handlers
            if let Some(window) = app.get_webview_window("main") {
                // Handle window close - minimize to tray instead
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });
            }

            // Handle file drop events
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.listen("tauri://file-drop", move |event| {
                    if let Some(payload) = event.payload() {
                        if let Ok(paths) = serde_json::from_str::<Vec<String>>(payload) {
                            if let Some(path) = paths.first() {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.emit("file-dropped", path);
                                }
                            }
                        }
                    }
                });
            }

            // Register keyboard shortcuts
            if let Err(e) = shortcuts::register_shortcuts(app.handle()) {
                tracing::warn!("Failed to register keyboard shortcuts: {}", e);
            }

            // Show notification on startup
            show_notification(app.handle(), "MockForge", "MockForge Desktop is running");

            // Start watching for system theme changes
            theme::watch_system_theme(app.handle().clone());

            // Start periodic update checking
            updater::start_periodic_update_check(app.handle().clone());

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
            theme::get_system_theme,
            theme::get_theme_preference,
            theme::save_theme_preference,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
