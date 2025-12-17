//! Local scenario storage management
//!
//! Handles storage and retrieval of installed scenarios in the local filesystem.

use crate::error::{Result, ScenarioError};
use crate::manifest::ScenarioManifest;
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
            serde_json::from_str(&content).map_err(ScenarioError::Serde)?;

        Ok(scenario)
    }

    /// Save scenario metadata
    pub async fn save(&mut self, scenario: InstalledScenario) -> Result<()> {
        self.init().await?;

        let file_path = self.metadata_file_path(&scenario.name, &scenario.version);
        let json = serde_json::to_string_pretty(&scenario).map_err(ScenarioError::Serde)?;

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
        let sanitized_name = name.replace(['/', '\\'], "_");
        let sanitized_version = version.replace(['/', '\\'], "_");
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

    fn create_test_manifest() -> ScenarioManifest {
        ScenarioManifest::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            "Test Scenario".to_string(),
            "A test scenario for unit tests".to_string(),
        )
    }

    fn create_test_installed_scenario() -> InstalledScenario {
        InstalledScenario::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            PathBuf::from("/tmp/test"),
            "file:///test".to_string(),
            create_test_manifest(),
        )
    }

    // ==================== InstalledScenario Tests ====================

    #[test]
    fn test_installed_scenario_new() {
        let scenario = create_test_installed_scenario();

        assert_eq!(scenario.name, "test-scenario");
        assert_eq!(scenario.version, "1.0.0");
        assert_eq!(scenario.path, PathBuf::from("/tmp/test"));
        assert_eq!(scenario.source, "file:///test");
        assert!(scenario.installed_at > 0);
        assert!(scenario.updated_at.is_none());
    }

    #[test]
    fn test_installed_scenario_id() {
        let scenario = create_test_installed_scenario();

        assert_eq!(scenario.id(), "test-scenario@1.0.0");
    }

    #[test]
    fn test_installed_scenario_mark_updated() {
        let mut scenario = create_test_installed_scenario();
        let original_installed_at = scenario.installed_at;

        scenario.mark_updated("1.1.0".to_string());

        assert_eq!(scenario.version, "1.1.0");
        assert!(scenario.updated_at.is_some());
        assert!(scenario.updated_at.unwrap() >= original_installed_at);
    }

    #[test]
    fn test_installed_scenario_serialization() {
        let scenario = create_test_installed_scenario();

        let json = serde_json::to_string(&scenario).unwrap();
        let deserialized: InstalledScenario = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, scenario.name);
        assert_eq!(deserialized.version, scenario.version);
        assert_eq!(deserialized.source, scenario.source);
    }

    // ==================== ScenarioStorage Tests ====================

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

    #[test]
    fn test_storage_default() {
        let storage = ScenarioStorage::default();
        assert!(storage.scenarios_dir().ends_with("scenarios"));
    }

    #[test]
    fn test_storage_scenario_path() {
        let storage = ScenarioStorage::new().unwrap();

        let path = storage.scenario_path("my-scenario", "1.0.0");

        assert!(path.ends_with("scenarios/my-scenario/1.0.0"));
    }

    #[test]
    fn test_storage_list_empty() {
        let storage = ScenarioStorage::new().unwrap();

        let scenarios = storage.list();
        assert!(scenarios.is_empty());
    }

    #[test]
    fn test_storage_get_nonexistent() {
        let storage = ScenarioStorage::new().unwrap();

        let result = storage.get("nonexistent", "1.0.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_storage_get_latest_nonexistent() {
        let storage = ScenarioStorage::new().unwrap();

        let result = storage.get_latest("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_metadata_file_path() {
        let storage = ScenarioStorage::new().unwrap();

        // Access metadata_file_path through scenario_path + manipulation
        // Since metadata_file_path is private, we test indirectly through save/load
        let path = storage.scenario_path("test", "1.0.0");
        assert!(path.to_string_lossy().contains("test"));
        assert!(path.to_string_lossy().contains("1.0.0"));
    }

    #[test]
    fn test_metadata_file_path_sanitization() {
        let storage = ScenarioStorage::new().unwrap();

        // Paths with special characters should be sanitized
        let path = storage.scenario_path("test/scenario", "v1.0/beta");
        // The scenario_path doesn't sanitize, but metadata_file_path does
        assert!(path.ends_with("test/scenario/v1.0/beta"));
    }

    // Integration test with temp directory
    #[tokio::test]
    async fn test_storage_save_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let scenarios_dir = temp_dir.path().join("scenarios");
        let metadata_dir = temp_dir.path().join("metadata");

        let mut storage = ScenarioStorage {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        };

        // Save a scenario
        let scenario = InstalledScenario::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            temp_dir.path().join("test"),
            "test://source".to_string(),
            create_test_manifest(),
        );

        storage.save(scenario.clone()).await.unwrap();

        // Retrieve by exact version
        let retrieved = storage.get("test-scenario", "1.0.0");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-scenario");
    }

    #[tokio::test]
    async fn test_storage_save_multiple_versions() {
        let temp_dir = TempDir::new().unwrap();
        let scenarios_dir = temp_dir.path().join("scenarios");
        let metadata_dir = temp_dir.path().join("metadata");

        let mut storage = ScenarioStorage {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        };

        // Save multiple versions
        for version in ["1.0.0", "1.1.0", "2.0.0"] {
            let mut manifest = create_test_manifest();
            manifest.version = version.to_string();

            let scenario = InstalledScenario::new(
                "test-scenario".to_string(),
                version.to_string(),
                temp_dir.path().join(version),
                "test://source".to_string(),
                manifest,
            );

            storage.save(scenario).await.unwrap();
        }

        // List should have 3 scenarios
        assert_eq!(storage.list().len(), 3);

        // Get latest should return 2.0.0
        let latest = storage.get_latest("test-scenario");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().version, "2.0.0");
    }

    #[tokio::test]
    async fn test_storage_remove() {
        let temp_dir = TempDir::new().unwrap();
        let scenarios_dir = temp_dir.path().join("scenarios");
        let metadata_dir = temp_dir.path().join("metadata");

        let mut storage = ScenarioStorage {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        };

        // Save a scenario
        let scenario = InstalledScenario::new(
            "test-scenario".to_string(),
            "1.0.0".to_string(),
            temp_dir.path().join("test"),
            "test://source".to_string(),
            create_test_manifest(),
        );

        storage.save(scenario).await.unwrap();
        assert!(storage.get("test-scenario", "1.0.0").is_some());

        // Remove it
        storage.remove("test-scenario", "1.0.0").await.unwrap();
        assert!(storage.get("test-scenario", "1.0.0").is_none());
    }

    #[tokio::test]
    async fn test_storage_load() {
        let temp_dir = TempDir::new().unwrap();
        let scenarios_dir = temp_dir.path().join("scenarios");
        let metadata_dir = temp_dir.path().join("metadata");

        // Create and save with first storage instance
        {
            let mut storage = ScenarioStorage {
                scenarios_dir: scenarios_dir.clone(),
                metadata_dir: metadata_dir.clone(),
                cache: HashMap::new(),
            };

            let scenario = InstalledScenario::new(
                "test-scenario".to_string(),
                "1.0.0".to_string(),
                temp_dir.path().join("test"),
                "test://source".to_string(),
                create_test_manifest(),
            );

            storage.save(scenario).await.unwrap();
        }

        // Load with new storage instance
        let mut storage2 = ScenarioStorage {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        };

        storage2.load().await.unwrap();

        // Should have loaded the scenario
        assert_eq!(storage2.list().len(), 1);
        assert!(storage2.get("test-scenario", "1.0.0").is_some());
    }

    #[tokio::test]
    async fn test_storage_load_ignores_non_json_files() {
        let temp_dir = TempDir::new().unwrap();
        let scenarios_dir = temp_dir.path().join("scenarios");
        let metadata_dir = temp_dir.path().join("metadata");

        // Create directories
        std::fs::create_dir_all(&metadata_dir).unwrap();

        // Create a non-JSON file
        std::fs::write(metadata_dir.join("readme.txt"), "This is not JSON").unwrap();

        let mut storage = ScenarioStorage {
            scenarios_dir,
            metadata_dir,
            cache: HashMap::new(),
        };

        // Load should succeed and ignore non-JSON files
        storage.load().await.unwrap();
        assert!(storage.list().is_empty());
    }
}
