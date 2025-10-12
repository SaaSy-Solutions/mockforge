//! Plugin storage backend

use crate::{RegistryEntry, Result, VersionEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Storage backend for plugin registry
pub struct RegistryStorage {
    /// Base directory for storage
    base_dir: PathBuf,

    /// In-memory index cache
    index: HashMap<String, RegistryEntry>,
}

impl RegistryStorage {
    /// Create a new storage backend
    pub async fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir).await?;

        let mut storage = Self {
            base_dir,
            index: HashMap::new(),
        };

        storage.load_index().await?;
        Ok(storage)
    }

    /// Get plugin entry by name
    pub fn get(&self, name: &str) -> Option<&RegistryEntry> {
        self.index.get(name)
    }

    /// Get plugin entry with specific version
    pub fn get_version(&self, name: &str, version: &str) -> Option<&VersionEntry> {
        self.index.get(name)?.versions.iter().find(|v| v.version == version)
    }

    /// Add or update plugin entry
    pub async fn put(&mut self, entry: RegistryEntry) -> Result<()> {
        let name = entry.name.clone();
        self.index.insert(name.clone(), entry.clone());
        self.save_entry(&entry).await?;
        self.save_index().await?;
        Ok(())
    }

    /// Remove plugin entry
    pub async fn remove(&mut self, name: &str) -> Result<()> {
        self.index.remove(name);
        let path = self.entry_path(name);
        if path.exists() {
            fs::remove_file(path).await?;
        }
        self.save_index().await?;
        Ok(())
    }

    /// Search plugins
    pub fn search(&self, query: Option<&str>, tags: &[String]) -> Vec<&RegistryEntry> {
        self.index
            .values()
            .filter(|entry| {
                // Filter by query
                if let Some(q) = query {
                    let q = q.to_lowercase();
                    if !entry.name.to_lowercase().contains(&q)
                        && !entry.description.to_lowercase().contains(&q)
                    {
                        return false;
                    }
                }

                // Filter by tags
                if !tags.is_empty() && !tags.iter().any(|tag| entry.tags.contains(tag)) {
                    return false;
                }

                true
            })
            .collect()
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&RegistryEntry> {
        self.index.values().collect()
    }

    /// Get index file path
    fn index_path(&self) -> PathBuf {
        self.base_dir.join("index.json")
    }

    /// Get entry file path
    fn entry_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", name))
    }

    /// Load index from disk
    async fn load_index(&mut self) -> Result<()> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(());
        }

        let contents = fs::read_to_string(path).await?;
        let names: Vec<String> = serde_json::from_str(&contents)?;

        for name in names {
            if let Ok(entry) = self.load_entry(&name).await {
                self.index.insert(name, entry);
            }
        }

        Ok(())
    }

    /// Save index to disk
    async fn save_index(&self) -> Result<()> {
        let names: Vec<String> = self.index.keys().cloned().collect();
        let contents = serde_json::to_string_pretty(&names)?;
        fs::write(self.index_path(), contents).await?;
        Ok(())
    }

    /// Load entry from disk
    async fn load_entry(&self, name: &str) -> Result<RegistryEntry> {
        let path = self.entry_path(name);
        let contents = fs::read_to_string(path).await?;
        let entry = serde_json::from_str(&contents)?;
        Ok(entry)
    }

    /// Save entry to disk
    async fn save_entry(&self, entry: &RegistryEntry) -> Result<()> {
        let path = self.entry_path(&entry.name);
        let contents = serde_json::to_string_pretty(entry)?;
        fs::write(path, contents).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthorInfo, PluginCategory};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_crud() {
        let dir = tempdir().unwrap();
        let mut storage = RegistryStorage::new(dir.path()).await.unwrap();

        let entry = RegistryEntry {
            name: "test-plugin".to_string(),
            description: "Test".to_string(),
            version: "1.0.0".to_string(),
            versions: vec![],
            author: AuthorInfo {
                name: "Test".to_string(),
                email: None,
                url: None,
            },
            tags: vec!["test".to_string()],
            category: PluginCategory::Auth,
            downloads: 0,
            rating: 0.0,
            reviews_count: 0,
            repository: None,
            homepage: None,
            license: "MIT".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        // Create
        storage.put(entry.clone()).await.unwrap();

        // Read
        let retrieved = storage.get("test-plugin").unwrap();
        assert_eq!(retrieved.name, "test-plugin");

        // Search
        let results = storage.search(Some("test"), &[]);
        assert_eq!(results.len(), 1);

        // Delete
        storage.remove("test-plugin").await.unwrap();
        assert!(storage.get("test-plugin").is_none());
    }
}
