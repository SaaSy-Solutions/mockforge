# Desktop App Polish - Complete Implementation Guide

Comprehensive guide for polishing the MockForge desktop application with auto-update, file associations, and dark mode integration.

## Table of Contents

- [Overview](#overview)
- [Dark Mode Integration](#dark-mode-integration)
- [Auto-Update Implementation](#auto-update-implementation)
- [File Associations](#file-associations)
- [Additional Polish](#additional-polish)
- [Testing](#testing)
- [Distribution](#distribution)

---

## Overview

The MockForge desktop app needs final polish to provide a production-ready experience:

- ✅ **Dark Mode**: Already implemented in UI, needs desktop integration
- ⏳ **Auto-Update**: Configured but needs implementation
- ⏳ **File Associations**: Needs OS-level registration
- ⏳ **Additional Polish**: System tray improvements, keyboard shortcuts, etc.

---

## Dark Mode Integration

### Current Status

Dark mode is **already implemented** in the Admin UI:
- Theme stores (`useThemeStore`, `useThemePaletteStore`)
- Theme toggle component (`ThemeToggle`)
- CSS variables for light/dark modes
- System preference detection

### Desktop Integration

The desktop app needs to:
1. **Detect system theme** and sync with UI
2. **Persist theme preference** across app restarts
3. **Update system tray icon** based on theme (optional)

### Implementation

**1. Add Theme Detection Command:**

```rust
// desktop-app/src/commands.rs

#[tauri::command]
pub async fn get_system_theme() -> Result<String, String> {
    // Detect system theme preference
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let personalization = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            .map_err(|e| format!("Failed to read registry: {}", e))?;

        let apps_use_light_theme: u32 = personalization.get_value("AppsUseLightTheme")
            .unwrap_or(1);

        Ok(if apps_use_light_theme == 0 { "dark".to_string() } else { "light".to_string() })
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        let output = Command::new("defaults")
            .args(&["read", "-g", "AppleInterfaceStyle"])
            .output()
            .map_err(|e| format!("Failed to read system theme: {}", e))?;

        let theme = String::from_utf8_lossy(&output.stdout);
        Ok(if theme.trim() == "Dark" { "dark".to_string() } else { "light".to_string() })
    }

    #[cfg(target_os = "linux")]
    {
        // Check GTK theme or use environment variable
        use std::env;

        if let Ok(theme) = env::var("GTK_THEME") {
            if theme.to_lowercase().contains("dark") {
                return Ok("dark".to_string());
            }
        }

        // Fallback: check gsettings
        use std::process::Command;
        if let Ok(output) = Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
        {
            let theme = String::from_utf8_lossy(&output.stdout);
            if theme.to_lowercase().contains("dark") {
                return Ok("dark".to_string());
            }
        }

        Ok("light".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok("light".to_string())
    }
}

#[tauri::command]
pub async fn watch_system_theme(
    window: tauri::Window,
) -> Result<(), String> {
    // Watch for system theme changes and emit events
    #[cfg(target_os = "windows")]
    {
        use winapi::um::winuser::*;
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                unsafe {
                    let msg = MSG {
                        hwnd: std::ptr::null_mut(),
                        message: WM_THEMECHANGED,
                        wParam: 0,
                        lParam: 0,
                        time: 0,
                        pt: POINT { x: 0, y: 0 },
                    };

                    if GetMessageW(&msg as *const _ as *mut _, std::ptr::null_mut(), 0, 0) > 0 {
                        if msg.message == WM_THEMECHANGED {
                            let _ = tx.send(());
                        }
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(_) = rx.recv() {
                let theme = get_system_theme().await.unwrap_or_else(|_| "light".to_string());
                let _ = window.emit("system-theme-changed", theme);
            }
        });
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Use NSDistributedNotificationCenter
        // This would require Objective-C bridge or Swift
        // For now, poll periodically
        tokio::spawn(async move {
            let mut last_theme = get_system_theme().await.unwrap_or_else(|_| "light".to_string());
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let current_theme = get_system_theme().await.unwrap_or_else(|_| "light".to_string());
                if current_theme != last_theme {
                    let _ = window.emit("system-theme-changed", &current_theme);
                    last_theme = current_theme;
                }
            }
        });
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Watch gsettings or use file watcher
        tokio::spawn(async move {
            let mut last_theme = get_system_theme().await.unwrap_or_else(|_| "light".to_string());
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let current_theme = get_system_theme().await.unwrap_or_else(|_| "light".to_string());
                if current_theme != last_theme {
                    let _ = window.emit("system-theme-changed", &current_theme);
                    last_theme = current_theme;
                }
            }
        });
    }

    Ok(())
}
```

**2. Update Frontend to Listen for System Theme:**

```typescript
// crates/mockforge-ui/ui/src/utils/tauri.ts

import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { useThemePaletteStore } from '@/stores/useThemePaletteStore';

export async function initSystemThemeSync() {
  if (!isTauri()) return;

  // Get initial system theme
  try {
    const systemTheme = await invoke<string>('get_system_theme');
    const themeStore = useThemePaletteStore.getState();

    // If user has "system" mode selected, apply system theme
    if (themeStore.mode === 'system') {
      themeStore.setMode('system'); // This will resolve to system theme
    }
  } catch (error) {
    console.error('Failed to get system theme:', error);
  }

  // Listen for system theme changes
  const unlisten = await listen<string>('system-theme-changed', (event) => {
    const themeStore = useThemePaletteStore.getState();
    if (themeStore.mode === 'system') {
      // Re-apply system mode to pick up new theme
      themeStore.setMode('system');
    }
  });

  // Start watching for system theme changes
  try {
    await invoke('watch_system_theme');
  } catch (error) {
    console.error('Failed to watch system theme:', error);
  }

  return unlisten;
}
```

**3. Initialize in App.tsx:**

```typescript
// crates/mockforge-ui/ui/src/App.tsx

useEffect(() => {
  // Initialize system theme sync for desktop app
  if (isTauri()) {
    import('@/utils/tauri').then(({ initSystemThemeSync }) => {
      initSystemThemeSync().catch((err) => {
        console.error('Failed to initialize system theme sync:', err);
      });
    });
  }
}, []);
```

---

## Auto-Update Implementation

### Current Status

Auto-update is **configured** in `tauri.conf.json` but needs implementation:
- Updater endpoints configured
- Update checking command exists
- Installation flow needs completion

### Implementation

**1. Enhanced Update Checker:**

```rust
// desktop-app/src/updater.rs

use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub release_date: String,
    pub release_notes: String,
    pub download_url: String,
    pub signature: String,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    use tauri::updater::builder;

    match builder(app.app_handle()).check().await {
        Ok(update) => {
            if update.is_some() {
                let update = update.unwrap();
                Ok(Some(UpdateInfo {
                    version: update.version,
                    release_date: update.date.unwrap_or_default(),
                    release_notes: update.body.unwrap_or_default(),
                    download_url: update.download_url,
                    signature: update.signature,
                }))
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri::updater::builder;

    match builder(app.app_handle()).check().await {
        Ok(Some(update)) => {
            // Show update dialog
            let should_install = show_update_dialog(&app, &update).await?;

            if should_install {
                // Download and install update
                update.download_and_install(
                    |chunk_length, content_length| {
                        // Progress callback
                        let _ = app.emit("update-progress", serde_json::json!({
                            "chunk_length": chunk_length,
                            "content_length": content_length,
                            "progress": if content_length > 0 {
                                (chunk_length as f64 / content_length as f64) * 100.0
                            } else {
                                0.0
                            }
                        }));
                    },
                    || {
                        // Finished callback
                        let _ = app.emit("update-finished", ());
                    },
                )
                .await
                .map_err(|e| format!("Update installation failed: {}", e))?;

                // Restart app
                app.restart();
            }

            Ok(())
        }
        Ok(None) => Err("No update available".to_string()),
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}

async fn show_update_dialog(app: &AppHandle, update: &tauri::updater::Update) -> Result<bool, String> {
    use tauri::api::dialog::MessageDialogBuilder;

    let message = format!(
        "A new version ({}) is available!\n\nRelease Notes:\n{}",
        update.version,
        update.body.as_deref().unwrap_or("Bug fixes and improvements.")
    );

    // Show native dialog
    let result = MessageDialogBuilder::new()
        .set_title("Update Available")
        .set_text(&message)
        .set_buttons(tauri::api::dialog::MessageDialogButtons::OkCancel)
        .show(|response| {
            response == tauri::api::dialog::MessageDialogButton::Ok
        });

    Ok(result)
}
```

**2. Auto-Check on Startup:**

```rust
// desktop-app/src/main.rs

.setup(move |app| {
    // ... existing setup ...

    // Check for updates on startup (optional, can be disabled)
    let app_handle = app.handle().clone();
    tokio::spawn(async move {
        // Wait a bit after startup
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Check for updates in background
        if let Ok(Some(update_info)) = updater::check_for_updates(app_handle.clone()).await {
            // Emit event to frontend
            if let Some(window) = app_handle.get_window("main") {
                let _ = window.emit("update-available", update_info);
            }
        }
    });

    Ok(())
})
```

**3. Frontend Update UI:**

```typescript
// crates/mockforge-ui/ui/src/components/desktop/UpdateNotification.tsx

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';
import { isTauri } from '@/utils/tauri';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Download, X } from 'lucide-react';

interface UpdateInfo {
  version: string;
  release_date: string;
  release_notes: string;
  download_url: string;
  signature: string;
}

export function UpdateNotification() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (!isTauri()) return;

    const setupUpdateListener = async () => {
      // Listen for update available event
      const unlisten = await listen<UpdateInfo>('update-available', (event) => {
        setUpdateInfo(event.payload);
      });

      // Listen for update progress
      const unlistenProgress = await listen<{ progress: number }>('update-progress', (event) => {
        setProgress(event.payload.progress);
        setIsInstalling(true);
      });

      // Listen for update finished
      const unlistenFinished = await listen('update-finished', () => {
        setIsInstalling(false);
        setProgress(100);
      });

      return () => {
        unlisten();
        unlistenProgress();
        unlistenFinished();
      };
    };

    setupUpdateListener();
  }, []);

  const handleInstall = async () => {
    if (!isTauri()) return;

    try {
      setIsInstalling(true);
      await invoke('install_update');
    } catch (error) {
      console.error('Failed to install update:', error);
      setIsInstalling(false);
    }
  };

  if (!updateInfo) return null;

  return (
    <Alert className="mb-4 border-orange-500 bg-orange-50 dark:bg-orange-900/20">
      <Download className="h-4 w-4 text-orange-600 dark:text-orange-400" />
      <AlertTitle className="text-orange-900 dark:text-orange-100">
        Update Available: v{updateInfo.version}
      </AlertTitle>
      <AlertDescription className="text-orange-800 dark:text-orange-200">
        <p className="mb-2">{updateInfo.release_notes}</p>
        {isInstalling ? (
          <div className="mt-2">
            <div className="w-full bg-orange-200 dark:bg-orange-800 rounded-full h-2">
              <div
                className="bg-orange-600 dark:bg-orange-400 h-2 rounded-full transition-all"
                style={{ width: `${progress}%` }}
              />
            </div>
            <p className="text-sm mt-1">Installing update... {Math.round(progress)}%</p>
          </div>
        ) : (
          <div className="flex gap-2 mt-2">
            <Button onClick={handleInstall} size="sm">
              Install Update
            </Button>
            <Button
              onClick={() => setUpdateInfo(null)}
              variant="ghost"
              size="sm"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}
      </AlertDescription>
    </Alert>
  );
}
```

---

## File Associations

### Current Status

File associations are **mentioned** but need OS-level registration.

### Implementation

**1. Update tauri.conf.json:**

```json
{
  "tauri": {
    "bundle": {
      "windows": {
        "fileAssociations": [
          {
            "ext": "yaml",
            "description": "MockForge Configuration",
            "icon": "icons/file-yaml.ico",
            "mimeType": "application/x-yaml"
          },
          {
            "ext": "yml",
            "description": "MockForge Configuration",
            "icon": "icons/file-yaml.ico",
            "mimeType": "application/x-yaml"
          },
          {
            "ext": "json",
            "description": "MockForge Configuration",
            "icon": "icons/file-json.ico",
            "mimeType": "application/json"
          }
        ]
      },
      "macOS": {
        "fileAssociations": [
          {
            "ext": "yaml",
            "icon": "icons/file-yaml.icns",
            "role": "Editor",
            "documentTypes": ["public.yaml", "public.text"]
          },
          {
            "ext": "yml",
            "icon": "icons/file-yaml.icns",
            "role": "Editor",
            "documentTypes": ["public.yaml", "public.text"]
          },
          {
            "ext": "json",
            "icon": "icons/file-json.icns",
            "role": "Editor",
            "documentTypes": ["public.json", "public.text"]
          }
        ]
      },
      "linux": {
        "fileAssociations": [
          {
            "ext": "yaml",
            "mimeType": "application/x-yaml",
            "icon": "icons/file-yaml.png"
          },
          {
            "ext": "yml",
            "mimeType": "application/x-yaml",
            "icon": "icons/file-yaml.png"
          },
          {
            "ext": "json",
            "mimeType": "application/json",
            "icon": "icons/file-json.png"
          }
        ]
      }
    }
  }
}
```

**2. Handle File Open Events:**

```rust
// desktop-app/src/main.rs

.setup(move |app| {
    // ... existing setup ...

    // Handle file open events (when app is opened with a file)
    if let Some(window) = app.get_window("main") {
        // Listen for file open events
        window.listen("tauri://file-open", move |event| {
            if let Some(paths) = event.payload() {
                if let Ok(paths) = serde_json::from_str::<Vec<String>>(paths) {
                    if let Some(path) = paths.first() {
                        // Emit event to frontend
                        if let Some(window) = app.get_window("main") {
                            let _ = window.emit("file-opened", path);
                        }
                    }
                }
            }
        });
    }

    Ok(())
})
```

**3. Frontend File Handler:**

```typescript
// crates/mockforge-ui/ui/src/utils/tauri.ts

export async function handleFileOpen(filePath: string) {
  if (!isTauri()) return;

  try {
    // Read file content
    const content = await invoke<string>('read_config_file', { path: filePath });

    // Parse and load into workspace
    const config = yaml.parse(content);

    // Emit event to load config
    // This would integrate with your workspace management
    window.dispatchEvent(new CustomEvent('config-loaded', { detail: { config, filePath } }));
  } catch (error) {
    console.error('Failed to open file:', error);
    // Show error notification
  }
}
```

---

## Additional Polish

### 1. System Tray Improvements

**Enhanced Tray Menu:**

```rust
// desktop-app/src/system_tray.rs

pub fn create_tray_menu(server_running: bool) -> SystemTrayMenu {
    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", "Show MockForge"))
        .add_item(CustomMenuItem::new("hide", "Hide"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new(
            if server_running { "stop-server" } else { "start-server" },
            if server_running { "Stop Server" } else { "Start Server" }
        ))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("settings", "Settings"))
        .add_item(CustomMenuItem::new("check-updates", "Check for Updates"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("about", "About"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "Quit"))
}
```

### 2. Keyboard Shortcuts

**Global Shortcuts:**

```rust
// desktop-app/src/shortcuts.rs

pub fn register_shortcuts(app: &AppHandle) -> Result<()> {
    use tauri::GlobalShortcutManager;

    let mut manager = app.global_shortcut_manager();

    // Show/Hide window
    manager.register("CmdOrCtrl+Shift+H", move |app| {
        if let Some(window) = app.get_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    })?;

    // Start/Stop server
    manager.register("CmdOrCtrl+Shift+S", move |app| {
        // Toggle server
        let _ = app.emit("toggle-server", ());
    })?;

    Ok(())
}
```

### 3. Native Notifications

**Enhanced Notifications:**

```rust
// desktop-app/src/notifications.rs

pub fn show_server_notification(app: &AppHandle, running: bool) {
    use tauri::api::notification::Notification;

    Notification::new(&app.app_handle())
        .title("MockForge")
        .body(if running {
            "Server started successfully"
        } else {
            "Server stopped"
        })
        .icon("icons/icon.png")
        .show()
        .ok();
}
```

---

## Testing

### Manual Testing Checklist

- [ ] **Dark Mode**
  - [ ] System theme detection works
  - [ ] Theme syncs with system changes
  - [ ] Theme preference persists across restarts
  - [ ] UI components render correctly in dark mode

- [ ] **Auto-Update**
  - [ ] Update check works on startup
  - [ ] Update notification appears
  - [ ] Update installation works
  - [ ] Progress indicator shows correctly
  - [ ] App restarts after update

- [ ] **File Associations**
  - [ ] Double-click .yaml file opens app
  - [ ] Double-click .yml file opens app
  - [ ] Double-click .json file opens app
  - [ ] File content loads correctly
  - [ ] Drag & drop works

- [ ] **System Tray**
  - [ ] Tray icon appears
  - [ ] Menu items work correctly
  - [ ] Show/hide works
  - [ ] Server start/stop works

- [ ] **Keyboard Shortcuts**
  - [ ] Show/hide shortcut works
  - [ ] Server toggle shortcut works
  - [ ] Shortcuts don't conflict with system

---

## Distribution

### Code Signing

**Windows:**
- Obtain code signing certificate
- Configure in `tauri.conf.json`:
```json
{
  "tauri": {
    "bundle": {
      "windows": {
        "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT"
      }
    }
  }
}
```

**macOS:**
- Apple Developer certificate required
- Configure signing identity:
```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "signingIdentity": "Developer ID Application: Your Name"
      }
    }
  }
}
```

### Update Server Setup

1. **Host Update Server:**
   - Serve update manifests at configured endpoints
   - Provide signed update packages
   - Version checking logic

2. **Update Manifest Format:**
```json
{
  "version": "0.2.9",
  "notes": "Bug fixes and improvements",
  "pub_date": "2024-01-01T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "...",
      "url": "https://updates.mockforge.dev/windows/MockForge_0.2.9_x64_en-US.msi"
    },
    "darwin-x86_64": {
      "signature": "...",
      "url": "https://updates.mockforge.dev/macos/MockForge_0.2.9_x64.app.tar.gz"
    },
    "linux-x86_64": {
      "signature": "...",
      "url": "https://updates.mockforge.dev/linux/mockforge_0.2.9_amd64.AppImage"
    }
  }
}
```

---

## Summary

### Implementation Checklist

- [x] Dark mode UI (already implemented)
- [ ] System theme detection
- [ ] System theme sync
- [ ] Auto-update checking
- [ ] Auto-update installation
- [ ] Update UI components
- [ ] File associations (OS-level)
- [ ] File open handling
- [ ] Enhanced system tray
- [ ] Keyboard shortcuts
- [ ] Native notifications
- [ ] Code signing setup
- [ ] Update server setup

---

**Last Updated**: 2024-01-01
**Version**: 1.0
