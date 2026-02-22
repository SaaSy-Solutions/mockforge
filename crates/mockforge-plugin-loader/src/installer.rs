//! Unified plugin installer
//!
//! This module provides a high-level API for installing plugins from various sources:
//! - Local file paths
//! - HTTP/HTTPS URLs
//! - Git repositories
//! - Plugin registries (future)
//!
//! It automatically detects the source type and uses the appropriate loader.

use super::*;
use crate::git::{GitPluginConfig, GitPluginLoader, GitPluginSource};
use crate::loader::PluginLoader;
use crate::metadata::{MetadataStore, PluginMetadata};
use crate::remote::{RemotePluginConfig, RemotePluginLoader};
use crate::signature::SignatureVerifier;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Plugin source specification
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Local file path or directory
    Local(PathBuf),
    /// HTTP/HTTPS URL
    Url {
        /// URL to download the plugin from
        url: String,
        /// Optional SHA-256 checksum for verification
        checksum: Option<String>,
    },
    /// Git repository
    Git(GitPluginSource),
    /// Plugin registry (future)
    Registry {
        /// Plugin name in the registry
        name: String,
        /// Optional version string (defaults to latest)
        version: Option<String>,
    },
}

impl PluginSource {
    /// Parse a plugin source from a string
    ///
    /// Automatically detects the source type:
    /// - Starts with "http://" or "https://" → URL
    /// - Contains ".git" or starts with "git@" → Git
    /// - Contains "/" or "\" → Local path
    /// - Otherwise → Registry name
    pub fn parse(input: &str) -> LoaderResult<Self> {
        let input = input.trim();

        // Check for URL
        if input.starts_with("http://") || input.starts_with("https://") {
            // Check if it's a Git repository URL
            if input.contains(".git")
                || input.contains("github.com")
                || input.contains("gitlab.com")
            {
                let source = GitPluginSource::parse(input)?;
                return Ok(PluginSource::Git(source));
            }
            return Ok(PluginSource::Url {
                url: input.to_string(),
                checksum: None,
            });
        }

        // Check for SSH Git URL
        if input.starts_with("git@") {
            let source = GitPluginSource::parse(input)?;
            return Ok(PluginSource::Git(source));
        }

        // Check for local path
        if input.contains('/') || input.contains('\\') || Path::new(input).exists() {
            return Ok(PluginSource::Local(PathBuf::from(input)));
        }

        // Parse as registry reference
        let (name, version) = if let Some((n, v)) = input.split_once('@') {
            (n.to_string(), Some(v.to_string()))
        } else {
            (input.to_string(), None)
        };

        Ok(PluginSource::Registry { name, version })
    }
}

impl std::fmt::Display for PluginSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginSource::Local(path) => write!(f, "local:{}", path.display()),
            PluginSource::Url { url, .. } => write!(f, "url:{}", url),
            PluginSource::Git(source) => write!(f, "git:{}", source),
            PluginSource::Registry { name, version } => {
                if let Some(v) = version {
                    write!(f, "registry:{}@{}", name, v)
                } else {
                    write!(f, "registry:{}", name)
                }
            }
        }
    }
}

/// Installation options
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Force reinstall even if plugin already exists
    pub force: bool,
    /// Skip validation checks
    pub skip_validation: bool,
    /// Verify plugin signature (if available)
    pub verify_signature: bool,
    /// Expected checksum for verification (URL sources)
    pub expected_checksum: Option<String>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            force: false,
            skip_validation: false,
            verify_signature: true,
            expected_checksum: None,
        }
    }
}

/// Unified plugin installer
pub struct PluginInstaller {
    loader: PluginLoader,
    remote_loader: RemotePluginLoader,
    git_loader: GitPluginLoader,
    config: PluginLoaderConfig,
    metadata_store: std::sync::Arc<tokio::sync::RwLock<MetadataStore>>,
}

impl PluginInstaller {
    /// Create a new plugin installer with default configuration
    pub fn new(loader_config: PluginLoaderConfig) -> LoaderResult<Self> {
        let loader = PluginLoader::new(loader_config.clone());
        let remote_loader = RemotePluginLoader::new(RemotePluginConfig::default())?;
        let git_loader = GitPluginLoader::new(GitPluginConfig::default())?;

        // Create metadata store in a standard location
        let metadata_dir = shellexpand::tilde("~/.mockforge/plugin-metadata");
        let metadata_store = MetadataStore::new(PathBuf::from(metadata_dir.as_ref()));

        Ok(Self {
            loader,
            remote_loader,
            git_loader,
            config: loader_config,
            metadata_store: std::sync::Arc::new(tokio::sync::RwLock::new(metadata_store)),
        })
    }

    /// Initialize the installer (creates directories, loads metadata)
    pub async fn init(&self) -> LoaderResult<()> {
        let mut store = self.metadata_store.write().await;
        store.load().await
    }

    /// Install a plugin from a source string
    ///
    /// Automatically detects and handles the source type
    pub async fn install(
        &self,
        source_str: &str,
        options: InstallOptions,
    ) -> LoaderResult<PluginId> {
        let source = PluginSource::parse(source_str)?;
        self.install_from_source(&source, options).await
    }

    /// Install a plugin from a specific source
    pub async fn install_from_source(
        &self,
        source: &PluginSource,
        options: InstallOptions,
    ) -> LoaderResult<PluginId> {
        tracing::info!("Installing plugin from source: {}", source);

        // Get the plugin directory
        let plugin_dir = match source {
            PluginSource::Local(path) => path.clone(),
            PluginSource::Url { url, checksum } => {
                let checksum_ref = checksum.as_deref().or(options.expected_checksum.as_deref());
                self.remote_loader.download_with_checksum(url, checksum_ref).await?
            }
            PluginSource::Git(git_source) => self.git_loader.clone_from_git(git_source).await?,
            PluginSource::Registry { name, version } => {
                self.install_from_registry(name, version.as_deref(), &options).await?
            }
        };

        // Verify signature if enabled
        if options.verify_signature && !options.skip_validation {
            if let Err(e) = self.verify_plugin_signature(&plugin_dir) {
                tracing::warn!("Plugin signature verification failed: {}", e);
                // Don't fail installation, just warn
            }
        }

        // Validate the plugin
        if !options.skip_validation {
            self.loader.validate_plugin(&plugin_dir).await?;
        }

        // Load the plugin
        let manifest = self.loader.validate_plugin(&plugin_dir).await?;
        let plugin_id = manifest.info.id.clone();

        // Check if plugin is already loaded
        if self.loader.get_plugin(&plugin_id).await.is_some() && !options.force {
            return Err(PluginLoaderError::already_loaded(plugin_id));
        }

        // Load the plugin
        self.loader.load_plugin(&plugin_id).await?;

        // Save metadata for future updates
        let version = manifest.info.version.to_string();
        let metadata = PluginMetadata::new(plugin_id.clone(), source.clone(), version);
        let mut store = self.metadata_store.write().await;
        store.save(metadata).await?;

        tracing::info!("Plugin installed successfully: {}", plugin_id);
        Ok(plugin_id)
    }

    /// Install plugin from a registry source.
    async fn install_from_registry(
        &self,
        name: &str,
        version: Option<&str>,
        options: &InstallOptions,
    ) -> LoaderResult<PathBuf> {
        let base_url = std::env::var("MOCKFORGE_PLUGIN_REGISTRY_URL")
            .unwrap_or_else(|_| "https://registry.mockforge.dev".to_string());
        let client = reqwest::Client::new();

        let (download_url, checksum) = if let Some(v) = version {
            let version_url = format!("{}/api/v1/plugins/{}/versions/{}", base_url, name, v);
            let response = client.get(&version_url).send().await.map_err(|e| {
                PluginLoaderError::load(format!(
                    "Failed to fetch registry version metadata for {}@{}: {}",
                    name, v, e
                ))
            })?;

            if !response.status().is_success() {
                return Err(PluginLoaderError::load(format!(
                    "Registry lookup failed for {}@{}: {}",
                    name,
                    v,
                    response.status()
                )));
            }

            let entry: RegistryVersionResponse = response.json().await.map_err(|e| {
                PluginLoaderError::load(format!(
                    "Invalid registry response for {}@{}: {}",
                    name, v, e
                ))
            })?;
            (entry.download_url, entry.checksum)
        } else {
            let plugin_url = format!("{}/api/v1/plugins/{}", base_url, name);
            let response = client.get(&plugin_url).send().await.map_err(|e| {
                PluginLoaderError::load(format!(
                    "Failed to fetch registry plugin metadata for {}: {}",
                    name, e
                ))
            })?;

            if !response.status().is_success() {
                return Err(PluginLoaderError::load(format!(
                    "Registry lookup failed for {}: {}",
                    name,
                    response.status()
                )));
            }

            let entry: RegistryPluginResponse = response.json().await.map_err(|e| {
                PluginLoaderError::load(format!("Invalid registry response for {}: {}", name, e))
            })?;

            let selected = select_registry_version(&entry).ok_or_else(|| {
                PluginLoaderError::load(format!(
                    "No installable versions found for plugin '{}'",
                    name
                ))
            })?;
            (selected.download_url.clone(), selected.checksum.clone())
        };

        let checksum_ref = options.expected_checksum.as_deref().or(checksum.as_deref());

        self.remote_loader.download_with_checksum(&download_url, checksum_ref).await
    }

    /// Verify plugin signature using cryptographic verification
    fn verify_plugin_signature(&self, plugin_dir: &Path) -> LoaderResult<()> {
        let verifier = SignatureVerifier::new(&self.config);
        verifier.verify_plugin_signature(plugin_dir)
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        self.loader.unload_plugin(plugin_id).await?;

        // Remove metadata
        let mut store = self.metadata_store.write().await;
        store.remove(plugin_id).await?;

        Ok(())
    }

    /// List installed plugins
    pub async fn list_installed(&self) -> Vec<PluginId> {
        self.loader.list_plugins().await
    }

    /// Update a plugin to the latest version
    pub async fn update(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        tracing::info!("Updating plugin: {}", plugin_id);

        // Get plugin metadata to find original source
        let metadata = {
            let store = self.metadata_store.read().await;
            store.get(plugin_id).cloned().ok_or_else(|| {
                PluginLoaderError::load(format!(
                    "No installation metadata found for plugin {}. Cannot update.",
                    plugin_id
                ))
            })?
        };

        tracing::info!("Updating plugin {} from source: {}", plugin_id, metadata.source);

        // Unload the plugin first
        if self.loader.get_plugin(plugin_id).await.is_some() {
            self.loader.unload_plugin(plugin_id).await?;
        }

        // Reinstall from original source with force flag
        let options = InstallOptions {
            force: true,
            skip_validation: false,
            verify_signature: true,
            expected_checksum: None,
        };

        let new_plugin_id = self.install_from_source(&metadata.source, options).await?;

        // Verify it's the same plugin
        if new_plugin_id != *plugin_id {
            return Err(PluginLoaderError::load(format!(
                "Plugin ID mismatch after update: expected {}, got {}",
                plugin_id, new_plugin_id
            )));
        }

        // Update metadata with new version
        let new_manifest = self
            .loader
            .get_plugin(&new_plugin_id)
            .await
            .ok_or_else(|| PluginLoaderError::load("Failed to get updated plugin"))?
            .manifest;

        let mut store = self.metadata_store.write().await;
        if let Some(meta) = store.get(plugin_id).cloned() {
            let mut updated_meta = meta;
            updated_meta.mark_updated(new_manifest.info.version.to_string());
            store.save(updated_meta).await?;
        }

        tracing::info!("Plugin {} updated successfully", plugin_id);
        Ok(())
    }

    /// Update all plugins to their latest versions
    pub async fn update_all(&self) -> LoaderResult<Vec<PluginId>> {
        tracing::info!("Updating all plugins");

        // Get list of all plugins with metadata
        let plugin_ids = {
            let store = self.metadata_store.read().await;
            store.list()
        };

        if plugin_ids.is_empty() {
            tracing::info!("No plugins found with metadata to update");
            return Ok(Vec::new());
        }

        tracing::info!("Found {} plugins to update", plugin_ids.len());

        let mut updated = Vec::new();
        let mut failed = Vec::new();

        // Update each plugin
        for plugin_id in plugin_ids {
            match self.update(&plugin_id).await {
                Ok(_) => {
                    tracing::info!("Successfully updated plugin: {}", plugin_id);
                    updated.push(plugin_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to update plugin {}: {}", plugin_id, e);
                    failed.push((plugin_id, e.to_string()));
                }
            }
        }

        tracing::info!(
            "Plugin update complete: {} succeeded, {} failed",
            updated.len(),
            failed.len()
        );

        if !failed.is_empty() {
            let failed_list = failed
                .iter()
                .map(|(id, err)| format!("{}: {}", id, err))
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!("Failed updates: {}", failed_list);
        }

        Ok(updated)
    }

    /// Clear all caches (downloads and Git repositories)
    pub async fn clear_caches(&self) -> LoaderResult<()> {
        self.remote_loader.clear_cache().await?;
        self.git_loader.clear_cache().await?;
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> LoaderResult<CacheStats> {
        let download_cache_size = self.remote_loader.get_cache_size()?;
        let git_cache_size = self.git_loader.get_cache_size()?;

        Ok(CacheStats {
            download_cache_size,
            git_cache_size,
            total_size: download_cache_size + git_cache_size,
        })
    }

    /// Get plugin metadata
    pub async fn get_plugin_metadata(&self, plugin_id: &PluginId) -> Option<PluginMetadata> {
        let store = self.metadata_store.read().await;
        store.get(plugin_id).cloned()
    }

    /// List all plugins with metadata
    pub async fn list_plugins_with_metadata(&self) -> Vec<(PluginId, PluginMetadata)> {
        let store = self.metadata_store.read().await;
        store
            .list()
            .into_iter()
            .filter_map(|id| store.get(&id).map(|meta| (id, meta.clone())))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct RegistryVersionResponse {
    download_url: String,
    #[serde(default)]
    checksum: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryPluginResponse {
    version: String,
    versions: Vec<RegistryVersionResponseWithVersion>,
}

#[derive(Debug, Deserialize)]
struct RegistryVersionResponseWithVersion {
    version: String,
    download_url: String,
    #[serde(default)]
    checksum: Option<String>,
    #[serde(default)]
    yanked: bool,
}

fn select_registry_version(
    entry: &RegistryPluginResponse,
) -> Option<&RegistryVersionResponseWithVersion> {
    if let Some(preferred) = entry.versions.iter().find(|v| v.version == entry.version && !v.yanked)
    {
        return Some(preferred);
    }

    entry.versions.iter().find(|v| !v.yanked)
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Download cache size in bytes
    pub download_cache_size: u64,
    /// Git repository cache size in bytes
    pub git_cache_size: u64,
    /// Total cache size in bytes
    pub total_size: u64,
}

impl CacheStats {
    /// Format cache size as human-readable string
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// Get download cache size as human-readable string
    pub fn download_cache_formatted(&self) -> String {
        Self::format_size(self.download_cache_size)
    }

    /// Get Git cache size as human-readable string
    pub fn git_cache_formatted(&self) -> String {
        Self::format_size(self.git_cache_size)
    }

    /// Get total cache size as human-readable string
    pub fn total_formatted(&self) -> String {
        Self::format_size(self.total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== PluginSource Tests =====

    #[test]
    fn test_plugin_source_parse_url() {
        let source = PluginSource::parse("https://example.com/plugin.zip").unwrap();
        assert!(matches!(source, PluginSource::Url { .. }));
    }

    #[test]
    fn test_plugin_source_parse_git_https() {
        let source = PluginSource::parse("https://github.com/user/repo").unwrap();
        assert!(matches!(source, PluginSource::Git(_)));
    }

    #[test]
    fn test_plugin_source_parse_git_ssh() {
        let source = PluginSource::parse("git@github.com:user/repo.git").unwrap();
        assert!(matches!(source, PluginSource::Git(_)));
    }

    #[test]
    fn test_plugin_source_parse_gitlab() {
        let source = PluginSource::parse("https://gitlab.com/user/repo").unwrap();
        assert!(matches!(source, PluginSource::Git(_)));
    }

    #[test]
    fn test_plugin_source_parse_local() {
        let source = PluginSource::parse("/path/to/plugin").unwrap();
        assert!(matches!(source, PluginSource::Local(_)));

        let source = PluginSource::parse("./relative/path").unwrap();
        assert!(matches!(source, PluginSource::Local(_)));
    }

    #[test]
    fn test_plugin_source_parse_registry() {
        let source = PluginSource::parse("auth-jwt").unwrap();
        assert!(matches!(source, PluginSource::Registry { .. }));

        let source = PluginSource::parse("auth-jwt@1.0.0").unwrap();
        if let PluginSource::Registry { name, version } = source {
            assert_eq!(name, "auth-jwt");
            assert_eq!(version, Some("1.0.0".to_string()));
        } else {
            panic!("Expected Registry source");
        }
    }

    #[test]
    fn test_plugin_source_parse_registry_without_version() {
        let source = PluginSource::parse("my-plugin").unwrap();
        if let PluginSource::Registry { name, version } = source {
            assert_eq!(name, "my-plugin");
            assert!(version.is_none());
        } else {
            panic!("Expected Registry source");
        }
    }

    #[test]
    fn test_plugin_source_parse_url_with_checksum() {
        let source = PluginSource::parse("https://example.com/plugin.zip").unwrap();
        if let PluginSource::Url { url, checksum } = source {
            assert_eq!(url, "https://example.com/plugin.zip");
            assert!(checksum.is_none());
        } else {
            panic!("Expected URL source");
        }
    }

    #[test]
    fn test_plugin_source_parse_empty_string() {
        let source = PluginSource::parse("").unwrap();
        // Empty string should be treated as registry name
        assert!(matches!(source, PluginSource::Registry { .. }));
    }

    #[test]
    fn test_plugin_source_parse_whitespace() {
        let source = PluginSource::parse("  https://example.com/plugin.zip  ").unwrap();
        assert!(matches!(source, PluginSource::Url { .. }));
    }

    #[test]
    fn test_plugin_source_display() {
        let source = PluginSource::Local(PathBuf::from("/tmp/plugin"));
        assert_eq!(source.to_string(), "local:/tmp/plugin");

        let source = PluginSource::Url {
            url: "https://example.com/plugin.zip".to_string(),
            checksum: None,
        };
        assert_eq!(source.to_string(), "url:https://example.com/plugin.zip");

        let source = PluginSource::Registry {
            name: "my-plugin".to_string(),
            version: Some("1.0.0".to_string()),
        };
        assert_eq!(source.to_string(), "registry:my-plugin@1.0.0");

        let source = PluginSource::Registry {
            name: "my-plugin".to_string(),
            version: None,
        };
        assert_eq!(source.to_string(), "registry:my-plugin");
    }

    #[test]
    fn test_plugin_source_clone() {
        let source = PluginSource::Local(PathBuf::from("/tmp"));
        let cloned = source.clone();
        assert_eq!(source.to_string(), cloned.to_string());
    }

    // ===== InstallOptions Tests =====

    #[test]
    fn test_install_options_default() {
        let options = InstallOptions::default();
        assert!(!options.force);
        assert!(!options.skip_validation);
        assert!(options.verify_signature);
        assert!(options.expected_checksum.is_none());
    }

    #[test]
    fn test_install_options_with_force() {
        let options = InstallOptions {
            force: true,
            ..Default::default()
        };
        assert!(options.force);
    }

    #[test]
    fn test_install_options_with_checksum() {
        let options = InstallOptions {
            expected_checksum: Some("abc123".to_string()),
            ..Default::default()
        };
        assert_eq!(options.expected_checksum, Some("abc123".to_string()));
    }

    #[test]
    fn test_install_options_skip_validation() {
        let options = InstallOptions {
            skip_validation: true,
            verify_signature: false,
            ..Default::default()
        };
        assert!(options.skip_validation);
        assert!(!options.verify_signature);
    }

    #[test]
    fn test_install_options_clone() {
        let options = InstallOptions {
            force: true,
            skip_validation: false,
            verify_signature: true,
            expected_checksum: Some("test".to_string()),
        };
        let cloned = options.clone();
        assert_eq!(options.force, cloned.force);
        assert_eq!(options.expected_checksum, cloned.expected_checksum);
    }

    // ===== CacheStats Tests =====

    #[test]
    fn test_cache_stats_formatting() {
        assert_eq!(CacheStats::format_size(512), "512 bytes");
        assert_eq!(CacheStats::format_size(1024), "1.00 KB");
        assert_eq!(CacheStats::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(CacheStats::format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_cache_stats_edge_cases() {
        assert_eq!(CacheStats::format_size(0), "0 bytes");
        assert_eq!(CacheStats::format_size(1), "1 bytes");
        assert_eq!(CacheStats::format_size(1023), "1023 bytes");
        assert_eq!(CacheStats::format_size(1025), "1.00 KB");
    }

    #[test]
    fn test_cache_stats_large_values() {
        let tb = 1024u64 * 1024 * 1024 * 1024;
        assert!(CacheStats::format_size(tb).contains("GB"));
    }

    #[test]
    fn test_cache_stats_formatted_methods() {
        let stats = CacheStats {
            download_cache_size: 1024 * 1024,
            git_cache_size: 2 * 1024 * 1024,
            total_size: 3 * 1024 * 1024,
        };

        assert_eq!(stats.download_cache_formatted(), "1.00 MB");
        assert_eq!(stats.git_cache_formatted(), "2.00 MB");
        assert_eq!(stats.total_formatted(), "3.00 MB");
    }

    #[test]
    fn test_cache_stats_total_calculation() {
        let stats = CacheStats {
            download_cache_size: 100,
            git_cache_size: 200,
            total_size: 300,
        };

        assert_eq!(stats.total_size, stats.download_cache_size + stats.git_cache_size);
    }

    #[test]
    fn test_cache_stats_clone() {
        let stats = CacheStats {
            download_cache_size: 1024,
            git_cache_size: 2048,
            total_size: 3072,
        };

        let cloned = stats.clone();
        assert_eq!(stats.download_cache_size, cloned.download_cache_size);
        assert_eq!(stats.git_cache_size, cloned.git_cache_size);
        assert_eq!(stats.total_size, cloned.total_size);
    }

    // ===== Edge Cases and Error Handling =====

    #[test]
    fn test_plugin_source_parse_http_url() {
        let source = PluginSource::parse("http://example.com/plugin.zip").unwrap();
        assert!(matches!(source, PluginSource::Url { .. }));
    }

    #[test]
    fn test_plugin_source_parse_windows_path() {
        let source = PluginSource::parse("C:\\Users\\plugin").unwrap();
        assert!(matches!(source, PluginSource::Local(_)));
    }

    #[test]
    fn test_plugin_source_parse_registry_with_special_chars() {
        let source = PluginSource::parse("my-plugin-name_v2@2.0.0-beta").unwrap();
        if let PluginSource::Registry { name, version } = source {
            assert_eq!(name, "my-plugin-name_v2");
            assert_eq!(version, Some("2.0.0-beta".to_string()));
        } else {
            panic!("Expected Registry source");
        }
    }

    #[test]
    fn test_plugin_source_parse_github_dotgit_in_url() {
        let source = PluginSource::parse("https://github.com/user/repo.git").unwrap();
        assert!(matches!(source, PluginSource::Git(_)));
    }

    #[test]
    fn test_select_registry_version_prefers_current_non_yanked() {
        let entry = RegistryPluginResponse {
            version: "2.0.0".to_string(),
            versions: vec![
                RegistryVersionResponseWithVersion {
                    version: "1.0.0".to_string(),
                    download_url: "https://example.com/1.0.0.wasm".to_string(),
                    checksum: None,
                    yanked: false,
                },
                RegistryVersionResponseWithVersion {
                    version: "2.0.0".to_string(),
                    download_url: "https://example.com/2.0.0.wasm".to_string(),
                    checksum: Some("abc".to_string()),
                    yanked: false,
                },
            ],
        };

        let selected = select_registry_version(&entry).expect("expected selected version");
        assert_eq!(selected.version, "2.0.0");
        assert_eq!(selected.download_url, "https://example.com/2.0.0.wasm");
    }

    #[test]
    fn test_select_registry_version_falls_back_to_first_non_yanked() {
        let entry = RegistryPluginResponse {
            version: "2.0.0".to_string(),
            versions: vec![
                RegistryVersionResponseWithVersion {
                    version: "2.0.0".to_string(),
                    download_url: "https://example.com/2.0.0.wasm".to_string(),
                    checksum: None,
                    yanked: true,
                },
                RegistryVersionResponseWithVersion {
                    version: "1.9.0".to_string(),
                    download_url: "https://example.com/1.9.0.wasm".to_string(),
                    checksum: None,
                    yanked: false,
                },
            ],
        };

        let selected = select_registry_version(&entry).expect("expected selected version");
        assert_eq!(selected.version, "1.9.0");
    }
}
