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
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::settings::OrgSetting;

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
        ApiToken::find_by_prefix(&self.pool, org_id, prefix).await.map_err(Into::into)
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

    async fn get_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<Option<OrgSetting>> {
        OrgSetting::get(&self.pool, org_id, key).await.map_err(Into::into)
    }

    async fn set_org_setting(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> StoreResult<OrgSetting> {
        OrgSetting::set(&self.pool, org_id, key, value).await.map_err(Into::into)
    }

    async fn delete_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<()> {
        OrgSetting::delete(&self.pool, org_id, key).await.map_err(Into::into)
    }

    async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        owner_id: Uuid,
        plan: Plan,
    ) -> StoreResult<Organization> {
        Organization::create(&self.pool, name, slug, owner_id, plan)
            .await
            .map_err(Into::into)
    }

    async fn find_organization_by_id(&self, org_id: Uuid) -> StoreResult<Option<Organization>> {
        Organization::find_by_id(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn find_organization_by_slug(&self, slug: &str) -> StoreResult<Option<Organization>> {
        Organization::find_by_slug(&self.pool, slug).await.map_err(Into::into)
    }

    async fn list_organizations_by_user(&self, user_id: Uuid) -> StoreResult<Vec<Organization>> {
        Organization::find_by_user(&self.pool, user_id).await.map_err(Into::into)
    }

    async fn update_organization_name(&self, org_id: Uuid, name: &str) -> StoreResult<()> {
        sqlx::query("UPDATE organizations SET name = $1, updated_at = NOW() WHERE id = $2")
            .bind(name)
            .bind(org_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn update_organization_slug(&self, org_id: Uuid, slug: &str) -> StoreResult<()> {
        sqlx::query("UPDATE organizations SET slug = $1, updated_at = NOW() WHERE id = $2")
            .bind(slug)
            .bind(org_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn update_organization_plan(&self, org_id: Uuid, plan: Plan) -> StoreResult<()> {
        Organization::update_plan(&self.pool, org_id, plan).await.map_err(Into::into)
    }

    async fn organization_has_active_subscription(&self, org_id: Uuid) -> StoreResult<bool> {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM subscriptions WHERE org_id = $1 AND status IN ('active', 'trialing'))",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn delete_organization(&self, org_id: Uuid) -> StoreResult<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn create_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<OrgMember> {
        OrgMember::create(&self.pool, org_id, user_id, role).await.map_err(Into::into)
    }

    async fn find_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<Option<OrgMember>> {
        OrgMember::find(&self.pool, org_id, user_id).await.map_err(Into::into)
    }

    async fn list_org_members(&self, org_id: Uuid) -> StoreResult<Vec<OrgMember>> {
        OrgMember::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_org_member_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<()> {
        OrgMember::update_role(&self.pool, org_id, user_id, role)
            .await
            .map_err(Into::into)
    }

    async fn delete_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<()> {
        OrgMember::delete(&self.pool, org_id, user_id).await.map_err(Into::into)
    }
}
