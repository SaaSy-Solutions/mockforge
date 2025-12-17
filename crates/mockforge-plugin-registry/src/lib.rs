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

    fn create_test_author() -> AuthorInfo {
        AuthorInfo {
            name: "Test Author".to_string(),
            email: Some("test@example.com".to_string()),
            url: Some("https://example.com".to_string()),
        }
    }

    fn create_test_version_entry() -> VersionEntry {
        VersionEntry {
            version: "1.0.0".to_string(),
            download_url: "https://example.com/plugin-1.0.0.tar.gz".to_string(),
            checksum: "abc123def456".to_string(),
            size: 12345,
            published_at: "2025-01-01T00:00:00Z".to_string(),
            yanked: false,
            min_mockforge_version: Some("0.3.0".to_string()),
            dependencies: HashMap::new(),
        }
    }

    fn create_test_registry_entry() -> RegistryEntry {
        RegistryEntry {
            name: "test-plugin".to_string(),
            description: "Test plugin".to_string(),
            version: "1.0.0".to_string(),
            versions: vec![create_test_version_entry()],
            author: create_test_author(),
            tags: vec!["test".to_string()],
            category: PluginCategory::Auth,
            downloads: 100,
            rating: 4.5,
            reviews_count: 10,
            repository: Some("https://github.com/test/plugin".to_string()),
            homepage: Some("https://plugin.example.com".to_string()),
            license: "MIT".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }

    // RegistryError tests
    #[test]
    fn test_registry_error_plugin_not_found() {
        let error = RegistryError::PluginNotFound("my-plugin".to_string());
        let display = error.to_string();
        assert!(display.contains("Plugin not found"));
        assert!(display.contains("my-plugin"));
    }

    #[test]
    fn test_registry_error_invalid_version() {
        let error = RegistryError::InvalidVersion("bad version".to_string());
        let display = error.to_string();
        assert!(display.contains("Invalid version"));
    }

    #[test]
    fn test_registry_error_plugin_exists() {
        let error = RegistryError::PluginExists("existing-plugin".to_string());
        let display = error.to_string();
        assert!(display.contains("Plugin already exists"));
    }

    #[test]
    fn test_registry_error_auth_required() {
        let error = RegistryError::AuthRequired;
        let display = error.to_string();
        assert!(display.contains("Authentication required"));
    }

    #[test]
    fn test_registry_error_permission_denied() {
        let error = RegistryError::PermissionDenied;
        let display = error.to_string();
        assert!(display.contains("Permission denied"));
    }

    #[test]
    fn test_registry_error_invalid_manifest() {
        let error = RegistryError::InvalidManifest("missing field".to_string());
        let display = error.to_string();
        assert!(display.contains("Invalid manifest"));
    }

    #[test]
    fn test_registry_error_storage() {
        let error = RegistryError::Storage("disk full".to_string());
        let display = error.to_string();
        assert!(display.contains("Storage error"));
    }

    #[test]
    fn test_registry_error_network() {
        let error = RegistryError::Network("connection refused".to_string());
        let display = error.to_string();
        assert!(display.contains("Network error"));
    }

    #[test]
    fn test_registry_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: RegistryError = io_error.into();
        assert!(matches!(error, RegistryError::Io(_)));
    }

    #[test]
    fn test_registry_error_debug() {
        let error = RegistryError::AuthRequired;
        let debug = format!("{:?}", error);
        assert!(debug.contains("AuthRequired"));
    }

    // AuthorInfo tests
    #[test]
    fn test_author_info_clone() {
        let author = create_test_author();
        let cloned = author.clone();
        assert_eq!(author.name, cloned.name);
        assert_eq!(author.email, cloned.email);
        assert_eq!(author.url, cloned.url);
    }

    #[test]
    fn test_author_info_debug() {
        let author = create_test_author();
        let debug = format!("{:?}", author);
        assert!(debug.contains("AuthorInfo"));
        assert!(debug.contains("Test Author"));
    }

    #[test]
    fn test_author_info_serialize() {
        let author = create_test_author();
        let json = serde_json::to_string(&author).unwrap();
        assert!(json.contains("\"name\":\"Test Author\""));
        assert!(json.contains("\"email\":\"test@example.com\""));
    }

    #[test]
    fn test_author_info_deserialize() {
        let json = r#"{"name":"Author","email":null,"url":null}"#;
        let author: AuthorInfo = serde_json::from_str(json).unwrap();
        assert_eq!(author.name, "Author");
        assert!(author.email.is_none());
    }

    // PluginCategory tests
    #[test]
    fn test_plugin_category_serialize_all() {
        assert_eq!(serde_json::to_string(&PluginCategory::Auth).unwrap(), "\"auth\"");
        assert_eq!(serde_json::to_string(&PluginCategory::Template).unwrap(), "\"template\"");
        assert_eq!(serde_json::to_string(&PluginCategory::Response).unwrap(), "\"response\"");
        assert_eq!(serde_json::to_string(&PluginCategory::DataSource).unwrap(), "\"datasource\"");
        assert_eq!(serde_json::to_string(&PluginCategory::Middleware).unwrap(), "\"middleware\"");
        assert_eq!(serde_json::to_string(&PluginCategory::Testing).unwrap(), "\"testing\"");
        assert_eq!(
            serde_json::to_string(&PluginCategory::Observability).unwrap(),
            "\"observability\""
        );
        assert_eq!(serde_json::to_string(&PluginCategory::Other).unwrap(), "\"other\"");
    }

    #[test]
    fn test_plugin_category_deserialize() {
        let category: PluginCategory = serde_json::from_str("\"middleware\"").unwrap();
        assert!(matches!(category, PluginCategory::Middleware));
    }

    #[test]
    fn test_plugin_category_clone() {
        let category = PluginCategory::Testing;
        let cloned = category.clone();
        assert!(matches!(cloned, PluginCategory::Testing));
    }

    #[test]
    fn test_plugin_category_debug() {
        let category = PluginCategory::Observability;
        let debug = format!("{:?}", category);
        assert!(debug.contains("Observability"));
    }

    // SortOrder tests
    #[test]
    fn test_sort_order_serialize_all() {
        assert_eq!(serde_json::to_string(&SortOrder::Relevance).unwrap(), "\"relevance\"");
        assert_eq!(serde_json::to_string(&SortOrder::Downloads).unwrap(), "\"downloads\"");
        assert_eq!(serde_json::to_string(&SortOrder::Rating).unwrap(), "\"rating\"");
        assert_eq!(serde_json::to_string(&SortOrder::Recent).unwrap(), "\"recent\"");
        assert_eq!(serde_json::to_string(&SortOrder::Name).unwrap(), "\"name\"");
    }

    #[test]
    fn test_sort_order_deserialize() {
        let sort: SortOrder = serde_json::from_str("\"downloads\"").unwrap();
        assert!(matches!(sort, SortOrder::Downloads));
    }

    #[test]
    fn test_sort_order_clone() {
        let sort = SortOrder::Rating;
        let cloned = sort.clone();
        assert!(matches!(cloned, SortOrder::Rating));
    }

    #[test]
    fn test_sort_order_debug() {
        let sort = SortOrder::Recent;
        let debug = format!("{:?}", sort);
        assert!(debug.contains("Recent"));
    }

    // VersionEntry tests
    #[test]
    fn test_version_entry_clone() {
        let entry = create_test_version_entry();
        let cloned = entry.clone();
        assert_eq!(entry.version, cloned.version);
        assert_eq!(entry.checksum, cloned.checksum);
    }

    #[test]
    fn test_version_entry_debug() {
        let entry = create_test_version_entry();
        let debug = format!("{:?}", entry);
        assert!(debug.contains("VersionEntry"));
        assert!(debug.contains("1.0.0"));
    }

    #[test]
    fn test_version_entry_serialize() {
        let entry = create_test_version_entry();
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"yanked\":false"));
    }

    #[test]
    fn test_version_entry_with_dependencies() {
        let mut entry = create_test_version_entry();
        entry.dependencies.insert("other-plugin".to_string(), "^1.0".to_string());

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("other-plugin"));
    }

    #[test]
    fn test_version_entry_yanked() {
        let mut entry = create_test_version_entry();
        entry.yanked = true;

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"yanked\":true"));
    }

    // RegistryEntry tests
    #[test]
    fn test_registry_entry_serialization() {
        let entry = create_test_registry_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.name, deserialized.name);
        assert_eq!(entry.version, deserialized.version);
        assert_eq!(entry.downloads, deserialized.downloads);
    }

    #[test]
    fn test_registry_entry_clone() {
        let entry = create_test_registry_entry();
        let cloned = entry.clone();
        assert_eq!(entry.name, cloned.name);
        assert_eq!(entry.rating, cloned.rating);
    }

    #[test]
    fn test_registry_entry_debug() {
        let entry = create_test_registry_entry();
        let debug = format!("{:?}", entry);
        assert!(debug.contains("RegistryEntry"));
        assert!(debug.contains("test-plugin"));
    }

    #[test]
    fn test_registry_entry_with_no_optional_fields() {
        let entry = RegistryEntry {
            name: "minimal".to_string(),
            description: "Minimal plugin".to_string(),
            version: "0.1.0".to_string(),
            versions: vec![],
            author: AuthorInfo {
                name: "Author".to_string(),
                email: None,
                url: None,
            },
            tags: vec![],
            category: PluginCategory::Other,
            downloads: 0,
            rating: 0.0,
            reviews_count: 0,
            repository: None,
            homepage: None,
            license: "MIT".to_string(),
            created_at: "2025-01-01".to_string(),
            updated_at: "2025-01-01".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert!(deserialized.repository.is_none());
    }

    // SearchQuery tests
    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert_eq!(query.page, 0);
        assert_eq!(query.per_page, 20);
        assert!(query.query.is_none());
        assert!(query.category.is_none());
        assert!(query.tags.is_empty());
        assert!(matches!(query.sort, SortOrder::Relevance));
    }

    #[test]
    fn test_search_query_clone() {
        let mut query = SearchQuery::default();
        query.query = Some("auth".to_string());
        query.page = 5;

        let cloned = query.clone();
        assert_eq!(query.query, cloned.query);
        assert_eq!(query.page, cloned.page);
    }

    #[test]
    fn test_search_query_serialize() {
        let query = SearchQuery {
            query: Some("jwt".to_string()),
            category: Some(PluginCategory::Auth),
            tags: vec!["security".to_string()],
            sort: SortOrder::Downloads,
            page: 1,
            per_page: 50,
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("\"query\":\"jwt\""));
        assert!(json.contains("\"category\":\"auth\""));
    }

    #[test]
    fn test_search_query_debug() {
        let query = SearchQuery::default();
        let debug = format!("{:?}", query);
        assert!(debug.contains("SearchQuery"));
    }

    // SearchResults tests
    #[test]
    fn test_search_results_serialize() {
        let results = SearchResults {
            plugins: vec![create_test_registry_entry()],
            total: 1,
            page: 0,
            per_page: 20,
        };

        let json = serde_json::to_string(&results).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("test-plugin"));
    }

    #[test]
    fn test_search_results_clone() {
        let results = SearchResults {
            plugins: vec![],
            total: 100,
            page: 5,
            per_page: 20,
        };

        let cloned = results.clone();
        assert_eq!(results.total, cloned.total);
        assert_eq!(results.page, cloned.page);
    }

    #[test]
    fn test_search_results_debug() {
        let results = SearchResults {
            plugins: vec![],
            total: 0,
            page: 0,
            per_page: 20,
        };

        let debug = format!("{:?}", results);
        assert!(debug.contains("SearchResults"));
    }

    #[test]
    fn test_search_results_empty() {
        let results = SearchResults {
            plugins: vec![],
            total: 0,
            page: 0,
            per_page: 20,
        };

        let json = serde_json::to_string(&results).unwrap();
        let deserialized: SearchResults = serde_json::from_str(&json).unwrap();
        assert!(deserialized.plugins.is_empty());
        assert_eq!(deserialized.total, 0);
    }

    // RegistryConfig tests
    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.url, "https://registry.mockforge.dev");
        assert!(config.token.is_none());
        assert!(config.cache_dir.is_none());
        assert_eq!(config.timeout, 30);
        assert!(config.alternative_registries.is_empty());
    }

    #[test]
    fn test_registry_config_clone() {
        let mut config = RegistryConfig::default();
        config.token = Some("secret-token".to_string());

        let cloned = config.clone();
        assert_eq!(config.url, cloned.url);
        assert_eq!(config.token, cloned.token);
    }

    #[test]
    fn test_registry_config_serialize() {
        let config = RegistryConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"url\":\"https://registry.mockforge.dev\""));
        assert!(json.contains("\"timeout\":30"));
    }

    #[test]
    fn test_registry_config_deserialize() {
        let json = r#"{
            "url": "https://custom.registry.com",
            "token": "my-token",
            "cache_dir": "/tmp/cache",
            "timeout": 60,
            "alternative_registries": ["https://alt.registry.com"]
        }"#;

        let config: RegistryConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.url, "https://custom.registry.com");
        assert_eq!(config.token, Some("my-token".to_string()));
        assert_eq!(config.cache_dir, Some("/tmp/cache".to_string()));
        assert_eq!(config.timeout, 60);
        assert_eq!(config.alternative_registries.len(), 1);
    }

    #[test]
    fn test_registry_config_debug() {
        let config = RegistryConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("RegistryConfig"));
    }

    #[test]
    fn test_registry_config_with_alternatives() {
        let mut config = RegistryConfig::default();
        config.alternative_registries = vec![
            "https://mirror1.registry.com".to_string(),
            "https://mirror2.registry.com".to_string(),
        ];

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("mirror1"));
        assert!(json.contains("mirror2"));
    }
}
