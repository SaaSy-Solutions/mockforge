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
use crate::remote::{RemotePluginConfig, RemotePluginLoader};
use std::path::{Path, PathBuf};

/// Plugin source specification
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Local file path or directory
    Local(PathBuf),
    /// HTTP/HTTPS URL
    Url {
        url: String,
        checksum: Option<String>,
    },
    /// Git repository
    Git(GitPluginSource),
    /// Plugin registry (future)
    Registry {
        name: String,
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
            if input.contains(".git") || input.contains("github.com") || input.contains("gitlab.com") {
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
}

impl PluginInstaller {
    /// Create a new plugin installer with default configuration
    pub fn new(loader_config: PluginLoaderConfig) -> LoaderResult<Self> {
        let loader = PluginLoader::new(loader_config);
        let remote_loader = RemotePluginLoader::new(RemotePluginConfig::default())?;
        let git_loader = GitPluginLoader::new(GitPluginConfig::default())?;

        Ok(Self {
            loader,
            remote_loader,
            git_loader,
        })
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
            PluginSource::Git(git_source) => {
                self.git_loader.clone_from_git(git_source).await?
            }
            PluginSource::Registry { name, version } => {
                return Err(PluginLoaderError::load(format!(
                    "Registry plugin installation not yet implemented: {}@{}",
                    name,
                    version.as_deref().unwrap_or("latest")
                )));
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

        tracing::info!("Plugin installed successfully: {}", plugin_id);
        Ok(plugin_id)
    }

    /// Verify plugin signature
    fn verify_plugin_signature(&self, plugin_dir: &Path) -> LoaderResult<()> {
        // Look for signature file
        let sig_file = plugin_dir.join("plugin.sig");
        if !sig_file.exists() {
            return Err(PluginLoaderError::security(
                "No signature file found (plugin.sig)",
            ));
        }

        // TODO: Implement GPG/RSA signature verification
        // For now, just check that the file exists
        tracing::debug!("Signature file found at: {}", sig_file.display());

        Ok(())
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        self.loader.unload_plugin(plugin_id).await
    }

    /// List installed plugins
    pub async fn list_installed(&self) -> Vec<PluginId> {
        self.loader.list_plugins().await
    }

    /// Update a plugin to the latest version
    pub async fn update(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        // TODO: Implement plugin update logic
        // Need to track original source and fetch latest version
        Err(PluginLoaderError::load(
            "Plugin update not yet implemented",
        ))
    }

    /// Update all plugins to their latest versions
    pub async fn update_all(&self) -> LoaderResult<Vec<PluginId>> {
        // TODO: Implement bulk plugin update
        Err(PluginLoaderError::load(
            "Bulk plugin update not yet implemented",
        ))
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

        // TODO: Add Git cache size calculation
        let git_cache_size = 0;

        Ok(CacheStats {
            download_cache_size,
            git_cache_size,
            total_size: download_cache_size + git_cache_size,
        })
    }
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
    fn test_cache_stats_formatting() {
        assert_eq!(CacheStats::format_size(512), "512 bytes");
        assert_eq!(CacheStats::format_size(1024), "1.00 KB");
        assert_eq!(CacheStats::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(CacheStats::format_size(1024 * 1024 * 1024), "1.00 GB");
    }
}
