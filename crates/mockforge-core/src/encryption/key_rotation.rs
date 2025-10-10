//! Key rotation and management for encryption
//!
//! This module provides automatic key rotation, versioning, and secure key lifecycle management

use super::algorithms::{EncryptionAlgorithm, EncryptionKey, EncryptedData};
use super::errors::{EncryptionError, EncryptionResult};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Key version identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyVersion(pub u64);

impl KeyVersion {
    /// Create a new key version
    pub fn new(version: u64) -> Self {
        Self(version)
    }

    /// Get the next version
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for KeyVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Versioned encryption key with metadata
#[derive(Debug, Clone)]
pub struct VersionedKey {
    /// Key version
    pub version: KeyVersion,
    /// The encryption key
    pub key: EncryptionKey,
    /// When this key was created
    pub created_at: DateTime<Utc>,
    /// When this key should be rotated (optional)
    pub rotate_at: Option<DateTime<Utc>>,
    /// Whether this key is active (current)
    pub is_active: bool,
}

impl VersionedKey {
    /// Create a new versioned key
    pub fn new(version: KeyVersion, key: EncryptionKey) -> Self {
        Self {
            version,
            key,
            created_at: Utc::now(),
            rotate_at: None,
            is_active: false,
        }
    }

    /// Set rotation time
    pub fn with_rotation(mut self, rotate_at: DateTime<Utc>) -> Self {
        self.rotate_at = Some(rotate_at);
        self
    }

    /// Mark as active
    pub fn activate(mut self) -> Self {
        self.is_active = true;
        self
    }

    /// Check if key should be rotated
    pub fn should_rotate(&self) -> bool {
        if let Some(rotate_at) = self.rotate_at {
            Utc::now() >= rotate_at
        } else {
            false
        }
    }
}

/// Key rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// Rotation interval in days (0 = disabled)
    pub rotation_interval_days: i64,
    /// Maximum number of previous keys to keep
    pub max_previous_keys: usize,
    /// Algorithm to use for new keys
    pub algorithm: EncryptionAlgorithm,
    /// Auto-rotate keys when due
    pub auto_rotate: bool,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            rotation_interval_days: 30, // Rotate every 30 days
            max_previous_keys: 5,        // Keep last 5 keys
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            auto_rotate: true,
        }
    }
}

/// Key manager for handling key rotation
pub struct KeyManager {
    /// Current active key
    current_key: VersionedKey,
    /// Previous keys (for decryption of old data)
    previous_keys: HashMap<KeyVersion, VersionedKey>,
    /// Configuration
    config: KeyRotationConfig,
}

impl KeyManager {
    /// Create a new key manager with a new key
    pub fn new(config: KeyRotationConfig) -> EncryptionResult<Self> {
        let key = EncryptionKey::generate(config.algorithm.clone())?;
        let rotate_at = if config.rotation_interval_days > 0 {
            Some(Utc::now() + Duration::days(config.rotation_interval_days))
        } else {
            None
        };

        let current_key = VersionedKey::new(KeyVersion::new(1), key)
            .with_rotation(rotate_at.unwrap_or_else(|| Utc::now() + Duration::days(365)))
            .activate();

        Ok(Self {
            current_key,
            previous_keys: HashMap::new(),
            config,
        })
    }

    /// Create a key manager with an existing key
    pub fn with_key(config: KeyRotationConfig, key: EncryptionKey) -> EncryptionResult<Self> {
        let rotate_at = if config.rotation_interval_days > 0 {
            Some(Utc::now() + Duration::days(config.rotation_interval_days))
        } else {
            None
        };

        let current_key = VersionedKey::new(KeyVersion::new(1), key)
            .with_rotation(rotate_at.unwrap_or_else(|| Utc::now() + Duration::days(365)))
            .activate();

        Ok(Self {
            current_key,
            previous_keys: HashMap::new(),
            config,
        })
    }

    /// Get the current active key
    pub fn current_key(&self) -> &EncryptionKey {
        &self.current_key.key
    }

    /// Get the current key version
    pub fn current_version(&self) -> KeyVersion {
        self.current_key.version
    }

    /// Check if rotation is due
    pub fn is_rotation_due(&self) -> bool {
        self.config.auto_rotate && self.current_key.should_rotate()
    }

    /// Rotate the key (create new key, archive old one)
    pub fn rotate_key(&mut self) -> EncryptionResult<KeyVersion> {
        // Generate new key
        let new_key = EncryptionKey::generate(self.config.algorithm.clone())?;
        let new_version = self.current_key.version.next();

        let rotate_at = if self.config.rotation_interval_days > 0 {
            Some(Utc::now() + Duration::days(self.config.rotation_interval_days))
        } else {
            None
        };

        // Create new versioned key
        let new_versioned_key = VersionedKey::new(new_version, new_key)
            .with_rotation(rotate_at.unwrap_or_else(|| Utc::now() + Duration::days(365)))
            .activate();

        // Archive current key
        let mut old_key = self.current_key.clone();
        old_key.is_active = false;
        self.previous_keys.insert(old_key.version, old_key);

        // Clean up old keys if we have too many
        if self.previous_keys.len() > self.config.max_previous_keys {
            // Remove oldest keys
            let mut versions: Vec<KeyVersion> = self.previous_keys.keys().copied().collect();
            versions.sort_by_key(|v| v.0);

            let to_remove = versions.len() - self.config.max_previous_keys;
            for version in versions.iter().take(to_remove) {
                self.previous_keys.remove(version);
            }
        }

        // Set new current key
        self.current_key = new_versioned_key;

        Ok(new_version)
    }

    /// Encrypt data with the current key
    pub fn encrypt(&self, plaintext: &[u8], aad: Option<&[u8]>) -> EncryptionResult<VersionedEncryptedData> {
        let encrypted_data = super::algorithms::EncryptionEngine::encrypt(
            &self.current_key.key,
            plaintext,
            aad,
        )?;

        Ok(VersionedEncryptedData {
            version: self.current_key.version,
            data: encrypted_data,
        })
    }

    /// Decrypt data with any available key (current or previous)
    pub fn decrypt(&self, encrypted_data: &VersionedEncryptedData) -> EncryptionResult<Vec<u8>> {
        // Try current key first
        if encrypted_data.version == self.current_key.version {
            return super::algorithms::EncryptionEngine::decrypt(
                &self.current_key.key,
                &encrypted_data.data,
            );
        }

        // Try previous keys
        if let Some(versioned_key) = self.previous_keys.get(&encrypted_data.version) {
            return super::algorithms::EncryptionEngine::decrypt(
                &versioned_key.key,
                &encrypted_data.data,
            );
        }

        Err(EncryptionError::invalid_key(format!(
            "No key found for version {}",
            encrypted_data.version
        )))
    }

    /// Get all key versions
    pub fn key_versions(&self) -> Vec<KeyVersion> {
        let mut versions: Vec<KeyVersion> = self.previous_keys.keys().copied().collect();
        versions.push(self.current_key.version);
        versions.sort_by_key(|v| v.0);
        versions
    }

    /// Get key metadata
    pub fn key_metadata(&self, version: KeyVersion) -> Option<KeyMetadata> {
        if version == self.current_key.version {
            Some(KeyMetadata {
                version,
                created_at: self.current_key.created_at,
                rotate_at: self.current_key.rotate_at,
                is_active: self.current_key.is_active,
            })
        } else {
            self.previous_keys.get(&version).map(|key| KeyMetadata {
                version,
                created_at: key.created_at,
                rotate_at: key.rotate_at,
                is_active: key.is_active,
            })
        }
    }
}

/// Metadata about a key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub version: KeyVersion,
    pub created_at: DateTime<Utc>,
    pub rotate_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Encrypted data with version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedEncryptedData {
    /// Key version used for encryption
    pub version: KeyVersion,
    /// The encrypted data
    pub data: EncryptedData,
}

impl VersionedEncryptedData {
    /// Create new versioned encrypted data
    pub fn new(version: KeyVersion, data: EncryptedData) -> Self {
        Self { version, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_version() {
        let v1 = KeyVersion::new(1);
        let v2 = v1.next();
        assert_eq!(v2.0, 2);
        assert_eq!(v1.to_string(), "v1");
    }

    #[test]
    fn test_key_manager_creation() {
        let config = KeyRotationConfig::default();
        let manager = KeyManager::new(config).unwrap();

        assert_eq!(manager.current_version(), KeyVersion::new(1));
        assert_eq!(manager.key_versions(), vec![KeyVersion::new(1)]);
    }

    #[test]
    fn test_key_rotation() {
        let config = KeyRotationConfig::default();
        let mut manager = KeyManager::new(config).unwrap();

        // Rotate key
        let new_version = manager.rotate_key().unwrap();
        assert_eq!(new_version, KeyVersion::new(2));
        assert_eq!(manager.current_version(), KeyVersion::new(2));

        // Should have 2 versions now
        let versions = manager.key_versions();
        assert_eq!(versions.len(), 2);
        assert!(versions.contains(&KeyVersion::new(1)));
        assert!(versions.contains(&KeyVersion::new(2)));
    }

    #[test]
    fn test_encrypt_decrypt_with_rotation() {
        let config = KeyRotationConfig::default();
        let mut manager = KeyManager::new(config).unwrap();

        // Encrypt with version 1
        let plaintext = b"secret data";
        let encrypted_v1 = manager.encrypt(plaintext, None).unwrap();
        assert_eq!(encrypted_v1.version, KeyVersion::new(1));

        // Rotate key
        manager.rotate_key().unwrap();

        // Encrypt with version 2
        let encrypted_v2 = manager.encrypt(plaintext, None).unwrap();
        assert_eq!(encrypted_v2.version, KeyVersion::new(2));

        // Should be able to decrypt both
        let decrypted_v1 = manager.decrypt(&encrypted_v1).unwrap();
        let decrypted_v2 = manager.decrypt(&encrypted_v2).unwrap();

        assert_eq!(decrypted_v1, plaintext);
        assert_eq!(decrypted_v2, plaintext);
    }

    #[test]
    fn test_max_previous_keys() {
        let config = KeyRotationConfig {
            rotation_interval_days: 30,
            max_previous_keys: 2,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            auto_rotate: true,
        };
        let mut manager = KeyManager::new(config).unwrap();

        // Rotate 5 times
        for _ in 0..5 {
            manager.rotate_key().unwrap();
        }

        // Should only keep last 2 previous keys + current key = 3 total
        let versions = manager.key_versions();
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn test_versioned_key_should_rotate() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();

        // Key with rotation in the past
        let past_rotation = Utc::now() - Duration::days(1);
        let versioned_key = VersionedKey::new(KeyVersion::new(1), key.clone())
            .with_rotation(past_rotation);
        assert!(versioned_key.should_rotate());

        // Key with rotation in the future
        let future_rotation = Utc::now() + Duration::days(1);
        let versioned_key = VersionedKey::new(KeyVersion::new(1), key)
            .with_rotation(future_rotation);
        assert!(!versioned_key.should_rotate());
    }

    #[test]
    fn test_key_metadata() {
        let config = KeyRotationConfig::default();
        let manager = KeyManager::new(config).unwrap();

        let metadata = manager.key_metadata(KeyVersion::new(1)).unwrap();
        assert_eq!(metadata.version, KeyVersion::new(1));
        assert!(metadata.is_active);
        assert!(metadata.rotate_at.is_some());
    }

    #[test]
    fn test_decrypt_with_unknown_version() {
        let config = KeyRotationConfig::default();
        let manager = KeyManager::new(config).unwrap();

        // Create encrypted data with unknown version
        let plaintext = b"test";
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let encrypted = super::super::algorithms::EncryptionEngine::encrypt(&key, plaintext, None).unwrap();

        let versioned = VersionedEncryptedData {
            version: KeyVersion::new(999), // Unknown version
            data: encrypted,
        };

        let result = manager.decrypt(&versioned);
        assert!(result.is_err());
    }
}
