# Plugin Signature Verification

This document explains how to use the plugin signature verification feature in MockForge.

## Overview

MockForge plugins support cryptographic signature verification using:
- **Ed25519**: Fast, modern elliptic curve signatures (recommended)
- **RSA PKCS#1 v1.5**: Traditional RSA signatures with SHA-256 (2048, 3072, or 4096-bit keys)

Signature verification ensures that plugins haven't been tampered with and come from trusted sources.

## Generating Keys

### Ed25519 Keys (Recommended)

```rust
use mockforge_plugin_loader::{generate_ed25519_keypair, save_keypair};

// Generate a new key pair
let (private_key, public_key) = generate_ed25519_keypair().unwrap();

// Save to files
save_keypair(
    &private_key,
    &public_key,
    Path::new("./keys"),
    "my-signing-key"
).unwrap();

// This creates:
// - ./keys/my-signing-key.private.key (hex-encoded private key)
// - ./keys/my-signing-key.public.key (hex-encoded public key)
```

## Signing a Plugin

To sign a plugin, you need:
1. The plugin directory containing `plugin.toml`
2. Your private key (PKCS#8 format)
3. A key ID to identify which key was used

```rust
use mockforge_plugin_loader::{sign_plugin_ed25519, load_key_from_file};
use std::path::Path;

// Load your private key
let private_key = load_key_from_file(Path::new("./keys/my-signing-key.private.key")).unwrap();

// Sign the plugin
let signature = sign_plugin_ed25519(
    Path::new("./my-plugin"),
    "my-signing-key",  // Key ID
    &private_key
).unwrap();

// This creates ./my-plugin/plugin.sig
```

## Signature File Format

The `plugin.sig` file is JSON:

```json
{
  "algorithm": "ED25519",
  "key_id": "my-signing-key",
  "signature": "hex-encoded-signature-bytes",
  "signed_content_hash": "hex-encoded-sha256-hash-of-manifest"
}
```

## Verifying Signatures

### Configuration

Configure the plugin loader with trusted keys:

```rust
use mockforge_plugin_loader::{PluginLoaderConfig, load_key_from_file};
use std::collections::HashMap;

// Load public keys
let public_key = load_key_from_file(
    Path::new("./keys/my-signing-key.public.key")
).unwrap();

// Create key data map
let mut key_data = HashMap::new();
key_data.insert("my-signing-key".to_string(), public_key);

// Configure loader
let config = PluginLoaderConfig {
    trusted_keys: vec!["my-signing-key".to_string()],
    key_data,
    allow_unsigned: false,  // Reject unsigned plugins
    ..Default::default()
};
```

### Using the Installer

The `PluginInstaller` automatically verifies signatures when `verify_signature` is enabled:

```rust
use mockforge_plugin_loader::{PluginInstaller, InstallOptions};

let installer = PluginInstaller::new(config).unwrap();

let options = InstallOptions {
    verify_signature: true,  // Enable signature verification
    force: false,
    skip_validation: false,
    expected_checksum: None,
};

// This will verify the signature before installing
installer.install("./my-plugin", options).await.unwrap();
```

### Manual Verification

You can also verify signatures manually:

```rust
use mockforge_plugin_loader::SignatureVerifier;

let verifier = SignatureVerifier::new(&config);
let result = verifier.verify_plugin_signature(Path::new("./my-plugin"));

match result {
    Ok(_) => println!("✓ Signature verified successfully"),
    Err(e) => eprintln!("✗ Signature verification failed: {}", e),
}
```

## Security Considerations

### Verification Process

The signature verification process:
1. Reads `plugin.sig` from the plugin directory
2. Checks that the signing key ID is in the trusted keys list
3. Retrieves the corresponding public key from `key_data`
4. Computes SHA-256 hash of `plugin.toml`
5. Verifies the hash matches the signed content hash
6. Verifies the cryptographic signature using the public key

### What is Protected

- **Plugin Manifest**: The `plugin.toml` file is signed and verified
- **Tampering Detection**: Any modification to the manifest after signing will cause verification to fail
- **Key Trust**: Only signatures from trusted keys are accepted

### What is NOT Protected

- **Plugin Code**: The WASM binary itself is not directly signed (only the manifest that references it)
- **Configuration Files**: Additional files in the plugin directory are not signed
- **Replay Attacks**: Signatures don't include timestamps or nonces

### Best Practices

1. **Keep Private Keys Secure**: Never commit private keys to version control
2. **Use Strong Keys**: Ed25519 (recommended) or RSA 3072/4096-bit
3. **Rotate Keys**: Periodically generate new keys and re-sign plugins
4. **Maintain Key Lists**: Keep trusted keys list minimal and up-to-date
5. **Disable Unsigned in Production**: Set `allow_unsigned: false` in production
6. **Verify Sources**: Only add keys from trusted plugin developers

## Development Mode

For development, you can allow unsigned plugins:

```rust
let config = PluginLoaderConfig {
    allow_unsigned: true,  // Allow unsigned plugins
    ..Default::default()
};
```

This is useful during plugin development but should be disabled in production.

## Troubleshooting

### "No signature file found"
- The plugin directory must contain `plugin.sig`
- If `allow_unsigned: false`, all plugins must be signed

### "Signature key is not in trusted keys list"
- Add the key ID to `config.trusted_keys`
- Ensure the key ID in `plugin.sig` matches a trusted key

### "Public key data not found"
- Add the public key bytes to `config.key_data`
- Ensure the key ID matches between `trusted_keys` and `key_data`

### "Signature verification failed"
- The signature is invalid or corrupted
- The public key doesn't match the private key used for signing
- The plugin manifest was modified after signing

### "Plugin manifest hash does not match"
- The `plugin.toml` was modified after signing
- Re-sign the plugin with the correct manifest content

## Example: Complete Workflow

```rust
use mockforge_plugin_loader::*;
use std::collections::HashMap;
use std::path::Path;

// 1. Generate keys (one-time setup)
let (private_key, public_key) = generate_ed25519_keypair().unwrap();
save_keypair(&private_key, &public_key, Path::new("."), "dev-key").unwrap();

// 2. Sign your plugin
sign_plugin_ed25519(
    Path::new("./my-plugin"),
    "dev-key",
    &private_key
).unwrap();

// 3. Configure loader with public key
let mut key_data = HashMap::new();
key_data.insert("dev-key".to_string(), public_key);

let config = PluginLoaderConfig {
    trusted_keys: vec!["dev-key".to_string()],
    key_data,
    allow_unsigned: false,
    ..Default::default()
};

// 4. Install plugin with verification
let installer = PluginInstaller::new(config).unwrap();
let options = InstallOptions {
    verify_signature: true,
    ..Default::default()
};

installer.install("./my-plugin", options).await.unwrap();
println!("Plugin installed and verified!");
```

## API Reference

### Key Generation
- `generate_ed25519_keypair() -> (Vec<u8>, Vec<u8>)` - Generate Ed25519 key pair

### Key Management
- `save_keypair(private, public, dir, name)` - Save key pair to files
- `load_key_from_file(path) -> Vec<u8>` - Load hex-encoded key from file

### Signing
- `sign_plugin_ed25519(plugin_dir, key_id, private_key) -> PluginSignature` - Sign a plugin

### Verification
- `SignatureVerifier::new(config) -> SignatureVerifier` - Create verifier
- `verifier.verify_plugin_signature(plugin_dir) -> Result<()>` - Verify plugin signature

### Types
- `SignatureAlgorithm` - Enum of supported algorithms (Ed25519, RSA variants)
- `PluginSignature` - Signature metadata structure
- `PluginLoaderConfig` - Configuration with `trusted_keys` and `key_data`
