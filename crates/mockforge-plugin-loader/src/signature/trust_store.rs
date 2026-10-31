//! Publisher key trust, revocation, expiry, and configured store resolution.
use super::{DEFAULT_TRUST_STORE_PATH, TRUST_STORE_ENV};
use crate::{LoaderResult, PluginLoaderConfig, PluginLoaderError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// One entry in a [`TrustStoreFile`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedKeyEntry {
    /// Key ID referenced by `plugin.sig`
    pub key_id: String,
    /// Hex-encoded public key bytes (32 bytes for Ed25519)
    pub public_key: String,
    /// Signatures from this key are rejected at or after this instant
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// Revoked keys are rejected even if also listed as trusted
    #[serde(default)]
    pub revoked: bool,
}

/// On-disk trust store for plugin signing keys.
///
/// ```json
/// { "keys": [ { "key_id": "acme-2026", "public_key": "<hex>",
///               "expires_at": "2027-01-01T00:00:00Z", "revoked": false } ] }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustStoreFile {
    /// Keys in the store
    #[serde(default)]
    pub keys: Vec<TrustedKeyEntry>,
}

impl TrustStoreFile {
    /// Read and parse a trust store file.
    pub fn load(path: &Path) -> LoaderResult<Self> {
        let raw = fs::read_to_string(path).map_err(|e| {
            PluginLoaderError::security(format!(
                "Failed to read trust store {}: {}",
                path.display(),
                e
            ))
        })?;
        serde_json::from_str(&raw).map_err(|e| {
            PluginLoaderError::security(format!(
                "Failed to parse trust store {}: {}",
                path.display(),
                e
            ))
        })
    }

    /// Add this store's keys to a loader configuration. Returns the
    /// number of keys now trusted from this file.
    pub fn apply_to(&self, config: &mut PluginLoaderConfig) -> LoaderResult<usize> {
        let mut trusted = 0;
        for entry in &self.keys {
            if entry.revoked {
                if !config.revoked_keys.contains(&entry.key_id) {
                    config.revoked_keys.push(entry.key_id.clone());
                }
                continue;
            }
            let key = hex::decode(entry.public_key.trim()).map_err(|e| {
                PluginLoaderError::security(format!(
                    "Invalid public key hex for key '{}': {}",
                    entry.key_id, e
                ))
            })?;
            if !config.trusted_keys.contains(&entry.key_id) {
                config.trusted_keys.push(entry.key_id.clone());
            }
            config.key_data.insert(entry.key_id.clone(), key);
            if let Some(expires_at) = entry.expires_at {
                config.key_expires_at.insert(entry.key_id.clone(), expires_at);
            }
            trusted += 1;
        }
        Ok(trusted)
    }
}

/// Resolve the trust store path: an explicit path, else
/// `MOCKFORGE_PLUGIN_TRUSTED_KEYS_FILE`, else
/// `~/.mockforge/trusted-keys.json` when it exists.
pub fn resolve_trust_store_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(path.to_path_buf());
    }
    if let Ok(path) = std::env::var(TRUST_STORE_ENV) {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(shellexpand::tilde(path.trim()).as_ref()));
        }
    }
    let default = PathBuf::from(shellexpand::tilde(DEFAULT_TRUST_STORE_PATH).as_ref());
    default.exists().then_some(default)
}
