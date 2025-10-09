//! Integration tests for plugin signature verification

use mockforge_plugin_loader::{
    generate_ed25519_keypair, sign_plugin_ed25519, LoaderResult, PluginLoaderConfig,
    PluginLoaderError, SignatureVerifier,
};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Create a test plugin directory with a manifest
fn create_test_plugin(dir: &TempDir) -> LoaderResult<()> {
    let manifest_content = r#"
[info]
id = "test-plugin"
name = "Test Plugin"
version = "1.0.0"
description = "A test plugin for signature verification"
author = "Test Author"

[capabilities]
http = false
database = false
file_system = false
network = false
"#;

    fs::write(dir.path().join("plugin.toml"), manifest_content).map_err(|e| {
        PluginLoaderError::fs(format!("Failed to write manifest: {}", e))
    })?;

    Ok(())
}

#[test]
fn test_signature_verification_success() {
    // Create test plugin
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Generate key pair
    let (private_key, public_key) = generate_ed25519_keypair().unwrap();

    // Sign the plugin
    let key_id = "test-key";
    let signature = sign_plugin_ed25519(plugin_dir.path(), key_id, &private_key).unwrap();

    // Verify signature is created
    assert!(plugin_dir.path().join("plugin.sig").exists());
    assert_eq!(signature.key_id, key_id);

    // Configure loader with public key
    let mut key_data = HashMap::new();
    key_data.insert(key_id.to_string(), public_key);

    let config = PluginLoaderConfig {
        trusted_keys: vec![key_id.to_string()],
        key_data,
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_ok(), "Signature verification should succeed");
}

#[test]
fn test_signature_verification_tampered_manifest() {
    // Create test plugin
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Generate key pair
    let (private_key, public_key) = generate_ed25519_keypair().unwrap();

    // Sign the plugin
    let key_id = "test-key";
    sign_plugin_ed25519(plugin_dir.path(), key_id, &private_key).unwrap();

    // Tamper with the manifest
    let tampered_content = r#"
[info]
id = "malicious-plugin"
name = "Malicious Plugin"
version = "1.0.0"
description = "Tampered plugin"
author = "Attacker"

[capabilities]
http = true
database = true
file_system = true
network = true
"#;
    fs::write(plugin_dir.path().join("plugin.toml"), tampered_content).unwrap();

    // Configure loader with public key
    let mut key_data = HashMap::new();
    key_data.insert(key_id.to_string(), public_key);

    let config = PluginLoaderConfig {
        trusted_keys: vec![key_id.to_string()],
        key_data,
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_err(), "Signature verification should fail for tampered manifest");

    // Check error message
    let error = result.unwrap_err();
    assert!(matches!(error, PluginLoaderError::SecurityViolation { .. }));
}

#[test]
fn test_signature_verification_untrusted_key() {
    // Create test plugin
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Generate key pair
    let (private_key, public_key) = generate_ed25519_keypair().unwrap();

    // Sign the plugin with "attacker-key"
    let signing_key_id = "attacker-key";
    sign_plugin_ed25519(plugin_dir.path(), signing_key_id, &private_key).unwrap();

    // Configure loader with different trusted key
    let mut key_data = HashMap::new();
    key_data.insert(signing_key_id.to_string(), public_key);

    let config = PluginLoaderConfig {
        trusted_keys: vec!["trusted-key".to_string()], // Different from signing key
        key_data,
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_err(), "Signature verification should fail for untrusted key");

    // Check error message
    let error = result.unwrap_err();
    assert!(matches!(error, PluginLoaderError::SecurityViolation { .. }));
}

#[test]
fn test_signature_verification_missing_signature() {
    // Create test plugin without signature
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Configure loader
    let config = PluginLoaderConfig {
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_err(), "Signature verification should fail when signature is missing");

    // Check error message
    let error = result.unwrap_err();
    assert!(matches!(error, PluginLoaderError::SecurityViolation { .. }));
}

#[test]
fn test_signature_verification_allow_unsigned() {
    // Create test plugin without signature
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Configure loader to allow unsigned plugins
    let config = PluginLoaderConfig {
        allow_unsigned: true,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_ok(), "Verification should succeed when unsigned plugins are allowed");
}

#[test]
fn test_signature_verification_missing_public_key_data() {
    // Create test plugin
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Generate key pair
    let (private_key, _public_key) = generate_ed25519_keypair().unwrap();

    // Sign the plugin
    let key_id = "test-key";
    sign_plugin_ed25519(plugin_dir.path(), key_id, &private_key).unwrap();

    // Configure loader with trusted key but no key data
    let config = PluginLoaderConfig {
        trusted_keys: vec![key_id.to_string()],
        key_data: HashMap::new(), // No public key data provided
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_err(), "Verification should fail when public key data is missing");

    // Check error message
    let error = result.unwrap_err();
    assert!(matches!(error, PluginLoaderError::SecurityViolation { .. }));
}

#[test]
fn test_signature_verification_wrong_public_key() {
    // Create test plugin
    let plugin_dir = TempDir::new().unwrap();
    create_test_plugin(&plugin_dir).unwrap();

    // Generate two different key pairs
    let (private_key1, _public_key1) = generate_ed25519_keypair().unwrap();
    let (_private_key2, public_key2) = generate_ed25519_keypair().unwrap();

    // Sign the plugin with key1
    let key_id = "test-key";
    sign_plugin_ed25519(plugin_dir.path(), key_id, &private_key1).unwrap();

    // Configure loader with public key from key2 (wrong key)
    let mut key_data = HashMap::new();
    key_data.insert(key_id.to_string(), public_key2);

    let config = PluginLoaderConfig {
        trusted_keys: vec![key_id.to_string()],
        key_data,
        allow_unsigned: false,
        ..Default::default()
    };

    // Verify the signature
    let verifier = SignatureVerifier::new(&config);
    let result = verifier.verify_plugin_signature(plugin_dir.path());
    assert!(result.is_err(), "Verification should fail with wrong public key");

    // Check error message
    let error = result.unwrap_err();
    assert!(matches!(error, PluginLoaderError::SecurityViolation { .. }));
}
