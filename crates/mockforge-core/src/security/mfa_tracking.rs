//! Multi-factor authentication (MFA) tracking for privileged users
//!
//! This module provides MFA status tracking and enforcement for privileged access management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// MFA method types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MfaMethod {
    /// Time-based one-time password (TOTP)
    Totp,
    /// SMS-based verification
    Sms,
    /// Email-based verification
    Email,
    /// Hardware security key (FIDO2/WebAuthn)
    HardwareKey,
    /// Push notification
    Push,
}

/// MFA status for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaStatus {
    /// User ID
    pub user_id: Uuid,
    /// Whether MFA is enabled
    pub enabled: bool,
    /// MFA methods configured
    pub methods: Vec<MfaMethod>,
    /// When MFA was enabled
    pub enabled_at: Option<DateTime<Utc>>,
    /// Last MFA verification
    pub last_verification: Option<DateTime<Utc>>,
    /// Backup codes remaining
    pub backup_codes_remaining: u32,
}

/// MFA storage trait
#[async_trait::async_trait]
pub trait MfaStorage: Send + Sync {
    /// Get MFA status for a user
    async fn get_mfa_status(&self, user_id: Uuid) -> Result<Option<MfaStatus>, crate::Error>;

    /// Set MFA status for a user
    async fn set_mfa_status(&self, status: MfaStatus) -> Result<(), crate::Error>;

    /// Get all users with MFA enabled
    async fn get_users_with_mfa(&self) -> Result<Vec<Uuid>, crate::Error>;

    /// Get all privileged users without MFA
    async fn get_privileged_users_without_mfa(
        &self,
        privileged_user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, crate::Error>;
}

/// In-memory MFA storage (for development/testing)
pub struct InMemoryMfaStorage {
    mfa_statuses: Arc<RwLock<HashMap<Uuid, MfaStatus>>>,
}

impl InMemoryMfaStorage {
    /// Create a new in-memory MFA storage
    pub fn new() -> Self {
        Self {
            mfa_statuses: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryMfaStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MfaStorage for InMemoryMfaStorage {
    async fn get_mfa_status(&self, user_id: Uuid) -> Result<Option<MfaStatus>, crate::Error> {
        let statuses = self.mfa_statuses.read().await;
        Ok(statuses.get(&user_id).cloned())
    }

    async fn set_mfa_status(&self, status: MfaStatus) -> Result<(), crate::Error> {
        let mut statuses = self.mfa_statuses.write().await;
        statuses.insert(status.user_id, status);
        Ok(())
    }

    async fn get_users_with_mfa(&self) -> Result<Vec<Uuid>, crate::Error> {
        let statuses = self.mfa_statuses.read().await;
        Ok(statuses
            .iter()
            .filter(|(_, status)| status.enabled)
            .map(|(user_id, _)| *user_id)
            .collect())
    }

    async fn get_privileged_users_without_mfa(
        &self,
        privileged_user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, crate::Error> {
        let statuses = self.mfa_statuses.read().await;
        Ok(privileged_user_ids
            .iter()
            .filter(|user_id| {
                statuses.get(user_id).map(|s| !s.enabled).unwrap_or(true) // If no status, assume MFA not enabled
            })
            .copied()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mfa_storage() {
        let storage = InMemoryMfaStorage::new();
        let user_id = Uuid::new_v4();
        let status = MfaStatus {
            user_id,
            enabled: true,
            methods: vec![MfaMethod::Totp],
            enabled_at: Some(Utc::now()),
            last_verification: Some(Utc::now()),
            backup_codes_remaining: 5,
        };

        storage.set_mfa_status(status).await.unwrap();
        let retrieved = storage.get_mfa_status(user_id).await.unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().enabled);
    }
}
