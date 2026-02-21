//! Native notification utilities using tauri-plugin-notification

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Show a native notification
pub fn show_notification(app: &AppHandle, title: &str, body: &str) {
    app.notification().builder().title(title).body(body).show().ok();
}

/// Show server start notification
pub fn notify_server_started(app: &AppHandle, http_port: u16, admin_port: u16) {
    show_notification(
        app,
        "MockForge Server Started",
        &format!("HTTP: {} | Admin: {}", http_port, admin_port),
    );
}

/// Show server stop notification
pub fn notify_server_stopped(app: &AppHandle) {
    show_notification(app, "MockForge Server Stopped", "The mock server has been stopped");
}

/// Show error notification
pub fn notify_error(app: &AppHandle, message: &str) {
    show_notification(app, "MockForge Error", message);
}

/// Show file opened notification
pub fn notify_file_opened(app: &AppHandle, file_path: &str) {
    let file_name = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);
    show_notification(app, "Configuration File Opened", &format!("Opened: {}", file_name));
}
