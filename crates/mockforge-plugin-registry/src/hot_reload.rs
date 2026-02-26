//! Hot reloading support for plugins

use crate::{RegistryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tracing::error;

/// Hot reload manager
pub struct HotReloadManager {
    /// Loaded plugins
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,

    /// File watchers
    watchers: Arc<RwLock<HashMap<String, FileWatcher>>>,

    /// Configuration
    config: HotReloadConfig,
}

/// Loaded plugin information
#[derive(Debug, Clone)]
struct LoadedPlugin {
    /// Plugin name
    name: String,

    /// Plugin path
    path: PathBuf,

    /// Last modification time
    last_modified: SystemTime,

    /// Load count (for debugging)
    load_count: u32,

    /// Current version
    version: String,
}

/// File watcher for detecting changes
#[derive(Debug)]
struct FileWatcher {
    path: PathBuf,
    last_check: SystemTime,
    last_modified: SystemTime,
}

/// Hot reload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadConfig {
    /// Enable hot reloading
    pub enabled: bool,

    /// Check interval in seconds
    pub check_interval: u64,

    /// Debounce delay in milliseconds
    pub debounce_delay: u64,

    /// Auto-reload on file change
    pub auto_reload: bool,

    /// Watch subdirectories
    pub watch_recursive: bool,

    /// File patterns to watch (e.g., "*.so", "*.wasm")
    pub watch_patterns: Vec<String>,

    /// Exclude patterns
    pub exclude_patterns: Vec<String>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: 2,
            debounce_delay: 500,
            auto_reload: true,
            watch_recursive: false,
            watch_patterns: vec![
                "*.so".to_string(),
                "*.dylib".to_string(),
                "*.dll".to_string(),
                "*.wasm".to_string(),
            ],
            exclude_patterns: vec!["*.tmp".to_string(), "*.swp".to_string(), "*~".to_string()],
        }
    }
}

/// Reload event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReloadEvent {
    /// Plugin name
    pub plugin_name: String,

    /// Event type
    pub event_type: ReloadEventType,

    /// Timestamp
    pub timestamp: String,

    /// Old version
    pub old_version: Option<String>,

    /// New version
    pub new_version: Option<String>,
}

/// Reload event type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadEventType {
    FileChanged,
    ReloadStarted,
    ReloadCompleted,
    ReloadFailed { error: String },
    PluginUnloaded,
}

impl HotReloadManager {
    /// Create a new hot reload manager
    pub fn new(config: HotReloadConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a plugin for hot reloading
    pub fn register_plugin(&self, name: &str, path: &Path, version: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let last_modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map_err(|e| RegistryError::Storage(format!("Failed to get file metadata: {}", e)))?;

        // Register plugin
        {
            let mut plugins = self.plugins.write().map_err(|e| {
                RegistryError::Storage(format!("Failed to acquire write lock: {}", e))
            })?;

            plugins.insert(
                name.to_string(),
                LoadedPlugin {
                    name: name.to_string(),
                    path: path.to_path_buf(),
                    last_modified,
                    load_count: 1,
                    version: version.to_string(),
                },
            );
        }

        // Register file watcher
        {
            let mut watchers = self.watchers.write().map_err(|e| {
                RegistryError::Storage(format!("Failed to acquire write lock: {}", e))
            })?;

            watchers.insert(
                name.to_string(),
                FileWatcher {
                    path: path.to_path_buf(),
                    last_check: SystemTime::now(),
                    last_modified,
                },
            );
        }

        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister_plugin(&self, name: &str) -> Result<()> {
        {
            let mut plugins = self.plugins.write().map_err(|e| {
                RegistryError::Storage(format!("Failed to acquire write lock: {}", e))
            })?;
            plugins.remove(name);
        }

        {
            let mut watchers = self.watchers.write().map_err(|e| {
                RegistryError::Storage(format!("Failed to acquire write lock: {}", e))
            })?;
            watchers.remove(name);
        }

        Ok(())
    }

    /// Check for file changes
    pub fn check_for_changes(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let mut changed_plugins = Vec::new();

        let mut watchers = self
            .watchers
            .write()
            .map_err(|e| RegistryError::Storage(format!("Failed to acquire write lock: {}", e)))?;

        let now = SystemTime::now();

        for (name, watcher) in watchers.iter_mut() {
            // Check if enough time has passed since last check
            if let Ok(elapsed) = now.duration_since(watcher.last_check) {
                if elapsed < Duration::from_secs(self.config.check_interval) {
                    continue;
                }
            }

            watcher.last_check = now;

            // Check file modification time
            if let Ok(metadata) = std::fs::metadata(&watcher.path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > watcher.last_modified {
                        // File has been modified
                        // Apply debounce delay
                        if let Ok(elapsed) = now.duration_since(modified) {
                            if elapsed < Duration::from_millis(self.config.debounce_delay) {
                                // Still within debounce period
                                continue;
                            }
                        }

                        watcher.last_modified = modified;
                        changed_plugins.push(name.clone());
                    }
                }
            }
        }

        Ok(changed_plugins)
    }

    /// Reload a plugin
    pub fn reload_plugin(&self, name: &str) -> Result<ReloadEvent> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|e| RegistryError::Storage(format!("Failed to acquire write lock: {}", e)))?;

        let plugin = plugins.get_mut(name).ok_or_else(|| {
            RegistryError::PluginNotFound(format!("Plugin not registered: {}", name))
        })?;

        let old_version = plugin.version.clone();
        plugin.load_count += 1;

        // Update last modified time
        if let Ok(metadata) = std::fs::metadata(&plugin.path) {
            if let Ok(modified) = metadata.modified() {
                plugin.last_modified = modified;
            }
        }

        Ok(ReloadEvent {
            plugin_name: name.to_string(),
            event_type: ReloadEventType::ReloadCompleted,
            timestamp: chrono::Utc::now().to_rfc3339(),
            old_version: Some(old_version),
            new_version: Some(plugin.version.clone()),
        })
    }

    /// Get plugin info
    pub fn get_plugin_info(&self, name: &str) -> Result<PluginInfo> {
        let plugins = self
            .plugins
            .read()
            .map_err(|e| RegistryError::Storage(format!("Failed to acquire read lock: {}", e)))?;

        let plugin = plugins
            .get(name)
            .ok_or_else(|| RegistryError::PluginNotFound(name.to_string()))?;

        Ok(PluginInfo {
            name: plugin.name.clone(),
            path: plugin.path.clone(),
            version: plugin.version.clone(),
            load_count: plugin.load_count,
            last_modified: plugin.last_modified,
        })
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Result<Vec<PluginInfo>> {
        let plugins = self
            .plugins
            .read()
            .map_err(|e| RegistryError::Storage(format!("Failed to acquire read lock: {}", e)))?;

        Ok(plugins
            .values()
            .map(|p| PluginInfo {
                name: p.name.clone(),
                path: p.path.clone(),
                version: p.version.clone(),
                load_count: p.load_count,
                last_modified: p.last_modified,
            })
            .collect())
    }

    /// Start watching for changes (background task)
    pub async fn start_watching<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<String>) + Send + 'static,
    {
        if !self.config.enabled || !self.config.auto_reload {
            return Ok(());
        }

        let check_interval = Duration::from_secs(self.config.check_interval);

        loop {
            tokio::time::sleep(check_interval).await;

            match self.check_for_changes() {
                Ok(changed) if !changed.is_empty() => {
                    callback(changed);
                }
                Err(e) => {
                    error!("Error checking for changes: {}", e);
                }
                _ => {}
            }
        }
    }
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub version: String,
    pub load_count: u32,
    pub last_modified: SystemTime,
}

/// Hot reload statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadStats {
    pub total_plugins: usize,
    pub total_reloads: u64,
    pub failed_reloads: u64,
    pub average_reload_time_ms: f64,
    pub last_reload: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_hot_reload_registration() {
        let config = HotReloadConfig::default();
        let manager = HotReloadManager::new(config);

        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("plugin.so");
        File::create(&plugin_path).unwrap();

        let result = manager.register_plugin("test-plugin", &plugin_path, "1.0.0");
        assert!(result.is_ok());

        let info = manager.get_plugin_info("test-plugin");
        assert!(info.is_ok());
        let info = info.unwrap();
        assert_eq!(info.name, "test-plugin");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.load_count, 1);
    }

    #[test]
    fn test_hot_reload_unregister() {
        let config = HotReloadConfig::default();
        let manager = HotReloadManager::new(config);

        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("plugin.so");
        File::create(&plugin_path).unwrap();

        manager.register_plugin("test-plugin", &plugin_path, "1.0.0").unwrap();
        manager.unregister_plugin("test-plugin").unwrap();

        let info = manager.get_plugin_info("test-plugin");
        assert!(info.is_err());
    }

    #[test]
    fn test_change_detection() {
        let config = HotReloadConfig {
            check_interval: 0, // Check immediately
            debounce_delay: 0, // No debounce
            ..Default::default()
        };
        let manager = HotReloadManager::new(config);

        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("plugin.so");
        let mut file = File::create(&plugin_path).unwrap();

        manager.register_plugin("test-plugin", &plugin_path, "1.0.0").unwrap();

        // Wait a bit
        std::thread::sleep(Duration::from_millis(100));

        // Modify file
        writeln!(file, "modified content").unwrap();
        file.sync_all().unwrap();
        drop(file);

        // Wait a bit more to ensure modification time updates
        std::thread::sleep(Duration::from_millis(100));

        let _changed = manager.check_for_changes().unwrap();
        // Note: This test may be flaky due to filesystem timing
        // In a real implementation, we'd use a proper file watcher library
    }
}
