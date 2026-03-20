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
    #[allow(dead_code)]
    pub fn encrypt_string(key: &EncryptionKey, plaintext: &str) -> EncryptionResult<EncryptedData> {
        Self::encrypt(key, plaintext.as_bytes(), None)
    }

    /// Decrypt a string using the default algorithm (AES-256-GCM)
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn generate_nonce(algorithm: &EncryptionAlgorithm) -> EncryptionResult<Vec<u8>> {
        let nonce_len = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 12,        // 96 bits
            EncryptionAlgorithm::ChaCha20Poly1305 => 12, // 96 bits
        };

        let mut nonce = vec![0u8; nonce_len];
        let mut rng = thread_rng();
        rng.fill(&mut nonce[..]);

        Ok(nonce)
    }

    /// Validate nonce length for algorithm
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn zeroize(data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== EncryptionAlgorithm Tests ====================

    #[test]
    fn test_encryption_algorithm_display_aes() {
        let algo = EncryptionAlgorithm::Aes256Gcm;
        assert_eq!(format!("{}", algo), "AES-256-GCM");
    }

    #[test]
    fn test_encryption_algorithm_display_chacha() {
        let algo = EncryptionAlgorithm::ChaCha20Poly1305;
        assert_eq!(format!("{}", algo), "ChaCha20-Poly1305");
    }

    #[test]
    fn test_encryption_algorithm_equality() {
        let algo1 = EncryptionAlgorithm::Aes256Gcm;
        let algo2 = EncryptionAlgorithm::Aes256Gcm;
        let algo3 = EncryptionAlgorithm::ChaCha20Poly1305;

        assert_eq!(algo1, algo2);
        assert_ne!(algo1, algo3);
    }

    #[test]
    fn test_encryption_algorithm_clone() {
        let algo = EncryptionAlgorithm::Aes256Gcm;
        let cloned = algo.clone();
        assert_eq!(algo, cloned);
    }

    #[test]
    fn test_encryption_algorithm_debug() {
        let algo = EncryptionAlgorithm::Aes256Gcm;
        let debug_str = format!("{:?}", algo);
        assert!(debug_str.contains("Aes256Gcm"));
    }

    // ==================== EncryptionKey Tests ====================

    #[test]
    fn test_encryption_key_new_valid() {
        let key_bytes = vec![0u8; 32];
        let key = EncryptionKey::new(key_bytes, EncryptionAlgorithm::Aes256Gcm);
        assert!(key.is_ok());
        assert_eq!(key.unwrap().len(), 32);
    }

    #[test]
    fn test_encryption_key_new_invalid_length() {
        let key_bytes = vec![0u8; 16]; // Wrong length for AES-256
        let key = EncryptionKey::new(key_bytes, EncryptionAlgorithm::Aes256Gcm);
        assert!(key.is_err());
    }

    #[test]
    fn test_encryption_key_generate_aes() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm);
        assert!(key.is_ok());
        let key = key.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(*key.algorithm(), EncryptionAlgorithm::Aes256Gcm);
    }

    #[test]
    fn test_encryption_key_generate_chacha() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305);
        assert!(key.is_ok());
        let key = key.unwrap();
        assert_eq!(key.len(), 32);
        assert_eq!(*key.algorithm(), EncryptionAlgorithm::ChaCha20Poly1305);
    }

    #[test]
    fn test_encryption_key_generates_different_keys() {
        let key1 = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let key2 = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();

        // Keys should be different (with overwhelming probability)
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_encryption_key_base64_roundtrip() {
        let original = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let encoded = original.to_base64();

        let decoded = EncryptionKey::from_base64(&encoded, EncryptionAlgorithm::Aes256Gcm).unwrap();

        assert_eq!(original.as_bytes(), decoded.as_bytes());
    }

    #[test]
    fn test_encryption_key_from_base64_invalid() {
        let result =
            EncryptionKey::from_base64("not-valid-base64!@#$", EncryptionAlgorithm::Aes256Gcm);
        assert!(result.is_err());
    }

    #[test]
    fn test_encryption_key_is_empty() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        assert!(!key.is_empty());
    }

    // ==================== EncryptedData Tests ====================

    #[test]
    fn test_encrypted_data_creation() {
        let ciphertext = vec![1, 2, 3, 4, 5];
        let nonce = vec![0u8; 12];
        let data = EncryptedData::new(
            ciphertext.clone(),
            nonce.clone(),
            EncryptionAlgorithm::Aes256Gcm,
            None,
        );

        assert_eq!(data.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(data.aad.is_none());
    }

    #[test]
    fn test_encrypted_data_with_aad() {
        let ciphertext = vec![1, 2, 3, 4, 5];
        let nonce = vec![0u8; 12];
        let aad = vec![10, 20, 30];
        let data = EncryptedData::new(
            ciphertext.clone(),
            nonce.clone(),
            EncryptionAlgorithm::Aes256Gcm,
            Some(aad.clone()),
        );

        assert!(data.aad.is_some());
        assert_eq!(data.aad_bytes().unwrap().unwrap(), aad);
    }

    #[test]
    fn test_encrypted_data_ciphertext_bytes() {
        let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let nonce = vec![0u8; 12];
        let data =
            EncryptedData::new(original.clone(), nonce, EncryptionAlgorithm::Aes256Gcm, None);

        let recovered = data.ciphertext_bytes().unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_encrypted_data_nonce_bytes() {
        let ciphertext = vec![1, 2, 3, 4, 5];
        let nonce = vec![0xAA; 12];
        let data =
            EncryptedData::new(ciphertext, nonce.clone(), EncryptionAlgorithm::Aes256Gcm, None);

        let recovered = data.nonce_bytes().unwrap();
        assert_eq!(nonce, recovered);
    }

    // ==================== EncryptionEngine AES-256-GCM Tests ====================

    #[test]
    fn test_encrypt_decrypt_aes_roundtrip() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = b"Hello, World!";

        let encrypted = EncryptionEngine::encrypt(&key, plaintext, None).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_aes_with_aad() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = b"Secret message";
        let aad = b"associated data";

        let encrypted = EncryptionEngine::encrypt(&key, plaintext, Some(aad)).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = b"Same message";

        let encrypted1 = EncryptionEngine::encrypt(&key, plaintext, None).unwrap();
        let encrypted2 = EncryptionEngine::encrypt(&key, plaintext, None).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1.ciphertext, encrypted2.ciphertext);
        assert_ne!(encrypted1.nonce, encrypted2.nonce);
    }

    #[test]
    fn test_decrypt_wrong_key_aes() {
        let key1 = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let key2 = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = b"Secret";

        let encrypted = EncryptionEngine::encrypt(&key1, plaintext, None).unwrap();
        let result = EncryptionEngine::decrypt(&key2, &encrypted);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_algorithm_mismatch() {
        let key_aes = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let key_chacha = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let plaintext = b"Secret";

        let encrypted = EncryptionEngine::encrypt(&key_aes, plaintext, None).unwrap();
        let result = EncryptionEngine::decrypt(&key_chacha, &encrypted);

        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_empty_plaintext() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = b"";

        let encrypted = EncryptionEngine::encrypt(&key, plaintext, None).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_encrypt_large_plaintext() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = vec![0xAB; 10000]; // 10KB of data

        let encrypted = EncryptionEngine::encrypt(&key, &plaintext, None).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    // ==================== EncryptionEngine ChaCha20-Poly1305 Tests ====================

    #[test]
    fn test_encrypt_decrypt_chacha_roundtrip() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let plaintext = b"Hello, ChaCha!";

        let encrypted = EncryptionEngine::encrypt(&key, plaintext, None).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_chacha_with_aad() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let plaintext = b"Secret message";
        let aad = b"metadata";

        let encrypted = EncryptionEngine::encrypt(&key, plaintext, Some(aad)).unwrap();
        let decrypted = EncryptionEngine::decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_decrypt_wrong_key_chacha() {
        let key1 = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let key2 = EncryptionKey::generate(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        let plaintext = b"Secret";

        let encrypted = EncryptionEngine::encrypt(&key1, plaintext, None).unwrap();
        let result = EncryptionEngine::decrypt(&key2, &encrypted);

        assert!(result.is_err());
    }

    // ==================== String Encryption Tests ====================

    #[test]
    fn test_encrypt_decrypt_string() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = "Hello, World! 🔐";

        let encrypted = EncryptionEngine::encrypt_string(&key, plaintext).unwrap();
        let decrypted = EncryptionEngine::decrypt_string(&key, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_empty_string() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let plaintext = "";

        let encrypted = EncryptionEngine::encrypt_string(&key, plaintext).unwrap();
        let decrypted = EncryptionEngine::decrypt_string(&key, &encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    // ==================== Key Strength Validation Tests ====================

    #[test]
    fn test_validate_key_strength_valid() {
        let key = EncryptionKey::generate(EncryptionAlgorithm::Aes256Gcm).unwrap();
        let result = EncryptionEngine::validate_key_strength(&key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_key_strength_all_zeros() {
        let key_bytes = vec![0u8; 32];
        let key = EncryptionKey::new(key_bytes, EncryptionAlgorithm::Aes256Gcm).unwrap();
        let result = EncryptionEngine::validate_key_strength(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_key_strength_all_ones() {
        let key_bytes = vec![0xFF; 32];
        let key = EncryptionKey::new(key_bytes, EncryptionAlgorithm::Aes256Gcm).unwrap();
        let result = EncryptionEngine::validate_key_strength(&key);
        assert!(result.is_err());
    }

    // ==================== Utils Module Tests ====================

    #[test]
    fn test_generate_nonce_aes() {
        let nonce = utils::generate_nonce(&EncryptionAlgorithm::Aes256Gcm).unwrap();
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_generate_nonce_chacha() {
        let nonce = utils::generate_nonce(&EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn test_generate_nonce_produces_different_values() {
        let nonce1 = utils::generate_nonce(&EncryptionAlgorithm::Aes256Gcm).unwrap();
        let nonce2 = utils::generate_nonce(&EncryptionAlgorithm::Aes256Gcm).unwrap();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_validate_nonce_valid() {
        let nonce = vec![0u8; 12];
        let result = utils::validate_nonce(&nonce, &EncryptionAlgorithm::Aes256Gcm);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_nonce_invalid_length() {
        let nonce = vec![0u8; 8]; // Wrong length
        let result = utils::validate_nonce(&nonce, &EncryptionAlgorithm::Aes256Gcm);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonces_equal_same() {
        let nonce1 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let nonce2 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        assert!(utils::nonces_equal(&nonce1, &nonce2));
    }

    #[test]
    fn test_nonces_equal_different() {
        let nonce1 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let nonce2 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13];
        assert!(!utils::nonces_equal(&nonce1, &nonce2));
    }

    #[test]
    fn test_nonces_equal_different_lengths() {
        let nonce1 = vec![1, 2, 3, 4, 5];
        let nonce2 = vec![1, 2, 3, 4, 5, 6];
        assert!(!utils::nonces_equal(&nonce1, &nonce2));
    }

    #[test]
    fn test_zeroize() {
        let mut data = vec![1, 2, 3, 4, 5];
        utils::zeroize(&mut data);
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_zeroize_empty() {
        let mut data: Vec<u8> = vec![];
        utils::zeroize(&mut data);
        assert!(data.is_empty());
    }
}
