//! Plugin signature verification
//!
//! This module provides cryptographic signature verification for plugins.
//! Supports RSA and Ed25519 signatures using the ring cryptography library.

use crate::{LoaderResult, PluginLoaderConfig, PluginLoaderError};
use chrono::{DateTime, Utc};
use ring::signature;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

mod artifact;
mod trust_store;
pub use artifact::compute_artifact_payload;
pub use trust_store::{resolve_trust_store_path, TrustStoreFile, TrustedKeyEntry};

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
    fn as_ring_algorithm(&self) -> &'static dyn signature::VerificationAlgorithm {
        match self {
            SignatureAlgorithm::RsaPkcs1_2048Sha256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::RsaPkcs1_3072Sha256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::RsaPkcs1_4096SHA256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            SignatureAlgorithm::Ed25519 => &signature::ED25519,
        }
    }
}

/// Name of the detached signature file at the root of a plugin directory.
pub const SIGNATURE_FILE: &str = "plugin.sig";

/// Signature format version produced and accepted by this crate.
///
/// Version 2 signs a digest of every file in the plugin directory
/// (see [`compute_artifact_payload`]). Version 1 signed only
/// `plugin.toml`, so a swapped WASM binary still verified; it is
/// rejected.
pub const SIGNATURE_FORMAT_VERSION: u32 = 2;

/// Domain separator for the v2 signed payload.
const ARTIFACT_PAYLOAD_DOMAIN: &[u8] = b"mockforge-plugin-signature-v2\0";

/// Environment variable naming a trust store file (see [`TrustStoreFile`]).
pub const TRUST_STORE_ENV: &str = "MOCKFORGE_PLUGIN_TRUSTED_KEYS_FILE";

/// Default trust store location used by the CLI.
pub const DEFAULT_TRUST_STORE_PATH: &str = "~/.mockforge/trusted-keys.json";

fn legacy_format_version() -> u32 {
    1
}

/// Plugin signature metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    /// Signature format version. Files without the field are v1.
    #[serde(default = "legacy_format_version")]
    pub format_version: u32,
    /// Signature algorithm used
    pub algorithm: SignatureAlgorithm,
    /// Key ID that was used to sign
    pub key_id: String,
    /// Hex-encoded signature bytes
    pub signature: String,
    /// Hex-encoded SHA-256 of the signed artifact payload
    pub signed_content_hash: String,
}

impl PluginSignature {
    /// Create a new signature in the current format
    pub fn new(
        algorithm: SignatureAlgorithm,
        key_id: String,
        signature: Vec<u8>,
        content_hash: Vec<u8>,
    ) -> Self {
        Self {
            format_version: SIGNATURE_FORMAT_VERSION,
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

/// Whether a plugin directory carries a detached signature file.
pub fn has_signature_file(plugin_dir: &Path) -> bool {
    !matches!(fs::symlink_metadata(plugin_dir.join(SIGNATURE_FILE)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound)
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
    /// 1. Reads the signature file (`plugin.sig`)
    /// 2. Rejects legacy, revoked, untrusted and expired keys
    /// 3. Recomputes the artifact payload over every plugin file
    /// 4. Verifies the signature over that payload with the trusted key
    ///
    /// A missing signature passes only when `allow_unsigned` is set. A
    /// signature that is present but invalid always fails.
    pub fn verify_plugin_signature(&self, plugin_dir: &Path) -> LoaderResult<()> {
        self.verify_plugin_signature_at(plugin_dir, Utc::now())
    }

    /// [`Self::verify_plugin_signature`] with an explicit clock, for
    /// deterministic expiry checks.
    pub fn verify_plugin_signature_at(
        &self,
        plugin_dir: &Path,
        now: DateTime<Utc>,
    ) -> LoaderResult<()> {
        let sig_file = plugin_dir.join(SIGNATURE_FILE);
        if !has_signature_file(plugin_dir) {
            if self.config.allow_unsigned {
                tracing::warn!("No signature file found, but unsigned plugins are allowed");
                return Ok(());
            }
            return Err(PluginLoaderError::security("No signature file found (plugin.sig)"));
        }

        let sig_contents = fs::read_to_string(&sig_file).map_err(|e| {
            PluginLoaderError::security(format!("Failed to read signature file: {}", e))
        })?;

        let signature: PluginSignature = serde_json::from_str(&sig_contents).map_err(|e| {
            PluginLoaderError::security(format!("Failed to parse signature file: {}", e))
        })?;

        if signature.format_version != SIGNATURE_FORMAT_VERSION {
            return Err(PluginLoaderError::security(format!(
                "Unsupported plugin signature format v{} (expected v{}). Older signatures \
                 covered only the manifest; re-sign the plugin.",
                signature.format_version, SIGNATURE_FORMAT_VERSION
            )));
        }

        tracing::debug!(
            "Verifying signature with algorithm {:?} and key_id {}",
            signature.algorithm,
            signature.key_id
        );

        if self.config.revoked_keys.contains(&signature.key_id) {
            return Err(PluginLoaderError::security(format!(
                "Signature key '{}' has been revoked",
                signature.key_id
            )));
        }

        if !self.config.trusted_keys.contains(&signature.key_id) {
            return Err(PluginLoaderError::security(format!(
                "Signature key '{}' is not in trusted keys list",
                signature.key_id
            )));
        }

        if let Some(expires_at) = self.config.key_expires_at.get(&signature.key_id) {
            if now >= *expires_at {
                return Err(PluginLoaderError::security(format!(
                    "Signature key '{}' expired at {}",
                    signature.key_id,
                    expires_at.to_rfc3339()
                )));
            }
        }

        let public_key_bytes = self.config.key_data.get(&signature.key_id).ok_or_else(|| {
            PluginLoaderError::security(format!(
                "Public key data not found for key_id '{}'",
                signature.key_id
            ))
        })?;

        let payload = compute_artifact_payload(plugin_dir)?;
        let computed_hash = ring::digest::digest(&ring::digest::SHA256, &payload);

        let signed_hash_bytes = signature.content_hash_bytes().map_err(|e| {
            PluginLoaderError::security(format!("Failed to decode signed content hash: {}", e))
        })?;

        if computed_hash.as_ref() != signed_hash_bytes.as_slice() {
            return Err(PluginLoaderError::security(
                "Plugin contents do not match the signed hash. The plugin was modified or \
                 the signature belongs to a different artifact.",
            ));
        }

        let signature_bytes = signature.signature_bytes().map_err(|e| {
            PluginLoaderError::security(format!("Failed to decode signature: {}", e))
        })?;

        let public_key = signature::UnparsedPublicKey::new(
            signature.algorithm.as_ring_algorithm(),
            public_key_bytes,
        );

        public_key.verify(&payload, &signature_bytes).map_err(|_| {
            PluginLoaderError::security("Signature verification failed. Invalid signature.")
        })?;

        tracing::info!("Plugin signature verified successfully with key '{}'", signature.key_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn dangling_signature_is_rejected_even_when_unsigned_plugins_are_allowed() {
        let directory = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("missing-signature", directory.path().join(SIGNATURE_FILE))
            .unwrap();
        let config = PluginLoaderConfig {
            allow_unsigned: true,
            ..Default::default()
        };
        assert!(has_signature_file(directory.path()));
        assert!(SignatureVerifier::new(&config)
            .verify_plugin_signature(directory.path())
            .is_err());
    }

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
        assert!(matches!(result.unwrap_err(), PluginLoaderError::SecurityViolation { .. }));
    }

    #[test]
    fn test_trust_store_file_applies_keys_expiry_and_revocation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("trusted-keys.json");
        fs::write(
            &path,
            r#"{"keys":[
                {"key_id":"active","public_key":"00ff","expires_at":"2030-01-01T00:00:00Z"},
                {"key_id":"gone","public_key":"aa","revoked":true}
            ]}"#,
        )
        .unwrap();

        let mut config = PluginLoaderConfig::default();
        let trusted = TrustStoreFile::load(&path).unwrap().apply_to(&mut config).unwrap();

        assert_eq!(trusted, 1);
        assert!(config.trusted_keys.contains(&"active".to_string()));
        assert_eq!(config.key_data.get("active"), Some(&vec![0x00, 0xff]));
        assert!(config.key_expires_at.contains_key("active"));
        assert!(!config.trusted_keys.contains(&"gone".to_string()));
        assert!(config.revoked_keys.contains(&"gone".to_string()));
    }

    #[test]
    fn test_trust_store_rejects_bad_hex() {
        let store = TrustStoreFile {
            keys: vec![TrustedKeyEntry {
                key_id: "k".to_string(),
                public_key: "not-hex".to_string(),
                expires_at: None,
                revoked: false,
            }],
        };
        let mut config = PluginLoaderConfig::default();
        assert!(store.apply_to(&mut config).is_err());
    }

    #[test]
    fn test_artifact_payload_covers_nested_files_and_ignores_signature() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("plugin.yaml"), "id: x").unwrap();
        let before = compute_artifact_payload(temp_dir.path()).unwrap();

        fs::write(temp_dir.path().join(SIGNATURE_FILE), "{}").unwrap();
        assert_eq!(compute_artifact_payload(temp_dir.path()).unwrap(), before);

        fs::create_dir(temp_dir.path().join("lib")).unwrap();
        fs::write(temp_dir.path().join("lib").join(SIGNATURE_FILE), "x").unwrap();
        assert_ne!(compute_artifact_payload(temp_dir.path()).unwrap(), before);
    }

    #[cfg(unix)]
    #[test]
    fn test_artifact_payload_rejects_symlinks() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("plugin.yaml"), "id: x").unwrap();
        std::os::unix::fs::symlink("/etc/hostname", temp_dir.path().join("link")).unwrap();
        let err = compute_artifact_payload(temp_dir.path()).unwrap_err();
        assert!(err.to_string().contains("symlink"));
    }

    #[test]
    fn test_expiry_boundary_uses_supplied_clock() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("plugin.yaml"), "id: x").unwrap();
        let (private_key, public_key) = crate::generate_ed25519_keypair().unwrap();
        crate::sign_plugin_ed25519(temp_dir.path(), "k", &private_key).unwrap();

        let expiry = Utc::now();
        let mut config = PluginLoaderConfig {
            trusted_keys: vec!["k".to_string()],
            ..Default::default()
        };
        config.key_data.insert("k".to_string(), public_key);
        config.key_expires_at.insert("k".to_string(), expiry);
        let verifier = SignatureVerifier::new(&config);

        let just_before = expiry - chrono::Duration::seconds(1);
        assert!(verifier.verify_plugin_signature_at(temp_dir.path(), just_before).is_ok());
        assert!(verifier.verify_plugin_signature_at(temp_dir.path(), expiry).is_err());
    }
}
