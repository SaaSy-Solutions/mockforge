//! Local scenario storage management
//!
//! Handles storage and retrieval of installed scenarios in the local filesystem.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
use crate::source::ScenarioSource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Information about an installed scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledScenario {
    /// Scenario name
    pub name: String,

    /// Scenario version
    pub version: String,

    /// Installation path
    pub path: PathBuf,

    /// Original installation source
    pub source: String,

    /// Installation timestamp (Unix epoch seconds)
    pub installed_at: u64,

    /// Last update timestamp (Unix epoch seconds, None if never updated)
    pub updated_at: Option<u64>,

    /// Scenario manifest
    pub manifest: ScenarioManifest,
}

impl InstalledScenario {
    /// Create new installed scenario info
    pub fn new(
        name: String,
        version: String,
        path: PathBuf,
        source: String,
        manifest: ScenarioManifest,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            name,
            version,
            path,
            source,
            installed_at: now,
            updated_at: None,
            manifest,
        }
    }

    /// Mark as updated
    pub fn mark_updated(&mut self, new_version: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.updated_at = Some(now);
        self.version = new_version;
    }

    /// Get scenario ID (name@version)
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
}

/// Scenario storage manager
pub struct ScenarioStorage {
    /// Path to scenarios directory
    scenarios_dir: PathBuf,

    /// Path to metadata directory
    metadata_dir: PathBuf,

    /// In-memory cache of installed scenarios
    cache: HashMap<String, InstalledScenario>,
}

impl ScenarioStorage {
    /// Create a new scenario storage
    pub fn new() -> Result<Self> {
        let base_dir = shellexpand::tilde("~/.mockforge");
        let base_path = PathBuf::from(base_dir.as_ref());

        let scenarios_dir = base_path.join("scenarios");
        let metadata_dir = base_path.join("scenario-metadata");

        Ok(Self {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        })
    }

    /// Initialize the storage (create directories if needed)
    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.scenarios_dir).await.map_err(|e| {
            ScenarioError::Storage(format!("Failed to create scenarios directory: {}", e))
        })?;

        fs::create_dir_all(&self.metadata_dir).await.map_err(|e| {
            ScenarioError::Storage(format!("Failed to create metadata directory: {}", e))
        })?;

        Ok(())
    }

    /// Load all installed scenarios from disk
    pub async fn load(&mut self) -> Result<()> {
        self.init().await?;

        // Load metadata files
        let mut entries = fs::read_dir(&self.metadata_dir).await.map_err(|e| {
            ScenarioError::Storage(format!("Failed to read metadata directory: {}", e))
        })?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_metadata_file(&path).await {
                Ok(scenario) => {
                    let id = scenario.id();
                    self.cache.insert(id, scenario);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load scenario metadata from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        tracing::info!("Loaded {} installed scenarios", self.cache.len());
        Ok(())
    }

    /// Load a single metadata file
    async fn load_metadata_file(&self, path: &Path) -> Result<InstalledScenario> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| ScenarioError::Storage(format!("Failed to read metadata file: {}", e)))?;

        let scenario: InstalledScenario =
            serde_json::from_str(&content).map_err(|e| ScenarioError::Serde(e))?;

        Ok(scenario)
    }

    /// Save scenario metadata
    pub async fn save(&mut self, scenario: InstalledScenario) -> Result<()> {
        self.init().await?;

        let file_path = self.metadata_file_path(&scenario.name, &scenario.version);
        let json = serde_json::to_string_pretty(&scenario).map_err(|e| ScenarioError::Serde(e))?;

        fs::write(&file_path, json)
            .await
            .map_err(|e| ScenarioError::Storage(format!("Failed to write metadata file: {}", e)))?;

        // Update cache
        let id = scenario.id();
        self.cache.insert(id, scenario);

        Ok(())
    }

    /// Get scenario by name and version
    pub fn get(&self, name: &str, version: &str) -> Option<&InstalledScenario> {
        let id = format!("{}@{}", name, version);
        self.cache.get(&id)
    }

    /// Get scenario by name (latest version)
    pub fn get_latest(&self, name: &str) -> Option<&InstalledScenario> {
        self.cache.values().filter(|s| s.name == name).max_by_key(|s| &s.version)
    }

    /// List all installed scenarios
    pub fn list(&self) -> Vec<&InstalledScenario> {
        self.cache.values().collect()
    }

    /// Remove scenario metadata
    pub async fn remove(&mut self, name: &str, version: &str) -> Result<()> {
        let id = format!("{}@{}", name, version);

        // Remove metadata file
        let file_path = self.metadata_file_path(name, version);
        if file_path.exists() {
            fs::remove_file(&file_path).await.map_err(|e| {
                ScenarioError::Storage(format!("Failed to remove metadata file: {}", e))
            })?;
        }

        // Remove from cache
        self.cache.remove(&id);

        Ok(())
    }

    /// Get the installation path for a scenario
    pub fn scenario_path(&self, name: &str, version: &str) -> PathBuf {
        self.scenarios_dir.join(name).join(version)
    }

    /// Get metadata file path
    fn metadata_file_path(&self, name: &str, version: &str) -> PathBuf {
        // Sanitize name and version for filename
        let sanitized_name = name.replace('/', "_").replace('\\', "_");
        let sanitized_version = version.replace('/', "_").replace('\\', "_");
        self.metadata_dir.join(format!("{}_{}.json", sanitized_name, sanitized_version))
    }

    /// Get scenarios directory
    pub fn scenarios_dir(&self) -> &Path {
        &self.scenarios_dir
    }
}

impl Default for ScenarioStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create scenario storage")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_creation() {
        let storage = ScenarioStorage::new().unwrap();
        assert!(storage.scenarios_dir().ends_with("scenarios"));
    }

    #[tokio::test]
    async fn test_storage_init() {
        let storage = ScenarioStorage::new().unwrap();
        storage.init().await.unwrap();
    }
}
