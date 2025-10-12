//! Key derivation functions (Argon2, PBKDF2)
//!
//! This module provides secure key derivation functions for generating
//! encryption keys from passwords and other secret material.

use crate::encryption::algorithms::{EncryptionAlgorithm, EncryptionKey};
use crate::encryption::errors::{EncryptionError, EncryptionResult};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};
use pbkdf2::pbkdf2_hmac;
use rand::{rng, Rng};
use sha2::Sha256;

/// Key derivation method
#[derive(Debug, Clone)]
pub enum KeyDerivationMethod {
    /// Argon2id (recommended for passwords)
    Argon2 {
        memory_kib: u32,
        iterations: u32,
        parallelism: u32,
    },
    /// PBKDF2-HMAC-SHA256
    Pbkdf2 { iterations: u32 },
}

/// Key derivation manager
#[derive(Debug, Clone)]
pub struct KeyDerivationManager {
    /// Default Argon2 parameters
    default_argon2_params: Argon2Params,
}

/// Argon2 parameters
#[derive(Debug, Clone)]
pub struct Argon2Params {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Self {
            memory_kib: 19456, // 19 MiB
            iterations: 2,
            parallelism: 1,
        }
    }
}

impl KeyDerivationManager {
    /// Create a new key derivation manager
    pub fn new() -> Self {
        Self {
            default_argon2_params: Argon2Params::default(),
        }
    }

    /// Derive a master key from a password (synchronous version)
    ///
    /// Note: This is CPU-intensive. Use `derive_master_key_async` when calling from async context.
    pub fn derive_master_key(&self, password: &str) -> EncryptionResult<EncryptionKey> {
        self.derive_key(
            password.as_bytes(),
            KeyDerivationMethod::Argon2 {
                memory_kib: self.default_argon2_params.memory_kib,
                iterations: self.default_argon2_params.iterations,
                parallelism: self.default_argon2_params.parallelism,
            },
            "master_key_salt",
            EncryptionAlgorithm::Aes256Gcm,
        )
    }

    /// Derive a master key from a password (async version using spawn_blocking)
    ///
    /// This method offloads the CPU-intensive Argon2 computation to a blocking thread pool.
    pub async fn derive_master_key_async(
        &self,
        password: String,
    ) -> EncryptionResult<EncryptionKey> {
        let params = self.default_argon2_params.clone();
        tokio::task::spawn_blocking(move || {
            let manager = Self::new();
            manager.derive_key(
                password.as_bytes(),
                KeyDerivationMethod::Argon2 {
                    memory_kib: params.memory_kib,
                    iterations: params.iterations,
                    parallelism: params.parallelism,
                },
                "master_key_salt",
                EncryptionAlgorithm::Aes256Gcm,
            )
        })
        .await
        .map_err(|e| EncryptionError::key_derivation_failed(format!("Task join error: {}", e)))?
    }

    /// Derive a workspace key from workspace ID and master key
    pub fn derive_workspace_key(
        &self,
        master_key: &EncryptionKey,
        workspace_id: &str,
    ) -> EncryptionResult<EncryptionKey> {
        let master_bytes = master_key.as_bytes();
        let workspace_bytes = workspace_id.as_bytes();

        let mut derived_key = vec![0u8; 32];
        pbkdf2_hmac::<Sha256>(
            master_bytes,
            workspace_bytes,
            10000, // iterations
            &mut derived_key,
        );

        EncryptionKey::new(derived_key, EncryptionAlgorithm::Aes256Gcm)
    }

    /// Derive a key using the specified method
    pub fn derive_key(
        &self,
        secret: &[u8],
        method: KeyDerivationMethod,
        salt: &str,
        algorithm: EncryptionAlgorithm,
    ) -> EncryptionResult<EncryptionKey> {
        match method {
            KeyDerivationMethod::Argon2 {
                memory_kib,
                iterations,
                parallelism,
            } => {
                self.derive_key_argon2(secret, salt, memory_kib, iterations, parallelism, algorithm)
            }
            KeyDerivationMethod::Pbkdf2 { iterations } => {
                self.derive_key_pbkdf2(secret, salt, iterations, algorithm)
            }
        }
    }

    /// Derive key using Argon2
    fn derive_key_argon2(
        &self,
        secret: &[u8],
        _salt: &str,
        memory_kib: u32,
        iterations: u32,
        parallelism: u32,
        algorithm: EncryptionAlgorithm,
    ) -> EncryptionResult<EncryptionKey> {
        let salt = SaltString::encode_b64(b"randomsalt12345678901234567890123456789012").unwrap();

        let params = Params::new(
            memory_kib,
            iterations,
            parallelism,
            Some(32), // output length
        )
        .map_err(|e| EncryptionError::key_derivation_failed(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let password_hash = argon2
            .hash_password(secret, &salt)
            .map_err(|e| EncryptionError::key_derivation_failed(e.to_string()))?;

        let hash_binding = password_hash.hash.unwrap();
        let hash_bytes = hash_binding.as_bytes();
        let key_bytes: Vec<u8> = hash_bytes.to_vec();

        // Take only the required number of bytes for the algorithm
        let key_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 => 32,
        };

        let final_key_bytes = if key_bytes.len() >= key_len {
            key_bytes[..key_len].to_vec()
        } else {
            return Err(EncryptionError::key_derivation_failed(
                "Derived key too short for algorithm",
            ));
        };

        EncryptionKey::new(final_key_bytes, algorithm)
    }

    /// Derive key using PBKDF2
    fn derive_key_pbkdf2(
        &self,
        secret: &[u8],
        salt: &str,
        iterations: u32,
        algorithm: EncryptionAlgorithm,
    ) -> EncryptionResult<EncryptionKey> {
        let salt_bytes = salt.as_bytes();
        let key_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 => 32,
        };

        let mut derived_key = vec![0u8; key_len];
        pbkdf2_hmac::<Sha256>(secret, salt_bytes, iterations, &mut derived_key);

        EncryptionKey::new(derived_key, algorithm)
    }

    /// Verify a password against a derived key (synchronous version)
    ///
    /// Note: This is CPU-intensive. Use `verify_password_async` when calling from async context.
    pub fn verify_password(
        &self,
        password: &str,
        expected_key: &EncryptionKey,
    ) -> EncryptionResult<bool> {
        let derived_key = self.derive_master_key(password)?;

        Ok(derived_key.as_bytes() == expected_key.as_bytes())
    }

    /// Verify a password against a derived key (async version using spawn_blocking)
    ///
    /// This method offloads the CPU-intensive Argon2 computation to a blocking thread pool.
    pub async fn verify_password_async(
        &self,
        password: String,
        expected_key: EncryptionKey,
    ) -> EncryptionResult<bool> {
        let params = self.default_argon2_params.clone();
        tokio::task::spawn_blocking(move || {
            let manager = KeyDerivationManager {
                default_argon2_params: params,
            };
            let derived_key = manager.derive_master_key(&password)?;
            Ok(derived_key.as_bytes() == expected_key.as_bytes())
        })
        .await
        .map_err(|e| EncryptionError::key_derivation_failed(format!("Task join error: {}", e)))?
    }

    /// Generate a secure random salt
    pub fn generate_salt() -> String {
        let mut salt = [0u8; 16];
        let mut rng = rng();
        rng.fill(&mut salt);
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt)
    }

    /// Validate key derivation parameters
    pub fn validate_parameters(&self, method: &KeyDerivationMethod) -> EncryptionResult<()> {
        match method {
            KeyDerivationMethod::Argon2 {
                memory_kib,
                iterations,
                parallelism,
            } => {
                if *memory_kib < 8 {
                    return Err(EncryptionError::invalid_algorithm(
                        "Argon2 memory must be at least 8 KiB",
                    ));
                }
                if *iterations < 1 {
                    return Err(EncryptionError::invalid_algorithm(
                        "Argon2 iterations must be at least 1",
                    ));
                }
                if *parallelism < 1 {
                    return Err(EncryptionError::invalid_algorithm(
                        "Argon2 parallelism must be at least 1",
                    ));
                }
            }
            KeyDerivationMethod::Pbkdf2 { iterations } => {
                if *iterations < 1000 {
                    return Err(EncryptionError::invalid_algorithm(
                        "PBKDF2 iterations should be at least 1000 for security",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Default for KeyDerivationManager {
    fn default() -> Self {
        Self::new()
    }
}
