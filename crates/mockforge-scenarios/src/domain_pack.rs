//! Domain-specific scenario packs
//!
//! Provides functionality for creating and managing domain-specific scenario packs
//! (e.g., e-commerce, fintech, IoT) that bundle multiple related scenarios together.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Domain pack manifest
///
/// Defines a collection of scenarios grouped by domain (e.g., e-commerce, fintech, IoT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPackManifest {
    /// Pack manifest version
    pub manifest_version: String,

    /// Pack name (e.g., "ecommerce-pack", "fintech-pack")
    pub name: String,

    /// Pack version
    pub version: String,

    /// Pack title
    pub title: String,

    /// Pack description
    pub description: String,

    /// Domain category (e.g., "ecommerce", "fintech", "iot")
    pub domain: String,

    /// Author of the pack
    pub author: String,

    /// List of scenarios included in this pack
    pub scenarios: Vec<PackScenario>,

    /// Optional tags for the pack
    #[serde(default)]
    pub tags: Vec<String>,

    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Scenario reference in a pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackScenario {
    /// Scenario name
    pub name: String,

    /// Scenario version (optional, defaults to latest)
    pub version: Option<String>,

    /// Scenario source (local path, URL, Git, or registry name)
    pub source: String,

    /// Whether this scenario is required or optional
    #[serde(default = "default_true")]
    pub required: bool,

    /// Optional description for this scenario in the pack context
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl DomainPackManifest {
    /// Create a new domain pack manifest
    pub fn new(
        name: String,
        version: String,
        title: String,
        description: String,
        domain: String,
        author: String,
    ) -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            name,
            version,
            title,
            description,
            domain,
            author,
            scenarios: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a scenario to the pack
    pub fn add_scenario(&mut self, scenario: PackScenario) {
        self.scenarios.push(scenario);
    }

    /// Load pack manifest from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| ScenarioError::Io(e))?;

        // Try to parse as YAML first, then JSON
        let manifest: DomainPackManifest = if path
            .as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "yaml" || ext == "yml")
            .unwrap_or(true)
        {
            serde_yaml::from_str(&content).map_err(|e| ScenarioError::Yaml(e))?
        } else {
            serde_json::from_str(&content).map_err(|e| ScenarioError::Serde(e))?
        };

        Ok(manifest)
    }

    /// Save pack manifest to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = if path
            .as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "yaml" || ext == "yml")
            .unwrap_or(true)
        {
            serde_yaml::to_string(self).map_err(|e| ScenarioError::Yaml(e))?
        } else {
            serde_json::to_string_pretty(self).map_err(|e| ScenarioError::Serde(e))?
        };

        std::fs::write(path.as_ref(), content).map_err(|e| ScenarioError::Io(e))?;

        Ok(())
    }
}

/// Domain pack information
///
/// Contains information about an installed or available domain pack.
#[derive(Debug, Clone)]
pub struct DomainPackInfo {
    /// Pack manifest
    pub manifest: DomainPackManifest,

    /// Path to pack directory (if installed)
    pub path: Option<PathBuf>,

    /// Whether all scenarios in the pack are installed
    pub all_scenarios_installed: bool,

    /// List of installed scenario names
    pub installed_scenarios: Vec<String>,

    /// List of missing scenario names
    pub missing_scenarios: Vec<String>,
}

impl DomainPackInfo {
    /// Create pack info from manifest
    pub fn from_manifest(manifest: DomainPackManifest, path: Option<PathBuf>) -> Self {
        Self {
            manifest,
            path,
            all_scenarios_installed: false,
            installed_scenarios: Vec::new(),
            missing_scenarios: Vec::new(),
        }
    }
}

/// Domain pack installer
///
/// Handles installation and management of domain packs.
pub struct DomainPackInstaller {
    /// Base directory for pack storage
    packs_dir: PathBuf,
}

impl DomainPackInstaller {
    /// Create a new domain pack installer
    pub fn new() -> Result<Self> {
        let packs_dir = dirs::data_dir()
            .ok_or_else(|| ScenarioError::Storage("Failed to get data directory".to_string()))?
            .join("mockforge")
            .join("packs");

        Ok(Self { packs_dir })
    }

    /// Initialize the pack installer (create directories)
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.packs_dir).map_err(|e| {
            ScenarioError::Storage(format!("Failed to create packs directory: {}", e))
        })?;
        Ok(())
    }

    /// List all installed packs
    pub fn list_installed(&self) -> Result<Vec<DomainPackInfo>> {
        if !self.packs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packs = Vec::new();

        for entry in std::fs::read_dir(&self.packs_dir).map_err(|e| ScenarioError::Io(e))? {
            let entry = entry.map_err(|e| ScenarioError::Io(e))?;
            let pack_path = entry.path();

            if pack_path.is_dir() {
                // Look for pack.yaml or pack.json
                let manifest_path = {
                    let yaml_path = pack_path.join("pack.yaml");
                    if yaml_path.exists() {
                        yaml_path
                    } else {
                        pack_path.join("pack.json")
                    }
                };

                if manifest_path.exists() {
                    match DomainPackManifest::from_file(&manifest_path) {
                        Ok(manifest) => {
                            packs.push(DomainPackInfo::from_manifest(manifest, Some(pack_path)));
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to load pack manifest from {}: {}",
                                pack_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(packs)
    }

    /// Get pack info by name
    pub fn get_pack(&self, name: &str) -> Result<Option<DomainPackInfo>> {
        let packs = self.list_installed()?;
        Ok(packs.into_iter().find(|p| p.manifest.name == name))
    }

    /// Install a pack from a manifest file
    pub fn install_from_manifest(&self, manifest_path: &Path) -> Result<DomainPackInfo> {
        let manifest = DomainPackManifest::from_file(manifest_path)?;

        // Create pack directory
        let pack_dir = self.packs_dir.join(&manifest.name);
        std::fs::create_dir_all(&pack_dir).map_err(|e| {
            ScenarioError::Storage(format!("Failed to create pack directory: {}", e))
        })?;

        // Copy manifest to pack directory
        let pack_manifest_path = pack_dir.join("pack.yaml");
        manifest.to_file(&pack_manifest_path)?;

        Ok(DomainPackInfo::from_manifest(manifest, Some(pack_dir)))
    }
}

impl Default for DomainPackInstaller {
    fn default() -> Self {
        Self::new().expect("Failed to create DomainPackInstaller")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_domain_pack_manifest() {
        let mut pack = DomainPackManifest::new(
            "ecommerce-pack".to_string(),
            "1.0.0".to_string(),
            "E-commerce Pack".to_string(),
            "A pack of e-commerce scenarios".to_string(),
            "ecommerce".to_string(),
            "test-author".to_string(),
        );

        pack.add_scenario(PackScenario {
            name: "product-catalog".to_string(),
            version: Some("1.0.0".to_string()),
            source: "product-catalog@1.0.0".to_string(),
            required: true,
            description: Some("Product catalog scenario".to_string()),
        });

        assert_eq!(pack.scenarios.len(), 1);
        assert_eq!(pack.scenarios[0].name, "product-catalog");
    }

    #[test]
    fn test_pack_manifest_serialization() {
        let mut pack = DomainPackManifest::new(
            "test-pack".to_string(),
            "1.0.0".to_string(),
            "Test Pack".to_string(),
            "Test description".to_string(),
            "test".to_string(),
            "test-author".to_string(),
        );

        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("pack.yaml");

        pack.to_file(&manifest_path).unwrap();
        let loaded = DomainPackManifest::from_file(&manifest_path).unwrap();

        assert_eq!(pack.name, loaded.name);
        assert_eq!(pack.version, loaded.version);
    }
}
