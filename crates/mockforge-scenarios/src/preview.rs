//! Scenario preview functionality
//!
//! Provides preview capabilities for scenarios before installation,
//! allowing users to inspect scenario contents without installing.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
use crate::package::ScenarioPackage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Preview information for a scenario package
///
/// Contains key information about a scenario that can be displayed
/// to users before they decide to install it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPreview {
    /// Scenario manifest metadata
    pub manifest: ScenarioManifest,

    /// Config file preview (first 50 lines)
    pub config_preview: Option<String>,

    /// OpenAPI endpoints summary
    pub openapi_endpoints: Vec<OpenApiEndpoint>,

    /// File tree structure
    pub file_tree: Vec<String>,

    /// Estimated installation size in bytes
    pub estimated_size: u64,

    /// Compatibility check results
    pub compatibility: CompatibilityCheck,
}

/// OpenAPI endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiEndpoint {
    /// HTTP method
    pub method: String,

    /// Endpoint path
    pub path: String,

    /// Operation summary (if available)
    pub summary: Option<String>,
}

/// Compatibility check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityCheck {
    /// Whether the scenario is compatible with current MockForge version
    pub compatible: bool,

    /// Current MockForge version
    pub current_version: String,

    /// Required minimum version
    pub min_version: String,

    /// Maximum supported version (if specified)
    pub max_version: Option<String>,

    /// Missing required features
    pub missing_features: Vec<String>,

    /// Missing required protocols
    pub missing_protocols: Vec<String>,
}

impl ScenarioPreview {
    /// Create a preview from a scenario package
    pub fn from_package(package: &ScenarioPackage) -> Result<Self> {
        // Extract config preview (first 50 lines)
        let config_preview = package.config_path().and_then(|path| {
            std::fs::read_to_string(&path)
                .ok()
                .map(|content| content.lines().take(50).collect::<Vec<_>>().join("\n"))
        });

        // Extract OpenAPI endpoints
        let openapi_endpoints = Self::extract_openapi_endpoints(package)?;

        // Generate file tree
        let file_tree = Self::generate_file_tree(package)?;

        // Calculate estimated size
        let estimated_size = Self::calculate_size(package)?;

        // Check compatibility
        let compatibility = Self::check_compatibility(&package.manifest)?;

        Ok(Self {
            manifest: package.manifest.clone(),
            config_preview,
            openapi_endpoints,
            file_tree,
            estimated_size,
            compatibility,
        })
    }

    /// Extract OpenAPI endpoints from the package
    fn extract_openapi_endpoints(package: &ScenarioPackage) -> Result<Vec<OpenApiEndpoint>> {
        let mut endpoints = Vec::new();

        if let Some(openapi_path) = package.openapi_path() {
            let content = std::fs::read_to_string(&openapi_path).map_err(ScenarioError::Io)?;

            // Try to parse as JSON first, then YAML
            let spec: serde_json::Value = if openapi_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "json")
                .unwrap_or(false)
            {
                serde_json::from_str(&content).map_err(ScenarioError::Serde)?
            } else {
                serde_yaml::from_str(&content).map_err(ScenarioError::Yaml)?
            };

            // Extract paths and operations
            if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
                for (path, path_item) in paths {
                    if let Some(path_obj) = path_item.as_object() {
                        for (method, operation) in path_obj {
                            if ["get", "post", "put", "patch", "delete", "options", "head"]
                                .contains(&method.to_lowercase().as_str())
                            {
                                let summary = operation
                                    .get("summary")
                                    .and_then(|s| s.as_str())
                                    .map(|s| s.to_string());

                                endpoints.push(OpenApiEndpoint {
                                    method: method.to_uppercase(),
                                    path: path.clone(),
                                    summary,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(endpoints)
    }

    /// Generate a file tree representation
    fn generate_file_tree(package: &ScenarioPackage) -> Result<Vec<String>> {
        let mut tree = Vec::new();
        let mut dirs = std::collections::HashSet::new();

        // Collect all directories
        for file in &package.files {
            if let Some(parent) = file.parent() {
                let mut path = PathBuf::new();
                for component in parent.components() {
                    path.push(component);
                    dirs.insert(path.clone());
                }
            }
        }

        // Sort directories
        let mut sorted_dirs: Vec<_> = dirs.iter().collect();
        sorted_dirs.sort();

        // Add directories to tree
        for dir in sorted_dirs {
            let display = if dir.as_os_str().is_empty() {
                ".".to_string()
            } else {
                format!("  {}/", dir.display())
            };
            tree.push(display);
        }

        // Add files to tree
        let mut sorted_files: Vec<_> = package.files.iter().collect();
        sorted_files.sort();

        for file in sorted_files {
            let display = if file.parent().is_some() {
                format!("  {}", file.display())
            } else {
                format!("  {}", file.file_name().unwrap_or_default().to_string_lossy())
            };
            tree.push(display);
        }

        Ok(tree)
    }

    /// Calculate estimated installation size
    fn calculate_size(package: &ScenarioPackage) -> Result<u64> {
        let mut total_size = 0u64;

        for file in &package.files {
            let full_path = package.root.join(file);
            if full_path.is_file() {
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Check compatibility with current MockForge version
    fn check_compatibility(manifest: &ScenarioManifest) -> Result<CompatibilityCheck> {
        // Get current version from environment
        let current_version = env!("CARGO_PKG_VERSION").to_string();

        // Parse versions using semver
        let min_version = &manifest.compatibility.min_version;
        let max_version = manifest.compatibility.max_version.as_ref();

        // Parse versions for comparison
        let current_ver = semver::Version::parse(&current_version)
            .unwrap_or_else(|_| semver::Version::parse("0.0.0").unwrap());
        let min_ver = semver::Version::parse(min_version)
            .unwrap_or_else(|_| semver::Version::parse("0.0.0").unwrap());

        // Check if current version meets minimum requirement
        let meets_min = current_ver >= min_ver;

        // Check if current version is within max requirement (if specified)
        let within_max = if let Some(max) = max_version {
            if let Ok(max_ver) = semver::Version::parse(max) {
                current_ver <= max_ver
            } else {
                true // If max version is invalid, assume compatible
            }
        } else {
            true // No max version specified
        };

        let compatible = meets_min && within_max;

        // Check for missing features (simplified - would check actual features in production)
        let missing_features = Vec::new();

        // Check for missing protocols (simplified - would check actual protocols in production)
        let missing_protocols = Vec::new();

        Ok(CompatibilityCheck {
            compatible,
            current_version,
            min_version: min_version.clone(),
            max_version: max_version.cloned(),
            missing_features,
            missing_protocols,
        })
    }

    /// Format preview for display
    pub fn format_display(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "📦 Scenario Preview: {}@{}\n",
            self.manifest.name, self.manifest.version
        ));
        output.push_str(&format!("   Title: {}\n", self.manifest.title));
        output.push_str(&format!("   Author: {}\n", self.manifest.author));
        if let Some(email) = &self.manifest.author_email {
            output.push_str(&format!("   Email: {}\n", email));
        }
        output.push_str(&format!("   Category: {:?}\n", self.manifest.category));
        output.push_str(&format!("   Tags: {}\n", self.manifest.tags.join(", ")));
        output.push('\n');

        // Description
        output.push_str("Description:\n");
        for line in self.manifest.description.lines() {
            output.push_str(&format!("   {}\n", line));
        }
        output.push('\n');

        // Compatibility
        output.push_str("Compatibility:\n");
        output.push_str(&format!(
            "   Status: {}\n",
            if self.compatibility.compatible {
                "✅ Compatible"
            } else {
                "❌ Incompatible"
            }
        ));
        output.push_str(&format!("   Current Version: {}\n", self.compatibility.current_version));
        output.push_str(&format!("   Required Min Version: {}\n", self.compatibility.min_version));
        if let Some(max) = &self.compatibility.max_version {
            output.push_str(&format!("   Max Version: {}\n", max));
        }
        output.push('\n');

        // File structure
        output.push_str("File Structure:\n");
        for line in &self.file_tree {
            output.push_str(&format!("   {}\n", line));
        }
        output.push('\n');

        // OpenAPI endpoints
        if !self.openapi_endpoints.is_empty() {
            output.push_str(&format!("OpenAPI Endpoints ({}):\n", self.openapi_endpoints.len()));
            for endpoint in &self.openapi_endpoints {
                if let Some(summary) = &endpoint.summary {
                    output.push_str(&format!(
                        "   {} {} - {}\n",
                        endpoint.method, endpoint.path, summary
                    ));
                } else {
                    output.push_str(&format!("   {} {}\n", endpoint.method, endpoint.path));
                }
            }
            output.push('\n');
        }

        // Config preview
        if let Some(config) = &self.config_preview {
            output.push_str("Config Preview (first 50 lines):\n");
            for line in config.lines().take(50) {
                output.push_str(&format!("   {}\n", line));
            }
            output.push('\n');
        }

        // Size estimate
        let size_mb = self.estimated_size as f64 / 1_048_576.0;
        output.push_str(&format!(
            "Estimated Size: {:.2} MB ({} bytes)\n",
            size_mb, self.estimated_size
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{CompatibilityInfo, ScenarioCategory, ScenarioManifest};
    use tempfile::TempDir;

    #[test]
    fn test_preview_creation() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create minimal scenario
        let manifest = ScenarioManifest::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            "Test Scenario".to_string(),
            "A test scenario".to_string(),
        );

        let package = ScenarioPackage {
            root: root.to_path_buf(),
            manifest: manifest.clone(),
            files: vec![],
        };

        let preview = ScenarioPreview::from_package(&package).unwrap();
        assert_eq!(preview.manifest.name, "test-scenario");
        assert_eq!(preview.estimated_size, 0);
    }
}
