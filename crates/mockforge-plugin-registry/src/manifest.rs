//! Plugin manifest validation for registry

use crate::{Result, RegistryError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validate plugin manifest for registry submission
pub fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    // Check required fields
    if manifest.name.is_empty() {
        return Err(RegistryError::InvalidManifest(
            "Plugin name cannot be empty".to_string(),
        ));
    }

    if !is_valid_plugin_name(&manifest.name) {
        return Err(RegistryError::InvalidManifest(
            "Plugin name must be lowercase alphanumeric with hyphens".to_string(),
        ));
    }

    if manifest.version.is_empty() {
        return Err(RegistryError::InvalidManifest(
            "Version cannot be empty".to_string(),
        ));
    }

    if !is_valid_semver(&manifest.version) {
        return Err(RegistryError::InvalidManifest(
            "Version must be valid semver (e.g., 1.0.0)".to_string(),
        ));
    }

    if manifest.description.is_empty() {
        return Err(RegistryError::InvalidManifest(
            "Description cannot be empty".to_string(),
        ));
    }

    if manifest.description.len() > 500 {
        return Err(RegistryError::InvalidManifest(
            "Description must be less than 500 characters".to_string(),
        ));
    }

    if manifest.license.is_empty() {
        return Err(RegistryError::InvalidManifest(
            "License cannot be empty".to_string(),
        ));
    }

    // Check author
    if manifest.author.name.is_empty() {
        return Err(RegistryError::InvalidManifest(
            "Author name cannot be empty".to_string(),
        ));
    }

    // Check tags
    if manifest.tags.len() > 10 {
        return Err(RegistryError::InvalidManifest(
            "Maximum 10 tags allowed".to_string(),
        ));
    }

    for tag in &manifest.tags {
        if tag.len() > 20 {
            return Err(RegistryError::InvalidManifest(
                "Tag must be less than 20 characters".to_string(),
            ));
        }
    }

    Ok(())
}

/// Check if plugin name is valid
fn is_valid_plugin_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 50 {
        return false;
    }

    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Check if version is valid semver
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() != 3 {
        return false;
    }

    parts.iter().all(|part| part.parse::<u32>().is_ok())
}

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: AuthorInfo,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    pub category: PluginCategory,
    pub min_mockforge_version: Option<String>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_manifest() -> PluginManifest {
        PluginManifest {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: AuthorInfo {
                name: "Test Author".to_string(),
                email: Some("test@example.com".to_string()),
                url: None,
            },
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            tags: vec!["test".to_string()],
            category: PluginCategory::Auth,
            min_mockforge_version: None,
            dependencies: HashMap::new(),
        }
    }

    #[test]
    fn test_valid_manifest() {
        let manifest = create_valid_manifest();
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_invalid_plugin_name() {
        let mut manifest = create_valid_manifest();
        manifest.name = "Invalid_Name".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_invalid_version() {
        let mut manifest = create_valid_manifest();
        manifest.version = "1.0".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_empty_description() {
        let mut manifest = create_valid_manifest();
        manifest.description = "".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_too_many_tags() {
        let mut manifest = create_valid_manifest();
        manifest.tags = (0..11).map(|i| format!("tag{}", i)).collect();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_is_valid_plugin_name() {
        assert!(is_valid_plugin_name("my-plugin"));
        assert!(is_valid_plugin_name("auth-jwt"));
        assert!(is_valid_plugin_name("plugin123"));
        assert!(!is_valid_plugin_name("My_Plugin"));
        assert!(!is_valid_plugin_name("plugin!"));
        assert!(!is_valid_plugin_name(""));
    }

    #[test]
    fn test_is_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.2"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver("v1.0.0"));
    }
}
