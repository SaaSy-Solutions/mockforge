//! Core encryption algorithms (AES-GCM, ChaCha20-Poly1305)
//!
//! This module provides the core encryption and decryption algorithms used
//! throughout MockForge, including AES-GCM and ChaCha20-Poly1305 implementations.

use crate::encryption::errors::{EncryptionError, EncryptionResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{ChaCha20Poly1305, Key as ChaChaKey};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, fmt};

/// Supported encryption algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM with 96-bit nonce
    Aes256Gcm,
    /// ChaCha20-Poly1305 with 96-bit nonce
    ChaCha20Poly1305,
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aes256Gcm => write!(f, "AES-256-GCM"),
            Self::ChaCha20Poly1305 => write!(f, "ChaCha20-Poly1305"),
        }
    }
}

/// Encryption key for symmetric encryption operations
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    /// The raw key bytes
    key: Vec<u8>,
    /// The encryption algorithm to use with this key
    algorithm: EncryptionAlgorithm,
}

impl EncryptionKey {
    /// Create a new encryption key from raw bytes
    pub fn new(key: Vec<u8>, algorithm: EncryptionAlgorithm) -> EncryptionResult<Self> {
        let expected_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32,        // 256 bits
            EncryptionAlgorithm::ChaCha20Poly1305 => 32, // 256 bits
        };

        if key.len() != expected_len {
            return Err(EncryptionError::invalid_key(format!(
                "Key length {} does not match expected length {} for algorithm {}",
                key.len(),
                expected_len,
                algorithm
            )));
        }

        Ok(Self { key, algorithm })
    }

    /// Generate a random key for the specified algorithm
    pub fn generate(algorithm: EncryptionAlgorithm) -> EncryptionResult<Self> {
        let key_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 => 32,
        };

        let mut key = vec![0u8; key_len];
        let mut rng = thread_rng();
        rng.fill(&mut key[..]);

        Self::new(key, algorithm)
    }

    /// Create a key from a base64-encoded string
    pub fn from_base64(encoded: &str, algorithm: EncryptionAlgorithm) -> EncryptionResult<Self> {
        let key = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| EncryptionError::base64_error(format!("Invalid base64: {}", e)))?;

        Self::new(key, algorithm)
    }

    /// Convert the key to base64 string
    pub fn to_base64(&self) -> String {
        general_purpose::STANDARD.encode(&self.key)
    }

    /// Get the raw key bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }

    /// Get the encryption algorithm
    pub fn algorithm(&self) -> &EncryptionAlgorithm {
        &self.algorithm
    }

    /// Get the key length in bytes
    pub fn len(&self) -> usize {
        self.key.len()
    }

    /// Check if the key is empty
    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

/// Encrypted data with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Base64-encoded ciphertext
    pub ciphertext: String,
    /// Base64-encoded nonce
    pub nonce: String,
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Additional authenticated data (optional)
    pub aad: Option<String>,
    /// Timestamp when data was encrypted
    pub encrypted_at: chrono::DateTime<chrono::Utc>,
}

impl EncryptedData {
    /// Create a new encrypted data instance
    pub fn new(
        ciphertext: Vec<u8>,
        nonce: Vec<u8>,
        algorithm: EncryptionAlgorithm,
        aad: Option<Vec<u8>>,
    ) -> Self {
        Self {
            ciphertext: general_purpose::STANDARD.encode(ciphertext),
            nonce: general_purpose::STANDARD.encode(nonce),
            algorithm,
            aad: aad.map(|data| general_purpose::STANDARD.encode(data)),
            encrypted_at: chrono::Utc::now(),
        }
    }

    /// Get the decrypted ciphertext bytes
    pub fn ciphertext_bytes(&self) -> EncryptionResult<Vec<u8>> {
        general_purpose::STANDARD
            .decode(&self.ciphertext)
            .map_err(|e| EncryptionError::base64_error(format!("Invalid ciphertext base64: {}", e)))
    }

    /// Get the nonce bytes
    pub fn nonce_bytes(&self) -> EncryptionResult<Vec<u8>> {
        general_purpose::STANDARD
            .decode(&self.nonce)
            .map_err(|e| EncryptionError::base64_error(format!("Invalid nonce base64: {}", e)))
    }

    /// Get the AAD bytes
    pub fn aad_bytes(&self) -> Option<EncryptionResult<Vec<u8>>> {
        self.aad.as_ref().map(|aad| {
            general_purpose::STANDARD
                .decode(aad)
                .map_err(|e| EncryptionError::base64_error(format!("Invalid AAD base64: {}", e)))
        })
    }
}

/// Core encryption operations
pub struct EncryptionEngine;

impl EncryptionEngine {
    /// Encrypt plaintext using the specified key and algorithm
    pub fn encrypt(
        key: &EncryptionKey,
        plaintext: &[u8],
        aad: Option<&[u8]>,
    ) -> EncryptionResult<EncryptedData> {
        match key.algorithm() {
            EncryptionAlgorithm::Aes256Gcm => Self::encrypt_aes256_gcm(key, plaintext, aad),
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                Self::encrypt_chacha20_poly1305(key, plaintext, aad)
            }
        }
    }

    /// Decrypt ciphertext using the specified key
    pub fn decrypt(
        key: &EncryptionKey,
        encrypted_data: &EncryptedData,
    ) -> EncryptionResult<Vec<u8>> {
        // Validate algorithm matches
        if key.algorithm() != &encrypted_data.algorithm {
            return Err(EncryptionError::invalid_algorithm(format!(
                "Key algorithm {} does not match encrypted data algorithm {}",
                key.algorithm(),
                encrypted_data.algorithm
            )));
        }

        match key.algorithm() {
            EncryptionAlgorithm::Aes256Gcm => Self::decrypt_aes256_gcm(key, encrypted_data),
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                Self::decrypt_chacha20_poly1305(key, encrypted_data)
            }
        }
    }

    /// Encrypt using AES-256-GCM
    fn encrypt_aes256_gcm(
        key: &EncryptionKey,
        plaintext: &[u8],
        aad: Option<&[u8]>,
    ) -> EncryptionResult<EncryptedData> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12]; // 96 bits
        let mut rng = thread_rng();
        rng.fill(&mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Convert key bytes to fixed-size array for Aes256Gcm::new()
        // We validate key length in EncryptionKey::new(), so this is safe
        let key_bytes = key.as_bytes();
        let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| {
            EncryptionError::invalid_key(
                "Key length mismatch during encryption (expected 32 bytes)".to_string(),
            )
        })?;
        let cipher = Aes256Gcm::new(&key_array.into());

        // Encrypt the plaintext
        let ciphertext = match aad {
            Some(aad_data) => cipher.encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: aad_data,
                },
            ),
            None => cipher.encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: &[],
                },
            ),
        }
        .map_err(|e| {
            EncryptionError::cipher_operation_failed(format!("AES-GCM encryption failed: {}", e))
        })?;

        Ok(EncryptedData::new(
            ciphertext,
            nonce_bytes.to_vec(),
            EncryptionAlgorithm::Aes256Gcm,
            aad.map(|data| data.to_vec()),
        ))
    }

    /// Decrypt using AES-256-GCM
    fn decrypt_aes256_gcm(
        key: &EncryptionKey,
        encrypted_data: &EncryptedData,
    ) -> EncryptionResult<Vec<u8>> {
        let nonce_bytes = encrypted_data.nonce_bytes()?;
        let ciphertext = encrypted_data.ciphertext_bytes()?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Convert key bytes to fixed-size array for Aes256Gcm::new()
        // We validate key length in EncryptionKey::new(), so this is safe
        let key_bytes = key.as_bytes();
        let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| {
            EncryptionError::invalid_key(
                "Key length mismatch during decryption (expected 32 bytes)".to_string(),
            )
        })?;
        let cipher = Aes256Gcm::new(&key_array.into());

        let plaintext = match encrypted_data.aad_bytes() {
            Some(Ok(aad)) => cipher.decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            ),
            Some(Err(e)) => return Err(e),
            None => cipher.decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: &ciphertext,
                    aad: &[],
                },
            ),
        }
        .map_err(|e| {
            if e.to_string().contains("authentication") {
                EncryptionError::authentication_failed("AES-GCM authentication failed")
            } else {
                EncryptionError::cipher_operation_failed(format!(
                    "AES-GCM decryption failed: {}",
                    e
                ))
            }
        })?;

        Ok(plaintext)
    }

    /// Encrypt using ChaCha20-Poly1305
    fn encrypt_chacha20_poly1305(
        key: &EncryptionKey,
        plaintext: &[u8],
        aad: Option<&[u8]>,
    ) -> EncryptionResult<EncryptedData> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12]; // 96 bits
        let mut rng = thread_rng();
        rng.fill(&mut nonce_bytes);

        let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
        let cipher_key = ChaChaKey::from_slice(key.as_bytes());
        let cipher = ChaCha20Poly1305::new(cipher_key);

        // Encrypt the plaintext
        let ciphertext = match aad {
            Some(aad_data) => cipher.encrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad: aad_data,
                },
            ),
            None => cipher.encrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad: &[],
                },
            ),
        }
        .map_err(|e| {
            EncryptionError::cipher_operation_failed(format!(
                "ChaCha20-Poly1305 encryption failed: {}",
                e
            ))
        })?;

        Ok(EncryptedData::new(
            ciphertext,
            nonce_bytes.to_vec(),
            EncryptionAlgorithm::ChaCha20Poly1305,
            aad.map(|data| data.to_vec()),
        ))
    }

    /// Decrypt using ChaCha20-Poly1305
    fn decrypt_chacha20_poly1305(
        key: &EncryptionKey,
        encrypted_data: &EncryptedData,
    ) -> EncryptionResult<Vec<u8>> {
        let nonce_bytes = encrypted_data.nonce_bytes()?;
        let ciphertext = encrypted_data.ciphertext_bytes()?;

        let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
        let cipher_key = ChaChaKey::from_slice(key.as_bytes());
        let cipher = ChaCha20Poly1305::new(cipher_key);

        let plaintext = match encrypted_data.aad_bytes() {
            Some(Ok(aad)) => cipher.decrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            ),
            Some(Err(e)) => return Err(e),
            None => cipher.decrypt(
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: &ciphertext,
                    aad: &[],
                },
            ),
        }
        .map_err(|e| {
            if e.to_string().contains("authentication") {
                EncryptionError::authentication_failed("ChaCha20-Poly1305 authentication failed")
            } else {
                EncryptionError::cipher_operation_failed(format!(
                    "ChaCha20-Poly1305 decryption failed: {}",
                    e
                ))
            }
        })?;

        Ok(plaintext)
    }

    /// Encrypt a string using the default algorithm (AES-256-GCM)
    pub fn encrypt_string(key: &EncryptionKey, plaintext: &str) -> EncryptionResult<EncryptedData> {
        Self::encrypt(key, plaintext.as_bytes(), None)
    }

    /// Decrypt a string using the default algorithm (AES-256-GCM)
    pub fn decrypt_string(
        key: &EncryptionKey,
        encrypted_data: &EncryptedData,
    ) -> EncryptionResult<String> {
        let plaintext = Self::decrypt(key, encrypted_data)?;
        String::from_utf8(plaintext).map_err(|e| {
            EncryptionError::invalid_ciphertext(format!("Invalid UTF-8 in decrypted data: {}", e))
        })
    }

    /// Validate key strength
    pub fn validate_key_strength(key: &EncryptionKey) -> EncryptionResult<()> {
        match key.algorithm() {
            EncryptionAlgorithm::Aes256Gcm => {
                // Check for weak keys (all zeros, all ones, etc.)
                if key.as_bytes().iter().all(|&b| b == 0) {
                    return Err(EncryptionError::invalid_key("Key cannot be all zeros"));
                }
                if key.as_bytes().iter().all(|&b| b == 0xFF) {
                    return Err(EncryptionError::invalid_key("Key cannot be all 0xFF"));
                }
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                // Similar checks for ChaCha20
                if key.as_bytes().iter().all(|&b| b == 0) {
                    return Err(EncryptionError::invalid_key("Key cannot be all zeros"));
                }
                if key.as_bytes().iter().all(|&b| b == 0xFF) {
                    return Err(EncryptionError::invalid_key("Key cannot be all 0xFF"));
                }
            }
        }

        Ok(())
    }
}

/// Utility functions for encryption operations
pub mod utils {
    use super::*;
    use crate::encryption::errors::EncryptionResult;

    /// Generate a cryptographically secure random nonce
    pub fn generate_nonce(algorithm: &EncryptionAlgorithm) -> EncryptionResult<Vec<u8>> {
        let nonce_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 12,        // 96 bits
            EncryptionAlgorithm::ChaCha20Poly1305 => 12, // 96 bits
        };

        let mut nonce = vec![0u8; nonce_len];
        let mut rng = rand::thread_rng();
        rng.fill(&mut nonce[..]);

        Ok(nonce)
    }

    /// Validate nonce length for algorithm
    pub fn validate_nonce(nonce: &[u8], algorithm: &EncryptionAlgorithm) -> EncryptionResult<()> {
        let expected_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 12,
            EncryptionAlgorithm::ChaCha20Poly1305 => 12,
        };

        if nonce.len() != expected_len {
            return Err(EncryptionError::invalid_nonce(format!(
                "Nonce length {} does not match expected length {} for algorithm {}",
                nonce.len(),
                expected_len,
                algorithm
            )));
        }

        Ok(())
    }

    /// Check if two nonces are equal (constant-time comparison)
    pub fn nonces_equal(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (byte_a, byte_b) in a.iter().zip(b.iter()) {
            result |= byte_a ^ byte_b;
        }

        result == 0
    }

    /// Zeroize sensitive data in memory
    pub fn zeroize(data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }
}
