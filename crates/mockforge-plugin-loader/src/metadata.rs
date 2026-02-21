//! Plugin installation metadata
//!
//! This module tracks the installation source of each plugin, enabling updates
//! by refetching from the original source.

use crate::{LoaderResult, PluginLoaderError, PluginSource};
use mockforge_plugin_core::PluginId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Plugin installation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin ID
    pub plugin_id: PluginId,
    /// Original installation source
    pub source: PluginSource,
    /// Installation timestamp (Unix epoch seconds)
    pub installed_at: u64,
    /// Last update timestamp (Unix epoch seconds, None if never updated)
    pub updated_at: Option<u64>,
    /// Current installed version
    pub version: String,
}

impl PluginMetadata {
    /// Create new plugin metadata
    pub fn new(plugin_id: PluginId, source: PluginSource, version: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            plugin_id,
            source,
            installed_at: now,
            updated_at: None,
            version,
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
}

/// Plugin metadata store
pub struct MetadataStore {
    /// Path to metadata directory
    metadata_dir: PathBuf,
    /// In-memory cache
    cache: HashMap<PluginId, PluginMetadata>,
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(metadata_dir: PathBuf) -> Self {
        Self {
            metadata_dir,
            cache: HashMap::new(),
        }
    }

    /// Initialize the metadata store (create directory if needed)
    pub async fn init(&self) -> LoaderResult<()> {
        if !self.metadata_dir.exists() {
            fs::create_dir_all(&self.metadata_dir).await.map_err(|e| {
                PluginLoaderError::fs(format!("Failed to create metadata directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Load all metadata from disk
    pub async fn load(&mut self) -> LoaderResult<()> {
        self.init().await?;

        let mut entries = fs::read_dir(&self.metadata_dir).await.map_err(|e| {
            PluginLoaderError::fs(format!("Failed to read metadata directory: {}", e))
        })?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_metadata_file(&path).await {
                Ok(metadata) => {
                    self.cache.insert(metadata.plugin_id.clone(), metadata);
                }
                Err(e) => {
                    tracing::warn!("Failed to load metadata file {}: {}", path.display(), e);
                }
            }
        }

        tracing::info!("Loaded {} plugin metadata entries", self.cache.len());
        Ok(())
    }

    /// Load a single metadata file
    async fn load_metadata_file(&self, path: &Path) -> LoaderResult<PluginMetadata> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| PluginLoaderError::fs(format!("Failed to read metadata file: {}", e)))?;

        let metadata: PluginMetadata = serde_json::from_str(&content).map_err(|e| {
            PluginLoaderError::load(format!("Failed to parse metadata JSON: {}", e))
        })?;

        Ok(metadata)
    }

    /// Save metadata for a plugin
    pub async fn save(&mut self, metadata: PluginMetadata) -> LoaderResult<()> {
        self.init().await?;

        let file_path = self.metadata_file_path(&metadata.plugin_id);
        let json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| PluginLoaderError::load(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&file_path, json)
            .await
            .map_err(|e| PluginLoaderError::fs(format!("Failed to write metadata file: {}", e)))?;

        // Update cache
        self.cache.insert(metadata.plugin_id.clone(), metadata);

        Ok(())
    }

    /// Get metadata for a plugin
    pub fn get(&self, plugin_id: &PluginId) -> Option<&PluginMetadata> {
        self.cache.get(plugin_id)
    }

    /// Get mutable metadata for a plugin
    pub fn get_mut(&mut self, plugin_id: &PluginId) -> Option<&mut PluginMetadata> {
        self.cache.get_mut(plugin_id)
    }

    /// Remove metadata for a plugin
    pub async fn remove(&mut self, plugin_id: &PluginId) -> LoaderResult<()> {
        let file_path = self.metadata_file_path(plugin_id);

        if file_path.exists() {
            fs::remove_file(&file_path).await.map_err(|e| {
                PluginLoaderError::fs(format!("Failed to remove metadata file: {}", e))
            })?;
        }

        self.cache.remove(plugin_id);

        Ok(())
    }

    /// List all plugin IDs with metadata
    pub fn list(&self) -> Vec<PluginId> {
        self.cache.keys().cloned().collect()
    }

    /// Check if metadata exists for a plugin
    pub fn has(&self, plugin_id: &PluginId) -> bool {
        self.cache.contains_key(plugin_id)
    }

    /// Get the file path for a plugin's metadata
    fn metadata_file_path(&self, plugin_id: &PluginId) -> PathBuf {
        self.metadata_dir.join(format!("{}.json", plugin_id.as_str()))
    }
}

// Implement Serialize for PluginSource (if not already implemented)
impl Serialize for PluginSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        match self {
            PluginSource::Local(path) => {
                let mut state = serializer.serialize_struct("PluginSource", 2)?;
                state.serialize_field("type", "local")?;
                state.serialize_field("path", &path.display().to_string())?;
                state.end()
            }
            PluginSource::Url { url, checksum } => {
                let mut state = serializer.serialize_struct("PluginSource", 3)?;
                state.serialize_field("type", "url")?;
                state.serialize_field("url", url)?;
                state.serialize_field("checksum", checksum)?;
                state.end()
            }
            PluginSource::Git(git_source) => {
                let mut state = serializer.serialize_struct("PluginSource", 2)?;
                state.serialize_field("type", "git")?;
                state.serialize_field("source", &git_source.to_string())?;
                state.end()
            }
            PluginSource::Registry { name, version } => {
                let mut state = serializer.serialize_struct("PluginSource", 3)?;
                state.serialize_field("type", "registry")?;
                state.serialize_field("name", name)?;
                state.serialize_field("version", version)?;
                state.end()
            }
        }
    }
}

// Implement Deserialize for PluginSource (if not already implemented)
impl<'de> Deserialize<'de> for PluginSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PluginSourceVisitor;

        impl<'de> Visitor<'de> for PluginSourceVisitor {
            type Value = PluginSource;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a plugin source")
            }

            fn visit_map<M>(self, mut map: M) -> Result<PluginSource, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut source_type: Option<String> = None;
                let mut path: Option<String> = None;
                let mut url: Option<String> = None;
                let mut checksum: Option<Option<String>> = None;
                let mut source: Option<String> = None;
                let mut name: Option<String> = None;
                let mut version: Option<Option<String>> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => source_type = Some(map.next_value()?),
                        "path" => path = Some(map.next_value()?),
                        "url" => url = Some(map.next_value()?),
                        "checksum" => checksum = Some(map.next_value()?),
                        "source" => source = Some(map.next_value()?),
                        "name" => name = Some(map.next_value()?),
                        "version" => version = Some(map.next_value()?),
                        _ => {
                            let _: de::IgnoredAny = map.next_value()?;
                        }
                    }
                }

                let source_type = source_type.ok_or_else(|| de::Error::missing_field("type"))?;

                match source_type.as_str() {
                    "local" => {
                        let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
                        Ok(PluginSource::Local(PathBuf::from(path)))
                    }
                    "url" => {
                        let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
                        let checksum =
                            checksum.ok_or_else(|| de::Error::missing_field("checksum"))?;
                        Ok(PluginSource::Url { url, checksum })
                    }
                    "git" => {
                        let source_str =
                            source.ok_or_else(|| de::Error::missing_field("source"))?;
                        let git_source = crate::git::GitPluginSource::parse(&source_str)
                            .map_err(|e| de::Error::custom(format!("Invalid git source: {}", e)))?;
                        Ok(PluginSource::Git(git_source))
                    }
                    "registry" => {
                        let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                        let version = version.ok_or_else(|| de::Error::missing_field("version"))?;
                        Ok(PluginSource::Registry { name, version })
                    }
                    _ => Err(de::Error::custom(format!("Unknown source type: {}", source_type))),
                }
            }
        }

        deserializer.deserialize_struct(
            "PluginSource",
            &[
                "type", "path", "url", "checksum", "source", "name", "version",
            ],
            PluginSourceVisitor,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_metadata_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = MetadataStore::new(temp_dir.path().to_path_buf());
        store.init().await.unwrap();

        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_save_and_load_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = MetadataStore::new(temp_dir.path().to_path_buf());

        let plugin_id = PluginId::new("test-plugin");
        let source = PluginSource::Url {
            url: "https://example.com/plugin.zip".to_string(),
            checksum: None,
        };
        let metadata = PluginMetadata::new(plugin_id.clone(), source, "1.0.0".to_string());

        store.save(metadata.clone()).await.unwrap();
        assert!(store.has(&plugin_id));

        // Create a new store and load from disk
        let mut new_store = MetadataStore::new(temp_dir.path().to_path_buf());
        new_store.load().await.unwrap();

        let loaded = new_store.get(&plugin_id).unwrap();
        assert_eq!(loaded.plugin_id, plugin_id);
        assert_eq!(loaded.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_remove_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = MetadataStore::new(temp_dir.path().to_path_buf());

        let plugin_id = PluginId::new("test-plugin");
        let source = PluginSource::Local(PathBuf::from("/tmp/test"));
        let metadata = PluginMetadata::new(plugin_id.clone(), source, "1.0.0".to_string());

        store.save(metadata).await.unwrap();
        assert!(store.has(&plugin_id));

        store.remove(&plugin_id).await.unwrap();
        assert!(!store.has(&plugin_id));
    }

    #[tokio::test]
    async fn test_mark_updated() {
        let plugin_id = PluginId::new("test-plugin");
        let source = PluginSource::Local(PathBuf::from("/tmp/test"));
        let mut metadata = PluginMetadata::new(plugin_id, source, "1.0.0".to_string());

        assert!(metadata.updated_at.is_none());

        metadata.mark_updated("1.1.0".to_string());

        assert!(metadata.updated_at.is_some());
        assert_eq!(metadata.version, "1.1.0");
    }
}
