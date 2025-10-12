//! Plugin signature generation utilities
//!
//! This module provides utilities for generating plugin signatures during development.
//! It's primarily used for testing and plugin development workflows.

use crate::signature::{PluginSignature, SignatureAlgorithm};
use crate::{LoaderResult, PluginLoaderError};
use ring::rand::SystemRandom;
use ring::signature::KeyPair;
use std::fs;
use std::path::Path;

/// Sign a plugin manifest with Ed25519
///
/// This function:
/// 1. Reads the plugin manifest (plugin.toml)
/// 2. Computes SHA-256 hash of the manifest
/// 3. Signs the hash with the provided private key
/// 4. Writes the signature to plugin.sig
pub fn sign_plugin_ed25519(
    plugin_dir: &Path,
    key_id: &str,
    private_key_bytes: &[u8],
) -> LoaderResult<PluginSignature> {
    // Read manifest
    let manifest_file = plugin_dir.join("plugin.toml");
    if !manifest_file.exists() {
        return Err(PluginLoaderError::security("Plugin manifest (plugin.toml) not found"));
    }

    let manifest_content = fs::read(&manifest_file).map_err(|e| {
        PluginLoaderError::security(format!("Failed to read plugin manifest: {}", e))
    })?;

    // Compute hash
    let hash = ring::digest::digest(&ring::digest::SHA256, &manifest_content);
    let hash_bytes = hash.as_ref();

    // Sign the manifest content (not just the hash)
    let key_pair = ring::signature::Ed25519KeyPair::from_pkcs8(private_key_bytes).map_err(|e| {
        PluginLoaderError::security(format!("Failed to parse Ed25519 private key: {:?}", e))
    })?;

    let signature_bytes = key_pair.sign(&manifest_content);

    // Create signature object
    let signature = PluginSignature::new(
        SignatureAlgorithm::Ed25519,
        key_id.to_string(),
        signature_bytes.as_ref().to_vec(),
        hash_bytes.to_vec(),
    );

    // Write signature file
    let sig_file = plugin_dir.join("plugin.sig");
    let sig_json = serde_json::to_string_pretty(&signature).map_err(|e| {
        PluginLoaderError::security(format!("Failed to serialize signature: {}", e))
    })?;

    fs::write(&sig_file, sig_json).map_err(|e| {
        PluginLoaderError::security(format!("Failed to write signature file: {}", e))
    })?;

    Ok(signature)
}

/// Generate a new Ed25519 key pair for plugin signing
///
/// Returns (private_key_pkcs8, public_key_bytes)
pub fn generate_ed25519_keypair() -> LoaderResult<(Vec<u8>, Vec<u8>)> {
    let rng = SystemRandom::new();
    let pkcs8 = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng).map_err(|e| {
        PluginLoaderError::security(format!("Failed to generate Ed25519 key pair: {:?}", e))
    })?;

    let key_pair = ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|e| {
        PluginLoaderError::security(format!("Failed to parse generated key pair: {:?}", e))
    })?;

    let public_key = key_pair.public_key().as_ref().to_vec();
    let private_key = pkcs8.as_ref().to_vec();

    Ok((private_key, public_key))
}

/// Save a key pair to files
pub fn save_keypair(
    private_key: &[u8],
    public_key: &[u8],
    output_dir: &Path,
    key_name: &str,
) -> LoaderResult<()> {
    let private_key_file = output_dir.join(format!("{}.private.key", key_name));
    let public_key_file = output_dir.join(format!("{}.public.key", key_name));

    fs::write(&private_key_file, hex::encode(private_key))
        .map_err(|e| PluginLoaderError::fs(format!("Failed to write private key: {}", e)))?;

    fs::write(&public_key_file, hex::encode(public_key))
        .map_err(|e| PluginLoaderError::fs(format!("Failed to write public key: {}", e)))?;

    Ok(())
}

/// Load a key from a hex-encoded file
pub fn load_key_from_file(path: &Path) -> LoaderResult<Vec<u8>> {
    let hex_content = fs::read_to_string(path)
        .map_err(|e| PluginLoaderError::fs(format!("Failed to read key file: {}", e)))?;

    hex::decode(hex_content.trim())
        .map_err(|e| PluginLoaderError::fs(format!("Failed to decode hex key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_keypair() {
        let result = generate_ed25519_keypair();
        assert!(result.is_ok());

        let (private_key, public_key) = result.unwrap();
        assert!(!private_key.is_empty());
        assert!(!public_key.is_empty());
    }

    #[test]
    fn test_save_and_load_keypair() {
        let temp_dir = TempDir::new().unwrap();
        let (private_key, public_key) = generate_ed25519_keypair().unwrap();

        // Save keys
        save_keypair(&private_key, &public_key, temp_dir.path(), "test-key").unwrap();

        // Load keys
        let loaded_private =
            load_key_from_file(&temp_dir.path().join("test-key.private.key")).unwrap();
        let loaded_public =
            load_key_from_file(&temp_dir.path().join("test-key.public.key")).unwrap();

        assert_eq!(private_key, loaded_private);
        assert_eq!(public_key, loaded_public);
    }

    #[test]
    fn test_sign_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let (private_key, _public_key) = generate_ed25519_keypair().unwrap();

        // Create a dummy plugin manifest
        let manifest_content = r#"
[info]
id = "test-plugin"
name = "Test Plugin"
version = "1.0.0"
description = "A test plugin"
author = "Test Author"
"#;
        fs::write(temp_dir.path().join("plugin.toml"), manifest_content).unwrap();

        // Sign the plugin
        let result = sign_plugin_ed25519(temp_dir.path(), "test-key", &private_key);
        assert!(result.is_ok());

        // Check that signature file was created
        assert!(temp_dir.path().join("plugin.sig").exists());

        // Read and verify signature format
        let sig_content = fs::read_to_string(temp_dir.path().join("plugin.sig")).unwrap();
        let signature: PluginSignature = serde_json::from_str(&sig_content).unwrap();
        assert_eq!(signature.algorithm, SignatureAlgorithm::Ed25519);
        assert_eq!(signature.key_id, "test-key");
    }
}
