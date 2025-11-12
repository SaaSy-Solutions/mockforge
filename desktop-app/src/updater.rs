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

/// Install update (Tauri 1.5 uses built-in updater)
pub async fn install_update(app: &AppHandle) -> Result<(), String> {
    // Tauri 1.5 has built-in updater support
    // This would trigger the Tauri updater dialog
    // For now, we'll emit an event to the frontend
    if let Some(window) = app.get_window("main") {
        window
            .emit("update-available", ())
            .map_err(|e| format!("Failed to emit update event: {}", e))?;
    }

    Ok(())
}
