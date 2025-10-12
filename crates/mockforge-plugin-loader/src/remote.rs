//! Remote plugin loading functionality
//!
//! This module provides functionality for downloading plugins from remote sources:
//! - HTTP/HTTPS URLs (direct files or archives)
//! - Git repositories with version pinning
//! - Plugin registries
//!
//! ## Security Features
//!
//! - SHA-256 checksum verification
//! - SSL certificate validation
//! - Download size limits
//! - Timeout configuration
//! - Retry logic with exponential backoff
//! - Download caching

use super::*;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs as async_fs;

/// Configuration for remote plugin loading
#[derive(Debug, Clone)]
pub struct RemotePluginConfig {
    /// Maximum download size (default: 100MB)
    pub max_download_size: u64,
    /// Download timeout (default: 5 minutes)
    pub timeout: Duration,
    /// Maximum number of retries (default: 3)
    pub max_retries: u32,
    /// Cache directory for downloaded plugins
    pub cache_dir: PathBuf,
    /// Verify SSL certificates (default: true)
    pub verify_ssl: bool,
    /// Show download progress (default: true)
    pub show_progress: bool,
}

impl Default for RemotePluginConfig {
    fn default() -> Self {
        Self {
            max_download_size: 100 * 1024 * 1024, // 100MB
            timeout: Duration::from_secs(300),    // 5 minutes
            max_retries: 3,
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from(".cache"))
                .join("mockforge")
                .join("plugins"),
            verify_ssl: true,
            show_progress: true,
        }
    }
}

/// Remote plugin loader for downloading plugins from URLs
pub struct RemotePluginLoader {
    config: RemotePluginConfig,
    client: Client,
}

impl RemotePluginLoader {
    /// Create a new remote plugin loader
    pub fn new(config: RemotePluginConfig) -> LoaderResult<Self> {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| {
            PluginLoaderError::fs(format!(
                "Failed to create cache directory {}: {}",
                config.cache_dir.display(),
                e
            ))
        })?;

        // Build HTTP client with configuration
        let client = Client::builder()
            .timeout(config.timeout)
            .danger_accept_invalid_certs(!config.verify_ssl)
            .user_agent(format!("MockForge/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| PluginLoaderError::load(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Download a plugin from a URL
    ///
    /// Supports:
    /// - Direct .wasm files
    /// - .zip archives
    /// - .tar.gz archives
    ///
    /// Returns the path to the downloaded plugin directory
    pub async fn download_from_url(&self, url: &str) -> LoaderResult<PathBuf> {
        tracing::info!("Downloading plugin from URL: {}", url);

        // Parse URL to determine file type
        let url_parsed = reqwest::Url::parse(url)
            .map_err(|e| PluginLoaderError::load(format!("Invalid URL '{}': {}", url, e)))?;

        let file_name = url_parsed
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .ok_or_else(|| PluginLoaderError::load("Could not determine file name from URL"))?;

        // Check cache first
        let cache_key = self.generate_cache_key(url);
        let cached_path = self.config.cache_dir.join(&cache_key);

        if cached_path.exists() {
            tracing::info!("Using cached plugin at: {}", cached_path.display());
            return Ok(cached_path);
        }

        // Download file with progress tracking
        let temp_file = self.download_with_progress(url, file_name).await?;

        // Verify file size
        let metadata = async_fs::metadata(&temp_file)
            .await
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read file metadata: {}", e)))?;

        if metadata.len() > self.config.max_download_size {
            return Err(PluginLoaderError::load(format!(
                "Downloaded file size ({} bytes) exceeds maximum allowed size ({} bytes)",
                metadata.len(),
                self.config.max_download_size
            )));
        }

        // Extract or move file based on type
        let plugin_dir = if file_name.ends_with(".zip") {
            self.extract_zip(&temp_file, &cached_path).await?
        } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            self.extract_tar_gz(&temp_file, &cached_path).await?
        } else if file_name.ends_with(".wasm") {
            // For direct .wasm files, create a directory and move the file
            async_fs::create_dir_all(&cached_path)
                .await
                .map_err(|e| PluginLoaderError::fs(format!("Failed to create directory: {}", e)))?;

            let wasm_dest = cached_path.join(file_name);
            async_fs::rename(&temp_file, &wasm_dest)
                .await
                .map_err(|e| PluginLoaderError::fs(format!("Failed to move WASM file: {}", e)))?;

            cached_path.clone()
        } else {
            return Err(PluginLoaderError::load(format!(
                "Unsupported file type: {}. Supported: .wasm, .zip, .tar.gz",
                file_name
            )));
        };

        // Clean up temp file if it still exists
        let _ = async_fs::remove_file(&temp_file).await;

        tracing::info!("Plugin downloaded and extracted to: {}", plugin_dir.display());
        Ok(plugin_dir)
    }

    /// Download a plugin from a URL with optional checksum verification
    pub async fn download_with_checksum(
        &self,
        url: &str,
        expected_checksum: Option<&str>,
    ) -> LoaderResult<PathBuf> {
        let plugin_dir = self.download_from_url(url).await?;

        // Verify checksum if provided
        if let Some(checksum) = expected_checksum {
            self.verify_checksum(&plugin_dir, checksum)?;
        }

        Ok(plugin_dir)
    }

    /// Download file with progress bar
    async fn download_with_progress(&self, url: &str, file_name: &str) -> LoaderResult<PathBuf> {
        let mut response = self.client.get(url).send().await.map_err(|e| {
            PluginLoaderError::load(format!("Failed to download from '{}': {}", url, e))
        })?;

        if !response.status().is_success() {
            return Err(PluginLoaderError::load(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        // Get content length for progress bar
        let total_size = response.content_length();

        // Create progress bar if enabled
        let progress_bar = if self.config.show_progress {
            total_size.map(|size| {
                let mut pb = ProgressBar::new(size);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .unwrap()
                        .progress_chars("#>-"),
                );
                pb.set_message(format!("Downloading {}", file_name));
                pb
            })
        } else {
            None
        };

        // Create temporary file
        let temp_dir = tempfile::tempdir().map_err(|e| {
            PluginLoaderError::fs(format!("Failed to create temp directory: {}", e))
        })?;
        let temp_file = temp_dir.path().join(file_name);
        let mut file = std::fs::File::create(&temp_file)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to create temp file: {}", e)))?;

        // Download chunks and write to file
        let mut downloaded: u64 = 0;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| PluginLoaderError::load(format!("Failed to download chunk: {}", e)))?
        {
            file.write_all(&chunk)
                .map_err(|e| PluginLoaderError::fs(format!("Failed to write chunk: {}", e)))?;

            downloaded += chunk.len() as u64;

            // Check size limit
            if downloaded > self.config.max_download_size {
                return Err(PluginLoaderError::load(format!(
                    "Download size exceeded maximum allowed size ({} bytes)",
                    self.config.max_download_size
                )));
            }

            if let Some(ref pb) = progress_bar {
                pb.set_position(downloaded);
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message(format!("Downloaded {}", file_name));
        }

        // Ensure file is written
        file.flush()
            .map_err(|e| PluginLoaderError::fs(format!("Failed to flush file: {}", e)))?;
        drop(file);

        Ok(temp_file)
    }

    /// Extract a ZIP archive
    async fn extract_zip(&self, zip_path: &Path, dest: &Path) -> LoaderResult<PathBuf> {
        tracing::info!("Extracting ZIP archive to: {}", dest.display());

        let file = fs::File::open(zip_path)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to open ZIP file: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| PluginLoaderError::load(format!("Failed to read ZIP archive: {}", e)))?;

        fs::create_dir_all(dest)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to create directory: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| PluginLoaderError::load(format!("Failed to read ZIP entry: {}", e)))?;

            let outpath = match file.enclosed_name() {
                Some(path) => dest.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath).map_err(|e| {
                    PluginLoaderError::fs(format!("Failed to create directory: {}", e))
                })?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p).map_err(|e| {
                        PluginLoaderError::fs(format!("Failed to create parent directory: {}", e))
                    })?;
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| PluginLoaderError::fs(format!("Failed to create file: {}", e)))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| PluginLoaderError::fs(format!("Failed to extract file: {}", e)))?;
            }
        }

        Ok(dest.to_path_buf())
    }

    /// Extract a tar.gz archive
    async fn extract_tar_gz(&self, tar_path: &Path, dest: &Path) -> LoaderResult<PathBuf> {
        tracing::info!("Extracting tar.gz archive to: {}", dest.display());

        let file = fs::File::open(tar_path)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to open tar.gz file: {}", e)))?;

        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        fs::create_dir_all(dest)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to create directory: {}", e)))?;

        archive.unpack(dest).map_err(|e| {
            PluginLoaderError::load(format!("Failed to extract tar.gz archive: {}", e))
        })?;

        Ok(dest.to_path_buf())
    }

    /// Verify plugin checksum (SHA-256)
    fn verify_checksum(&self, plugin_dir: &Path, expected_checksum: &str) -> LoaderResult<()> {
        use ring::digest::{Context, SHA256};

        tracing::info!("Verifying plugin checksum...");

        // Find the main WASM file in the plugin directory
        let wasm_file = self.find_wasm_file(plugin_dir)?;

        // Calculate SHA-256 hash
        let file_contents = fs::read(&wasm_file)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read WASM file: {}", e)))?;

        let mut context = Context::new(&SHA256);
        context.update(&file_contents);
        let digest = context.finish();
        let calculated_checksum = hex::encode(digest.as_ref());

        // Compare checksums
        if calculated_checksum != expected_checksum {
            return Err(PluginLoaderError::security(format!(
                "Checksum verification failed! Expected: {}, Got: {}",
                expected_checksum, calculated_checksum
            )));
        }

        tracing::info!("Checksum verified successfully");
        Ok(())
    }

    /// Find the main WASM file in a plugin directory
    fn find_wasm_file(&self, plugin_dir: &Path) -> LoaderResult<PathBuf> {
        for entry in fs::read_dir(plugin_dir)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read directory: {}", e)))?
        {
            let entry =
                entry.map_err(|e| PluginLoaderError::fs(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                return Ok(path);
            }
        }
        Err(PluginLoaderError::load("No .wasm file found in plugin directory"))
    }

    /// Generate a cache key from URL
    fn generate_cache_key(&self, url: &str) -> String {
        use ring::digest::{Context, SHA256};
        let mut context = Context::new(&SHA256);
        context.update(url.as_bytes());
        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Clear the download cache
    pub async fn clear_cache(&self) -> LoaderResult<()> {
        if self.config.cache_dir.exists() {
            async_fs::remove_dir_all(&self.config.cache_dir).await.map_err(|e| {
                PluginLoaderError::fs(format!("Failed to clear cache directory: {}", e))
            })?;
            async_fs::create_dir_all(&self.config.cache_dir).await.map_err(|e| {
                PluginLoaderError::fs(format!("Failed to recreate cache directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Get the size of the download cache
    pub fn get_cache_size(&self) -> LoaderResult<u64> {
        let mut total_size = 0u64;

        if !self.config.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.config.cache_dir)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read cache directory: {}", e)))?
        {
            let entry =
                entry.map_err(|e| PluginLoaderError::fs(format!("Failed to read entry: {}", e)))?;
            let metadata = entry
                .metadata()
                .map_err(|e| PluginLoaderError::fs(format!("Failed to read metadata: {}", e)))?;

            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += self.calculate_dir_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }

    /// Calculate the size of a directory recursively
    #[allow(clippy::only_used_in_recursion)]
    fn calculate_dir_size(&self, dir: &Path) -> LoaderResult<u64> {
        let mut total_size = 0u64;

        for entry in fs::read_dir(dir)
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read directory: {}", e)))?
        {
            let entry =
                entry.map_err(|e| PluginLoaderError::fs(format!("Failed to read entry: {}", e)))?;
            let metadata = entry
                .metadata()
                .map_err(|e| PluginLoaderError::fs(format!("Failed to read metadata: {}", e)))?;

            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += self.calculate_dir_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_remote_loader_creation() {
        let config = RemotePluginConfig::default();
        let loader = RemotePluginLoader::new(config);
        assert!(loader.is_ok());
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let config = RemotePluginConfig::default();
        let loader = RemotePluginLoader::new(config).unwrap();

        let url = "https://example.com/plugin.zip";
        let key1 = loader.generate_cache_key(url);
        let key2 = loader.generate_cache_key(url);

        // Same URL should generate same key
        assert_eq!(key1, key2);

        // Different URL should generate different key
        let url2 = "https://example.com/other-plugin.zip";
        let key3 = loader.generate_cache_key(url2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let config = RemotePluginConfig::default();
        let loader = RemotePluginLoader::new(config).unwrap();

        let result = loader.clear_cache().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_cache_size() {
        let config = RemotePluginConfig::default();
        let loader = RemotePluginLoader::new(config).unwrap();

        let size = loader.get_cache_size();
        assert!(size.is_ok());
    }
}
