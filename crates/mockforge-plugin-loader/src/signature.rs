//! Plugin signature verification
//!
//! This module provides cryptographic signature verification for plugins.
//! Supports RSA and Ed25519 signatures using the ring cryptography library.

use crate::{LoaderResult, PluginLoaderConfig, PluginLoaderError};
use ring::signature;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Signature algorithm types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA PKCS#1 v1.5 with SHA-256 (2048-bit key)
    #[serde(rename = "RSA_PKCS1_2048_SHA256")]
    RsaPkcs1_2048Sha256,
    /// RSA PKCS#1 v1.5 with SHA-256 (3072-bit key)
    #[serde(rename = "RSA_PKCS1_3072_SHA256")]
    RsaPkcs1_3072Sha256,
    /// RSA PKCS#1 v1.5 with SHA-256 (4096-bit key)
    #[serde(rename = "RSA_PKCS1_4096_SHA256")]
    RsaPkcs1_4096SHA256,
    /// Ed25519 signature scheme
    #[serde(rename = "ED25519")]
    Ed25519,
}

impl SignatureAlgorithm {
    /// Convert to ring's verification algorithm
    fn to_ring_algorithm(&self) -> &'static dyn signature::VerificationAlgorithm {
        match self {
            SignatureAlgorithm::RsaPkcs1_2048Sha256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::RsaPkcs1_3072Sha256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::RsaPkcs1_4096SHA256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::Ed25519 => &signature::ED25519,
        }
    }
}

/// Plugin signature metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    /// Signature algorithm used
    pub algorithm: SignatureAlgorithm,
    /// Key ID that was used to sign
    pub key_id: String,
    /// Hex-encoded signature bytes
    pub signature: String,
    /// Hex-encoded hash of signed content (for verification)
    pub signed_content_hash: String,
}

impl PluginSignature {
    /// Create a new signature
    pub fn new(
        algorithm: SignatureAlgorithm,
        key_id: String,
        signature: Vec<u8>,
        content_hash: Vec<u8>,
    ) -> Self {
        Self {
            algorithm,
            key_id,
            signature: hex::encode(signature),
            signed_content_hash: hex::encode(content_hash),
        }
    }

    /// Get signature bytes
    pub fn signature_bytes(&self) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(&self.signature)
    }

    /// Get content hash bytes
    pub fn content_hash_bytes(&self) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(&self.signed_content_hash)
    }
}

/// Plugin signature verifier
pub struct SignatureVerifier<'a> {
    config: &'a PluginLoaderConfig,
}

impl<'a> SignatureVerifier<'a> {
    /// Create a new signature verifier
    pub fn new(config: &'a PluginLoaderConfig) -> Self {
        Self { config }
    }

    /// Verify a plugin's signature
    ///
    /// This function:
    /// 1. Reads the signature file (plugin.sig)
    /// 2. Computes the hash of the plugin manifest
    /// 3. Verifies the signature using the trusted public key
    pub fn verify_plugin_signature(&self, plugin_dir: &Path) -> LoaderResult<()> {
        // Look for signature file
        let sig_file = plugin_dir.join("plugin.sig");
        if !sig_file.exists() {
            if self.config.allow_unsigned {
                tracing::warn!("No signature file found, but unsigned plugins are allowed");
                return Ok(());
            }
            return Err(PluginLoaderError::security(
                "No signature file found (plugin.sig)",
            ));
        }

        // Read and parse signature file
        let sig_contents = fs::read_to_string(&sig_file).map_err(|e| {
            PluginLoaderError::security(format!("Failed to read signature file: {}", e))
        })?;

        let signature: PluginSignature = serde_json::from_str(&sig_contents).map_err(|e| {
            PluginLoaderError::security(format!("Failed to parse signature file: {}", e))
        })?;

        tracing::debug!(
            "Verifying signature with algorithm {:?} and key_id {}",
            signature.algorithm,
            signature.key_id
        );

        // Check if key is trusted
        if !self.config.trusted_keys.contains(&signature.key_id) {
            return Err(PluginLoaderError::security(format!(
                "Signature key '{}' is not in trusted keys list",
                signature.key_id
            )));
        }

        // Get public key data
        let public_key_bytes = self
            .config
            .key_data
            .get(&signature.key_id)
            .ok_or_else(|| {
                PluginLoaderError::security(format!(
                    "Public key data not found for key_id '{}'",
                    signature.key_id
                ))
            })?;

        // Compute hash of plugin manifest
        let manifest_file = plugin_dir.join("plugin.toml");
        if !manifest_file.exists() {
            return Err(PluginLoaderError::security(
                "Plugin manifest (plugin.toml) not found",
            ));
        }

        let manifest_content = fs::read(&manifest_file).map_err(|e| {
            PluginLoaderError::security(format!("Failed to read plugin manifest: {}", e))
        })?;

        // Compute SHA-256 hash
        let computed_hash = ring::digest::digest(&ring::digest::SHA256, &manifest_content);
        let computed_hash_bytes = computed_hash.as_ref();

        // Verify the hash matches what was signed
        let signed_hash_bytes = signature.content_hash_bytes().map_err(|e| {
            PluginLoaderError::security(format!("Failed to decode signed content hash: {}", e))
        })?;

        if computed_hash_bytes != signed_hash_bytes.as_slice() {
            return Err(PluginLoaderError::security(
                "Plugin manifest hash does not match signed hash. The plugin may have been modified.",
            ));
        }

        // Get signature bytes
        let signature_bytes = signature.signature_bytes().map_err(|e| {
            PluginLoaderError::security(format!("Failed to decode signature: {}", e))
        })?;

        // Verify signature
        let public_key =
            signature::UnparsedPublicKey::new(signature.algorithm.to_ring_algorithm(), public_key_bytes);

        public_key
            .verify(&manifest_content, &signature_bytes)
            .map_err(|_| {
                PluginLoaderError::security("Signature verification failed. Invalid signature.")
            })?;

        tracing::info!(
            "Plugin signature verified successfully with key '{}'",
            signature.key_id
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_algorithm_serialization() {
        let alg = SignatureAlgorithm::Ed25519;
        let json = serde_json::to_string(&alg).unwrap();
        assert_eq!(json, "\"ED25519\"");

        let alg = SignatureAlgorithm::RsaPkcs1_2048Sha256;
        let json = serde_json::to_string(&alg).unwrap();
        assert_eq!(json, "\"RSA_PKCS1_2048_SHA256\"");
    }

    #[test]
    fn test_plugin_signature_encoding() {
        let sig = PluginSignature::new(
            SignatureAlgorithm::Ed25519,
            "test-key".to_string(),
            vec![0x01, 0x02, 0x03],
            vec![0xaa, 0xbb, 0xcc],
        );

        assert_eq!(sig.signature, "010203");
        assert_eq!(sig.signed_content_hash, "aabbcc");
        assert_eq!(sig.signature_bytes().unwrap(), vec![0x01, 0x02, 0x03]);
        assert_eq!(sig.content_hash_bytes().unwrap(), vec![0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn test_signature_verification_with_unsigned_allowed() {
        let config = PluginLoaderConfig {
            allow_unsigned: true,
            ..Default::default()
        };

        let verifier = SignatureVerifier::new(&config);
        let temp_dir = tempfile::tempdir().unwrap();

        // No signature file, but unsigned is allowed
        let result = verifier.verify_plugin_signature(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_verification_missing_file() {
        let config = PluginLoaderConfig {
            allow_unsigned: false,
            ..Default::default()
        };

        let verifier = SignatureVerifier::new(&config);
        let temp_dir = tempfile::tempdir().unwrap();

        // No signature file and unsigned not allowed
        let result = verifier.verify_plugin_signature(temp_dir.path());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PluginLoaderError::SecurityViolation { .. }
        ));
    }
}
