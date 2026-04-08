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
use crate::models::audit_log::{record_audit_event, AuditEventType, AuditLog};
use crate::models::cloud_fixture::CloudFixture;
use crate::models::cloud_service::CloudService;
use crate::models::cloud_workspace::Workspace as CloudWorkspace;
use crate::models::feature_usage::{FeatureType, FeatureUsage};
use crate::models::federation::Federation;
use crate::models::hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::settings::OrgSetting;
use crate::models::subscription::UsageCounter;
use crate::models::suspicious_activity::{
    record_suspicious_activity, SuspiciousActivity, SuspiciousActivityType,
};
use crate::models::user::User;
use crate::models::verification_token::VerificationToken;
use crate::models::waitlist::WaitlistSubscriber;

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

    async fn record_audit_event(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event_type: AuditEventType,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) {
        record_audit_event(
            &self.pool,
            org_id,
            user_id,
            event_type,
            description,
            metadata,
            ip_address,
            user_agent,
        )
        .await;
    }

    async fn list_audit_logs(
        &self,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<Vec<AuditLog>> {
        AuditLog::get_by_org(&self.pool, org_id, limit, offset, event_type)
            .await
            .map_err(Into::into)
    }

    async fn count_audit_logs(
        &self,
        org_id: Uuid,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<i64> {
        let count: (i64,) = if let Some(evt) = event_type {
            sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE org_id = $1 AND event_type = $2")
                .bind(org_id)
                .bind(evt)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE org_id = $1")
                .bind(org_id)
                .fetch_one(&self.pool)
                .await?
        };
        Ok(count.0)
    }

    async fn record_feature_usage(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        feature: FeatureType,
        metadata: Option<serde_json::Value>,
    ) {
        if let Err(e) = FeatureUsage::record(&self.pool, org_id, user_id, feature, metadata).await {
            tracing::warn!("Failed to record feature usage: {}", e);
        }
    }

    async fn count_feature_usage_by_org(
        &self,
        org_id: Uuid,
        feature: FeatureType,
        days: i64,
    ) -> StoreResult<i64> {
        FeatureUsage::count_by_org(&self.pool, org_id, feature, days)
            .await
            .map_err(Into::into)
    }

    async fn record_suspicious_activity(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        activity_type: SuspiciousActivityType,
        severity: &str,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) {
        record_suspicious_activity(
            &self.pool,
            org_id,
            user_id,
            activity_type,
            severity,
            description,
            metadata,
            ip_address,
            user_agent,
        )
        .await;
    }

    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> StoreResult<User> {
        User::create(&self.pool, username, email, password_hash)
            .await
            .map_err(Into::into)
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> StoreResult<Option<User>> {
        User::find_by_id(&self.pool, user_id).await.map_err(Into::into)
    }

    async fn find_user_by_email(&self, email: &str) -> StoreResult<Option<User>> {
        User::find_by_email(&self.pool, email).await.map_err(Into::into)
    }

    async fn find_user_by_username(&self, username: &str) -> StoreResult<Option<User>> {
        User::find_by_username(&self.pool, username).await.map_err(Into::into)
    }

    async fn find_users_by_ids(&self, ids: &[Uuid]) -> StoreResult<Vec<User>> {
        User::find_by_ids(&self.pool, ids).await.map_err(Into::into)
    }

    async fn set_user_api_token(&self, user_id: Uuid, token: &str) -> StoreResult<()> {
        User::set_api_token(&self.pool, user_id, token).await.map_err(Into::into)
    }

    async fn enable_user_2fa(
        &self,
        user_id: Uuid,
        secret: &str,
        backup_codes: &[String],
    ) -> StoreResult<()> {
        User::enable_2fa(&self.pool, user_id, secret, backup_codes)
            .await
            .map_err(Into::into)
    }

    async fn disable_user_2fa(&self, user_id: Uuid) -> StoreResult<()> {
        User::disable_2fa(&self.pool, user_id).await.map_err(Into::into)
    }

    async fn update_user_2fa_verified(&self, user_id: Uuid) -> StoreResult<()> {
        User::update_2fa_verified(&self.pool, user_id).await.map_err(Into::into)
    }

    async fn remove_user_backup_code(&self, user_id: Uuid, code_index: usize) -> StoreResult<()> {
        User::remove_backup_code(&self.pool, user_id, code_index)
            .await
            .map_err(Into::into)
    }

    async fn update_user_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> StoreResult<()> {
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn mark_user_verified(&self, user_id: Uuid) -> StoreResult<()> {
        sqlx::query("UPDATE users SET is_verified = TRUE, updated_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    async fn create_verification_token(&self, user_id: Uuid) -> StoreResult<VerificationToken> {
        VerificationToken::create(&self.pool, user_id).await.map_err(Into::into)
    }

    async fn set_verification_token_expiry_hours(
        &self,
        token_id: Uuid,
        hours: i64,
    ) -> StoreResult<()> {
        sqlx::query(
            "UPDATE verification_tokens SET expires_at = NOW() + make_interval(hours => $1) WHERE id = $2",
        )
        .bind(hours)
        .bind(token_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    async fn find_verification_token_by_token(
        &self,
        token: &str,
    ) -> StoreResult<Option<VerificationToken>> {
        VerificationToken::find_by_token(&self.pool, token).await.map_err(Into::into)
    }

    async fn mark_verification_token_used(&self, token_id: Uuid) -> StoreResult<()> {
        VerificationToken::mark_as_used(&self.pool, token_id).await.map_err(Into::into)
    }

    async fn create_federation(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> StoreResult<Federation> {
        Federation::create(&self.pool, org_id, created_by, name, description, services)
            .await
            .map_err(Into::into)
    }

    async fn find_federation_by_id(&self, id: Uuid) -> StoreResult<Option<Federation>> {
        Federation::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_federations_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Federation>> {
        Federation::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_federation(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> StoreResult<Option<Federation>> {
        Federation::update(&self.pool, id, name, description, services)
            .await
            .map_err(Into::into)
    }

    async fn delete_federation(&self, id: Uuid) -> StoreResult<()> {
        Federation::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_unresolved_suspicious_activities(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> StoreResult<Vec<SuspiciousActivity>> {
        SuspiciousActivity::get_unresolved(&self.pool, org_id, user_id, severity, limit)
            .await
            .map_err(Into::into)
    }

    async fn count_unresolved_suspicious_activities(&self, org_id: Uuid) -> StoreResult<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM suspicious_activities WHERE org_id = $1 AND resolved = FALSE",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn resolve_suspicious_activity(
        &self,
        activity_id: Uuid,
        resolved_by: Uuid,
    ) -> StoreResult<()> {
        SuspiciousActivity::resolve(&self.pool, activity_id, resolved_by)
            .await
            .map_err(Into::into)
    }

    async fn create_cloud_workspace(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
    ) -> StoreResult<CloudWorkspace> {
        CloudWorkspace::create(&self.pool, org_id, created_by, name, description)
            .await
            .map_err(Into::into)
    }

    async fn find_cloud_workspace_by_id(&self, id: Uuid) -> StoreResult<Option<CloudWorkspace>> {
        CloudWorkspace::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_cloud_workspaces_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudWorkspace>> {
        CloudWorkspace::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_cloud_workspace(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudWorkspace>> {
        CloudWorkspace::update(&self.pool, id, name, description, is_active, settings)
            .await
            .map_err(Into::into)
    }

    async fn delete_cloud_workspace(&self, id: Uuid) -> StoreResult<()> {
        CloudWorkspace::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn create_cloud_service(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        base_url: &str,
    ) -> StoreResult<CloudService> {
        CloudService::create(&self.pool, org_id, created_by, name, description, base_url)
            .await
            .map_err(Into::into)
    }

    async fn find_cloud_service_by_id(&self, id: Uuid) -> StoreResult<Option<CloudService>> {
        CloudService::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_cloud_services_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudService>> {
        CloudService::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_cloud_service(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        base_url: Option<&str>,
        enabled: Option<bool>,
        tags: Option<&serde_json::Value>,
        routes: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudService>> {
        CloudService::update(&self.pool, id, name, description, base_url, enabled, tags, routes)
            .await
            .map_err(Into::into)
    }

    async fn delete_cloud_service(&self, id: Uuid) -> StoreResult<()> {
        CloudService::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn create_cloud_fixture(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        path: &str,
        method: &str,
        content: Option<&serde_json::Value>,
    ) -> StoreResult<CloudFixture> {
        CloudFixture::create(
            &self.pool,
            org_id,
            created_by,
            name,
            description,
            path,
            method,
            content,
        )
        .await
        .map_err(Into::into)
    }

    async fn find_cloud_fixture_by_id(&self, id: Uuid) -> StoreResult<Option<CloudFixture>> {
        CloudFixture::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_cloud_fixtures_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudFixture>> {
        CloudFixture::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_cloud_fixture(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        path: Option<&str>,
        method: Option<&str>,
        content: Option<&serde_json::Value>,
        tags: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudFixture>> {
        CloudFixture::update(&self.pool, id, name, description, path, method, content, tags)
            .await
            .map_err(Into::into)
    }

    async fn delete_cloud_fixture(&self, id: Uuid) -> StoreResult<()> {
        CloudFixture::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn create_hosted_mock(
        &self,
        org_id: Uuid,
        project_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: Option<&str>,
        config_json: serde_json::Value,
        openapi_spec_url: Option<&str>,
        region: Option<&str>,
    ) -> StoreResult<HostedMock> {
        HostedMock::create(
            &self.pool,
            org_id,
            project_id,
            name,
            slug,
            description,
            config_json,
            openapi_spec_url,
            region,
        )
        .await
        .map_err(Into::into)
    }

    async fn find_hosted_mock_by_id(&self, id: Uuid) -> StoreResult<Option<HostedMock>> {
        HostedMock::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn find_hosted_mock_by_slug(
        &self,
        org_id: Uuid,
        slug: &str,
    ) -> StoreResult<Option<HostedMock>> {
        HostedMock::find_by_slug(&self.pool, org_id, slug).await.map_err(Into::into)
    }

    async fn list_hosted_mocks_by_org(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>> {
        HostedMock::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_hosted_mock_status(
        &self,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> StoreResult<()> {
        HostedMock::update_status(&self.pool, id, status, error_message)
            .await
            .map_err(Into::into)
    }

    async fn update_hosted_mock_urls(
        &self,
        id: Uuid,
        deployment_url: Option<&str>,
        internal_url: Option<&str>,
    ) -> StoreResult<()> {
        HostedMock::update_urls(&self.pool, id, deployment_url, internal_url)
            .await
            .map_err(Into::into)
    }

    async fn update_hosted_mock_health(
        &self,
        id: Uuid,
        health_status: HealthStatus,
        health_check_url: Option<&str>,
    ) -> StoreResult<()> {
        HostedMock::update_health(&self.pool, id, health_status, health_check_url)
            .await
            .map_err(Into::into)
    }

    async fn delete_hosted_mock(&self, id: Uuid) -> StoreResult<()> {
        HostedMock::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn subscribe_waitlist(
        &self,
        email: &str,
        source: &str,
    ) -> StoreResult<WaitlistSubscriber> {
        WaitlistSubscriber::subscribe(&self.pool, email, source)
            .await
            .map_err(Into::into)
    }

    async fn unsubscribe_waitlist_by_token(&self, token: Uuid) -> StoreResult<bool> {
        WaitlistSubscriber::unsubscribe_by_token(&self.pool, token)
            .await
            .map_err(Into::into)
    }

    async fn get_or_create_current_usage_counter(&self, org_id: Uuid) -> StoreResult<UsageCounter> {
        UsageCounter::get_or_create_current(&self.pool, org_id)
            .await
            .map_err(Into::into)
    }

    async fn list_usage_counters_by_org(&self, org_id: Uuid) -> StoreResult<Vec<UsageCounter>> {
        UsageCounter::get_all_for_org(&self.pool, org_id).await.map_err(Into::into)
    }
}
