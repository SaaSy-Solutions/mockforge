//! Access justification storage for privileged access management
//!
//! This module provides storage and retrieval of access justifications
//! for privileged users.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Access justification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessJustification {
    /// User ID
    pub user_id: Uuid,
    /// Justification text
    pub justification: String,
    /// Business need description
    pub business_need: Option<String>,
    /// Requested by (manager/user ID)
    pub requested_by: Option<Uuid>,
    /// Approved by
    pub approved_by: Option<Uuid>,
    /// Approval date
    pub approved_at: Option<DateTime<Utc>>,
    /// Expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Created date
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl AccessJustification {
    /// Create a new justification
    pub fn new(
        user_id: Uuid,
        justification: String,
        business_need: Option<String>,
        requested_by: Option<Uuid>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            justification,
            business_need,
            requested_by,
            approved_by: None,
            approved_at: None,
            expires_at,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if justification is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
    }

    /// Check if justification is approved
    pub fn is_approved(&self) -> bool {
        self.approved_by.is_some() && self.approved_at.is_some()
    }
}

/// Justification storage trait
#[async_trait::async_trait]
pub trait JustificationStorage: Send + Sync {
    /// Get justification for a user
    async fn get_justification(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AccessJustification>, crate::Error>;

    /// Set justification for a user
    async fn set_justification(
        &self,
        justification: AccessJustification,
    ) -> Result<(), crate::Error>;

    /// Get all justifications
    async fn get_all_justifications(&self) -> Result<Vec<AccessJustification>, crate::Error>;

    /// Get expired justifications
    async fn get_expired_justifications(&self) -> Result<Vec<AccessJustification>, crate::Error>;

    /// Delete justification for a user
    async fn delete_justification(&self, user_id: Uuid) -> Result<(), crate::Error>;
}

/// In-memory justification storage (for development/testing)
pub struct InMemoryJustificationStorage {
    justifications: Arc<RwLock<HashMap<Uuid, AccessJustification>>>,
}

impl InMemoryJustificationStorage {
    /// Create a new in-memory justification storage
    pub fn new() -> Self {
        Self {
            justifications: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryJustificationStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl JustificationStorage for InMemoryJustificationStorage {
    async fn get_justification(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AccessJustification>, crate::Error> {
        let justifications = self.justifications.read().await;
        Ok(justifications.get(&user_id).cloned())
    }

    async fn set_justification(
        &self,
        justification: AccessJustification,
    ) -> Result<(), crate::Error> {
        let mut justifications = self.justifications.write().await;
        justifications.insert(justification.user_id, justification);
        Ok(())
    }

    async fn get_all_justifications(&self) -> Result<Vec<AccessJustification>, crate::Error> {
        let justifications = self.justifications.read().await;
        Ok(justifications.values().cloned().collect())
    }

    async fn get_expired_justifications(&self) -> Result<Vec<AccessJustification>, crate::Error> {
        let justifications = self.justifications.read().await;
        Ok(justifications.values().filter(|j| j.is_expired()).cloned().collect())
    }

    async fn delete_justification(&self, user_id: Uuid) -> Result<(), crate::Error> {
        let mut justifications = self.justifications.write().await;
        justifications.remove(&user_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_justification_storage() {
        let storage = InMemoryJustificationStorage::new();
        let user_id = Uuid::new_v4();
        let justification = AccessJustification::new(
            user_id,
            "Required for system administration".to_string(),
            Some("Manage production infrastructure".to_string()),
            Some(Uuid::new_v4()),
            Some(Utc::now() + chrono::Duration::days(365)),
        );

        storage.set_justification(justification.clone()).await.unwrap();
        let retrieved = storage.get_justification(user_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().justification, "Required for system administration");
    }

    #[test]
    fn test_justification_expiration() {
        let justification = AccessJustification::new(
            Uuid::new_v4(),
            "Test".to_string(),
            None,
            None,
            Some(Utc::now() - chrono::Duration::days(1)), // Expired
        );

        assert!(justification.is_expired());
    }
}
