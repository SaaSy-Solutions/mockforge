//! Plugin manifest loading utilities
//!
//! This module provides functionality for loading and parsing plugin manifests
//! from files and strings.

use crate::{PluginError, Result};
use std::path::Path;

use super::models::PluginManifest;

/// Manifest loader utility
pub struct ManifestLoader;

impl ManifestLoader {
    /// Load manifest from file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<PluginManifest> {
        PluginManifest::from_file(path)
    }

    /// Load manifest from string content
    pub fn load_from_string(content: &str) -> Result<PluginManifest> {
        PluginManifest::parse_from_str(content)
    }

    /// Load and validate manifest from file
    pub fn load_and_validate_from_file<P: AsRef<Path>>(path: P) -> Result<PluginManifest> {
        let manifest = Self::load_from_file(path)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load and validate manifest from string
    pub fn load_and_validate_from_string(content: &str) -> Result<PluginManifest> {
        let manifest = Self::load_from_string(content)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load multiple manifests from directory
    pub fn load_from_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
                match Self::load_from_file(&path) {
                    Ok(manifest) => manifests.push(manifest),
                    Err(e) => {
                        // Log error but continue loading other manifests
                        eprintln!("Failed to load manifest from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Load and validate multiple manifests from directory
    pub fn load_and_validate_from_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<PluginManifest>> {
        let manifests = Self::load_from_directory(dir)?;
        let mut validated = Vec::new();

        for manifest in manifests {
            match manifest.validate() {
                Ok(_) => validated.push(manifest),
                Err(e) => {
                    eprintln!("Failed to validate manifest for plugin {}: {}", manifest.id(), e);
                }
            }
        }

        Ok(validated)
    }
}

impl PluginManifest {
    /// Load manifest from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            PluginError::config_error(&format!("Failed to read manifest file: {}", e))
        })?;

        Self::parse_from_str(&content)
    }

    /// Parse manifest from string
    pub fn parse_from_str(content: &str) -> Result<Self> {
        serde_yaml::from_str(content)
            .map_err(|e| PluginError::config_error(&format!("Failed to parse manifest: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
