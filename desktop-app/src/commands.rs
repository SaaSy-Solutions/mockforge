//! Tauri command handlers for desktop app

use crate::app::AppState;
use crate::notifications;
use crate::server::MockServerManager;
use mockforge_core::ServerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::RwLock;

/// Server status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub running: bool,
    pub http_port: Option<u16>,
    pub admin_port: Option<u16>,
    pub error: Option<String>,
}

/// Start the mock server
#[tauri::command]
pub async fn start_server(
    config_path: Option<String>,
    http_port: Option<u16>,
    admin_port: Option<u16>,
    server_manager: State<'_, Arc<RwLock<MockServerManager>>>,
    app_state: State<'_, Arc<RwLock<AppState>>>,
    window: WebviewWindow,
    app: AppHandle,
) -> Result<ServerStatus, String> {
    // Load configuration
    let mut config = if let Some(path) = config_path {
        // Load from file
        let config_str = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_yaml::from_str(&config_str).map_err(|e| format!("Failed to parse config: {}", e))?
    } else {
        // Use default config with specified ports
        let mut default_config = ServerConfig::default();
        if let Some(port) = http_port {
            default_config.http.port = port;
        }
        if let Some(port) = admin_port {
            default_config.admin.port = port;
        }
        default_config
    };

    // Ensure admin is enabled
    config.admin.enabled = true;
    if let Some(port) = admin_port {
        config.admin.port = port;
    }

    // Start server
    let mut manager = server_manager.write().await;
    manager
        .start(config.clone())
        .await
        .map_err(|e| format!("Failed to start server: {}", e))?;

    // Update app state
    let mut state = app_state.write().await;
    state.server_running = true;
    state.http_port = Some(config.http.port);
    state.admin_port = Some(config.admin.port);

    // Show notification
    window
        .emit("server-started", ())
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    // Show native notification
    if let Some(http) = state.http_port {
        if let Some(admin) = state.admin_port {
            notifications::notify_server_started(&app, http, admin);
        }
    }

    Ok(ServerStatus {
        running: true,
        http_port: state.http_port,
        admin_port: state.admin_port,
        error: None,
    })
}

/// Stop the mock server
#[tauri::command]
pub async fn stop_server(
    server_manager: State<'_, Arc<RwLock<MockServerManager>>>,
    app_state: State<'_, Arc<RwLock<AppState>>>,
    window: WebviewWindow,
    app: AppHandle,
) -> Result<ServerStatus, String> {
    let mut manager = server_manager.write().await;
    manager.stop().await.map_err(|e| format!("Failed to stop server: {}", e))?;

    // Update app state
    let mut state = app_state.write().await;
    state.server_running = false;

    // Show notification
    window
        .emit("server-stopped", ())
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    // Show native notification
    notifications::notify_server_stopped(&app);

    Ok(ServerStatus {
        running: false,
        http_port: state.http_port,
        admin_port: state.admin_port,
        error: None,
    })
}

/// Get current server status
#[tauri::command]
pub async fn get_server_status(
    server_manager: State<'_, Arc<RwLock<MockServerManager>>>,
    app_state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<ServerStatus, String> {
    let manager = server_manager.read().await;
    let state = app_state.read().await;

    Ok(ServerStatus {
        running: manager.is_running(),
        http_port: state.http_port,
        admin_port: state.admin_port,
        error: state.last_error.clone(),
    })
}

/// Open a configuration file using tauri-plugin-dialog
#[tauri::command]
pub async fn open_config_file(app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("YAML", &["yaml", "yml"])
        .add_filter("JSON", &["json"])
        .add_filter("All", &["*"])
        .blocking_pick_file();

    match file_path {
        Some(path) => {
            let content = tokio::fs::read_to_string(path.path())
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            Ok(Some(content))
        }
        None => Ok(None),
    }
}

/// Save a configuration file using tauri-plugin-dialog
#[tauri::command]
pub async fn save_config_file(content: String, app: AppHandle) -> Result<Option<String>, String> {
    let file_path = app
        .dialog()
        .file()
        .add_filter("YAML", &["yaml", "yml"])
        .add_filter("JSON", &["json"])
        .set_file_name("mockforge.yaml")
        .blocking_save_file();

    match file_path {
        Some(path) => {
            tokio::fs::write(path.path(), content)
                .await
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(Some(path.path().to_string_lossy().to_string()))
        }
        None => Ok(None),
    }
}

/// Get app version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Handle file open event (from file association or command line)
#[tauri::command]
pub async fn handle_file_open(
    file_path: String,
    server_manager: State<'_, Arc<RwLock<MockServerManager>>>,
    app_state: State<'_, Arc<RwLock<AppState>>>,
    window: WebviewWindow,
    app: AppHandle,
) -> Result<(), String> {
    // Read the config file
    let config_str = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // Parse config
    let config: ServerConfig =
        serde_yaml::from_str(&config_str).map_err(|e| format!("Failed to parse config: {}", e))?;

    // Update app state with config path
    let mut state = app_state.write().await;
    state.config_path = Some(PathBuf::from(&file_path));

    // Show notification
    notifications::notify_file_opened(&app, &file_path);

    // Emit event to frontend
    window
        .emit("config-file-opened", &config_str)
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    // Optionally auto-start server if not running
    let manager = server_manager.read().await;
    if !manager.is_running() {
        drop(manager);
        // Auto-start server with this config
        let mut manager = server_manager.write().await;
        manager
            .start(config.clone())
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        let mut state = app_state.write().await;
        state.server_running = true;
        state.http_port = Some(config.http.port);
        state.admin_port = Some(config.admin.port);

        notifications::notify_server_started(&app, config.http.port, config.admin.port);
    }

    Ok(())
}

/// Check for app updates
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<crate::updater::UpdateInfo, String> {
    crate::updater::check_for_updates(&app).await
}

/// Install available update
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    crate::updater::install_update(&app).await
}
