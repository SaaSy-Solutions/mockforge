//! Data Source Abstraction
//!
//! Provides a unified interface for loading test data from various sources:
//! - Local filesystem
//! - Git repositories
//! - HTTP endpoints
//!
//! This enables injecting test data into mocks from multiple sources.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Data source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataSourceType {
    /// Local filesystem
    Local,
    /// Git repository
    Git,
    /// HTTP/HTTPS endpoint
    Http,
}

/// Configuration for a data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    /// Type of data source
    #[serde(rename = "type")]
    pub source_type: DataSourceType,
    /// Source location (path, URL, or Git repo URL)
    pub location: String,
    /// Optional branch/tag for Git sources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Optional authentication token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    /// Optional path within the source (for Git repos or subdirectories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional cache directory for Git sources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    /// Optional refresh interval in seconds (for HTTP sources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<u64>,
}

/// Loaded data from a source
#[derive(Debug, Clone)]
pub struct DataSourceContent {
    /// The content as bytes
    pub content: Vec<u8>,
    /// Content type (if known)
    pub content_type: Option<String>,
    /// Metadata about the source
    pub metadata: HashMap<String, String>,
}

/// Trait for data source implementations
#[async_trait::async_trait]
pub trait DataSource: Send + Sync {
    /// Load data from the source
    async fn load(&self) -> Result<DataSourceContent>;

    /// Check if the source has been updated (for caching)
    async fn check_updated(&self) -> Result<bool>;

    /// Get the source type
    fn source_type(&self) -> DataSourceType;
}

/// Local filesystem data source
pub struct LocalDataSource {
    path: PathBuf,
}

impl LocalDataSource {
    /// Create a new local data source
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl DataSource for LocalDataSource {
    async fn load(&self) -> Result<DataSourceContent> {
        debug!("Loading data from local file: {}", self.path.display());

        let content = tokio::fs::read(&self.path).await.map_err(|e| {
            Error::generic(format!("Failed to read local file {}: {}", self.path.display(), e))
        })?;

        let content_type =
            self.path.extension().and_then(|ext| ext.to_str()).map(|ext| match ext {
                "json" => "application/json".to_string(),
                "yaml" | "yml" => "application/x-yaml".to_string(),
                "xml" => "application/xml".to_string(),
                "csv" => "text/csv".to_string(),
                _ => format!("text/{}", ext),
            });

        let mut metadata = HashMap::new();
        if let Ok(metadata_info) = tokio::fs::metadata(&self.path).await {
            metadata.insert("size".to_string(), metadata_info.len().to_string());
            if let Ok(modified) = metadata_info.modified() {
                metadata.insert("modified".to_string(), format!("{:?}", modified));
            }
        }
        metadata.insert("path".to_string(), self.path.display().to_string());

        Ok(DataSourceContent {
            content,
            content_type,
            metadata,
        })
    }

    async fn check_updated(&self) -> Result<bool> {
        // For local files, we can check modification time
        // This is a simple implementation - always returns true
        // A more sophisticated version could track modification times
        Ok(true)
    }

    fn source_type(&self) -> DataSourceType {
        DataSourceType::Local
    }
}

/// Git repository data source
pub struct GitDataSource {
    config: DataSourceConfig,
    repo_path: PathBuf,
}

impl GitDataSource {
    /// Create a new Git data source
    pub fn new(config: DataSourceConfig) -> Result<Self> {
        // Extract repo name from URL
        let repo_name = Self::extract_repo_name(&config.location)?;
        let cache_dir = config
            .cache_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("./.mockforge-data-cache"));
        let repo_path = cache_dir.join(repo_name);

        Ok(Self { config, repo_path })
    }

    /// Extract repository name from URL
    fn extract_repo_name(url: &str) -> Result<String> {
        let name = if let Some(stripped) = url.strip_suffix(".git") {
            stripped
        } else {
            url
        };

        let parts: Vec<&str> = name.split('/').collect();
        if let Some(last) = parts.last() {
            let clean = last.split('?').next().unwrap_or(last);
            Ok(clean.to_string())
        } else {
            Err(Error::generic(format!("Invalid Git repository URL: {}", url)))
        }
    }

    /// Initialize or update the repository
    async fn ensure_repo(&self) -> Result<()> {
        use std::process::Command;

        // Create cache directory if needed
        if let Some(parent) = self.repo_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::generic(format!("Failed to create cache directory: {}", e)))?;
        }

        if self.repo_path.exists() {
            // Update existing repository
            debug!("Updating Git repository: {}", self.repo_path.display());
            let branch = self.config.branch.as_deref().unwrap_or("main");
            let repo_path_str = self.repo_path.to_str().unwrap();

            // Fetch latest changes
            let output = Command::new("git")
                .args(["-C", repo_path_str, "fetch", "origin", branch])
                .output()
                .map_err(|e| Error::generic(format!("Failed to fetch: {}", e)))?;

            if !output.status.success() {
                warn!("Git fetch failed, continuing anyway");
            }

            // Reset to remote branch
            let output = Command::new("git")
                .args([
                    "-C",
                    repo_path_str,
                    "reset",
                    "--hard",
                    &format!("origin/{}", branch),
                ])
                .output()
                .map_err(|e| Error::generic(format!("Failed to reset: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::generic(format!("Git reset failed: {}", stderr)));
            }
        } else {
            // Clone repository
            debug!("Cloning Git repository: {}", self.config.location);
            let url = if let Some(ref token) = self.config.auth_token {
                Self::inject_auth_token(&self.config.location, token)?
            } else {
                self.config.location.clone()
            };

            let branch = self.config.branch.as_deref().unwrap_or("main");
            let repo_path_str = self.repo_path.to_str().unwrap();

            let output = Command::new("git")
                .args([
                    "clone",
                    "--branch",
                    branch,
                    "--depth",
                    "1",
                    &url,
                    repo_path_str,
                ])
                .output()
                .map_err(|e| Error::generic(format!("Failed to clone: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::generic(format!("Git clone failed: {}", stderr)));
            }
        }

        Ok(())
    }

    /// Inject authentication token into URL
    fn inject_auth_token(url: &str, token: &str) -> Result<String> {
        if url.starts_with("https://") {
            if let Some(rest) = url.strip_prefix("https://") {
                return Ok(format!("https://{}@{}", token, rest));
            }
        }
        if url.contains('@') {
            warn!("SSH URL detected. Token authentication may not work.");
        }
        Ok(url.to_string())
    }
}

#[async_trait::async_trait]
impl DataSource for GitDataSource {
    async fn load(&self) -> Result<DataSourceContent> {
        // Ensure repository is cloned/updated
        self.ensure_repo().await?;

        // Determine file path
        let file_path = if let Some(ref path) = self.config.path {
            self.repo_path.join(path)
        } else {
            return Err(Error::generic(
                "Git data source requires a 'path' to specify the file within the repository"
                    .to_string(),
            ));
        };

        if !file_path.exists() {
            return Err(Error::generic(format!(
                "File not found in Git repository: {}",
                file_path.display()
            )));
        }

        // Load file content
        let content = tokio::fs::read(&file_path).await.map_err(|e| {
            Error::generic(format!("Failed to read file from Git repository: {}", e))
        })?;

        let content_type =
            file_path.extension().and_then(|ext| ext.to_str()).map(|ext| match ext {
                "json" => "application/json".to_string(),
                "yaml" | "yml" => "application/x-yaml".to_string(),
                _ => format!("text/{}", ext),
            });

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "git".to_string());
        metadata.insert("repository".to_string(), self.config.location.clone());
        if let Some(ref branch) = self.config.branch {
            metadata.insert("branch".to_string(), branch.clone());
        }
        metadata.insert("path".to_string(), file_path.display().to_string());

        // Get commit hash
        use std::process::Command;
        if let Ok(output) = Command::new("git")
            .args(["-C", self.repo_path.to_str().unwrap(), "rev-parse", "HEAD"])
            .output()
        {
            if output.status.success() {
                if let Ok(commit) = String::from_utf8(output.stdout) {
                    metadata.insert("commit".to_string(), commit.trim().to_string());
                }
            }
        }

        Ok(DataSourceContent {
            content,
            content_type,
            metadata,
        })
    }

    async fn check_updated(&self) -> Result<bool> {
        // Check if remote has new commits
        use std::process::Command;

        let branch = self.config.branch.as_deref().unwrap_or("main");
        let repo_path_str = self.repo_path.to_str().unwrap();

        // Fetch without updating
        let _output = Command::new("git")
            .args(["-C", repo_path_str, "fetch", "origin", branch])
            .output();

        // Compare local and remote
        let output = Command::new("git")
            .args([
                "-C",
                repo_path_str,
                "rev-list",
                "--count",
                &format!("HEAD..origin/{}", branch),
            ])
            .output()
            .map_err(|e| Error::generic(format!("Failed to check for updates: {}", e)))?;

        if output.status.success() {
            if let Ok(count_str) = String::from_utf8(output.stdout) {
                if let Ok(count) = count_str.trim().parse::<u32>() {
                    return Ok(count > 0);
                }
            }
        }

        Ok(false)
    }

    fn source_type(&self) -> DataSourceType {
        DataSourceType::Git
    }
}

/// HTTP/HTTPS data source
pub struct HttpDataSource {
    url: String,
    auth_token: Option<String>,
    refresh_interval: Option<u64>,
    last_fetch: std::sync::Arc<std::sync::Mutex<Option<std::time::Instant>>>,
    cached_content: std::sync::Arc<std::sync::Mutex<Option<DataSourceContent>>>,
}

impl HttpDataSource {
    /// Create a new HTTP data source
    pub fn new(config: DataSourceConfig) -> Self {
        Self {
            url: config.location.clone(),
            auth_token: config.auth_token.clone(),
            refresh_interval: config.refresh_interval,
            last_fetch: std::sync::Arc::new(std::sync::Mutex::new(None)),
            cached_content: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Fetch data from HTTP endpoint
    async fn fetch(&self) -> Result<DataSourceContent> {
        let client = reqwest::Client::new();

        // Add authentication if provided
        let mut request = client.get(&self.url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await.map_err(|e| {
            Error::generic(format!("Failed to fetch data from {}: {}", self.url, e))
        })?;

        // Extract status and content type before consuming the response
        let status = response.status();
        let status_code = status.as_u16();

        if !status.is_success() {
            return Err(Error::generic(format!("HTTP request failed with status {}", status)));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let content = response
            .bytes()
            .await
            .map_err(|e| Error::generic(format!("Failed to read response body: {}", e)))?
            .to_vec();

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "http".to_string());
        metadata.insert("url".to_string(), self.url.clone());
        metadata.insert("status".to_string(), status_code.to_string());
        if let Some(content_type) = &content_type {
            metadata.insert("content_type".to_string(), content_type.clone());
        }

        Ok(DataSourceContent {
            content,
            content_type,
            metadata,
        })
    }
}

#[async_trait::async_trait]
impl DataSource for HttpDataSource {
    async fn load(&self) -> Result<DataSourceContent> {
        // Check if we should use cached content
        {
            let cached_guard =
                self.cached_content.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let last_fetch_guard =
                self.last_fetch.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

            if let (Some(cached), Some(last_fetch), Some(refresh_interval)) =
                (cached_guard.as_ref(), last_fetch_guard.as_ref(), self.refresh_interval)
            {
                if last_fetch.elapsed().as_secs() < refresh_interval {
                    debug!("Using cached HTTP data");
                    return Ok(cached.clone());
                }
            }
        }

        // Fetch fresh data
        let content = self.fetch().await?;
        {
            let mut last_fetch =
                self.last_fetch.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut cached =
                self.cached_content.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *last_fetch = Some(std::time::Instant::now());
            *cached = Some(content.clone());
        }

        Ok(content)
    }

    async fn check_updated(&self) -> Result<bool> {
        // For HTTP sources, we check if cache is expired
        let last_fetch = self.last_fetch.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let (Some(last_fetch), Some(refresh_interval)) =
            (last_fetch.as_ref(), self.refresh_interval)
        {
            Ok(last_fetch.elapsed().as_secs() >= refresh_interval)
        } else {
            // No cache, always consider updated
            Ok(true)
        }
    }

    fn source_type(&self) -> DataSourceType {
        DataSourceType::Http
    }
}

/// Data source factory
pub struct DataSourceFactory;

impl DataSourceFactory {
    /// Create a data source from configuration
    pub fn create(config: DataSourceConfig) -> Result<Box<dyn DataSource + Send + Sync>> {
        match config.source_type {
            DataSourceType::Local => Ok(Box::new(LocalDataSource::new(&config.location))),
            DataSourceType::Git => {
                let git_source = GitDataSource::new(config)?;
                Ok(Box::new(git_source))
            }
            DataSourceType::Http => Ok(Box::new(HttpDataSource::new(config))),
        }
    }
}

/// Data source manager for handling multiple sources
pub struct DataSourceManager {
    sources: HashMap<String, std::sync::Arc<dyn DataSource + Send + Sync>>,
}

impl DataSourceManager {
    /// Create a new data source manager
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Register a data source
    pub fn register(&mut self, name: String, source: Box<dyn DataSource + Send + Sync>) {
        self.sources.insert(name, std::sync::Arc::from(source));
    }

    /// Load data from a named source
    pub async fn load(&self, name: &str) -> Result<DataSourceContent> {
        let source = self
            .sources
            .get(name)
            .ok_or_else(|| Error::generic(format!("Data source '{}' not found", name)))?;

        source.load().await
    }

    /// Check if a source has been updated
    pub async fn check_updated(&self, name: &str) -> Result<bool> {
        let source = self
            .sources
            .get(name)
            .ok_or_else(|| Error::generic(format!("Data source '{}' not found", name)))?;

        source.check_updated().await
    }

    /// List all registered sources
    pub fn list_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
}

impl Default for DataSourceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_data_source_creation() {
        let source = LocalDataSource::new("./test.json");
        assert_eq!(source.source_type(), DataSourceType::Local);
    }

    #[test]
    fn test_git_data_source_config() {
        let config = DataSourceConfig {
            source_type: DataSourceType::Git,
            location: "https://github.com/user/repo.git".to_string(),
            branch: Some("main".to_string()),
            auth_token: None,
            path: Some("data/test.json".to_string()),
            cache_dir: None,
            refresh_interval: None,
        };

        let source = GitDataSource::new(config).unwrap();
        assert_eq!(source.source_type(), DataSourceType::Git);
    }

    #[test]
    fn test_http_data_source_config() {
        let config = DataSourceConfig {
            source_type: DataSourceType::Http,
            location: "https://api.example.com/data.json".to_string(),
            branch: None,
            auth_token: Some("token123".to_string()),
            path: None,
            cache_dir: None,
            refresh_interval: Some(60),
        };

        let source = HttpDataSource::new(config);
        assert_eq!(source.source_type(), DataSourceType::Http);
    }

    #[test]
    fn test_data_source_factory() {
        let local_config = DataSourceConfig {
            source_type: DataSourceType::Local,
            location: "./test.json".to_string(),
            branch: None,
            auth_token: None,
            path: None,
            cache_dir: None,
            refresh_interval: None,
        };

        let source = DataSourceFactory::create(local_config).unwrap();
        assert_eq!(source.source_type(), DataSourceType::Local);
    }
}
