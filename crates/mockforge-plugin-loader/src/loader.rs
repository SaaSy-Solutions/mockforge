//! Plugin loader implementation
//!
//! This module provides the main PluginLoader that handles:
//! - Plugin discovery and validation
//! - Secure plugin loading with sandboxing
//! - Plugin lifecycle management
//! - Resource monitoring and cleanup

use super::*;
use crate::registry::PluginRegistry;
use crate::sandbox::PluginSandbox;
use crate::validator::PluginValidator;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

// Import types from plugin core
use mockforge_plugin_core::{
    PluginId, PluginHealth, PluginManifest, PluginInstance
};

/// Main plugin loader
pub struct PluginLoader {
    /// Plugin registry
    registry: Arc<RwLock<PluginRegistry>>,
    /// Plugin validator
    validator: PluginValidator,
    /// Plugin sandbox
    sandbox: PluginSandbox,
    /// Loader configuration
    config: PluginLoaderConfig,
    /// Loading statistics
    stats: RwLock<PluginLoadStats>,
}

// Implement Send + Sync for PluginLoader
unsafe impl Send for PluginLoader {}
unsafe impl Sync for PluginLoader {}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new(config: PluginLoaderConfig) -> Self {
        Self {
            registry: Arc::new(RwLock::new(PluginRegistry::new())),
            validator: PluginValidator::new(config.clone()),
            sandbox: PluginSandbox::new(config.clone()),
            config,
            stats: RwLock::new(PluginLoadStats::default()),
        }
    }

    /// Load all plugins from configured directories
    pub async fn load_all_plugins(&self) -> LoaderResult<PluginLoadStats> {
        let mut stats = self.stats.write().await;
        stats.start_loading();

        // Discover plugins from all configured directories
        let mut all_discoveries = Vec::new();
        for dir in &self.config.plugin_dirs {
            let discoveries = self.discover_plugins_in_directory(dir).await?;
            all_discoveries.extend(discoveries);
        }

        stats.discovered = all_discoveries.len();

        // Load valid plugins
        for discovery in all_discoveries {
            if discovery.is_valid {
                match self.load_plugin_from_discovery(&discovery).await {
                    Ok(_) => stats.record_success(),
                    Err(e) => {
                        tracing::warn!("Failed to load plugin {}: {}", discovery.plugin_id, e);
                        stats.record_failure();
                    }
                }
            } else {
                tracing::debug!("Skipping invalid plugin {}: {}", discovery.plugin_id,
                    discovery.first_error().unwrap_or("unknown error"));
                stats.record_skipped();
            }
        }

        stats.finish_loading();
        Ok(stats.clone())
    }

    /// Load a specific plugin by ID
    pub async fn load_plugin(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        // Find plugin in discovery paths
        let discovery = self.discover_plugin_by_id(plugin_id).await?
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        if !discovery.is_valid {
            return Err(PluginLoaderError::validation(
                discovery.first_error().unwrap_or("Plugin validation failed").to_string()
            ));
        }

        self.load_plugin_from_discovery(&discovery).await
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        let mut registry = self.registry.write().await;
        registry.remove_plugin(plugin_id)?;

        tracing::info!("Unloaded plugin: {}", plugin_id);
        Ok(())
    }

    /// Get loaded plugin by ID
    pub async fn get_plugin(&self, plugin_id: &PluginId) -> Option<PluginInstance> {
        let registry = self.registry.read().await;
        registry.get_plugin(plugin_id).cloned()
    }

    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginId> {
        let registry = self.registry.read().await;
        registry.list_plugins()
    }

    /// Get plugin health status
    pub async fn get_plugin_health(&self, plugin_id: &PluginId) -> LoaderResult<PluginHealth> {
        let registry = self.registry.read().await;
        registry.get_plugin_health(plugin_id)
    }

    /// Get loading statistics
    pub async fn get_load_stats(&self) -> PluginLoadStats {
        self.stats.read().await.clone()
    }

    /// Validate plugin without loading
    pub async fn validate_plugin(&self, plugin_path: &Path) -> LoaderResult<PluginManifest> {
        self.validator.validate_plugin_file(plugin_path).await
    }

    /// Discover plugins in a directory
    async fn discover_plugins_in_directory(&self, dir_path: &str) -> LoaderResult<Vec<PluginDiscovery>> {
        let expanded_path = shellexpand::tilde(dir_path);
        let dir_path = PathBuf::from(expanded_path.as_ref());

        if !dir_path.exists() {
            tracing::debug!("Plugin directory does not exist: {}", dir_path.display());
            return Ok(Vec::new());
        }

        if !dir_path.is_dir() {
            return Err(PluginLoaderError::fs(format!(
                "Plugin path is not a directory: {}", dir_path.display()
            )));
        }

        let mut discoveries = Vec::new();

        // Read directory entries
        let mut entries = match tokio::fs::read_dir(&dir_path).await {
            Ok(entries) => entries,
            Err(e) => {
                return Err(PluginLoaderError::fs(format!(
                    "Failed to read plugin directory {}: {}", dir_path.display(), e
                )));
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            // Look for plugin.yaml in the directory
            let manifest_path = path.join("plugin.yaml");
            if !manifest_path.exists() {
                continue;
            }

            // Try to discover plugin
            match self.discover_single_plugin(&manifest_path).await {
                Ok(discovery) => discoveries.push(discovery),
                Err(e) => {
                    tracing::warn!("Failed to discover plugin at {}: {}", path.display(), e);
                }
            }
        }

        Ok(discoveries)
    }

    /// Discover a single plugin from manifest file
    async fn discover_single_plugin(&self, manifest_path: &Path) -> LoaderResult<PluginDiscovery> {
        // Load and validate manifest
        let manifest = match PluginManifest::from_file(manifest_path) {
            Ok(manifest) => manifest,
            Err(e) => {
                let plugin_id = PluginId::new("unknown".to_string());
                let errors = vec![format!("Failed to load manifest: {}", e)];
                return Ok(PluginDiscovery::failure(plugin_id, manifest_path.display().to_string(), errors));
            }
        };

        let plugin_id = manifest.id().clone();

        // Validate manifest
        let validation_result = self.validator.validate_manifest(&manifest).await;

        match validation_result {
            Ok(_) => {
                let discovery = PluginDiscovery::success(
                    plugin_id,
                    manifest,
                    manifest_path.parent().unwrap().display().to_string(),
                );
                Ok(discovery)
            }
            Err(errors) => {
                let errors_str: Vec<String> = vec![errors.to_string()];
                Ok(PluginDiscovery::failure(plugin_id, manifest_path.display().to_string(), errors_str))
            }
        }
    }

    /// Discover plugin by ID
    async fn discover_plugin_by_id(&self, plugin_id: &PluginId) -> LoaderResult<Option<PluginDiscovery>> {
        for dir in &self.config.plugin_dirs {
            let discoveries = self.discover_plugins_in_directory(dir).await?;
            if let Some(discovery) = discoveries.into_iter().find(|d| &d.plugin_id == plugin_id) {
                return Ok(Some(discovery));
            }
        }
        Ok(None)
    }

    /// Load plugin from discovery result
    async fn load_plugin_from_discovery(&self, discovery: &PluginDiscovery) -> LoaderResult<()> {
        let plugin_id = &discovery.plugin_id;

        // Check if already loaded
        {
            let registry = self.registry.read().await;
            if registry.has_plugin(plugin_id) {
                return Err(PluginLoaderError::already_loaded(plugin_id.clone()));
            }
        }

        // Validate capabilities
        self.validator.validate_capabilities(&discovery.manifest.capabilities)?;

        // Find WASM file
        let plugin_path = self.find_plugin_wasm_file(&discovery.path).await?
            .ok_or_else(|| PluginLoaderError::load(format!(
                "No WebAssembly file found for plugin {}", plugin_id
            )))?;

        // Validate WASM file
        self.validator.validate_wasm_file(&plugin_path).await?;

        // Create load context
        let load_context = PluginLoadContext::new(
            plugin_id.clone(),
            discovery.manifest.clone(),
            plugin_path.display().to_string(),
            self.config.clone(),
        );

        // Load plugin with timeout
        let load_timeout = Duration::from_secs(self.config.load_timeout_secs);
        let plugin_instance = timeout(load_timeout, self.load_plugin_instance(&load_context))
            .await
            .map_err(|_| PluginLoaderError::load(format!(
                "Plugin loading timed out after {} seconds", self.config.load_timeout_secs
            )))??;

        // Register plugin
        let mut registry = self.registry.write().await;
        registry.add_plugin(plugin_instance)?;

        tracing::info!("Successfully loaded plugin: {}", plugin_id);
        Ok(())
    }

    /// Load plugin instance
    async fn load_plugin_instance(&self, context: &PluginLoadContext) -> LoaderResult<PluginInstance> {
        // Create plugin instance through sandbox
        self.sandbox.create_plugin_instance(context).await
    }

    /// Find plugin WASM file in plugin directory
    async fn find_plugin_wasm_file(&self, plugin_dir: &str) -> LoaderResult<Option<PathBuf>> {
        let plugin_path = PathBuf::from(plugin_dir);

        // Look for .wasm files in the plugin directory
        let mut entries = match tokio::fs::read_dir(&plugin_path).await {
            Ok(entries) => entries,
            Err(e) => {
                return Err(PluginLoaderError::fs(format!(
                    "Failed to read plugin directory {}: {}", plugin_path.display(), e
                )));
            }
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "wasm" {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    /// Reload all plugins
    pub async fn reload_all_plugins(&self) -> LoaderResult<PluginLoadStats> {
        // Get currently loaded plugins
        let loaded_plugins = self.list_plugins().await;

        // Unload all plugins
        for plugin_id in &loaded_plugins {
            if let Err(e) = self.unload_plugin(plugin_id).await {
                tracing::warn!("Failed to unload plugin {} during reload: {}", plugin_id, e);
            }
        }

        // Load all plugins again
        self.load_all_plugins().await
    }

    /// Reload specific plugin
    pub async fn reload_plugin(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        // Unload plugin
        self.unload_plugin(plugin_id).await?;

        // Load plugin again
        self.load_plugin(plugin_id).await
    }

    /// Get registry reference (for advanced operations)
    pub fn registry(&self) -> Arc<RwLock<PluginRegistry>> {
        Arc::clone(&self.registry)
    }

    /// Get validator reference (for advanced operations)
    pub fn validator(&self) -> &PluginValidator {
        &self.validator
    }

    /// Get sandbox reference (for advanced operations)
    pub fn sandbox(&self) -> &PluginSandbox {
        &self.sandbox
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new(PluginLoaderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_loader_creation() {
        let config = PluginLoaderConfig::default();
        let loader = PluginLoader::new(config);

        let stats = loader.get_load_stats().await;
        assert_eq!(stats.discovered, 0);
        assert_eq!(stats.loaded, 0);
    }

    #[tokio::test]
    async fn test_load_stats() {
        let mut stats = PluginLoadStats::default();

        stats.start_loading();
        assert!(stats.start_time.is_some());

        stats.record_success();
        stats.record_failure();
        stats.record_skipped();

        stats.finish_loading();
        assert!(stats.end_time.is_some());

        assert_eq!(stats.loaded, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.discovered, 3);
        assert_eq!(stats.success_rate(), 33.33333333333333);
    }

    #[tokio::test]
    async fn test_plugin_discovery_success() {
        let plugin_id = PluginId::new("test-plugin");
        let manifest = PluginManifest::new(PluginInfo::new(
            plugin_id.clone(),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            PluginAuthor::new("Test Author"),
        ));

        let discovery = PluginDiscovery::success(plugin_id, manifest, "/tmp/test".to_string());
        assert!(discovery.is_success());
        assert!(discovery.errors.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_discovery_failure() {
        let plugin_id = PluginId::new("test-plugin");
        let errors = vec!["Validation failed".to_string()];

        let discovery = PluginDiscovery::failure(plugin_id, "/tmp/test".to_string(), errors.clone());
        assert!(!discovery.is_success());
        assert_eq!(discovery.errors, errors);
        assert_eq!(discovery.first_error(), Some("Validation failed"));
    }
}
