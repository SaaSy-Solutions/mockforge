//! Plugin manifest validation for registry

use crate::{RegistryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validate plugin manifest for registry submission
pub fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    // Check required fields
    if manifest.name.is_empty() {
        return Err(RegistryError::InvalidManifest("Plugin name cannot be empty".to_string()));
    }

    if !is_valid_plugin_name(&manifest.name) {
        return Err(RegistryError::InvalidManifest(
            "Plugin name must be lowercase alphanumeric with hyphens".to_string(),
        ));
    }

    if manifest.version.is_empty() {
        return Err(RegistryError::InvalidManifest("Version cannot be empty".to_string()));
    }

    if !is_valid_semver(&manifest.version) {
        return Err(RegistryError::InvalidManifest(
            "Version must be valid semver (e.g., 1.0.0)".to_string(),
        ));
    }

    if manifest.description.is_empty() {
        return Err(RegistryError::InvalidManifest("Description cannot be empty".to_string()));
    }

    if manifest.description.len() > 500 {
        return Err(RegistryError::InvalidManifest(
            "Description must be less than 500 characters".to_string(),
        ));
    }

    if manifest.license.is_empty() {
        return Err(RegistryError::InvalidManifest("License cannot be empty".to_string()));
    }

    // Check author
    if manifest.author.name.is_empty() {
        return Err(RegistryError::InvalidManifest("Author name cannot be empty".to_string()));
    }

    // Check tags
    if manifest.tags.len() > 10 {
        return Err(RegistryError::InvalidManifest("Maximum 10 tags allowed".to_string()));
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

    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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

    // Validation tests
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
    fn test_empty_plugin_name() {
        let mut manifest = create_valid_manifest();
        manifest.name = "".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_invalid_version() {
        let mut manifest = create_valid_manifest();
        manifest.version = "1.0".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_empty_version() {
        let mut manifest = create_valid_manifest();
        manifest.version = "".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Version cannot be empty"));
    }

    #[test]
    fn test_empty_description() {
        let mut manifest = create_valid_manifest();
        manifest.description = "".to_string();
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn test_description_too_long() {
        let mut manifest = create_valid_manifest();
        manifest.description = "a".repeat(501);
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500 characters"));
    }

    #[test]
    fn test_description_at_limit() {
        let mut manifest = create_valid_manifest();
        manifest.description = "a".repeat(500);
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_empty_license() {
        let mut manifest = create_valid_manifest();
        manifest.license = "".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("License cannot be empty"));
    }

    #[test]
    fn test_empty_author_name() {
        let mut manifest = create_valid_manifest();
        manifest.author.name = "".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Author name cannot be empty"));
    }

    #[test]
    fn test_too_many_tags() {
        let mut manifest = create_valid_manifest();
        manifest.tags = (0..11).map(|i| format!("tag{}", i)).collect();
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("10 tags"));
    }

    #[test]
    fn test_tag_too_long() {
        let mut manifest = create_valid_manifest();
        manifest.tags = vec!["a".repeat(21)];
        let result = validate_manifest(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("20 characters"));
    }

    #[test]
    fn test_tag_at_limit() {
        let mut manifest = create_valid_manifest();
        manifest.tags = vec!["a".repeat(20)];
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_max_tags() {
        let mut manifest = create_valid_manifest();
        manifest.tags = (0..10).map(|i| format!("tag{}", i)).collect();
        assert!(validate_manifest(&manifest).is_ok());
    }

    // is_valid_plugin_name tests
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
    fn test_is_valid_plugin_name_lowercase_only() {
        assert!(is_valid_plugin_name("lowercase"));
        assert!(!is_valid_plugin_name("UPPERCASE"));
        assert!(!is_valid_plugin_name("MixedCase"));
    }

    #[test]
    fn test_is_valid_plugin_name_with_numbers() {
        assert!(is_valid_plugin_name("plugin1"));
        assert!(is_valid_plugin_name("1plugin"));
        assert!(is_valid_plugin_name("12345"));
    }

    #[test]
    fn test_is_valid_plugin_name_with_hyphens() {
        assert!(is_valid_plugin_name("my-plugin"));
        assert!(is_valid_plugin_name("my-cool-plugin"));
        assert!(is_valid_plugin_name("-starts-with-hyphen"));
        assert!(is_valid_plugin_name("ends-with-hyphen-"));
    }

    #[test]
    fn test_is_valid_plugin_name_invalid_chars() {
        assert!(!is_valid_plugin_name("my_plugin")); // underscore
        assert!(!is_valid_plugin_name("my.plugin")); // dot
        assert!(!is_valid_plugin_name("my plugin")); // space
        assert!(!is_valid_plugin_name("my@plugin")); // at sign
    }

    #[test]
    fn test_is_valid_plugin_name_too_long() {
        let long_name = "a".repeat(51);
        assert!(!is_valid_plugin_name(&long_name));
    }

    #[test]
    fn test_is_valid_plugin_name_at_limit() {
        let max_name = "a".repeat(50);
        assert!(is_valid_plugin_name(&max_name));
    }

    // is_valid_semver tests
    #[test]
    fn test_is_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.2"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver("v1.0.0"));
    }

    #[test]
    fn test_is_valid_semver_parts() {
        assert!(is_valid_semver("0.0.0"));
        assert!(is_valid_semver("999.999.999"));
        assert!(is_valid_semver("10.20.30"));
    }

    #[test]
    fn test_is_valid_semver_invalid_parts() {
        assert!(!is_valid_semver("a.b.c"));
        assert!(!is_valid_semver("1.2.x"));
        assert!(!is_valid_semver("-1.0.0"));
    }

    #[test]
    fn test_is_valid_semver_wrong_format() {
        assert!(!is_valid_semver("1"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.0.0.0"));
        assert!(!is_valid_semver(""));
    }

    // PluginManifest tests
    #[test]
    fn test_plugin_manifest_clone() {
        let manifest = create_valid_manifest();
        let cloned = manifest.clone();
        assert_eq!(manifest.name, cloned.name);
        assert_eq!(manifest.version, cloned.version);
    }

    #[test]
    fn test_plugin_manifest_debug() {
        let manifest = create_valid_manifest();
        let debug = format!("{:?}", manifest);
        assert!(debug.contains("PluginManifest"));
        assert!(debug.contains("test-plugin"));
    }

    #[test]
    fn test_plugin_manifest_serialize() {
        let manifest = create_valid_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"name\":\"test-plugin\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"license\":\"MIT\""));
    }

    #[test]
    fn test_plugin_manifest_deserialize() {
        let json = r#"{
            "name": "deserialized-plugin",
            "version": "2.0.0",
            "description": "Deserialized plugin",
            "author": {"name": "Author", "email": null, "url": null},
            "license": "Apache-2.0",
            "repository": null,
            "homepage": null,
            "tags": [],
            "category": "middleware",
            "min_mockforge_version": "0.4.0",
            "dependencies": {}
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "deserialized-plugin");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.min_mockforge_version, Some("0.4.0".to_string()));
    }

    #[test]
    fn test_plugin_manifest_with_dependencies() {
        let mut manifest = create_valid_manifest();
        manifest.dependencies.insert("other-plugin".to_string(), "^1.0.0".to_string());
        manifest.dependencies.insert("another-dep".to_string(), ">=2.0.0".to_string());

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("other-plugin"));
        assert!(json.contains("another-dep"));
    }

    #[test]
    fn test_plugin_manifest_with_optional_fields() {
        let mut manifest = create_valid_manifest();
        manifest.repository = Some("https://github.com/test/plugin".to_string());
        manifest.homepage = Some("https://plugin.example.com".to_string());
        manifest.min_mockforge_version = Some("0.3.0".to_string());

        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("github.com"));
        assert!(json.contains("plugin.example.com"));
    }

    // AuthorInfo tests
    #[test]
    fn test_author_info_clone() {
        let author = AuthorInfo {
            name: "Test".to_string(),
            email: Some("test@test.com".to_string()),
            url: Some("https://test.com".to_string()),
        };
        let cloned = author.clone();
        assert_eq!(author.name, cloned.name);
        assert_eq!(author.email, cloned.email);
    }

    #[test]
    fn test_author_info_debug() {
        let author = AuthorInfo {
            name: "Test Author".to_string(),
            email: None,
            url: None,
        };
        let debug = format!("{:?}", author);
        assert!(debug.contains("AuthorInfo"));
    }

    #[test]
    fn test_author_info_serialize() {
        let author = AuthorInfo {
            name: "Test".to_string(),
            email: Some("test@test.com".to_string()),
            url: None,
        };
        let json = serde_json::to_string(&author).unwrap();
        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("test@test.com"));
    }

    // PluginCategory tests
    #[test]
    fn test_plugin_category_serialize_all_variants() {
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
        let category: PluginCategory = serde_json::from_str("\"testing\"").unwrap();
        assert!(matches!(category, PluginCategory::Testing));
    }

    #[test]
    fn test_plugin_category_clone() {
        let category = PluginCategory::DataSource;
        let cloned = category.clone();
        assert!(matches!(cloned, PluginCategory::DataSource));
    }

    #[test]
    fn test_plugin_category_debug() {
        let category = PluginCategory::Observability;
        let debug = format!("{:?}", category);
        assert!(debug.contains("Observability"));
    }

    // Full manifest validation edge cases
    #[test]
    fn test_manifest_minimal_valid() {
        let manifest = PluginManifest {
            name: "a".to_string(),
            version: "0.0.1".to_string(),
            description: "x".to_string(),
            author: AuthorInfo {
                name: "A".to_string(),
                email: None,
                url: None,
            },
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            tags: vec![],
            category: PluginCategory::Other,
            min_mockforge_version: None,
            dependencies: HashMap::new(),
        };
        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_manifest_all_categories_valid() {
        let categories = vec![
            PluginCategory::Auth,
            PluginCategory::Template,
            PluginCategory::Response,
            PluginCategory::DataSource,
            PluginCategory::Middleware,
            PluginCategory::Testing,
            PluginCategory::Observability,
            PluginCategory::Other,
        ];

        for category in categories {
            let mut manifest = create_valid_manifest();
            manifest.category = category;
            assert!(validate_manifest(&manifest).is_ok());
        }
    }
}
