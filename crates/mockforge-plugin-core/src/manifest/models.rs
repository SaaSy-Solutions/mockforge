//! Core plugin manifest data models
//!
//! This module defines the fundamental data structures for plugin manifests,
//! including the main PluginManifest struct and related types.

use crate::{PluginCapabilities, PluginError, PluginId, PluginVersion, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::schema::ConfigSchema;
use semver;

/// Plugin manifest structure
///
/// The manifest contains all metadata about a plugin, including its capabilities,
/// dependencies, and configuration requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin manifest format version
    pub manifest_version: String,
    /// Plugin basic information
    pub plugin: PluginInfo,
    /// Plugin capabilities and permissions
    pub capabilities: PluginCapabilities,
    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Plugin configuration schema
    pub config_schema: Option<ConfigSchema>,
    /// Plugin metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PluginManifest {
    /// Create a new plugin manifest
    pub fn new(plugin: PluginInfo) -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            plugin,
            capabilities: PluginCapabilities::default(),
            dependencies: Vec::new(),
            config_schema: None,
            metadata: HashMap::new(),
        }
    }

    /// Validate manifest
    pub fn validate(&self) -> Result<()> {
        // Validate manifest version
        if self.manifest_version != "1.0" {
            return Err(PluginError::config_error(&format!(
                "Unsupported manifest version: {}",
                self.manifest_version
            )));
        }

        // Validate plugin info
        self.plugin.validate()?;

        // Validate dependencies
        for dep in &self.dependencies {
            dep.validate()?;
        }

        // Validate config schema if present
        if let Some(schema) = &self.config_schema {
            schema.validate()?;
        }

        Ok(())
    }

    /// Get plugin ID
    pub fn id(&self) -> &PluginId {
        &self.plugin.id
    }

    /// Get plugin version
    pub fn version(&self) -> &PluginVersion {
        &self.plugin.version
    }

    /// Check if plugin supports a specific type
    pub fn supports_type(&self, plugin_type: &str) -> bool {
        self.plugin.types.contains(&plugin_type.to_string())
    }

    /// Get plugin display name
    pub fn display_name(&self) -> &str {
        &self.plugin.name
    }

    /// Get plugin description
    pub fn description(&self) -> Option<&str> {
        self.plugin.description.as_deref()
    }

    /// Get plugin author
    pub fn author(&self) -> Option<&PluginAuthor> {
        self.plugin.author.as_ref()
    }

    /// Check if plugin has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.has_capability(capability)
    }

    /// Get all plugin types
    pub fn types(&self) -> &[String] {
        &self.plugin.types
    }

    /// Get plugin dependencies
    pub fn dependencies(&self) -> &[PluginDependency] {
        &self.dependencies
    }

    /// Check if plugin requires configuration
    pub fn requires_config(&self) -> bool {
        self.config_schema.is_some()
    }
}

/// Plugin basic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub id: PluginId,
    /// Plugin name (display name)
    pub name: String,
    /// Plugin version
    pub version: PluginVersion,
    /// Plugin description
    pub description: Option<String>,
    /// Plugin author information
    pub author: Option<PluginAuthor>,
    /// Supported plugin types
    pub types: Vec<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// License
    pub license: Option<String>,
    /// Keywords for plugin discovery
    pub keywords: Vec<String>,
}

impl PluginInfo {
    /// Create new plugin info
    pub fn new(id: PluginId, name: String, version: PluginVersion) -> Self {
        Self {
            id,
            name,
            version,
            description: None,
            author: None,
            types: Vec::new(),
            homepage: None,
            repository: None,
            license: None,
            keywords: Vec::new(),
        }
    }

    /// Validate plugin info
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(PluginError::config_error("Plugin name cannot be empty"));
        }

        if self.types.is_empty() {
            return Err(PluginError::config_error("Plugin must specify at least one type"));
        }

        for plugin_type in &self.types {
            if plugin_type.trim().is_empty() {
                return Err(PluginError::config_error("Plugin type cannot be empty"));
            }
        }

        Ok(())
    }

    /// Check if plugin matches keywords
    pub fn matches_keywords(&self, keywords: &[String]) -> bool {
        if keywords.is_empty() {
            return true;
        }

        keywords.iter().any(|keyword| {
            self.keywords.iter().any(|plugin_keyword| {
                plugin_keyword.to_lowercase().contains(&keyword.to_lowercase())
            })
        })
    }
}

/// Plugin author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    /// Author name
    pub name: String,
    /// Author email
    pub email: Option<String>,
    /// Author homepage
    pub url: Option<String>,
}

impl PluginAuthor {
    /// Create new author
    pub fn new(name: String) -> Self {
        Self {
            name,
            email: None,
            url: None,
        }
    }

    /// Validate author info
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(PluginError::config_error("Author name cannot be empty"));
        }
        Ok(())
    }
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency plugin ID
    pub id: PluginId,
    /// Version requirement (semver)
    pub version: String,
    /// Whether this dependency is optional
    pub optional: bool,
}

impl PluginDependency {
    /// Create new dependency
    pub fn new(id: PluginId, version: String) -> Self {
        Self {
            id,
            version,
            optional: false,
        }
    }

    /// Create optional dependency
    pub fn optional(id: PluginId, version: String) -> Self {
        Self {
            id,
            version,
            optional: true,
        }
    }

    /// Validate dependency
    pub fn validate(&self) -> Result<()> {
        if self.version.trim().is_empty() {
            return Err(PluginError::config_error(&format!(
                "Dependency {} version cannot be empty",
                self.id
            )));
        }
        Ok(())
    }

    /// Check if version requirement is satisfied
    pub fn satisfies_version(&self, version: &PluginVersion) -> bool {
        // Handle wildcard
        if self.version == "*" {
            return true;
        }

        // For plugin dependencies, treat bare versions as exact matches
        // Prepend "=" if the version doesn't start with a comparator
        let req_str = if self.version.starts_with(|c: char| c.is_ascii_digit()) {
            format!("={}", self.version)
        } else {
            self.version.clone()
        };

        // Parse version requirement
        let req = match semver::VersionReq::parse(&req_str) {
            Ok(req) => req,
            Err(_) => return false, // Invalid requirement
        };

        // Convert PluginVersion to semver::Version
        let semver_version = match version.to_semver() {
            Ok(v) => v,
            Err(_) => return false, // Invalid version
        };

        req.matches(&semver_version)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
    }
}
