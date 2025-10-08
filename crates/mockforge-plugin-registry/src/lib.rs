//! MockForge Plugin Registry
//!
//! Central registry for discovering, publishing, and installing plugins.

pub mod api;
pub mod config;
pub mod dependencies;
pub mod hot_reload;
pub mod index;
pub mod manifest;
pub mod reviews;
pub mod runtime;
pub mod security;
pub mod storage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Registry errors
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Plugin already exists: {0}")]
    PluginExists(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RegistryError>;

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Plugin name
    pub name: String,

    /// Plugin description
    pub description: String,

    /// Current version
    pub version: String,

    /// All available versions
    pub versions: Vec<VersionEntry>,

    /// Author information
    pub author: AuthorInfo,

    /// Plugin tags
    pub tags: Vec<String>,

    /// Plugin category
    pub category: PluginCategory,

    /// Download count
    pub downloads: u64,

    /// Rating (0.0 - 5.0)
    pub rating: f64,

    /// Total reviews
    pub reviews_count: u32,

    /// Repository URL
    pub repository: Option<String>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// License
    pub license: String,

    /// Created timestamp
    pub created_at: String,

    /// Updated timestamp
    pub updated_at: String,
}

/// Version-specific entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Version string (semver)
    pub version: String,

    /// Download URL
    pub download_url: String,

    /// SHA-256 checksum
    pub checksum: String,

    /// File size in bytes
    pub size: u64,

    /// Published timestamp
    pub published_at: String,

    /// Yanked (removed from index)
    pub yanked: bool,

    /// Minimum MockForge version required
    pub min_mockforge_version: Option<String>,

    /// Dependencies
    pub dependencies: HashMap<String, String>,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

/// Plugin category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginCategory {
    Auth,
    Template,
    Response,
    DataSource,
    Middleware,
    Testing,
    Observability,
    Other,
}

/// Search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub query: Option<String>,

    /// Filter by category
    pub category: Option<PluginCategory>,

    /// Filter by tags
    pub tags: Vec<String>,

    /// Sort order
    pub sort: SortOrder,

    /// Page number (0-indexed)
    pub page: usize,

    /// Results per page
    pub per_page: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            tags: vec![],
            sort: SortOrder::Relevance,
            page: 0,
            per_page: 20,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Relevance,
    Downloads,
    Rating,
    Recent,
    Name,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub plugins: Vec<RegistryEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry URL
    pub url: String,

    /// API token (optional)
    pub token: Option<String>,

    /// Cache directory
    pub cache_dir: Option<String>,

    /// Timeout in seconds
    pub timeout: u64,

    /// Alternative registries
    pub alternative_registries: Vec<String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "https://registry.mockforge.dev".to_string(),
            token: None,
            cache_dir: None,
            timeout: 30,
            alternative_registries: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_entry_serialization() {
        let entry = RegistryEntry {
            name: "test-plugin".to_string(),
            description: "Test plugin".to_string(),
            version: "1.0.0".to_string(),
            versions: vec![],
            author: AuthorInfo {
                name: "Test Author".to_string(),
                email: Some("test@example.com".to_string()),
                url: None,
            },
            tags: vec!["test".to_string()],
            category: PluginCategory::Auth,
            downloads: 100,
            rating: 4.5,
            reviews_count: 10,
            repository: None,
            homepage: None,
            license: "MIT".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.name, deserialized.name);
    }

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.page, 0);
        assert_eq!(query.per_page, 20);
    }
}
