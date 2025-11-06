//! Scenario package format and validation
//!
//! Handles the structure and validation of scenario packages, including
//! file organization and package integrity checks.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
use std::path::{Path, PathBuf};

/// Scenario package structure
///
/// Represents a complete scenario package with its manifest and files.
#[derive(Debug, Clone)]
pub struct ScenarioPackage {
    /// Package root directory
    pub root: PathBuf,

    /// Scenario manifest
    pub manifest: ScenarioManifest,

    /// Package files (relative to root)
    pub files: Vec<PathBuf>,
}

impl ScenarioPackage {
    /// Load a scenario package from a directory
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let root = path.as_ref().to_path_buf();

        // Load manifest
        let manifest_path = root.join("scenario.yaml");
        if !manifest_path.exists() {
            return Err(ScenarioError::InvalidManifest(format!(
                "Manifest not found: {}",
                manifest_path.display()
            )));
        }

        let manifest = ScenarioManifest::from_file(&manifest_path)?;

        // Discover package files
        let files = Self::discover_files(&root)?;

        Ok(Self {
            root,
            manifest,
            files,
        })
    }

    /// Discover all files in the package
    fn discover_files(root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !root.exists() {
            return Err(ScenarioError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Package directory not found: {}", root.display()),
            )));
        }

        Self::collect_files(root, root, &mut files)?;

        Ok(files)
    }

    /// Recursively collect files from directory
    fn collect_files(
        root: &Path,
        current: &Path,
        files: &mut Vec<PathBuf>,
    ) -> Result<()> {
        if !current.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(current)
            .map_err(|e| ScenarioError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| ScenarioError::Io(e))?;
            let path = entry.path();

            // Skip hidden files and directories
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            // Skip common ignore patterns
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if matches!(name, "target" | "node_modules" | ".git" | ".DS_Store") {
                continue;
            }

            if path.is_dir() {
                Self::collect_files(root, &path, files)?;
            } else {
                // Store relative path
                let relative = path.strip_prefix(root)
                    .map_err(|e| ScenarioError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Failed to compute relative path: {}", e),
                    )))?;
                files.push(relative.to_path_buf());
            }
        }

        Ok(())
    }

    /// Validate the package structure
    pub fn validate(&self) -> Result<PackageValidation> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate manifest
        if let Err(e) = self.manifest.validate() {
            errors.push(format!("Manifest validation failed: {}", e));
        }

        // Check required files exist
        let required_files = ["scenario.yaml"];
        for file in &required_files {
            let path = self.root.join(file);
            if !path.exists() {
                errors.push(format!("Required file missing: {}", file));
            }
        }

        // Check if files listed in manifest exist
        for file in &self.manifest.files {
            let path = self.root.join(file);
            if !path.exists() {
                warnings.push(format!("File listed in manifest not found: {}", file));
            }
        }

        // Check for config.yaml (recommended)
        if !self.root.join("config.yaml").exists() {
            warnings.push("config.yaml not found (recommended)".to_string());
        }

        // Check for README.md (recommended)
        if !self.root.join("README.md").exists() {
            warnings.push("README.md not found (recommended)".to_string());
        }

        let is_valid = errors.is_empty();

        Ok(PackageValidation {
            is_valid,
            errors,
            warnings,
        })
    }

    /// Get the config file path if it exists
    pub fn config_path(&self) -> Option<PathBuf> {
        let path = self.root.join("config.yaml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Get the OpenAPI spec path if it exists
    pub fn openapi_path(&self) -> Option<PathBuf> {
        let path = self.root.join("openapi.json");
        if path.exists() {
            Some(path)
        } else {
            let path = self.root.join("openapi.yaml");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        }
    }

    /// Get the fixtures directory path if it exists
    pub fn fixtures_path(&self) -> Option<PathBuf> {
        let path = self.root.join("fixtures");
        if path.exists() && path.is_dir() {
            Some(path)
        } else {
            None
        }
    }

    /// Get the examples directory path if it exists
    pub fn examples_path(&self) -> Option<PathBuf> {
        let path = self.root.join("examples");
        if path.exists() && path.is_dir() {
            Some(path)
        } else {
            None
        }
    }
}

/// Package validation result
#[derive(Debug, Clone)]
pub struct PackageValidation {
    /// Whether the package is valid
    pub is_valid: bool,

    /// Validation errors
    pub errors: Vec<String>,

    /// Validation warnings
    pub warnings: Vec<String>,
}

impl PackageValidation {
    /// Check if package is valid
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get all validation messages
    pub fn messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        messages.extend(self.errors.iter().map(|e| format!("ERROR: {}", e)));
        messages.extend(self.warnings.iter().map(|w| format!("WARNING: {}", w)));
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_package_validation() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create minimal valid package
        std::fs::write(root.join("scenario.yaml"), r#"
manifest_version: "1.0"
name: test-scenario
version: "1.0.0"
title: Test Scenario
description: A test scenario
author: test
category: other
compatibility:
  min_version: "0.2.0"
files: []
"#).unwrap();

        let package = ScenarioPackage::from_directory(root).unwrap();
        let validation = package.validate().unwrap();

        // Should be valid (warnings are ok)
        assert!(validation.is_valid);
    }
}
