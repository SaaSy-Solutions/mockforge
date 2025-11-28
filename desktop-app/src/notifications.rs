//! Native notification utilities

use tauri::AppHandle;

/// Show a native notification
pub fn show_notification(app: &AppHandle, title: &str, body: &str) {
    #[cfg(not(target_os = "macos"))]
    {
        use tauri::api::notification::Notification;
        let app_id = app.config().tauri.bundle.identifier.clone();
        Notification::new(&app_id).title(title).body(body).show().ok();
    }

    #[cfg(target_os = "macos")]
    {
        // macOS notifications require different handling
        // For now, we'll use the system tray tooltip
        if let Some(window) = app.get_window("main") {
            window.set_title(&format!("{} - {}", title, body)).ok();
        }
    }
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
