//! Git Watch Mode
//!
//! Monitors a Git repository for OpenAPI spec changes and auto-syncs mocks.
//! This enables contract-driven mocking where mocks stay in sync with API specifications.

use crate::Error;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Git watch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWatchConfig {
    /// Repository URL (HTTPS or SSH)
    pub repository_url: String,
    /// Branch to watch (default: "main")
    #[serde(default = "default_branch")]
    pub branch: String,
    /// Path to OpenAPI spec file(s) in the repository
    /// Supports glob patterns (e.g., "**/*.yaml", "specs/*.json")
    pub spec_paths: Vec<String>,
    /// Polling interval in seconds (default: 60)
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
    /// Authentication token for private repositories (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    /// Local cache directory for cloned repository
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    /// Whether to enable watch mode (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_poll_interval() -> u64 {
    60
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from("./.mockforge-git-cache")
}

fn default_true() -> bool {
    true
}

/// Git watch service that monitors a repository for changes
pub struct GitWatchService {
    config: GitWatchConfig,
    last_commit: Option<String>,
    repo_path: PathBuf,
}

impl GitWatchService {
    /// Create a new Git watch service
    pub fn new(config: GitWatchConfig) -> Result<Self> {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&config.cache_dir).map_err(|e| {
            Error::generic(format!(
                "Failed to create cache directory {}: {}",
                config.cache_dir.display(),
                e
            ))
        })?;

        // Generate repository path from URL
        let repo_name = Self::extract_repo_name(&config.repository_url)?;
        let repo_path = config.cache_dir.join(repo_name);

        Ok(Self {
            config,
            last_commit: None,
            repo_path,
        })
    }

    /// Extract repository name from URL
    fn extract_repo_name(url: &str) -> Result<String> {
        // Handle various URL formats:
        // - https://github.com/user/repo.git
        // - git@github.com:user/repo.git
        // - https://github.com/user/repo
        let name = if url.ends_with(".git") {
            &url[..url.len() - 4]
        } else {
            url
        };

        // Extract the last component
        let parts: Vec<&str> = name.split('/').collect();
        if let Some(last) = parts.last() {
            // Remove any query parameters or fragments
            let clean = last.split('?').next().unwrap_or(last);
            Ok(clean.to_string())
        } else {
            Err(Error::generic(format!("Invalid repository URL: {}", url)))
        }
    }

    /// Initialize the repository (clone if needed, update if exists)
    pub async fn initialize(&mut self) -> Result<()> {
        info!(
            "Initializing Git watch for repository: {} (branch: {})",
            self.config.repository_url, self.config.branch
        );

        if self.repo_path.exists() {
            debug!("Repository exists, updating...");
            self.update_repository().await?;
        } else {
            debug!("Repository does not exist, cloning...");
            self.clone_repository().await?;
        }

        // Get initial commit hash
        self.last_commit = Some(self.get_current_commit()?);

        info!("Git watch initialized successfully");
        Ok(())
    }

    /// Clone the repository
    async fn clone_repository(&self) -> Result<()> {
        use std::process::Command;

        let url = if let Some(ref token) = self.config.auth_token {
            self.inject_auth_token(&self.config.repository_url, token)?
        } else {
            self.config.repository_url.clone()
        };

        let output = Command::new("git")
            .args([
                "clone",
                "--branch",
                &self.config.branch,
                "--depth",
                "1", // Shallow clone for performance
                &url,
                self.repo_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| Error::generic(format!("Failed to execute git clone: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::generic(format!("Git clone failed: {}", stderr)));
        }

        info!("Repository cloned successfully");
        Ok(())
    }

    /// Update the repository (fetch and checkout)
    async fn update_repository(&self) -> Result<()> {
        use std::process::Command;

        let repo_path_str = self.repo_path.to_str().unwrap();

        // Fetch latest changes
        let output = Command::new("git")
            .args(["-C", repo_path_str, "fetch", "origin", &self.config.branch])
            .output()
            .map_err(|e| Error::generic(format!("Failed to execute git fetch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git fetch failed: {}", stderr);
            // Continue anyway, might be network issue
        }

        // Reset to remote branch
        let output = Command::new("git")
            .args([
                "-C",
                repo_path_str,
                "reset",
                "--hard",
                &format!("origin/{}", self.config.branch),
            ])
            .output()
            .map_err(|e| Error::generic(format!("Failed to execute git reset: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::generic(format!("Git reset failed: {}", stderr)));
        }

        debug!("Repository updated successfully");
        Ok(())
    }

    /// Get current commit hash
    fn get_current_commit(&self) -> Result<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args([
                "-C",
                self.repo_path.to_str().unwrap(),
                "rev-parse",
                "HEAD",
            ])
            .output()
            .map_err(|e| Error::generic(format!("Failed to execute git rev-parse: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::generic(format!("Git rev-parse failed: {}", stderr)));
        }

        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(commit)
    }

    /// Inject authentication token into repository URL
    fn inject_auth_token(&self, url: &str, token: &str) -> Result<String> {
        // Handle HTTPS URLs
        if url.starts_with("https://") {
            // Insert token before the hostname
            // https://github.com/user/repo -> https://token@github.com/user/repo
            if let Some(rest) = url.strip_prefix("https://") {
                return Ok(format!("https://{}@{}", token, rest));
            }
        }
        // For SSH URLs, token injection is more complex and typically uses SSH keys
        // For now, return the original URL and log a warning
        if url.contains('@') {
            warn!("SSH URL detected. Token authentication may not work. Consider using HTTPS or SSH keys.");
        }
        Ok(url.to_string())
    }

    /// Check for changes in the repository
    pub async fn check_for_changes(&mut self) -> Result<bool> {
        // Update repository
        self.update_repository().await?;

        // Get current commit
        let current_commit = self.get_current_commit()?;

        // Compare with last known commit
        if let Some(ref last) = self.last_commit {
            if last == &current_commit {
                debug!("No changes detected (commit: {})", &current_commit[..8]);
                return Ok(false);
            }
        }

        info!(
            "Changes detected! Previous: {}, Current: {}",
            self.last_commit
                .as_ref()
                .map(|c| &c[..8])
                .unwrap_or("none"),
            &current_commit[..8]
        );

        // Update last commit
        self.last_commit = Some(current_commit);

        Ok(true)
    }

    /// Get paths to OpenAPI spec files
    pub fn get_spec_files(&self) -> Result<Vec<PathBuf>> {
        use globwalk::GlobWalkerBuilder;

        let mut spec_files = Vec::new();

        for pattern in &self.config.spec_paths {
            let walker = GlobWalkerBuilder::from_patterns(&self.repo_path, &[pattern])
                .build()
                .map_err(|e| {
                    Error::generic(format!("Failed to build glob walker for {}: {}", pattern, e))
                })?;

            for entry in walker {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            spec_files.push(path.to_path_buf());
                        }
                    }
                    Err(e) => {
                        warn!("Error walking path: {}", e);
                    }
                }
            }
        }

        // Remove duplicates and sort
        spec_files.sort();
        spec_files.dedup();

        info!("Found {} OpenAPI spec file(s)", spec_files.len());
        Ok(spec_files)
    }

    /// Start watching the repository
    pub async fn watch<F>(&mut self, mut on_change: F) -> Result<()>
    where
        F: FnMut(Vec<PathBuf>) -> Result<()>,
    {
        info!(
            "Starting Git watch mode (polling every {} seconds)",
            self.config.poll_interval_seconds
        );

        let mut interval = interval(Duration::from_secs(self.config.poll_interval_seconds));

        loop {
            interval.tick().await;

            match self.check_for_changes().await {
                Ok(true) => {
                    // Changes detected, get spec files and notify
                    match self.get_spec_files() {
                        Ok(spec_files) => {
                            if let Err(e) = on_change(spec_files) {
                                error!("Error handling spec changes: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to get spec files: {}", e);
                        }
                    }
                }
                Ok(false) => {
                    // No changes, continue
                }
                Err(e) => {
                    error!("Error checking for changes: {}", e);
                    // Continue watching despite errors
                }
            }
        }
    }

    /// Get the repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name() {
        let test_cases = vec![
            ("https://github.com/user/repo.git", "repo"),
            ("https://github.com/user/repo", "repo"),
            ("git@github.com:user/repo.git", "repo"),
            ("https://gitlab.com/group/project.git", "project"),
        ];

        for (url, expected) in test_cases {
            let result = GitWatchService::extract_repo_name(url);
            assert!(result.is_ok(), "Failed to extract repo name from: {}", url);
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_inject_auth_token() {
        let config = GitWatchConfig {
            repository_url: "https://github.com/user/repo.git".to_string(),
            branch: "main".to_string(),
            spec_paths: vec!["*.yaml".to_string()],
            poll_interval_seconds: 60,
            auth_token: None,
            cache_dir: PathBuf::from("./test-cache"),
            enabled: true,
        };

        let service = GitWatchService::new(config).unwrap();
        let url = "https://github.com/user/repo.git";
        let token = "ghp_token123";

        let result = service.inject_auth_token(url, token).unwrap();
        assert_eq!(result, "https://ghp_token123@github.com/user/repo.git");
    }
}
