//! Token lifecycle management
//!
//! This module provides functionality for managing token lifecycle scenarios:
//! - Token revocation tracking
//! - Key rotation management
//! - Clock skew simulation
//! - Prebuilt test scenarios

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use mockforge_core::Error;

/// Token revocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedToken {
    /// Token identifier (jti claim or full token hash)
    pub token_id: String,
    /// User ID (sub claim)
    pub user_id: Option<String>,
    /// When the token was revoked
    pub revoked_at: i64,
    /// Reason for revocation
    pub reason: String,
    /// Token expiration time (if known)
    pub expires_at: Option<i64>,
}

/// Token revocation store
#[derive(Debug, Clone)]
pub struct TokenRevocationStore {
    /// Map of token_id -> RevokedToken
    revoked_tokens: Arc<RwLock<HashMap<String, RevokedToken>>>,
    /// Map of user_id -> set of revoked token IDs
    user_revoked_tokens: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl TokenRevocationStore {
    /// Create a new token revocation store
    pub fn new() -> Self {
        Self {
            revoked_tokens: Arc::new(RwLock::new(HashMap::new())),
            user_revoked_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Revoke a token
    pub async fn revoke_token(
        &self,
        token_id: String,
        user_id: Option<String>,
        reason: String,
        expires_at: Option<i64>,
    ) {
        let revoked = RevokedToken {
            token_id: token_id.clone(),
            user_id: user_id.clone(),
            revoked_at: Utc::now().timestamp(),
            reason,
            expires_at,
        };

        let mut tokens = self.revoked_tokens.write().await;
        tokens.insert(token_id.clone(), revoked);

        if let Some(uid) = user_id {
            let mut user_tokens = self.user_revoked_tokens.write().await;
            user_tokens.entry(uid).or_insert_with(Vec::new).push(token_id);
        }
    }

    /// Revoke all tokens for a user
    pub async fn revoke_user_tokens(&self, user_id: String, reason: String) {
        let user_tokens = self.user_revoked_tokens.write().await;
        if let Some(token_ids) = user_tokens.get(&user_id) {
            let mut tokens = self.revoked_tokens.write().await;
            for token_id in token_ids {
                if let Some(revoked) = tokens.get_mut(token_id) {
                    revoked.revoked_at = Utc::now().timestamp();
                    revoked.reason = reason.clone();
                }
            }
        }
    }

    /// Check if a token is revoked
    pub async fn is_revoked(&self, token_id: &str) -> Option<RevokedToken> {
        let tokens = self.revoked_tokens.read().await;
        tokens.get(token_id).cloned()
    }

    /// Get revocation status
    pub async fn get_revocation_status(&self, token_id: &str) -> Option<RevokedToken> {
        self.is_revoked(token_id).await
    }

    /// Clean up expired revoked tokens
    pub async fn cleanup_expired(&self) {
        let now = Utc::now().timestamp();
        let mut tokens = self.revoked_tokens.write().await;
        tokens.retain(|_, revoked| revoked.expires_at.is_none_or(|exp| exp > now));
    }
}

impl Default for TokenRevocationStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Key rotation state
#[derive(Debug, Clone)]
pub struct KeyRotationState {
    /// Active keys (kid -> key info)
    active_keys: Arc<RwLock<HashMap<String, ActiveKey>>>,
    /// Grace period for old keys (seconds)
    grace_period_seconds: i64,
}

/// Active key information
#[derive(Debug, Clone)]
pub struct ActiveKey {
    /// Key ID
    pub kid: String,
    /// When the key was created
    pub created_at: i64,
    /// When the key becomes inactive (after grace period)
    pub inactive_at: Option<i64>,
    /// Whether this is the primary key
    pub is_primary: bool,
}

impl KeyRotationState {
    /// Create new key rotation state
    pub fn new(grace_period_seconds: i64) -> Self {
        Self {
            active_keys: Arc::new(RwLock::new(HashMap::new())),
            grace_period_seconds,
        }
    }

    /// Add a new key
    pub async fn add_key(&self, kid: String, is_primary: bool) {
        let mut keys = self.active_keys.write().await;
        keys.insert(
            kid.clone(),
            ActiveKey {
                kid,
                created_at: Utc::now().timestamp(),
                inactive_at: None,
                is_primary,
            },
        );
    }

    /// Rotate to a new key
    pub async fn rotate_key(&self, new_kid: String) -> Result<(), Error> {
        let mut keys = self.active_keys.write().await;

        // Mark all existing keys as non-primary
        for key in keys.values_mut() {
            key.is_primary = false;
            // Set inactive_at after grace period
            key.inactive_at = Some(Utc::now().timestamp() + self.grace_period_seconds);
        }

        // Add new primary key
        keys.insert(
            new_kid.clone(),
            ActiveKey {
                kid: new_kid,
                created_at: Utc::now().timestamp(),
                inactive_at: None,
                is_primary: true,
            },
        );

        Ok(())
    }

    /// Get active keys (including those in grace period)
    pub async fn get_active_keys(&self) -> Vec<ActiveKey> {
        let now = Utc::now().timestamp();
        let keys = self.active_keys.read().await;
        keys.values()
            .filter(|key| key.inactive_at.is_none_or(|inactive_at| inactive_at > now))
            .cloned()
            .collect()
    }

    /// Get primary key
    pub async fn get_primary_key(&self) -> Option<ActiveKey> {
        let keys = self.active_keys.read().await;
        keys.values().find(|key| key.is_primary).cloned()
    }

    /// Remove old keys (after grace period)
    pub async fn cleanup_old_keys(&self) {
        let now = Utc::now().timestamp();
        let mut keys = self.active_keys.write().await;
        keys.retain(|_, key| key.inactive_at.is_none_or(|inactive_at| inactive_at > now));
    }
}

/// Clock skew configuration
#[derive(Debug, Clone)]
pub struct ClockSkewState {
    /// Clock skew in seconds (positive = server ahead, negative = server behind)
    skew_seconds: Arc<RwLock<i64>>,
    /// Whether to apply skew to token issuance
    apply_to_issuance: bool,
    /// Whether to apply skew to token validation
    apply_to_validation: bool,
}

impl ClockSkewState {
    /// Create new clock skew state
    pub fn new() -> Self {
        Self {
            skew_seconds: Arc::new(RwLock::new(0)),
            apply_to_issuance: true,
            apply_to_validation: true,
        }
    }

    /// Set clock skew
    pub async fn set_skew(&self, skew_seconds: i64) {
        let mut skew = self.skew_seconds.write().await;
        *skew = skew_seconds;
    }

    /// Get current clock skew
    pub async fn get_skew(&self) -> i64 {
        let skew = self.skew_seconds.read().await;
        *skew
    }

    /// Get adjusted time (current time + skew)
    pub async fn get_adjusted_time(&self) -> i64 {
        let skew = self.skew_seconds.read().await;
        Utc::now().timestamp() + *skew
    }

    /// Apply skew to a timestamp (for issuance)
    pub async fn apply_issuance_skew(&self, timestamp: i64) -> i64 {
        if self.apply_to_issuance {
            let skew = self.skew_seconds.read().await;
            timestamp + *skew
        } else {
            timestamp
        }
    }

    /// Apply skew to a timestamp (for validation)
    pub async fn apply_validation_skew(&self, timestamp: i64) -> i64 {
        if self.apply_to_validation {
            let skew = self.skew_seconds.read().await;
            timestamp - *skew
        } else {
            timestamp
        }
    }
}

impl Default for ClockSkewState {
    fn default() -> Self {
        Self::new()
    }
}

/// Token lifecycle manager combining all lifecycle features
#[derive(Debug, Clone)]
pub struct TokenLifecycleManager {
    /// Token revocation store
    pub revocation: TokenRevocationStore,
    /// Key rotation state
    pub key_rotation: KeyRotationState,
    /// Clock skew state
    pub clock_skew: ClockSkewState,
}

impl TokenLifecycleManager {
    /// Create new token lifecycle manager
    pub fn new(grace_period_seconds: i64) -> Self {
        Self {
            revocation: TokenRevocationStore::new(),
            key_rotation: KeyRotationState::new(grace_period_seconds),
            clock_skew: ClockSkewState::new(),
        }
    }
}

impl Default for TokenLifecycleManager {
    fn default() -> Self {
        Self::new(3600) // 1 hour default grace period
    }
}

/// Extract token ID from JWT (using jti claim or token hash)
pub fn extract_token_id(token: &str) -> String {
    // For now, use a hash of the token as ID
    // In production, prefer jti claim from decoded token
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
