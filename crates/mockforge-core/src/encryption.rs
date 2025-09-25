//! End-to-end encryption module for MockForge
//!
//! This module has been refactored into sub-modules for better organization:
//! - algorithms: Core encryption algorithms (AES-GCM, ChaCha20-Poly1305)
//! - key_management: Key generation, storage, and lifecycle management
//! - auto_encryption: Automatic encryption configuration and processing
//! - derivation: Key derivation functions (Argon2, PBKDF2)
//! - errors: Error types and handling for encryption operations
//!
//! ## Key Management Architecture
//!
//! MockForge uses a hierarchical key management system:
//!
//! 1. **Master Key**: Stored in OS keychain, used to encrypt workspace keys
//! 2. **Workspace Key**: Generated per workspace, encrypted with master key
//! 3. **Session Keys**: Derived from workspace keys for specific operations
//!
//! ## Template Functions
//!
//! - `{{encrypt "text"}}` - Encrypt using AES-256-GCM
//! - `{{secure "text"}}` - Encrypt using ChaCha20-Poly1305 (24-byte nonce)
//! - `{{decrypt "ciphertext"}}` - Decrypt ciphertext back to plaintext
//!
//! ## Automatic Encryption
//!
//! MockForge can automatically encrypt sensitive data in requests and responses:
//!
//! - **Authorization headers** (Bearer tokens, API keys, Basic auth)
//! - **Password fields** in request bodies
//! - **Environment variables** containing sensitive data
//! - **Custom fields** configured by the user
//!
//! Automatic encryption is enabled per workspace and can be configured in workspace settings.

// Re-export sub-modules for backward compatibility
pub mod algorithms;
pub mod key_management;
pub mod auto_encryption;
pub mod derivation;
pub mod errors;

// Re-export commonly used types
pub use algorithms::*;
pub use key_management::{KeyStore as KeyManagementStore, KeyStorage, FileKeyStorage};
pub use auto_encryption::*;
pub use derivation::*;
pub use errors::*;

use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Params,
};
use base64::{Engine as _, engine::general_purpose};
use chacha20poly1305::{ChaCha20Poly1305, Key as ChaChaKey};
use pbkdf2::pbkdf2_hmac;
use rand::{rng, Rng};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tracing;
use crate::workspace_persistence::WorkspacePersistence;

#[cfg(target_os = "windows")]
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

/// Errors that can occur during encryption/decryption operations
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Encryption failure: {0}")]
    Encryption(String),
    #[error("Decryption failure: {0}")]
    Decryption(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Invalid ciphertext: {0}")]
    InvalidCiphertext(String),
    #[error("Key derivation failure: {0}")]
    KeyDerivation(String),
    #[error("Generic encryption error: {message}")]
    Generic { message: String },
}

pub type Result<T> = std::result::Result<T, EncryptionError>;

/// Encryption algorithms supported
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionAlgorithm::Aes256Gcm => write!(f, "aes256-gcm"),
            EncryptionAlgorithm::ChaCha20Poly1305 => write!(f, "chacha20-poly1305"),
        }
    }
}

/// Key derivation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationMethod {
    Pbkdf2,
    Argon2,
}

impl fmt::Display for KeyDerivationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyDerivationMethod::Pbkdf2 => write!(f, "pbkdf2"),
            KeyDerivationMethod::Argon2 => write!(f, "argon2"),
        }
    }
}

/// Cryptographic key for encryption operations
pub struct EncryptionKey {
    algorithm: EncryptionAlgorithm,
    key_data: Vec<u8>,
}

impl EncryptionKey {
    /// Create a new encryption key from raw bytes
    pub fn new(algorithm: EncryptionAlgorithm, key_data: Vec<u8>) -> Result<Self> {
        let expected_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32, // 256 bits
            EncryptionAlgorithm::ChaCha20Poly1305 => 32, // 256 bits
        };

        if key_data.len() != expected_len {
            return Err(EncryptionError::InvalidKey(format!(
                "Key must be {} bytes for {}, got {}",
                expected_len,
                algorithm,
                key_data.len()
            )));
        }

        Ok(Self {
            algorithm,
            key_data,
        })
    }

    /// Derive a key from a password using PBKDF2
    pub fn from_password_pbkdf2(
        password: &str,
        salt: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<Self> {
        let salt = salt
            .map(|s| s.to_vec())
            .unwrap_or_else(|| rng().random::<[u8; 32]>().to_vec());

        let mut key = vec![0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, 100_000, &mut key);

        Self::new(algorithm, key)
    }

    /// Derive a key from a password using Argon2
    pub fn from_password_argon2(
        password: &str,
        salt: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<Self> {
        let salt_string = if let Some(salt) = salt {
            SaltString::encode_b64(salt)
                .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?
        } else {
            // Generate a random salt
            let mut salt_bytes = [0u8; 32];
            rng().fill(&mut salt_bytes);
            SaltString::encode_b64(&salt_bytes)
                .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?
        };

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::new(65536, 3, 1, Some(32))
                .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?,
        );

        let hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;

        let key_bytes = hash.hash.unwrap().as_bytes().to_vec();
        Self::new(algorithm, key_bytes)
    }

    /// Encrypt plaintext data
    pub fn encrypt(&self, plaintext: &str, associated_data: Option<&[u8]>) -> Result<String> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.encrypt_aes_gcm(plaintext, associated_data),
            EncryptionAlgorithm::ChaCha20Poly1305 => self.encrypt_chacha20(plaintext, associated_data),
        }
    }

    /// Decrypt ciphertext data
    pub fn decrypt(&self, ciphertext: &str, associated_data: Option<&[u8]>) -> Result<String> {
        match self.algorithm {
            EncryptionAlgorithm::Aes256Gcm => self.decrypt_aes_gcm(ciphertext, associated_data),
            EncryptionAlgorithm::ChaCha20Poly1305 => self.decrypt_chacha20(ciphertext, associated_data),
        }
    }

    fn encrypt_aes_gcm(&self, plaintext: &str, associated_data: Option<&[u8]>) -> Result<String> {
        let key = GenericArray::from_slice(&self.key_data);
        let cipher = Aes256Gcm::new(key);
        let nonce: [u8; 12] = rng().random(); // 96-bit nonce
        let nonce = Nonce::from(nonce);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        // If associated data is provided, include it
        if let Some(aad) = associated_data {
            result.extend_from_slice(aad);
        }

        Ok(general_purpose::STANDARD.encode(&result))
    }

    fn decrypt_aes_gcm(&self, ciphertext: &str, associated_data: Option<&[u8]>) -> Result<String> {
        let data = general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| EncryptionError::InvalidCiphertext(e.to_string()))?;

        if data.len() < 12 {
            return Err(EncryptionError::InvalidCiphertext(
                "Ciphertext too short".to_string(),
            ));
        }

        let nonce = GenericArray::from_slice(&data[0..12]);
        let ciphertext_len = if let Some(aad) = &associated_data {
            // Associated data is included at the end
            let aad_len = aad.len();
            data.len() - 12 - aad_len
        } else {
            data.len() - 12
        };

        let ciphertext = &data[12..12 + ciphertext_len];
        let key = GenericArray::from_slice(&self.key_data);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::Decryption(format!("Invalid UTF-8: {}", e)))
    }

    pub fn encrypt_chacha20(&self, plaintext: &str, _associated_data: Option<&[u8]>) -> Result<String> {
        let key = ChaChaKey::from_slice(&self.key_data);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce: [u8; 24] = rng().random(); // 192-bit nonce as specified
        let nonce = chacha20poly1305::Nonce::from_slice(&nonce);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&result))
    }

    pub fn decrypt_chacha20(&self, ciphertext: &str, _associated_data: Option<&[u8]>) -> Result<String> {
        let data = general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| EncryptionError::InvalidCiphertext(e.to_string()))?;

        if data.len() < 24 {
            return Err(EncryptionError::InvalidCiphertext(
                "Ciphertext too short".to_string(),
            ));
        }

        let nonce = chacha20poly1305::Nonce::from_slice(&data[0..24]);
        let ciphertext_data = &data[24..];
        let key = ChaChaKey::from_slice(&self.key_data);
        let cipher = ChaCha20Poly1305::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext_data.as_ref())
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::Decryption(format!("Invalid UTF-8: {}", e)))
    }
}

/// Key store for managing encryption keys
pub struct KeyStore {
    keys: std::collections::HashMap<String, EncryptionKey>,
}

impl KeyStore {
    /// Create a new empty key store
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    /// Store a key with a given identifier
    pub fn store_key(&mut self, id: String, key: EncryptionKey) {
        self.keys.insert(id, key);
    }

    /// Retrieve a key by identifier
    pub fn get_key(&self, id: &str) -> Option<&EncryptionKey> {
        self.keys.get(id)
    }

    /// Remove a key
    pub fn remove_key(&mut self, id: &str) -> bool {
        self.keys.remove(id).is_some()
    }

    /// List all key identifiers
    pub fn list_keys(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }

    /// Derive and store a key from password
    pub fn derive_and_store_key(
        &mut self,
        id: String,
        password: &str,
        algorithm: EncryptionAlgorithm,
        method: KeyDerivationMethod,
    ) -> Result<()> {
        let key = match method {
            KeyDerivationMethod::Pbkdf2 => EncryptionKey::from_password_pbkdf2(password, None, algorithm)?,
            KeyDerivationMethod::Argon2 => EncryptionKey::from_password_argon2(password, None, algorithm)?,
        };
        self.store_key(id, key);
        Ok(())
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Global key store instance
static KEY_STORE: once_cell::sync::OnceCell<KeyStore> = once_cell::sync::OnceCell::new();

/// Initialize the global key store
pub fn init_key_store() -> &'static KeyStore {
    KEY_STORE.get_or_init(KeyStore::default)
}

/// Get the global key store
pub fn get_key_store() -> Option<&'static KeyStore> {
    KEY_STORE.get()
}

/// Master key manager for OS keychain integration
pub struct MasterKeyManager {
    _service_name: String,
    _account_name: String,
}

impl MasterKeyManager {
    /// Create a new master key manager
    pub fn new() -> Self {
        Self {
            _service_name: "com.mockforge.encryption".to_string(),
            _account_name: "master_key".to_string(),
        }
    }

    /// Generate and store a new master key in the OS keychain
    pub fn generate_master_key(&self) -> Result<()> {
        let master_key_bytes: [u8; 32] = rand::random();
        let master_key_b64 = general_purpose::STANDARD.encode(master_key_bytes);

        // In a real implementation, this would use OS-specific keychain APIs
        // For now, we'll store it in a secure location or environment variable
        #[cfg(target_os = "macos")]
        {
            // Use macOS Keychain
            self.store_in_macos_keychain(&master_key_b64)?;
        }
        #[cfg(target_os = "linux")]
        {
            // Use Linux keyring or secure storage
            self.store_in_linux_keyring(&master_key_b64)?;
        }
        #[cfg(target_os = "windows")]
        {
            // Use Windows Credential Manager
            self.store_in_windows_credential_manager(&master_key_b64)?;
        }

        Ok(())
    }

    /// Retrieve the master key from OS keychain
    pub fn get_master_key(&self) -> Result<EncryptionKey> {
        let master_key_b64 = self.retrieve_from_keychain()?;
        let master_key_bytes = general_purpose::STANDARD
            .decode(master_key_b64)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;

        if master_key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKey("Invalid master key length".to_string()));
        }

        EncryptionKey::new(EncryptionAlgorithm::ChaCha20Poly1305, master_key_bytes)
    }

    /// Check if master key exists
    pub fn has_master_key(&self) -> bool {
        self.retrieve_from_keychain().is_ok()
    }

    // Platform-specific implementations (simplified for now)
    #[cfg(target_os = "macos")]
    fn store_in_macos_keychain(&self, key: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let home = std::env::var("HOME")
            .map_err(|_| EncryptionError::InvalidKey("HOME environment variable not set".to_string()))?;
        let key_path = std::path::Path::new(&home).join(".mockforge").join("master_key");

        // Create directory if it doesn't exist
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EncryptionError::InvalidKey(format!("Failed to create directory: {}", e)))?;
        }

        // Write the key
        std::fs::write(&key_path, key)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to write master key: {}", e)))?;

        // Set permissions to 600 (owner read/write only)
        let mut perms = std::fs::metadata(&key_path)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to get metadata: {}", e)))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&key_path, perms)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to set permissions: {}", e)))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn store_in_linux_keyring(&self, key: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let home = std::env::var("HOME")
            .map_err(|_| EncryptionError::InvalidKey("HOME environment variable not set".to_string()))?;
        let key_path = std::path::Path::new(&home).join(".mockforge").join("master_key");

        // Create directory if it doesn't exist
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EncryptionError::InvalidKey(format!("Failed to create directory: {}", e)))?;
        }

        // Write the key
        std::fs::write(&key_path, key)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to write master key: {}", e)))?;

        // Set permissions to 600 (owner read/write only)
        let mut perms = std::fs::metadata(&key_path)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to get metadata: {}", e)))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&key_path, perms)
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to set permissions: {}", e)))?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn store_in_windows_credential_manager(&self, key: &str) -> Result<()> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::ERROR_NO_SUCH_LOGON_SESSION;
        use windows::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };

        let target_name = "MockForge/MasterKey";
        let target_name_wide: Vec<u16> = OsString::from(target_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let credential_blob: Vec<u16> = OsString::from(key)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: PCWSTR::from_raw(target_name_wide.as_ptr()),
            Comment: PCWSTR::null(),
            LastWritten: windows::Win32::Foundation::FILETIME::default(),
            CredentialBlobSize: (credential_blob.len() * 2) as u32,
            CredentialBlob: credential_blob.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PCWSTR::null(),
            UserName: PCWSTR::null(),
        };

        unsafe {
            CredWriteW(&mut credential, 0).map_err(|e| {
                EncryptionError::InvalidKey(format!("Failed to store credential: {:?}", e))
            })?;
        }

        Ok(())
    }

    fn retrieve_from_keychain(&self) -> Result<String> {
        // Try platform-specific keychain first
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| EncryptionError::InvalidKey("HOME environment variable not set".to_string()))?;
            let key_path = std::path::Path::new(&home).join(".mockforge").join("master_key");
            std::fs::read_to_string(&key_path)
                .map_err(|_| EncryptionError::InvalidKey("Master key not found in keychain".to_string()))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| EncryptionError::InvalidKey("HOME environment variable not set".to_string()))?;
            let key_path = std::path::Path::new(&home).join(".mockforge").join("master_key");
            std::fs::read_to_string(&key_path)
                .map_err(|_| EncryptionError::InvalidKey("Master key not found in keychain".to_string()))
        }

        #[cfg(target_os = "windows")]
        {
            // Windows Credential Manager
            self.retrieve_from_windows_credential_manager()
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            // Fallback for other platforms
            std::env::var("MOCKFORGE_MASTER_KEY")
                .map_err(|_| EncryptionError::InvalidKey("Master key not found in keychain".to_string()))
        }
    }

    #[cfg(target_os = "windows")]
    fn retrieve_from_windows_credential_manager(&self) -> Result<String> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::core::PCWSTR;
        use windows::Win32::Security::Credentials::{CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC};

        let target_name = "MockForge/MasterKey";
        let target_name_wide: Vec<u16> = OsString::from(target_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut credential_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        unsafe {
            CredReadW(
                PCWSTR::from_raw(target_name_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut credential_ptr,
            )
            .map_err(|e| {
                EncryptionError::InvalidKey(format!("Failed to read credential: {:?}", e))
            })?;

            if credential_ptr.is_null() {
                return Err(EncryptionError::InvalidKey("Credential not found".to_string()));
            }

            // Dereference the credential pointer
            let credential = &*credential_ptr;

            // Convert the credential blob back to string
            // The blob is stored as UTF-16, so we need to convert it properly
            let blob_slice = std::slice::from_raw_parts(
                credential.CredentialBlob as *const u16,
                credential.CredentialBlobSize as usize / 2, // Divide by 2 for UTF-16
            );

            let credential_str = OsString::from_wide(blob_slice)
                .to_string_lossy()
                .trim_end_matches('\0')
                .to_string();

            // Free the credential
            CredFree(credential_ptr as *mut std::ffi::c_void);

            Ok(credential_str)
        }
    }
}

impl Default for MasterKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace key manager for handling per-workspace encryption keys
pub struct WorkspaceKeyManager {
    master_key_manager: MasterKeyManager,
    key_storage: std::cell::RefCell<FileKeyStorage>,
}

impl WorkspaceKeyManager {
    /// Create a new workspace key manager
    pub fn new() -> Self {
        Self {
            master_key_manager: MasterKeyManager::new(),
            key_storage: std::cell::RefCell::new(FileKeyStorage::new()),
        }
    }

    /// Create a workspace key manager with custom key storage path
    pub fn with_storage_path<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            master_key_manager: MasterKeyManager::new(),
            key_storage: std::cell::RefCell::new(FileKeyStorage::with_path(path)),
        }
    }

    /// Generate a new workspace key and encrypt it with the master key
    pub fn generate_workspace_key(&self, workspace_id: &str) -> Result<String> {
        // Generate a new 32-byte workspace key
        let workspace_key_bytes: [u8; 32] = rand::random();

        // Get the master key to encrypt the workspace key
        let master_key = self.master_key_manager.get_master_key()?;

        // Encrypt the workspace key with the master key
        let workspace_key_b64 = master_key.encrypt_chacha20(
            &general_purpose::STANDARD.encode(workspace_key_bytes),
            Some(workspace_id.as_bytes())
        )?;

        // Store the encrypted workspace key (in database or secure storage)
        self.store_workspace_key(workspace_id, &workspace_key_b64)?;

        Ok(workspace_key_b64)
    }

    /// Get the decrypted workspace key for a given workspace
    pub fn get_workspace_key(&self, workspace_id: &str) -> Result<EncryptionKey> {
        let encrypted_key_b64 = self.retrieve_workspace_key(workspace_id)?;
        let master_key = self.master_key_manager.get_master_key()?;

        let decrypted_key_b64 = master_key.decrypt_chacha20(
            &encrypted_key_b64,
            Some(workspace_id.as_bytes())
        )?;

        let workspace_key_bytes = general_purpose::STANDARD
            .decode(decrypted_key_b64)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;

        if workspace_key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKey("Invalid workspace key length".to_string()));
        }

        EncryptionKey::new(EncryptionAlgorithm::ChaCha20Poly1305, workspace_key_bytes)
    }

    /// Check if workspace key exists
    pub fn has_workspace_key(&self, workspace_id: &str) -> bool {
        self.retrieve_workspace_key(workspace_id).is_ok()
    }

    /// Generate a backup string for the workspace key (for sharing between devices)
    pub fn generate_workspace_key_backup(&self, workspace_id: &str) -> Result<String> {
        let encrypted_key = self.retrieve_workspace_key(workspace_id)?;

        // Create a human-readable backup format like:
        // YKV2DK-HT1MD0-8EB48W-PPWHVA-TYJT14-1NWBYN-V874M9-RKJ41R-W95MY0
        let backup_string = self.format_backup_string(&encrypted_key);

        Ok(backup_string)
    }

    /// Restore workspace key from backup string
    pub fn restore_workspace_key_from_backup(&self, workspace_id: &str, backup_string: &str) -> Result<()> {
        let encrypted_key = self.parse_backup_string(backup_string)?;
        self.store_workspace_key(workspace_id, &encrypted_key)
    }

    // Storage methods using secure file-based storage
    fn store_workspace_key(&self, workspace_id: &str, encrypted_key: &str) -> Result<()> {
        self.key_storage.borrow_mut().store_key(&workspace_id.to_string(), encrypted_key.as_bytes())
            .map_err(|e| EncryptionError::InvalidKey(format!("Failed to store workspace key: {:?}", e)))
    }

    fn retrieve_workspace_key(&self, workspace_id: &str) -> Result<String> {
        // First try the new secure storage
        match self.key_storage.borrow().retrieve_key(&workspace_id.to_string()) {
            Ok(encrypted_bytes) => {
                String::from_utf8(encrypted_bytes)
                    .map_err(|e| EncryptionError::InvalidKey(format!("Invalid UTF-8 in stored key: {}", e)))
            }
            Err(_) => {
                // Fall back to old file-based storage for backward compatibility
                let old_key_file = format!("workspace_{}_key.enc", workspace_id);
                match std::fs::read_to_string(&old_key_file) {
                    Ok(encrypted_key) => {
                        // Migrate to new storage
                        if let Err(e) = self.key_storage.borrow_mut().store_key(&workspace_id.to_string(), encrypted_key.as_bytes()) {
                            tracing::warn!("Failed to migrate workspace key to new storage: {:?}", e);
                        } else {
                            // Try to remove old file
                            let _ = std::fs::remove_file(&old_key_file);
                        }
                        Ok(encrypted_key)
                    }
                    Err(_) => Err(EncryptionError::InvalidKey(format!("Workspace key not found for: {}", workspace_id))),
                }
            }
        }
    }

    fn format_backup_string(&self, encrypted_key: &str) -> String {
        // Convert to a format like: XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX
        let chars: Vec<char> = encrypted_key.chars().collect();
        let mut result = String::new();

        for (i, &ch) in chars.iter().enumerate() {
            if i > 0 && i % 6 == 0 && i < chars.len() - 1 {
                result.push('-');
            }
            result.push(ch);
        }

        // Pad or truncate to create consistent format
        if result.len() > 59 { // 9 groups of 6 chars + 8 dashes = 54 + 8 = 62, but we want 59 for readability
            result.truncate(59);
        }

        result
    }

    fn parse_backup_string(&self, backup_string: &str) -> Result<String> {
        // Remove dashes and return the encrypted key
        Ok(backup_string.replace("-", ""))
    }
}

impl Default for WorkspaceKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for automatic encryption of sensitive fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoEncryptionConfig {
    /// Whether automatic encryption is enabled
    pub enabled: bool,
    /// List of header names to automatically encrypt
    pub sensitive_headers: Vec<String>,
    /// List of JSON field paths to automatically encrypt in request/response bodies
    pub sensitive_fields: Vec<String>,
    /// List of environment variable names to automatically encrypt
    pub sensitive_env_vars: Vec<String>,
    /// Custom patterns for detecting sensitive data (regex)
    pub sensitive_patterns: Vec<String>,
}

impl Default for AutoEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sensitive_headers: vec![
                "authorization".to_string(),
                "x-api-key".to_string(),
                "x-auth-token".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
            ],
            sensitive_fields: vec![
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "key".to_string(),
                "credentials".to_string(),
            ],
            sensitive_env_vars: vec![
                "API_KEY".to_string(),
                "SECRET_KEY".to_string(),
                "PASSWORD".to_string(),
                "TOKEN".to_string(),
                "DATABASE_URL".to_string(),
            ],
            sensitive_patterns: vec![
                r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(), // Credit card numbers
                r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b".to_string(), // SSN pattern
            ],
        }
    }
}

/// Automatic encryption processor for sensitive data
pub struct AutoEncryptionProcessor {
    config: AutoEncryptionConfig,
    workspace_manager: WorkspaceKeyManager,
    workspace_id: String,
}

impl AutoEncryptionProcessor {
    /// Create a new auto-encryption processor
    pub fn new(workspace_id: &str, config: AutoEncryptionConfig) -> Self {
        Self {
            config,
            workspace_manager: WorkspaceKeyManager::new(),
            workspace_id: workspace_id.to_string(),
        }
    }

    /// Process headers and encrypt sensitive ones
    pub fn process_headers(&self, headers: &mut std::collections::HashMap<String, String>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let workspace_key = self.workspace_manager.get_workspace_key(&self.workspace_id)?;

        for (key, value) in headers.iter_mut() {
            if self.is_sensitive_header(key) && !self.is_already_encrypted(value) {
                *value = workspace_key.encrypt_chacha20(value, Some(key.as_bytes()))?;
            }
        }

        Ok(())
    }

    /// Process JSON data and encrypt sensitive fields
    pub fn process_json(&self, json: &mut serde_json::Value) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let workspace_key = self.workspace_manager.get_workspace_key(&self.workspace_id)?;
        self.process_json_recursive(json, &workspace_key, Vec::new())?;

        Ok(())
    }

    /// Process environment variables and encrypt sensitive ones
    pub fn process_env_vars(&self, env_vars: &mut std::collections::HashMap<String, String>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let workspace_key = self.workspace_manager.get_workspace_key(&self.workspace_id)?;

        for (key, value) in env_vars.iter_mut() {
            if self.is_sensitive_env_var(key) && !self.is_already_encrypted(value) {
                *value = workspace_key.encrypt_chacha20(value, Some(key.as_bytes()))?;
            }
        }

        Ok(())
    }

    /// Check if a header should be encrypted
    fn is_sensitive_header(&self, header_name: &str) -> bool {
        self.config.sensitive_headers.iter()
            .any(|h| h.eq_ignore_ascii_case(header_name))
    }

    /// Check if an environment variable should be encrypted
    fn is_sensitive_env_var(&self, var_name: &str) -> bool {
        self.config.sensitive_env_vars.iter()
            .any(|v| v.eq_ignore_ascii_case(var_name))
    }

    /// Check if a field path should be encrypted
    fn is_sensitive_field(&self, field_path: &[String]) -> bool {
        let default_field = String::new();
        let field_name = field_path.last().unwrap_or(&default_field);

        // Check exact field names
        if self.config.sensitive_fields.iter()
            .any(|f| f.eq_ignore_ascii_case(field_name)) {
            return true;
        }

        // Check field path patterns
        let path_str = field_path.join(".");
        for pattern in &self.config.sensitive_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(&path_str) || regex.is_match(field_name) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a value appears to already be encrypted
    fn is_already_encrypted(&self, value: &str) -> bool {
        // Simple heuristic: encrypted values are usually base64 and longer than plaintext
        value.len() > 100 && general_purpose::STANDARD.decode(value).is_ok()
    }

    /// Recursively process JSON to encrypt sensitive fields
    fn process_json_recursive(
        &self,
        json: &mut serde_json::Value,
        workspace_key: &EncryptionKey,
        current_path: Vec<String>
    ) -> Result<()> {
        match json {
            serde_json::Value::Object(obj) => {
                for (key, value) in obj.iter_mut() {
                    let mut new_path = current_path.clone();
                    new_path.push(key.clone());

                    if let serde_json::Value::String(ref mut s) = value {
                        if self.is_sensitive_field(&new_path) && !self.is_already_encrypted(s) {
                            let path_str = new_path.join(".");
                            let path_bytes = path_str.as_bytes();
                            *s = workspace_key.encrypt_chacha20(s, Some(path_bytes))?;
                        }
                    } else {
                        self.process_json_recursive(value, workspace_key, new_path)?;
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for (index, item) in arr.iter_mut().enumerate() {
                    let mut new_path = current_path.clone();
                    new_path.push(index.to_string());
                    self.process_json_recursive(item, workspace_key, new_path)?;
                }
            }
            _ => {} // Primitive values are handled at the object level
        }

        Ok(())
    }
}

/// Utility functions for encryption operations
pub mod utils {
    use super::*;

    /// Check if encryption is enabled for a workspace
    pub async fn is_encryption_enabled_for_workspace(persistence: &WorkspacePersistence, workspace_id: &str) -> Result<bool> {
        // Try to load workspace and check settings
        if let Ok(workspace) = persistence.load_workspace(workspace_id).await {
            return Ok(workspace.config.auto_encryption.enabled);
        }
        // Fallback: check if workspace key exists (for backward compatibility)
        let manager = WorkspaceKeyManager::new();
        Ok(manager.has_workspace_key(workspace_id))
    }

    /// Get the auto-encryption config for a workspace
    pub async fn get_auto_encryption_config(persistence: &WorkspacePersistence, workspace_id: &str) -> Result<AutoEncryptionConfig> {
        let workspace = persistence.load_workspace(workspace_id).await
            .map_err(|e| EncryptionError::Generic { message: format!("Failed to load workspace: {}", e) })?;
        Ok(workspace.config.auto_encryption)
    }

    /// Encrypt data for a specific workspace
    pub fn encrypt_for_workspace(workspace_id: &str, data: &str) -> Result<String> {
        let manager = WorkspaceKeyManager::new();
        let key = manager.get_workspace_key(workspace_id)?;
        key.encrypt_chacha20(data, None)
    }

    /// Decrypt data for a specific workspace
    pub fn decrypt_for_workspace(workspace_id: &str, encrypted_data: &str) -> Result<String> {
        let manager = WorkspaceKeyManager::new();
        let key = manager.get_workspace_key(workspace_id)?;
        key.decrypt_chacha20(encrypted_data, None)
    }
}

/// Encrypt text using a stored key
pub fn encrypt_with_key(key_id: &str, plaintext: &str, associated_data: Option<&[u8]>) -> Result<String> {
    let store = get_key_store()
        .ok_or_else(|| EncryptionError::InvalidKey("Key store not initialized".to_string()))?;

    let key = store
        .get_key(key_id)
        .ok_or_else(|| EncryptionError::InvalidKey(format!("Key '{}' not found", key_id)))?;

    key.encrypt(plaintext, associated_data)
}

/// Decrypt text using a stored key
pub fn decrypt_with_key(key_id: &str, ciphertext: &str, associated_data: Option<&[u8]>) -> Result<String> {
    let store = get_key_store()
        .ok_or_else(|| EncryptionError::InvalidKey("Key store not initialized".to_string()))?;

    let key = store
        .get_key(key_id)
        .ok_or_else(|| EncryptionError::InvalidKey(format!("Key '{}' not found", key_id)))?;

    key.decrypt(ciphertext, associated_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_gcm_encrypt_decrypt() {
        let key = EncryptionKey::from_password_pbkdf2("test_password", None, EncryptionAlgorithm::Aes256Gcm)
            .unwrap();

        let plaintext = "Hello, World!";
        let ciphertext = key.encrypt(plaintext, None).unwrap();
        let decrypted = key.decrypt(&ciphertext, None).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_chacha20_encrypt_decrypt() {
        let key = EncryptionKey::from_password_pbkdf2("test_password", None, EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let plaintext = "Hello, World!";
        let ciphertext = key.encrypt(plaintext, None).unwrap();
        let decrypted = key.decrypt(&ciphertext, None).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_key_store() {
        let mut store = KeyStore::new();

        store.derive_and_store_key(
            "test_key".to_string(),
            "test_password",
            EncryptionAlgorithm::Aes256Gcm,
            KeyDerivationMethod::Pbkdf2,
        ).unwrap();

        assert!(store.get_key("test_key").is_some());
        assert!(store.list_keys().contains(&"test_key".to_string()));

        store.remove_key("test_key");
        assert!(store.get_key("test_key").is_none());
    }

    #[test]
    fn test_invalid_key_length() {
        let result = EncryptionKey::new(EncryptionAlgorithm::Aes256Gcm, vec![1, 2, 3]);
        assert!(matches!(result, Err(EncryptionError::InvalidKey(_))));
    }

    #[test]
    fn test_invalid_ciphertext() {
        let key = EncryptionKey::from_password_pbkdf2("test", None, EncryptionAlgorithm::Aes256Gcm).unwrap();
        let result = key.decrypt("invalid_base64!", None);
        assert!(matches!(result, Err(EncryptionError::InvalidCiphertext(_))));
    }

    #[test]
    fn test_chacha20_encrypt_decrypt_24byte_nonce() {
        let key = EncryptionKey::from_password_pbkdf2("test_password", None, EncryptionAlgorithm::ChaCha20Poly1305)
            .unwrap();

        let plaintext = "Hello, World! This is a test of ChaCha20-Poly1305 with 24-byte nonce.";
        let ciphertext = key.encrypt_chacha20(plaintext, None).unwrap();
        let decrypted = key.decrypt_chacha20(&ciphertext, None).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_secure_function_template() {
        use crate::templating::expand_str;

        // Test that secure() function uses ChaCha20-Poly1305
        let template = r#"{{secure "test message"}}"#;
        let result = expand_str(template);

        // The result should be a base64-encoded ciphertext (not the original message)
        assert_ne!(result, "test message");
        assert!(!result.is_empty());

        // The result should be valid base64
        assert!(general_purpose::STANDARD.decode(&result).is_ok());
    }

    #[test]
    fn test_master_key_manager() {
        let manager = MasterKeyManager::new();

        // Initially should not have a master key
        assert!(!manager.has_master_key());

        // Generate a master key
        manager.generate_master_key().unwrap();
        assert!(manager.has_master_key());

        // Should be able to retrieve the master key
        let master_key = manager.get_master_key().unwrap();
        assert_eq!(master_key.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn test_workspace_key_manager() {
        // First ensure we have a master key
        let master_manager = MasterKeyManager::new();
        if !master_manager.has_master_key() {
            master_manager.generate_master_key().unwrap();
        }

        let workspace_manager = WorkspaceKeyManager::new();
        let workspace_id = "test_workspace";

        // Initially should not have a workspace key
        assert!(!workspace_manager.has_workspace_key(workspace_id));

        // Generate a workspace key
        let encrypted_key = workspace_manager.generate_workspace_key(workspace_id).unwrap();
        assert!(workspace_manager.has_workspace_key(workspace_id));
        assert!(!encrypted_key.is_empty());

        // Should be able to retrieve and use the workspace key
        let workspace_key = workspace_manager.get_workspace_key(workspace_id).unwrap();
        assert_eq!(workspace_key.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);

        // Test encryption/decryption with workspace key
        let test_data = "sensitive workspace data";
        let ciphertext = workspace_key.encrypt_chacha20(test_data, None).unwrap();
        let decrypted = workspace_key.decrypt_chacha20(&ciphertext, None).unwrap();
        assert_eq!(test_data, decrypted);
    }

    #[test]
    fn test_backup_string_formatting() {
        let manager = WorkspaceKeyManager::new();

        // Test backup string formatting
        let test_key = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let backup = manager.format_backup_string(test_key);

        // Should contain dashes
        assert!(backup.contains('-'));

        // Should be able to parse back
        let parsed = manager.parse_backup_string(&backup).unwrap();
        assert_eq!(parsed, test_key.replace("-", ""));
    }

    #[test]
    fn test_auto_encryption_processor() {
        // Setup workspace with encryption enabled
        let master_manager = MasterKeyManager::new();
        if !master_manager.has_master_key() {
            master_manager.generate_master_key().unwrap();
        }

        let workspace_manager = WorkspaceKeyManager::new();
        let workspace_id = "test_auto_encrypt_workspace";

        if !workspace_manager.has_workspace_key(workspace_id) {
            workspace_manager.generate_workspace_key(workspace_id).unwrap();
        }

        let config = AutoEncryptionConfig {
            enabled: true,
            ..AutoEncryptionConfig::default()
        };

        let processor = AutoEncryptionProcessor::new(workspace_id, config);

        // Test header encryption
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer my-secret-token".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        processor.process_headers(&mut headers).unwrap();

        // Authorization header should be encrypted
        assert_ne!(headers["Authorization"], "Bearer my-secret-token");
        assert!(general_purpose::STANDARD.decode(&headers["Authorization"]).is_ok());

        // Content-Type should remain unchanged
        assert_eq!(headers["Content-Type"], "application/json");
    }

    #[test]
    fn test_json_field_encryption() {
        // Setup workspace
        let master_manager = MasterKeyManager::new();
        if !master_manager.has_master_key() {
            master_manager.generate_master_key().unwrap();
        }

        let workspace_manager = WorkspaceKeyManager::new();
        let workspace_id = "test_json_workspace";

        if !workspace_manager.has_workspace_key(workspace_id) {
            workspace_manager.generate_workspace_key(workspace_id).unwrap();
        }

        let config = AutoEncryptionConfig {
            enabled: true,
            ..AutoEncryptionConfig::default()
        };

        let processor = AutoEncryptionProcessor::new(workspace_id, config);

        // Test JSON encryption
        let mut json = serde_json::json!({
            "username": "testuser",
            "password": "secret123",
            "email": "test@example.com",
            "nested": {
                "token": "my-api-token",
                "normal_field": "normal_value"
            }
        });

        processor.process_json(&mut json).unwrap();

        // Password and token should be encrypted
        assert_ne!(json["password"], "secret123");
        assert_ne!(json["nested"]["token"], "my-api-token");

        // Username and email should remain unchanged
        assert_eq!(json["username"], "testuser");
        assert_eq!(json["email"], "test@example.com");
        assert_eq!(json["nested"]["normal_field"], "normal_value");
    }

    #[test]
    fn test_env_var_encryption() {
        // Setup workspace
        let master_manager = MasterKeyManager::new();
        if !master_manager.has_master_key() {
            master_manager.generate_master_key().unwrap();
        }

        let workspace_manager = WorkspaceKeyManager::new();
        let workspace_id = "test_env_workspace";

        if !workspace_manager.has_workspace_key(workspace_id) {
            workspace_manager.generate_workspace_key(workspace_id).unwrap();
        }

        let config = AutoEncryptionConfig {
            enabled: true,
            ..AutoEncryptionConfig::default()
        };

        let processor = AutoEncryptionProcessor::new(workspace_id, config);

        // Test environment variable encryption
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("API_KEY".to_string(), "sk-1234567890abcdef".to_string());
        env_vars.insert("DATABASE_URL".to_string(), "postgres://user:pass@host:5432/db".to_string());
        env_vars.insert("NORMAL_VAR".to_string(), "normal_value".to_string());

        processor.process_env_vars(&mut env_vars).unwrap();

        // Sensitive env vars should be encrypted
        assert_ne!(env_vars["API_KEY"], "sk-1234567890abcdef");
        assert_ne!(env_vars["DATABASE_URL"], "postgres://user:pass@host:5432/db");

        // Normal var should remain unchanged
        assert_eq!(env_vars["NORMAL_VAR"], "normal_value");
    }

    #[test]
    fn test_encryption_utils() {
        // Setup workspace
        let master_manager = MasterKeyManager::new();
        if !master_manager.has_master_key() {
            master_manager.generate_master_key().unwrap();
        }

        let workspace_manager = WorkspaceKeyManager::new();
        let workspace_id = "test_utils_workspace";
        workspace_manager.generate_workspace_key(workspace_id).unwrap();

        // Test utility functions - check if key exists (encryption enabled)
        assert!(workspace_manager.has_workspace_key(workspace_id));

        let test_data = "test data for utils";
        let encrypted = utils::encrypt_for_workspace(workspace_id, test_data).unwrap();
        let decrypted = utils::decrypt_for_workspace(workspace_id, &encrypted).unwrap();

        assert_eq!(test_data, decrypted);
        assert_ne!(encrypted, test_data);
    }
}
