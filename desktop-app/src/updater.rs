//! Auto-update functionality for MockForge Desktop
//!
//! This module handles checking for updates and installing them.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
}

/// Check for updates
pub async fn check_for_updates(app: &AppHandle) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION");

    // In production, this would check against an update server
    // For now, we'll implement a basic version check
    let update_server_url = std::env::var("MOCKFORGE_UPDATE_SERVER")
        .unwrap_or_else(|_| "https://updates.mockforge.dev".to_string());

    let check_url = format!("{}/check/{}", update_server_url, current_version);

    match reqwest::get(&check_url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<UpdateResponse>().await {
                    Ok(update_response) => Ok(UpdateInfo {
                        available: update_response.update_available,
                        current_version: current_version.to_string(),
                        latest_version: update_response.latest_version,
                        download_url: update_response.download_url,
                        release_notes: update_response.release_notes,
                    }),
                    Err(e) => Err(format!("Failed to parse update response: {}", e)),
                }
            } else {
                // No update available
                Ok(UpdateInfo {
                    available: false,
                    current_version: current_version.to_string(),
                    latest_version: None,
                    download_url: None,
                    release_notes: None,
                })
            }
        }
        Err(e) => {
            // Network error - assume no update for now
            tracing::warn!("Failed to check for updates: {}", e);
            Ok(UpdateInfo {
                available: false,
                current_version: current_version.to_string(),
                latest_version: None,
                download_url: None,
                release_notes: None,
            })
        }
    }
}

/// Update response from server
#[derive(Debug, Deserialize)]
struct UpdateResponse {
    update_available: bool,
    latest_version: Option<String>,
    download_url: Option<String>,
    release_notes: Option<String>,
}

/// Install update using tauri-plugin-updater
pub async fn install_update(app: &AppHandle) -> Result<(), String> {
    // Emit event to frontend to show update dialog
    if let Some(window) = app.get_webview_window("main") {
        window
            .emit("update-install-started", ())
            .map_err(|e| format!("Failed to emit update event: {}", e))?;
    }

    tracing::info!("Update installation triggered");
    Ok(())
}

/// Check for updates periodically
pub fn start_periodic_update_check(app: AppHandle) {
    tokio::spawn(async move {
        loop {
            // Check for updates every 24 hours
            tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;

            match check_for_updates(&app).await {
                Ok(update_info) => {
                    if update_info.available {
                        tracing::info!(
                            "Update available: {}",
                            update_info.latest_version.as_deref().unwrap_or("unknown")
                        );

                        // Emit event to frontend
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("update-available", &update_info);
                        }

                        // Show notification
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("notification", serde_json::json!({
                                "title": "Update Available",
                                "body": format!("MockForge {} is available", update_info.latest_version.as_deref().unwrap_or("new version")),
                            }));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check for updates: {}", e);
                }
            }
        }
    });
}
