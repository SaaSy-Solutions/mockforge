//! System theme detection and management for desktop app
//!
//! Provides cross-platform system theme detection and change monitoring
//! to sync the desktop app with the system's dark/light mode preference.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Manager};

/// System theme preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemTheme {
    Light,
    Dark,
}

/// Get the current system theme
#[tauri::command]
pub fn get_system_theme() -> Result<String, String> {
    let theme = detect_system_theme();
    Ok(match theme {
        SystemTheme::Dark => "dark".to_string(),
        SystemTheme::Light => "light".to_string(),
    })
}

/// Detect system theme preference
fn detect_system_theme() -> SystemTheme {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
        {
            if let Ok(apps_use_light_theme) = hkcu.get_value::<u32, _>("AppsUseLightTheme") {
                return if apps_use_light_theme == 0 {
                    SystemTheme::Dark
                } else {
                    SystemTheme::Light
                };
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        if let Ok(output) =
            Command::new("defaults").args(&["read", "-g", "AppleInterfaceStyle"]).output()
        {
            let theme = String::from_utf8_lossy(&output.stdout);
            if theme.trim() == "Dark" {
                return SystemTheme::Dark;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::env;

        // Check GTK theme
        if let Ok(theme) = env::var("GTK_THEME") {
            if theme.to_lowercase().contains("dark") {
                return SystemTheme::Dark;
            }
        }

        // Check gsettings (GNOME)
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
        {
            let theme = String::from_utf8_lossy(&output.stdout);
            if theme.to_lowercase().contains("dark") {
                return SystemTheme::Dark;
            }
        }

        // Check color-scheme preference (newer GNOME)
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let scheme = String::from_utf8_lossy(&output.stdout);
            if scheme.to_lowercase().contains("dark") {
                return SystemTheme::Dark;
            }
        }
    }

    // Default to light theme if detection fails
    SystemTheme::Light
}

/// Watch for system theme changes and emit events to frontend
pub fn watch_system_theme(app: AppHandle) {
    let (tx, rx) = mpsc::channel();

    // Spawn thread to watch for theme changes
    thread::spawn(move || {
        let mut last_theme = detect_system_theme();

        loop {
            // Check theme every 2 seconds
            thread::sleep(std::time::Duration::from_secs(2));

            let current_theme = detect_system_theme();
            if current_theme != last_theme {
                last_theme = current_theme;
                let _ = tx.send(current_theme);
            }
        }
    });

    // Spawn async task to handle theme change events
    tokio::spawn(async move {
        while let Ok(theme) = rx.recv() {
            let theme_str = match theme {
                SystemTheme::Dark => "dark",
                SystemTheme::Light => "light",
            };

            // Emit event to all windows
            if let Some(window) = app.get_window("main") {
                let _ = window.emit("system-theme-changed", theme_str);
            }
        }
    });
}

/// Get and persist theme preference
#[tauri::command]
pub async fn get_theme_preference() -> Result<String, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to get config directory".to_string())?
        .join("mockforge");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config_file = config_dir.join("theme.json");

    if config_file.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&config_file).await {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(theme) = config.get("theme").and_then(|v| v.as_str()) {
                    return Ok(theme.to_string());
                }
            }
        }
    }

    // Default to system theme
    let system_theme = detect_system_theme();
    Ok(match system_theme {
        SystemTheme::Dark => "dark".to_string(),
        SystemTheme::Light => "light".to_string(),
    })
}

/// Save theme preference
#[tauri::command]
pub async fn save_theme_preference(theme: String) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Failed to get config directory".to_string())?
        .join("mockforge");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config_file = config_dir.join("theme.json");
    let config = json!({
        "theme": theme,
        "updated_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    });

    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize theme config: {}", e))?;
    tokio::fs::write(&config_file, config_str)
        .await
        .map_err(|e| format!("Failed to save theme preference: {}", e))?;

    Ok(())
}
