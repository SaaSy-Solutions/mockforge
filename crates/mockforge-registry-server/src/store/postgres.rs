//! PostgreSQL implementation of [`RegistryStore`].
//!
//! This is a thin adapter over the existing inherent methods on
//! [`crate::models::api_token::ApiToken`]. Later phases will move the SQL
//! directly into these impls and delete the inherent methods.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{RegistryStore, StoreResult};
use crate::models::api_token::{ApiToken, TokenScope};

/// Postgres-backed [`RegistryStore`] implementation.
#[derive(Clone)]
pub struct PgRegistryStore {
    pool: PgPool,
}

impl PgRegistryStore {
    /// Wrap an existing [`PgPool`] in a registry store.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool. Exposed so the binary bootstrap can still
    /// share the pool with SaaS-only subsystems (deployment orchestrator,
    /// worker tasks, etc.) that have not yet been migrated to the trait.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl RegistryStore for PgRegistryStore {
    async fn health_check(&self) -> StoreResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn create_api_token(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name: &str,
        scopes: &[TokenScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<(String, ApiToken)> {
        ApiToken::create(&self.pool, org_id, user_id, name, scopes, expires_at)
            .await
            .map_err(Into::into)
    }

    async fn find_api_token_by_id(&self, token_id: Uuid) -> StoreResult<Option<ApiToken>> {
        ApiToken::find_by_id(&self.pool, token_id).await.map_err(Into::into)
    }

    async fn list_api_tokens_by_org(&self, org_id: Uuid) -> StoreResult<Vec<ApiToken>> {
        ApiToken::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn find_api_token_by_prefix(
        &self,
        org_id: Uuid,
        prefix: &str,
    ) -> StoreResult<Option<ApiToken>> {
        ApiToken::find_by_prefix(&self.pool, org_id, prefix)
            .await
            .map_err(Into::into)
    }

    async fn verify_api_token(&self, token: &str) -> StoreResult<Option<ApiToken>> {
        ApiToken::verify_token(&self.pool, token).await.map_err(Into::into)
    }

    async fn delete_api_token(&self, token_id: Uuid) -> StoreResult<()> {
        ApiToken::delete(&self.pool, token_id).await.map_err(Into::into)
    }

    async fn rotate_api_token(
        &self,
        token_id: Uuid,
        new_name: Option<&str>,
        delete_old: bool,
    ) -> StoreResult<(String, ApiToken, Option<ApiToken>)> {
        ApiToken::rotate(&self.pool, token_id, new_name, delete_old)
            .await
            .map_err(Into::into)
    }

    async fn find_api_tokens_needing_rotation(
        &self,
        org_id: Option<Uuid>,
        days_old: i64,
    ) -> StoreResult<Vec<ApiToken>> {
        ApiToken::find_tokens_needing_rotation(&self.pool, org_id, days_old)
            .await
            .map_err(Into::into)
    }
}
