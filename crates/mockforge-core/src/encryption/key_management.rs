//! Key generation, storage, and lifecycle management
//!
//! This module provides comprehensive key management functionality including
//! key generation, storage, rotation, and secure key lifecycle management.

use crate::encryption::algorithms::{EncryptionAlgorithm, EncryptionKey};
use crate::encryption::derivation::KeyDerivationManager;
use crate::encryption::errors::{EncryptionError, EncryptionResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Key identifier for lookup and management
pub type KeyId = String;

/// Key storage interface for different key storage backends
pub trait KeyStorage: Send + Sync {
    /// Store an encrypted key
    fn store_key(&mut self, key_id: &KeyId, encrypted_key: &[u8]) -> EncryptionResult<()>;

    /// Retrieve an encrypted key
    fn retrieve_key(&self, key_id: &KeyId) -> EncryptionResult<Vec<u8>>;

    /// Delete a key
    fn delete_key(&mut self, key_id: &KeyId) -> EncryptionResult<()>;

    /// Check if a key exists
    fn key_exists(&self, key_id: &KeyId) -> bool;

    /// List all key IDs
    fn list_keys(&self) -> Vec<KeyId>;
}

/// In-memory key storage implementation
#[derive(Debug, Clone)]
pub struct MemoryKeyStorage {
    keys: HashMap<KeyId, Vec<u8>>,
}

impl MemoryKeyStorage {
    /// Create a new in-memory key storage
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

impl KeyStorage for MemoryKeyStorage {
    fn store_key(&mut self, key_id: &KeyId, encrypted_key: &[u8]) -> EncryptionResult<()> {
        self.keys.insert(key_id.clone(), encrypted_key.to_vec());
        Ok(())
    }

    fn retrieve_key(&self, key_id: &KeyId) -> EncryptionResult<Vec<u8>> {
        self.keys
            .get(key_id)
            .cloned()
            .ok_or_else(|| EncryptionError::key_not_found(key_id.clone()))
    }

    fn delete_key(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        self.keys.remove(key_id);
        Ok(())
    }

    fn key_exists(&self, key_id: &KeyId) -> bool {
        self.keys.contains_key(key_id)
    }

    fn list_keys(&self) -> Vec<KeyId> {
        self.keys.keys().cloned().collect()
    }
}

impl Default for MemoryKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// File-based key storage implementation
#[derive(Debug, Clone)]
pub struct FileKeyStorage {
    base_path: std::path::PathBuf,
}

impl FileKeyStorage {
    /// Create a new file-based key storage with default path
    pub fn new() -> Self {
        Self {
            base_path: std::path::PathBuf::from(".mockforge/keys"),
        }
    }

    /// Create a new file-based key storage with custom base path
    pub fn with_path<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            base_path: path.as_ref().to_path_buf(),
        }
    }

    /// Get the file path for a key
    fn key_file_path(&self, key_id: &KeyId) -> std::path::PathBuf {
        self.base_path.join(format!("{}.key", key_id))
    }

    /// Ensure the base directory exists (blocking)
    fn ensure_base_dir(&self) -> EncryptionResult<()> {
        if !self.base_path.exists() {
            std::fs::create_dir_all(&self.base_path).map_err(|e| {
                EncryptionError::generic(format!("Failed to create key storage directory: {}", e))
            })?;
        }
        Ok(())
    }

    /// Ensure the base directory exists (async)
    async fn ensure_base_dir_async(&self) -> EncryptionResult<()> {
        let base_path = self.base_path.clone();
        tokio::task::spawn_blocking(move || {
            if !base_path.exists() {
                std::fs::create_dir_all(&base_path).map_err(|e| {
                    EncryptionError::generic(format!(
                        "Failed to create key storage directory: {}",
                        e
                    ))
                })
            } else {
                Ok(())
            }
        })
        .await
        .map_err(|e| EncryptionError::generic(format!("Task join error: {}", e)))?
    }

    /// Store a key asynchronously (non-blocking)
    pub async fn store_key_async(
        &mut self,
        key_id: &KeyId,
        encrypted_key: &[u8],
    ) -> EncryptionResult<()> {
        self.ensure_base_dir_async().await?;
        let file_path = self.key_file_path(key_id);
        let key_id = key_id.clone();
        let encrypted_key = encrypted_key.to_vec();

        tokio::task::spawn_blocking(move || {
            std::fs::write(&file_path, encrypted_key).map_err(|e| {
                EncryptionError::generic(format!("Failed to store key {}: {}", key_id, e))
            })
        })
        .await
        .map_err(|e| EncryptionError::generic(format!("Task join error: {}", e)))?
    }

    /// Retrieve a key asynchronously (non-blocking)
    pub async fn retrieve_key_async(&self, key_id: &KeyId) -> EncryptionResult<Vec<u8>> {
        let file_path = self.key_file_path(key_id);
        let key_id = key_id.clone();

        tokio::task::spawn_blocking(move || {
            std::fs::read(&file_path).map_err(|_| EncryptionError::key_not_found(key_id))
        })
        .await
        .map_err(|e| EncryptionError::generic(format!("Task join error: {}", e)))?
    }

    /// Delete a key asynchronously (non-blocking)
    pub async fn delete_key_async(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        let file_path = self.key_file_path(key_id);
        let key_id = key_id.clone();

        tokio::task::spawn_blocking(move || match std::fs::remove_file(&file_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                Err(EncryptionError::generic(format!("Failed to delete key {}: {}", key_id, e)))
            }
        })
        .await
        .map_err(|e| EncryptionError::generic(format!("Task join error: {}", e)))?
    }
}

impl KeyStorage for FileKeyStorage {
    fn store_key(&mut self, key_id: &KeyId, encrypted_key: &[u8]) -> EncryptionResult<()> {
        self.ensure_base_dir()?;
        let file_path = self.key_file_path(key_id);
        std::fs::write(&file_path, encrypted_key)
            .map_err(|e| EncryptionError::generic(format!("Failed to store key {}: {}", key_id, e)))
    }

    fn retrieve_key(&self, key_id: &KeyId) -> EncryptionResult<Vec<u8>> {
        let file_path = self.key_file_path(key_id);
        std::fs::read(&file_path).map_err(|_| EncryptionError::key_not_found(key_id.clone()))
    }

    fn delete_key(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        let file_path = self.key_file_path(key_id);
        match std::fs::remove_file(&file_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), // Key doesn't exist, consider it deleted
            Err(e) => {
                Err(EncryptionError::generic(format!("Failed to delete key {}: {}", key_id, e)))
            }
        }
    }

    fn key_exists(&self, key_id: &KeyId) -> bool {
        self.key_file_path(key_id).exists()
    }

    fn list_keys(&self) -> Vec<KeyId> {
        if !self.base_path.exists() {
            return Vec::new();
        }

        std::fs::read_dir(&self.base_path)
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry.ok().and_then(|e| {
                            e.path()
                                .file_stem()
                                .and_then(|stem| stem.to_str())
                                .map(|s| s.to_string())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for FileKeyStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Key metadata for tracking key properties and lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key identifier
    pub key_id: KeyId,
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Key creation timestamp
    pub created_at: DateTime<Utc>,
    /// Key last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,
    /// Key expiration timestamp (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Key version for rotation
    pub version: u32,
    /// Key purpose/description
    pub purpose: String,
    /// Whether the key is currently active
    pub is_active: bool,
    /// Usage count for analytics
    pub usage_count: u64,
}

/// Key store for managing encryption keys
pub struct KeyStore {
    /// Key storage backend
    storage: Box<dyn KeyStorage + Send + Sync>,
    /// Key metadata tracking
    metadata: HashMap<KeyId, KeyMetadata>,
    /// Master key for encrypting stored keys
    master_key: Option<EncryptionKey>,
    /// Key derivation manager
    derivation_manager: Arc<KeyDerivationManager>,
}

impl std::fmt::Debug for KeyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyStore")
            .field("storage", &"KeyStorage")
            .field("metadata", &self.metadata)
            .field("master_key", &self.master_key.is_some())
            .field("derivation_manager", &self.derivation_manager)
            .finish()
    }
}

impl KeyStore {
    /// Create a new key store with memory storage
    pub fn new() -> Self {
        Self {
            storage: Box::new(MemoryKeyStorage::new()),
            metadata: HashMap::new(),
            master_key: None,
            derivation_manager: Arc::new(KeyDerivationManager::new()),
        }
    }

    /// Create a key store with custom storage backend
    pub fn with_storage(storage: Box<dyn KeyStorage + Send + Sync>) -> Self {
        Self {
            storage,
            metadata: HashMap::new(),
            master_key: None,
            derivation_manager: Arc::new(KeyDerivationManager::new()),
        }
    }

    /// Initialize the master key for the key store
    pub fn initialize_master_key(&mut self, master_password: &str) -> EncryptionResult<()> {
        let master_key = self.derivation_manager.derive_master_key(master_password)?;

        self.master_key = Some(master_key);
        Ok(())
    }

    /// Generate a new encryption key
    pub fn generate_key(
        &mut self,
        key_id: KeyId,
        algorithm: EncryptionAlgorithm,
        purpose: String,
    ) -> EncryptionResult<()> {
        if self.storage.key_exists(&key_id) {
            return Err(EncryptionError::generic(format!("Key {} already exists", key_id)));
        }

        // Generate the key
        let key = EncryptionKey::generate(algorithm.clone())?;

        // Store the key encrypted with master key
        let encrypted_key = if let Some(master_key) = &self.master_key {
            let key_data = key.to_base64();
            let encrypted = crate::encryption::algorithms::EncryptionEngine::encrypt(
                master_key,
                key_data.as_bytes(),
                None,
            )?;
            serde_json::to_vec(&encrypted)
                .map_err(|e| EncryptionError::serialization_error(e.to_string()))?
        } else {
            // Store unencrypted (development/testing only)
            key.to_base64().into_bytes()
        };

        // Store the key
        self.storage.store_key(&key_id, &encrypted_key)?;

        // Create metadata
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            algorithm,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            version: 1,
            purpose,
            is_active: true,
            usage_count: 0,
        };

        self.metadata.insert(key_id, metadata);
        Ok(())
    }

    /// Retrieve and decrypt a key
    pub fn get_key(&self, key_id: &KeyId) -> EncryptionResult<EncryptionKey> {
        // Get encrypted key data
        let encrypted_data: Vec<u8> = self.storage.retrieve_key(key_id)?;

        // Deserialize encrypted data
        let encrypted: crate::encryption::algorithms::EncryptedData =
            serde_json::from_slice(&encrypted_data)
                .map_err(|e| EncryptionError::serialization_error(e.to_string()))?;

        // Decrypt the key
        let master_key = self
            .master_key
            .as_ref()
            .ok_or_else(|| EncryptionError::key_store_error("Master key not initialized"))?;

        let decrypted_bytes =
            crate::encryption::algorithms::EncryptionEngine::decrypt(master_key, &encrypted)?;

        let key_data = String::from_utf8(decrypted_bytes)
            .map_err(|e| EncryptionError::serialization_error(e.to_string()))?;

        // Get metadata to determine algorithm
        let metadata = self
            .metadata
            .get(key_id)
            .ok_or_else(|| EncryptionError::key_not_found(key_id.clone()))?;

        EncryptionKey::from_base64(&key_data, metadata.algorithm.clone())
    }

    /// Update key usage statistics
    pub fn record_key_usage(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        if let Some(metadata) = self.metadata.get_mut(key_id) {
            metadata.last_used_at = Some(Utc::now());
            metadata.usage_count += 1;
        }
        Ok(())
    }

    /// Rotate a key (generate new version)
    pub fn rotate_key(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        let old_metadata = self
            .metadata
            .get(key_id)
            .ok_or_else(|| EncryptionError::key_not_found(key_id.clone()))?
            .clone();

        if !old_metadata.is_active {
            return Err(EncryptionError::generic(format!("Key {} is not active", key_id)));
        }

        // Generate new key with same algorithm
        let new_key = EncryptionKey::generate(old_metadata.algorithm.clone())?;

        // Store new key
        let encrypted_key = if let Some(master_key) = &self.master_key {
            let key_data = new_key.to_base64();
            let encrypted = crate::encryption::algorithms::EncryptionEngine::encrypt(
                master_key,
                key_data.as_bytes(),
                None,
            )?;
            serde_json::to_vec(&encrypted)
                .map_err(|e| EncryptionError::serialization_error(e.to_string()))?
        } else {
            return Err(EncryptionError::key_store_error("Master key not initialized"));
        };

        self.storage.store_key(key_id, &encrypted_key)?;

        // Update metadata
        if let Some(metadata) = self.metadata.get_mut(key_id) {
            metadata.version += 1;
            metadata.created_at = Utc::now(); // Update creation time for new version
        }

        Ok(())
    }

    /// Delete a key
    pub fn delete_key(&mut self, key_id: &KeyId) -> EncryptionResult<()> {
        self.storage.delete_key(key_id)?;
        self.metadata.remove(key_id);
        Ok(())
    }

    /// List all keys with metadata
    pub fn list_keys(&self) -> Vec<&KeyMetadata> {
        self.metadata.values().collect()
    }

    /// Get key metadata
    pub fn get_key_metadata(&self, key_id: &KeyId) -> Option<&KeyMetadata> {
        self.metadata.get(key_id)
    }

    /// Check if a key exists and is active
    pub fn key_exists(&self, key_id: &KeyId) -> bool {
        self.storage.key_exists(key_id)
            && self.metadata.get(key_id).map(|meta| meta.is_active).unwrap_or(false)
    }

    /// Set key expiration
    pub fn set_key_expiration(
        &mut self,
        key_id: &KeyId,
        expires_at: DateTime<Utc>,
    ) -> EncryptionResult<()> {
        if let Some(metadata) = self.metadata.get_mut(key_id) {
            metadata.expires_at = Some(expires_at);
            Ok(())
        } else {
            Err(EncryptionError::key_not_found(key_id.clone()))
        }
    }

    /// Clean up expired keys
    pub fn cleanup_expired_keys(&mut self) -> EncryptionResult<Vec<KeyId>> {
        let now = Utc::now();

        // Find expired keys
        let expired_key_ids: Vec<KeyId> = self
            .metadata
            .iter()
            .filter_map(|(key_id, metadata)| {
                if let Some(expires_at) = metadata.expires_at {
                    if now > expires_at && metadata.is_active {
                        Some(key_id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Mark expired keys as inactive
        for key_id in &expired_key_ids {
            if let Some(metadata) = self.metadata.get_mut(key_id) {
                metadata.is_active = false;
            }
        }

        let expired_keys = expired_key_ids;
        Ok(expired_keys)
    }

    /// Get key statistics
    pub fn get_statistics(&self) -> KeyStoreStatistics {
        let total_keys = self.metadata.len();
        let active_keys = self.metadata.values().filter(|meta| meta.is_active).count();
        let expired_keys = self
            .metadata
            .values()
            .filter(|meta| meta.expires_at.is_some_and(|exp| Utc::now() > exp))
            .count();

        let total_usage: u64 = self.metadata.values().map(|meta| meta.usage_count).sum();

        KeyStoreStatistics {
            total_keys,
            active_keys,
            expired_keys,
            total_usage,
            oldest_key: self
                .metadata
                .values()
                .min_by_key(|meta| meta.created_at)
                .map(|meta| meta.created_at),
            newest_key: self
                .metadata
                .values()
                .max_by_key(|meta| meta.created_at)
                .map(|meta| meta.created_at),
        }
    }

    /// Export key metadata for backup
    pub fn export_metadata(&self) -> EncryptionResult<String> {
        let metadata: Vec<&KeyMetadata> = self.metadata.values().collect();
        serde_json::to_string_pretty(&metadata)
            .map_err(|e| EncryptionError::serialization_error(e.to_string()))
    }

    /// Import key metadata
    pub fn import_metadata(&mut self, metadata_json: &str) -> EncryptionResult<()> {
        let imported_metadata: Vec<KeyMetadata> = serde_json::from_str(metadata_json)
            .map_err(|e| EncryptionError::serialization_error(e.to_string()))?;

        for metadata in imported_metadata {
            self.metadata.insert(metadata.key_id.clone(), metadata);
        }

        Ok(())
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Global key store instance
#[allow(dead_code)]
static GLOBAL_KEY_STORE: OnceLock<Arc<RwLock<KeyStore>>> = OnceLock::new();

/// Initialize the global key store
#[allow(dead_code)]
pub fn init_key_store() -> &'static Arc<RwLock<KeyStore>> {
    GLOBAL_KEY_STORE.get_or_init(|| Arc::new(RwLock::new(KeyStore::new())))
}

/// Get the global key store instance
#[allow(dead_code)]
pub fn get_key_store() -> Option<&'static Arc<RwLock<KeyStore>>> {
    GLOBAL_KEY_STORE.get()
}

/// Key store statistics
#[derive(Debug, Clone)]
pub struct KeyStoreStatistics {
    /// Total number of keys
    pub total_keys: usize,
    /// Number of active keys
    pub active_keys: usize,
    /// Number of expired keys
    pub expired_keys: usize,
    /// Total key usage count
    pub total_usage: u64,
    /// Timestamp of the oldest key
    pub oldest_key: Option<DateTime<Utc>>,
    /// Timestamp of the newest key
    pub newest_key: Option<DateTime<Utc>>,
}

/// Key management utilities
pub mod utils {
    use super::*;
    use crate::encryption::errors::EncryptionResult;

    /// Generate a unique key ID
    #[allow(dead_code)]
    pub fn generate_key_id() -> KeyId {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();

        format!("key_{}", timestamp)
    }

    /// Validate key ID format
    #[allow(dead_code)]
    pub fn validate_key_id(key_id: &str) -> EncryptionResult<()> {
        if key_id.is_empty() {
            return Err(EncryptionError::invalid_key("Key ID cannot be empty"));
        }

        if key_id.len() > 255 {
            return Err(EncryptionError::invalid_key("Key ID too long (max 255 characters)"));
        }

        if !key_id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
            return Err(EncryptionError::invalid_key("Key ID contains invalid characters"));
        }

        Ok(())
    }

    /// Sanitize key ID for safe storage
    #[allow(dead_code)]
    pub fn sanitize_key_id(key_id: &str) -> String {
        key_id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }

    /// Check if a key is expired
    #[allow(dead_code)]
    pub fn is_key_expired(metadata: &KeyMetadata) -> bool {
        if let Some(expires_at) = metadata.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Get key age in days
    #[allow(dead_code)]
    pub fn get_key_age_days(metadata: &KeyMetadata) -> i64 {
        let now = Utc::now();
        (now - metadata.created_at).num_days()
    }

    /// Format key size for display
    #[allow(dead_code)]
    pub fn format_key_size(key: &EncryptionKey) -> String {
        let bits = key.len() * 8;
        format!("{} bits", bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::algorithms::EncryptionAlgorithm;

    #[test]
    fn test_memory_key_storage_new() {
        let storage = MemoryKeyStorage::new();
        assert!(storage.keys.is_empty());
    }

    #[test]
    fn test_memory_key_storage_default() {
        let storage = MemoryKeyStorage::default();
        assert!(storage.keys.is_empty());
    }

    #[test]
    fn test_memory_key_storage_store_and_retrieve() {
        let mut storage = MemoryKeyStorage::new();
        let key_id = "test-key".to_string();
        let encrypted_key = b"encrypted_data".to_vec();

        storage.store_key(&key_id, &encrypted_key).unwrap();
        assert!(storage.key_exists(&key_id));

        let retrieved = storage.retrieve_key(&key_id).unwrap();
        assert_eq!(retrieved, encrypted_key);
    }

    #[test]
    fn test_memory_key_storage_delete() {
        let mut storage = MemoryKeyStorage::new();
        let key_id = "test-key".to_string();
        storage.store_key(&key_id, b"data".as_slice()).unwrap();

        storage.delete_key(&key_id).unwrap();
        assert!(!storage.key_exists(&key_id));
    }

    #[test]
    fn test_memory_key_storage_list_keys() {
        let mut storage = MemoryKeyStorage::new();
        storage.store_key(&"key1".to_string(), b"data1".as_slice()).unwrap();
        storage.store_key(&"key2".to_string(), b"data2".as_slice()).unwrap();

        let keys = storage.list_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[test]
    fn test_memory_key_storage_retrieve_nonexistent() {
        let storage = MemoryKeyStorage::new();
        let result = storage.retrieve_key(&"nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_storage_new() {
        let storage = FileKeyStorage::new();
        // Just verify it can be created
        let _ = storage;
    }

    #[test]
    fn test_file_key_storage_with_path() {
        let storage = FileKeyStorage::with_path("/tmp/test-keys");
        // Just verify it can be created
        let _ = storage;
    }

    #[test]
    fn test_file_key_storage_default() {
        let storage = FileKeyStorage::default();
        // Just verify it can be created
        let _ = storage;
    }

    #[test]
    fn test_key_metadata_creation() {
        let metadata = KeyMetadata {
            key_id: "test-key".to_string(),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            created_at: Utc::now(),
            last_used_at: Some(Utc::now()),
            expires_at: None,
            version: 1,
            purpose: "test".to_string(),
            is_active: true,
            usage_count: 0,
        };

        assert_eq!(metadata.key_id, "test-key");
        assert_eq!(metadata.version, 1);
        assert!(metadata.is_active);
    }

    #[test]
    fn test_key_metadata_serialization() {
        let metadata = KeyMetadata {
            key_id: "test-key".to_string(),
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            created_at: Utc::now(),
            last_used_at: None,
            expires_at: None,
            version: 1,
            purpose: "test".to_string(),
            is_active: true,
            usage_count: 5,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("test-key"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_key_store_new() {
        let store = KeyStore::new();
        // Just verify it can be created
        let _ = store;
    }

    #[test]
    fn test_key_store_with_storage() {
        let storage = Box::new(MemoryKeyStorage::new());
        let store = KeyStore::with_storage(storage);
        // Just verify it can be created
        let _ = store;
    }

    #[test]
    fn test_key_store_default() {
        let store = KeyStore::default();
        // Just verify it can be created
        let _ = store;
    }

    #[test]
    fn test_key_store_list_keys_empty() {
        let store = KeyStore::new();
        let keys = store.list_keys();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_key_store_get_key_metadata_nonexistent() {
        let store = KeyStore::new();
        let metadata = store.get_key_metadata(&"nonexistent".to_string());
        assert!(metadata.is_none());
    }

    #[test]
    fn test_key_store_key_exists_false() {
        let store = KeyStore::new();
        assert!(!store.key_exists(&"nonexistent".to_string()));
    }

    #[test]
    fn test_key_store_statistics_creation() {
        let stats = KeyStoreStatistics {
            total_keys: 10,
            active_keys: 8,
            expired_keys: 2,
            total_usage: 100,
            oldest_key: Some(Utc::now()),
            newest_key: Some(Utc::now()),
        };

        assert_eq!(stats.total_keys, 10);
        assert_eq!(stats.active_keys, 8);
        assert_eq!(stats.expired_keys, 2);
        assert_eq!(stats.total_usage, 100);
    }

    #[test]
    fn test_key_store_statistics_with_none_timestamps() {
        let stats = KeyStoreStatistics {
            total_keys: 5,
            active_keys: 4,
            expired_keys: 1,
            total_usage: 50,
            oldest_key: None,
            newest_key: None,
        };

        assert_eq!(stats.total_keys, 5);
        assert!(stats.oldest_key.is_none());
        assert!(stats.newest_key.is_none());
    }

    #[test]
    fn test_key_store_get_statistics() {
        let store = KeyStore::new();
        let stats = store.get_statistics();
        assert_eq!(stats.total_keys, 0);
        assert_eq!(stats.active_keys, 0);
        assert_eq!(stats.expired_keys, 0);
    }
}
