//! API token storage and management for access reviews
//!
//! This module provides storage and retrieval of API tokens for review purposes.

use crate::security::access_review::ApiTokenInfo;
use crate::Error;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// API token storage trait
///
/// This allows different storage backends (database, in-memory, etc.)
#[async_trait::async_trait]
pub trait ApiTokenStorage: Send + Sync {
    /// Get all API tokens
    async fn get_all_tokens(&self) -> Result<Vec<ApiTokenInfo>, Error>;

    /// Get token by ID
    async fn get_token(&self, token_id: &str) -> Result<Option<ApiTokenInfo>, Error>;

    /// Create a new token
    async fn create_token(&self, token: ApiTokenInfo) -> Result<(), Error>;

    /// Update token (e.g., last_used timestamp)
    async fn update_token(&self, token_id: &str, token: ApiTokenInfo) -> Result<(), Error>;

    /// Delete/revoke a token
    async fn revoke_token(&self, token_id: &str) -> Result<(), Error>;

    /// Get tokens by owner
    async fn get_tokens_by_owner(&self, owner_id: Uuid) -> Result<Vec<ApiTokenInfo>, Error>;
}

/// In-memory API token storage (for development/testing)
pub struct InMemoryApiTokenStorage {
    tokens: Arc<RwLock<HashMap<String, ApiTokenInfo>>>,
}

impl InMemoryApiTokenStorage {
    /// Create a new in-memory token storage
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryApiTokenStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ApiTokenStorage for InMemoryApiTokenStorage {
    async fn get_all_tokens(&self) -> Result<Vec<ApiTokenInfo>, Error> {
        let tokens = self.tokens.read().await;
        Ok(tokens.values().cloned().collect())
    }

    async fn get_token(&self, token_id: &str) -> Result<Option<ApiTokenInfo>, Error> {
        let tokens = self.tokens.read().await;
        Ok(tokens.get(token_id).cloned())
    }

    async fn create_token(&self, token: ApiTokenInfo) -> Result<(), Error> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(token.token_id.clone(), token);
        Ok(())
    }

    async fn update_token(&self, token_id: &str, token: ApiTokenInfo) -> Result<(), Error> {
        let mut tokens = self.tokens.write().await;
        tokens.insert(token_id.to_string(), token);
        Ok(())
    }

    async fn revoke_token(&self, token_id: &str) -> Result<(), Error> {
        let mut tokens = self.tokens.write().await;
        if let Some(mut token) = tokens.remove(token_id) {
            token.is_active = false;
            tokens.insert(token_id.to_string(), token);
        }
        Ok(())
    }

    async fn get_tokens_by_owner(&self, owner_id: Uuid) -> Result<Vec<ApiTokenInfo>, Error> {
        let tokens = self.tokens.read().await;
        Ok(tokens.values().filter(|t| t.owner_id == owner_id).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryApiTokenStorage::new();
        let token = ApiTokenInfo {
            token_id: "test-token".to_string(),
            name: Some("Test Token".to_string()),
            owner_id: Uuid::new_v4(),
            scopes: vec!["read".to_string(), "write".to_string()],
            created_at: Utc::now(),
            last_used: None,
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            days_unused: None,
            is_active: true,
        };

        storage.create_token(token.clone()).await.unwrap();
        let retrieved = storage.get_token("test-token").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().token_id, "test-token");
    }
}
