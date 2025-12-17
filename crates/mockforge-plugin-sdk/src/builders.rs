//! Builder patterns for plugin manifests and configurations
//!
//! This module provides fluent builder APIs for creating plugin manifests,
//! making it easier to configure plugins without dealing with raw structs.

use mockforge_plugin_core::*;

/// Builder for plugin manifests
///
/// # Example
///
/// ```rust
/// use mockforge_plugin_sdk::builders::ManifestBuilder;
///
/// let manifest = ManifestBuilder::new("my-plugin", "1.0.0")
///     .name("My Plugin")
///     .description("A custom plugin for authentication")
///     .author("Your Name", "your.email@example.com")
///     .capability("network")
///     .capability("filesystem.read")
///     .build();
/// ```
pub struct ManifestBuilder {
    manifest: PluginManifest,
}

impl ManifestBuilder {
    /// Create a new manifest builder
    pub fn new(id: &str, version: &str) -> Self {
        let version = PluginVersion::parse(version).unwrap_or_else(|_| PluginVersion::new(0, 1, 0));
        let info = PluginInfo {
            id: PluginId::new(id),
            version,
            name: String::new(),
            description: String::new(),
            author: PluginAuthor::new("Unknown"),
        };

        Self {
            manifest: PluginManifest::new(info),
        }
    }

    /// Set plugin name
    pub fn name(mut self, name: &str) -> Self {
        self.manifest.info.name = name.to_string();
        self
    }

    /// Set plugin description
    pub fn description(mut self, description: &str) -> Self {
        self.manifest.info.description = description.to_string();
        self
    }

    /// Set plugin author
    pub fn author(mut self, name: &str, email: &str) -> Self {
        self.manifest.info.author = PluginAuthor::with_email(name, email);
        self
    }

    /// Set plugin author (name only)
    pub fn author_name(mut self, name: &str) -> Self {
        self.manifest.info.author = PluginAuthor::new(name);
        self
    }

    /// Add a capability
    ///
    /// Common capabilities: "network", "filesystem.read", "filesystem.write"
    pub fn capability(mut self, capability: &str) -> Self {
        self.manifest.capabilities.push(capability.to_string());
        self
    }

    /// Add multiple capabilities
    pub fn capabilities(mut self, capabilities: &[&str]) -> Self {
        for cap in capabilities {
            self.manifest.capabilities.push(cap.to_string());
        }
        self
    }

    /// Add a dependency
    pub fn dependency(mut self, plugin_id: &str, version: &str) -> Self {
        if let Ok(parsed_version) = PluginVersion::parse(version) {
            self.manifest.dependencies.insert(PluginId::new(plugin_id), parsed_version);
        }
        self
    }

    /// Build the manifest
    pub fn build(self) -> PluginManifest {
        self.manifest
    }

    /// Build and save to file
    pub fn build_and_save(self, path: &str) -> std::result::Result<PluginManifest, std::io::Error> {
        let manifest = self.manifest;
        let yaml = serde_yaml::to_string(&manifest)
            .map_err(|e| std::io::Error::other(format!("YAML error: {}", e)))?;
        std::fs::write(path, yaml)?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_builder() {
        let manifest = ManifestBuilder::new("test-plugin", "1.0.0")
            .name("Test Plugin")
            .description("A test plugin")
            .author("Test Author", "test@example.com")
            .capability("network")
            .capability("filesystem.read")
            .build();

        assert_eq!(manifest.info.id, PluginId::new("test-plugin"));
        assert_eq!(manifest.info.name, "Test Plugin");
        assert_eq!(manifest.info.description, "A test plugin");
        assert_eq!(manifest.capabilities.len(), 2);
        assert!(manifest.capabilities.contains(&"network".to_string()));
        assert!(manifest.capabilities.contains(&"filesystem.read".to_string()));
    }

    #[test]
    fn test_manifest_with_dependencies() {
        let manifest = ManifestBuilder::new("test-plugin", "2.0.0")
            .name("Test Plugin")
            .dependency("dep1", "1.0.0")
            .dependency("dep2", "1.5.0")
            .build();

        assert_eq!(manifest.dependencies.len(), 2);
    }

    #[test]
    fn test_manifest_save() {
        use tempfile::NamedTempFile;

        let manifest = ManifestBuilder::new("test-plugin", "1.0.0")
            .name("Test Plugin")
            .description("A test plugin")
            .author("Test", "test@example.com")
            .build();

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let yaml = serde_yaml::to_string(&manifest).unwrap();
        std::fs::write(path, yaml).unwrap();

        let loaded = PluginManifest::from_file(path).unwrap();
        assert_eq!(loaded.info.id, manifest.info.id);
        assert_eq!(loaded.info.name, manifest.info.name);
    }

    #[test]
    fn test_manifest_builder_new() {
        let manifest = ManifestBuilder::new("my-plugin", "1.2.3").build();
        assert_eq!(manifest.info.id.as_str(), "my-plugin");
        assert_eq!(manifest.info.version.major, 1);
        assert_eq!(manifest.info.version.minor, 2);
        assert_eq!(manifest.info.version.patch, 3);
    }

    #[test]
    fn test_manifest_builder_invalid_version() {
        // Invalid version defaults to 0.1.0
        let manifest = ManifestBuilder::new("my-plugin", "invalid").build();
        assert_eq!(manifest.info.version.major, 0);
        assert_eq!(manifest.info.version.minor, 1);
        assert_eq!(manifest.info.version.patch, 0);
    }

    #[test]
    fn test_manifest_builder_default_author() {
        let manifest = ManifestBuilder::new("my-plugin", "1.0.0").build();
        assert_eq!(manifest.info.author.name, "Unknown");
    }

    #[test]
    fn test_manifest_builder_author_name() {
        let manifest = ManifestBuilder::new("test", "1.0.0").author_name("John Doe").build();
        assert_eq!(manifest.info.author.name, "John Doe");
    }

    #[test]
    fn test_manifest_builder_author_with_email() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .author("Jane Doe", "jane@example.com")
            .build();
        assert_eq!(manifest.info.author.name, "Jane Doe");
        assert_eq!(manifest.info.author.email, Some("jane@example.com".to_string()));
    }

    #[test]
    fn test_manifest_builder_capabilities_batch() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .capabilities(&["network", "filesystem.read", "filesystem.write"])
            .build();
        assert_eq!(manifest.capabilities.len(), 3);
        assert!(manifest.capabilities.contains(&"network".to_string()));
        assert!(manifest.capabilities.contains(&"filesystem.read".to_string()));
        assert!(manifest.capabilities.contains(&"filesystem.write".to_string()));
    }

    #[test]
    fn test_manifest_builder_empty_capabilities() {
        let manifest = ManifestBuilder::new("test", "1.0.0").capabilities(&[]).build();
        assert!(manifest.capabilities.is_empty());
    }

    #[test]
    fn test_manifest_builder_dependency_invalid_version() {
        // Invalid dependency version is ignored
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .dependency("dep1", "invalid-version")
            .build();
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_manifest_builder_chaining() {
        let manifest = ManifestBuilder::new("chain-test", "1.0.0")
            .name("Chain Test Plugin")
            .description("Testing method chaining")
            .author_name("Test Author")
            .capability("network")
            .capability("database")
            .dependency("base-plugin", "2.0.0")
            .build();

        assert_eq!(manifest.info.name, "Chain Test Plugin");
        assert_eq!(manifest.info.description, "Testing method chaining");
        assert_eq!(manifest.info.author.name, "Test Author");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_manifest_builder_overwrite_name() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .name("First Name")
            .name("Second Name")
            .build();
        assert_eq!(manifest.info.name, "Second Name");
    }

    #[test]
    fn test_manifest_builder_overwrite_description() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .description("First description")
            .description("Second description")
            .build();
        assert_eq!(manifest.info.description, "Second description");
    }

    #[test]
    fn test_manifest_builder_overwrite_author() {
        let manifest = ManifestBuilder::new("test", "1.0.0")
            .author("First Author", "first@example.com")
            .author("Second Author", "second@example.com")
            .build();
        assert_eq!(manifest.info.author.name, "Second Author");
        assert_eq!(manifest.info.author.email, Some("second@example.com".to_string()));
    }

    #[test]
    fn test_build_and_save() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = ManifestBuilder::new("test-plugin", "1.0.0")
            .name("Test Plugin")
            .description("A test plugin")
            .author("Test", "test@example.com")
            .build_and_save(path);

        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest.info.name, "Test Plugin");

        // Verify file was written
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("test-plugin"));
        assert!(content.contains("Test Plugin"));
    }

    #[test]
    fn test_build_and_save_invalid_path() {
        let result =
            ManifestBuilder::new("test", "1.0.0").build_and_save("/nonexistent/path/manifest.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_builder_empty_strings() {
        let manifest = ManifestBuilder::new("", "").name("").description("").build();
        assert_eq!(manifest.info.id.as_str(), "");
        assert_eq!(manifest.info.name, "");
        assert_eq!(manifest.info.description, "");
    }

    #[test]
    fn test_manifest_builder_special_characters() {
        let manifest = ManifestBuilder::new("plugin-with-dashes", "1.0.0")
            .name("Plugin with Special Characters: !@#$%")
            .description("Description with\nnewlines and\ttabs")
            .build();
        assert_eq!(manifest.info.id.as_str(), "plugin-with-dashes");
        assert!(manifest.info.name.contains("Special Characters"));
        assert!(manifest.info.description.contains("\n"));
    }
}
