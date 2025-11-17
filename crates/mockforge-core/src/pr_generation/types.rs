//! Types for PR generation

use serde::{Deserialize, Serialize};

/// PR provider type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PRProvider {
    /// GitHub
    GitHub,
    /// GitLab
    GitLab,
}

/// Configuration for PR generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PRGenerationConfig {
    /// Whether PR generation is enabled
    pub enabled: bool,
    /// PR provider
    pub provider: PRProvider,
    /// Repository owner/org
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Authentication token (GitHub PAT or GitLab token)
    /// Can be set via environment variable: GITHUB_TOKEN or GITLAB_TOKEN
    #[serde(skip_serializing)]
    pub token: Option<String>,
    /// Base branch (default: main)
    pub base_branch: String,
    /// Branch prefix for generated branches
    pub branch_prefix: String,
    /// Whether to auto-merge PRs (requires auto-merge enabled in repo)
    pub auto_merge: bool,
    /// List of reviewers (usernames)
    pub reviewers: Vec<String>,
    /// Labels to add to PRs
    pub labels: Vec<String>,
}

impl Default for PRGenerationConfig {
    fn default() -> Self {
        // Try to load token from environment variables
        let token = std::env::var("GITHUB_TOKEN")
            .ok()
            .or_else(|| std::env::var("GITLAB_TOKEN").ok());

        Self {
            enabled: false,
            provider: PRProvider::GitHub,
            owner: String::new(),
            repo: String::new(),
            token,
            base_branch: "main".to_string(),
            branch_prefix: "mockforge/contract-update".to_string(),
            auto_merge: false,
            reviewers: vec![],
            labels: vec!["automated".to_string(), "contract-update".to_string()],
        }
    }
}

impl PRGenerationConfig {
    /// Load configuration from environment variables
    ///
    /// This method loads configuration from environment variables, with the following priority:
    /// 1. Explicit config values (if set)
    /// 2. Environment variables
    /// 3. Default values
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Load provider from environment
        if let Ok(provider_str) = std::env::var("PR_PROVIDER") {
            config.provider = match provider_str.to_lowercase().as_str() {
                "gitlab" => PRProvider::GitLab,
                _ => PRProvider::GitHub,
            };
        }

        // Load repository info from environment
        if let Ok(owner) = std::env::var("PR_REPO_OWNER") {
            config.owner = owner;
        }
        if let Ok(repo) = std::env::var("PR_REPO_NAME") {
            config.repo = repo;
        }
        if let Ok(base_branch) = std::env::var("PR_BASE_BRANCH") {
            config.base_branch = base_branch;
        }

        // Load token from environment (provider-specific)
        if config.token.is_none() {
            config.token = match config.provider {
                PRProvider::GitHub => std::env::var("GITHUB_TOKEN").ok(),
                PRProvider::GitLab => std::env::var("GITLAB_TOKEN").ok(),
            };
        }

        // Enable if token and repo info are available
        if config.token.is_some() && !config.owner.is_empty() && !config.repo.is_empty() {
            config.enabled = true;
        }

        config
    }
}

/// File change for PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRFileChange {
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Change type
    pub change_type: PRFileChangeType,
}

/// Type of file change
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PRFileChangeType {
    /// Create new file
    Create,
    /// Update existing file
    Update,
    /// Delete file
    Delete,
}

/// PR creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRRequest {
    /// PR title
    pub title: String,
    /// PR body/description
    pub body: String,
    /// Branch name (will be created)
    pub branch: String,
    /// Files to change
    pub files: Vec<PRFileChange>,
    /// Labels to add
    pub labels: Vec<String>,
    /// Reviewers to request
    pub reviewers: Vec<String>,
}

/// PR creation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRResult {
    /// PR number
    pub number: u64,
    /// PR URL
    pub url: String,
    /// Branch name
    pub branch: String,
    /// PR title
    pub title: String,
}
