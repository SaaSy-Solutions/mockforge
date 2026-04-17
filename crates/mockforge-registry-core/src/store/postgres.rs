//! PostgreSQL implementation of [`RegistryStore`].
//!
//! This is a thin adapter over the existing inherent methods on
//! [`crate::models::api_token::ApiToken`]. Later phases will move the SQL
//! directly into these impls and delete the inherent methods.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    AdminAnalyticsSnapshot, ConversionFunnelSnapshot, OrgSettingRow, ProjectRow, RegistryStore,
    StoreResult, SubscriptionRow, UserSettingRow,
};
use crate::models::api_token::{ApiToken, TokenScope};
use crate::models::audit_log::{record_audit_event, AuditEventType, AuditLog};
use crate::models::cloud_fixture::CloudFixture;
use crate::models::cloud_service::CloudService;
use crate::models::cloud_workspace::Workspace as CloudWorkspace;
use crate::models::feature_usage::{FeatureType, FeatureUsage};
use crate::models::federation::Federation;
use crate::models::hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
use crate::models::org_template::OrgTemplate;
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::plugin::{PendingScanJob, Plugin, PluginSecurityScan, PluginVersion};
use crate::models::review::Review;
use crate::models::saml_assertion::SAMLAssertionId;
use crate::models::scenario::Scenario;
use crate::models::scenario_review::ScenarioReview;
use crate::models::settings::OrgSetting;
use crate::models::sso::{SSOConfiguration, SSOProvider};
use crate::models::subscription::UsageCounter;
use crate::models::suspicious_activity::{
    record_suspicious_activity, SuspiciousActivity, SuspiciousActivityType,
};
use crate::models::template::{Template, TemplateCategory};
use crate::models::template_review::TemplateReview;
use crate::models::template_star::TemplateStar;
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

    async fn find_user_by_github_id(&self, github_id: &str) -> StoreResult<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE github_id = $1")
            .bind(github_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn find_user_by_google_id(&self, google_id: &str) -> StoreResult<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE google_id = $1")
            .bind(google_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn link_user_github_account(
        &self,
        user_id: Uuid,
        github_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()> {
        sqlx::query(
            "UPDATE users SET github_id = $1, auth_provider = 'github', avatar_url = $2 WHERE id = $3",
        )
        .bind(github_id)
        .bind(avatar_url)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    async fn link_user_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()> {
        sqlx::query(
            "UPDATE users SET google_id = $1, auth_provider = 'google', avatar_url = $2 WHERE id = $3",
        )
        .bind(google_id)
        .bind(avatar_url)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    async fn create_oauth_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        auth_provider: &str,
        github_id: Option<&str>,
        google_id: Option<&str>,
        avatar_url: Option<&str>,
    ) -> StoreResult<User> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, auth_provider, github_id, google_id, avatar_url, is_verified)
            VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(auth_provider)
        .bind(github_id)
        .bind(google_id)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn get_or_create_personal_org(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> StoreResult<Organization> {
        Organization::get_or_create_personal_org(&self.pool, user_id, username)
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

    async fn find_sso_config_by_org(&self, org_id: Uuid) -> StoreResult<Option<SSOConfiguration>> {
        SSOConfiguration::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn upsert_sso_config(
        &self,
        org_id: Uuid,
        provider: SSOProvider,
        saml_entity_id: Option<&str>,
        saml_sso_url: Option<&str>,
        saml_slo_url: Option<&str>,
        saml_x509_cert: Option<&str>,
        saml_name_id_format: Option<&str>,
        attribute_mapping: Option<serde_json::Value>,
        require_signed_assertions: bool,
        require_signed_responses: bool,
        allow_unsolicited_responses: bool,
    ) -> StoreResult<SSOConfiguration> {
        SSOConfiguration::upsert(
            &self.pool,
            org_id,
            provider,
            saml_entity_id,
            saml_sso_url,
            saml_slo_url,
            saml_x509_cert,
            saml_name_id_format,
            attribute_mapping,
            require_signed_assertions,
            require_signed_responses,
            allow_unsolicited_responses,
        )
        .await
        .map_err(Into::into)
    }

    async fn enable_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        SSOConfiguration::enable(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn disable_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        SSOConfiguration::disable(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn delete_sso_config(&self, org_id: Uuid) -> StoreResult<()> {
        SSOConfiguration::delete(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn is_saml_assertion_used(&self, assertion_id: &str, org_id: Uuid) -> StoreResult<bool> {
        SAMLAssertionId::is_used(&self.pool, assertion_id, org_id)
            .await
            .map_err(Into::into)
    }

    async fn record_saml_assertion_used(
        &self,
        assertion_id: &str,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name_id: Option<&str>,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> StoreResult<SAMLAssertionId> {
        SAMLAssertionId::record_used(
            &self.pool,
            assertion_id,
            org_id,
            user_id,
            name_id,
            issued_at,
            expires_at,
        )
        .await
        .map_err(Into::into)
    }

    async fn create_org_template(
        &self,
        org_id: Uuid,
        name: &str,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        created_by: Uuid,
        is_default: bool,
    ) -> StoreResult<OrgTemplate> {
        OrgTemplate::create(
            &self.pool,
            org_id,
            name,
            description,
            blueprint_config,
            security_baseline,
            created_by,
            is_default,
        )
        .await
        .map_err(Into::into)
    }

    async fn find_org_template_by_id(&self, id: Uuid) -> StoreResult<Option<OrgTemplate>> {
        OrgTemplate::find_by_id(&self.pool, id).await.map_err(Into::into)
    }

    async fn list_org_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<OrgTemplate>> {
        OrgTemplate::list_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn update_org_template(
        &self,
        template: &OrgTemplate,
        name: Option<&str>,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        is_default: Option<bool>,
    ) -> StoreResult<OrgTemplate> {
        template
            .update(&self.pool, name, description, blueprint_config, security_baseline, is_default)
            .await
            .map_err(Into::into)
    }

    async fn delete_org_template(&self, id: Uuid) -> StoreResult<()> {
        OrgTemplate::delete(&self.pool, id).await.map_err(Into::into)
    }

    async fn create_template(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        version: &str,
        category: TemplateCategory,
        content_json: serde_json::Value,
    ) -> StoreResult<Template> {
        Template::create(
            &self.pool,
            org_id,
            name,
            slug,
            description,
            author_id,
            version,
            category,
            content_json,
        )
        .await
        .map_err(Into::into)
    }

    async fn find_template_by_name_version(
        &self,
        name: &str,
        version: &str,
    ) -> StoreResult<Option<Template>> {
        Template::find_by_name_version(&self.pool, name, version)
            .await
            .map_err(Into::into)
    }

    async fn list_templates_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Template>> {
        Template::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Template>> {
        Template::search(&self.pool, query, category, tags, org_id, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_search_templates(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        Template::count_search(&self.pool, query, category, tags, org_id)
            .await
            .map_err(Into::into)
    }

    async fn create_scenario(
        &self,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        current_version: &str,
        category: &str,
        license: &str,
        manifest_json: serde_json::Value,
    ) -> StoreResult<Scenario> {
        Scenario::create(
            &self.pool,
            org_id,
            name,
            slug,
            description,
            author_id,
            current_version,
            category,
            license,
            manifest_json,
        )
        .await
        .map_err(Into::into)
    }

    async fn find_scenario_by_name(&self, name: &str) -> StoreResult<Option<Scenario>> {
        Scenario::find_by_name(&self.pool, name).await.map_err(Into::into)
    }

    async fn list_scenarios_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Scenario>> {
        Scenario::find_by_org(&self.pool, org_id).await.map_err(Into::into)
    }

    async fn search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Scenario>> {
        Scenario::search(&self.pool, query, category, tags, org_id, sort, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_search_scenarios(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> StoreResult<i64> {
        Scenario::count_search(&self.pool, query, category, tags, org_id)
            .await
            .map_err(Into::into)
    }

    async fn search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        language: Option<&str>,
        tags: &[String],
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Plugin>> {
        Plugin::search(&self.pool, query, category, language, tags, sort_by, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_search_plugins(
        &self,
        query: Option<&str>,
        category: Option<&str>,
        language: Option<&str>,
        tags: &[String],
    ) -> StoreResult<i64> {
        Plugin::count_search(&self.pool, query, category, language, tags)
            .await
            .map_err(Into::into)
    }

    async fn find_plugin_by_name(&self, name: &str) -> StoreResult<Option<Plugin>> {
        Plugin::find_by_name(&self.pool, name).await.map_err(Into::into)
    }

    async fn get_plugin_tags(&self, plugin_id: Uuid) -> StoreResult<Vec<String>> {
        Plugin::get_tags(&self.pool, plugin_id).await.map_err(Into::into)
    }

    async fn create_plugin(
        &self,
        name: &str,
        description: &str,
        version: &str,
        category: &str,
        license: &str,
        repository: Option<&str>,
        homepage: Option<&str>,
        author_id: Uuid,
        language: &str,
    ) -> StoreResult<Plugin> {
        Plugin::create(
            &self.pool,
            name,
            description,
            version,
            category,
            license,
            repository,
            homepage,
            author_id,
            language,
        )
        .await
        .map_err(Into::into)
    }

    async fn list_plugin_versions(&self, plugin_id: Uuid) -> StoreResult<Vec<PluginVersion>> {
        PluginVersion::get_by_plugin(&self.pool, plugin_id).await.map_err(Into::into)
    }

    async fn find_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
    ) -> StoreResult<Option<PluginVersion>> {
        PluginVersion::find(&self.pool, plugin_id, version).await.map_err(Into::into)
    }

    async fn create_plugin_version(
        &self,
        plugin_id: Uuid,
        version: &str,
        download_url: &str,
        checksum: &str,
        file_size: i64,
        min_mockforge_version: Option<&str>,
        sbom_json: Option<&serde_json::Value>,
    ) -> StoreResult<PluginVersion> {
        PluginVersion::create(
            &self.pool,
            plugin_id,
            version,
            download_url,
            checksum,
            file_size,
            min_mockforge_version,
            sbom_json,
        )
        .await
        .map_err(Into::into)
    }

    async fn get_plugin_version_sbom(
        &self,
        plugin_version_id: Uuid,
    ) -> StoreResult<Option<serde_json::Value>> {
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx::query_as("SELECT sbom_json FROM plugin_versions WHERE id = $1")
                .bind(plugin_version_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|(s,)| s))
    }

    async fn yank_plugin_version(&self, version_id: Uuid) -> StoreResult<()> {
        PluginVersion::yank(&self.pool, version_id).await.map_err(Into::into)
    }

    async fn get_plugin_version_dependencies(
        &self,
        version_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<String, String>> {
        PluginVersion::get_dependencies(&self.pool, version_id)
            .await
            .map_err(Into::into)
    }

    async fn add_plugin_version_dependency(
        &self,
        version_id: Uuid,
        plugin_name: &str,
        version_req: &str,
    ) -> StoreResult<()> {
        PluginVersion::add_dependency(&self.pool, version_id, plugin_name, version_req)
            .await
            .map_err(Into::into)
    }

    // --- Plugin security scans ---

    async fn upsert_plugin_security_scan(
        &self,
        plugin_version_id: Uuid,
        status: &str,
        score: i16,
        findings: &serde_json::Value,
        scanner_version: Option<&str>,
    ) -> StoreResult<()> {
        sqlx::query(
            r#"
            INSERT INTO plugin_security_scans
                (plugin_version_id, status, score, findings, scanner_version, scanned_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (plugin_version_id) DO UPDATE
                SET status = EXCLUDED.status,
                    score = EXCLUDED.score,
                    findings = EXCLUDED.findings,
                    scanner_version = EXCLUDED.scanner_version,
                    scanned_at = NOW()
            "#,
        )
        .bind(plugin_version_id)
        .bind(status)
        .bind(score)
        .bind(findings)
        .bind(scanner_version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn latest_security_scan_for_plugin(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<Option<PluginSecurityScan>> {
        sqlx::query_as::<_, PluginSecurityScan>(
            r#"
            SELECT s.*
            FROM plugin_security_scans s
            INNER JOIN plugin_versions v ON v.id = s.plugin_version_id
            INNER JOIN plugins p ON p.id = v.plugin_id AND p.current_version = v.version
            WHERE v.plugin_id = $1
            ORDER BY s.scanned_at DESC
            LIMIT 1
            "#,
        )
        .bind(plugin_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn list_pending_security_scans(&self, limit: i64) -> StoreResult<Vec<PendingScanJob>> {
        // Oldest-first so a burst of publishes drains in order; joining up to
        // `plugins` lets the worker rebuild the storage key without a second
        // query per row.
        sqlx::query_as::<_, PendingScanJob>(
            r#"
            SELECT
                v.id AS plugin_version_id,
                p.name AS plugin_name,
                v.version AS version,
                v.file_size AS file_size,
                v.checksum AS checksum
            FROM plugin_security_scans s
            INNER JOIN plugin_versions v ON v.id = s.plugin_version_id
            INNER JOIN plugins p ON p.id = v.plugin_id
            WHERE s.status = 'pending'
            ORDER BY s.scanned_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    // --- Plugin reviews ---

    async fn get_plugin_reviews(
        &self,
        plugin_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<Review>> {
        Review::get_by_plugin(&self.pool, plugin_id, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_plugin_reviews(&self, plugin_id: Uuid) -> StoreResult<i64> {
        Review::count_by_plugin(&self.pool, plugin_id).await.map_err(Into::into)
    }

    async fn create_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
        version: &str,
        rating: i16,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<Review> {
        Review::create(&self.pool, plugin_id, user_id, version, rating, title, comment)
            .await
            .map_err(Into::into)
    }

    async fn get_plugin_review_stats(&self, plugin_id: Uuid) -> StoreResult<(f64, i64)> {
        let row = sqlx::query_as::<_, (f64, i64)>(
            "SELECT COALESCE(AVG(rating), 0.0)::float8, COUNT(*) FROM reviews WHERE plugin_id = $1",
        )
        .bind(plugin_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_plugin_review_distribution(
        &self,
        plugin_id: Uuid,
    ) -> StoreResult<std::collections::HashMap<i16, i64>> {
        let rows = sqlx::query_as::<_, (i16, i64)>(
            "SELECT rating, COUNT(*) FROM reviews WHERE plugin_id = $1 GROUP BY rating",
        )
        .bind(plugin_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().collect())
    }

    async fn find_existing_plugin_review(
        &self,
        plugin_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM reviews WHERE plugin_id = $1 AND user_id = $2",
        )
        .bind(plugin_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    async fn update_plugin_rating_stats(
        &self,
        plugin_id: Uuid,
        avg: f64,
        count: i32,
    ) -> StoreResult<()> {
        sqlx::query("UPDATE plugins SET rating_avg = $1, rating_count = $2 WHERE id = $3")
            .bind(avg)
            .bind(count)
            .bind(plugin_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn increment_plugin_review_vote(
        &self,
        plugin_id: Uuid,
        review_id: Uuid,
        helpful: bool,
    ) -> StoreResult<()> {
        let field = if helpful {
            "helpful_count"
        } else {
            "unhelpful_count"
        };
        let q = format!(
            "UPDATE reviews SET {} = {} + 1 WHERE id = $1 AND plugin_id = $2",
            field, field
        );
        sqlx::query(&q).bind(review_id).bind(plugin_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_user_public_info(&self, user_id: Uuid) -> StoreResult<Option<(String, String)>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT id::text, username FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Template reviews ---

    async fn get_template_reviews(
        &self,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<TemplateReview>> {
        TemplateReview::get_by_template(&self.pool, template_id, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_template_reviews(&self, template_id: Uuid) -> StoreResult<i64> {
        TemplateReview::count_by_template(&self.pool, template_id)
            .await
            .map_err(Into::into)
    }

    async fn create_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<TemplateReview> {
        TemplateReview::create(&self.pool, template_id, reviewer_id, rating, title, comment)
            .await
            .map_err(Into::into)
    }

    async fn update_template_review_stats(&self, template_id: Uuid) -> StoreResult<()> {
        TemplateReview::update_template_stats(&self.pool, template_id)
            .await
            .map_err(Into::into)
    }

    async fn find_existing_template_review(
        &self,
        template_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM template_reviews WHERE template_id = $1 AND reviewer_id = $2",
        )
        .bind(template_id)
        .bind(reviewer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    // --- Template stars ---

    async fn toggle_template_star(
        &self,
        template_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<(bool, i64)> {
        TemplateStar::toggle(&self.pool, template_id, user_id).await.map_err(Into::into)
    }

    async fn is_template_starred_by(&self, template_id: Uuid, user_id: Uuid) -> StoreResult<bool> {
        TemplateStar::is_starred_by(&self.pool, template_id, user_id)
            .await
            .map_err(Into::into)
    }

    async fn count_template_stars(&self, template_id: Uuid) -> StoreResult<i64> {
        TemplateStar::count_for_template(&self.pool, template_id)
            .await
            .map_err(Into::into)
    }

    async fn count_template_stars_batch(
        &self,
        template_ids: &[Uuid],
    ) -> StoreResult<std::collections::HashMap<Uuid, i64>> {
        TemplateStar::counts_for_templates(&self.pool, template_ids)
            .await
            .map_err(Into::into)
    }

    // --- Scenario reviews ---

    async fn get_scenario_reviews(
        &self,
        scenario_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> StoreResult<Vec<ScenarioReview>> {
        ScenarioReview::get_by_scenario(&self.pool, scenario_id, limit, offset)
            .await
            .map_err(Into::into)
    }

    async fn count_scenario_reviews(&self, scenario_id: Uuid) -> StoreResult<i64> {
        ScenarioReview::count_by_scenario(&self.pool, scenario_id)
            .await
            .map_err(Into::into)
    }

    async fn create_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> StoreResult<ScenarioReview> {
        ScenarioReview::create(&self.pool, scenario_id, reviewer_id, rating, title, comment)
            .await
            .map_err(Into::into)
    }

    async fn update_scenario_review_stats(&self, scenario_id: Uuid) -> StoreResult<()> {
        ScenarioReview::update_scenario_stats(&self.pool, scenario_id)
            .await
            .map_err(Into::into)
    }

    async fn find_existing_scenario_review(
        &self,
        scenario_id: Uuid,
        reviewer_id: Uuid,
    ) -> StoreResult<Option<Uuid>> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM scenario_reviews WHERE scenario_id = $1 AND reviewer_id = $2",
        )
        .bind(scenario_id)
        .bind(reviewer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id,)| id))
    }

    // --- Admin analytics snapshots ---

    async fn get_admin_analytics_snapshot(&self) -> StoreResult<AdminAnalyticsSnapshot> {
        let pool = &self.pool;

        let (total_users,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
        let (verified_users,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_verified = TRUE")
                .fetch_one(pool)
                .await?;
        let auth_providers = sqlx::query_as::<_, (Option<String>, i64)>(
            "SELECT auth_provider, COUNT(*) FROM users GROUP BY auth_provider",
        )
        .fetch_all(pool)
        .await?;
        let (new_users_7d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '7 days'",
        )
        .fetch_one(pool)
        .await?;
        let (new_users_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(pool)
        .await?;

        let (total_orgs,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM organizations").fetch_one(pool).await?;
        let plan_distribution = sqlx::query_as::<_, (String, i64)>(
            "SELECT plan, COUNT(*) FROM organizations GROUP BY plan",
        )
        .fetch_all(pool)
        .await?;
        let (active_subs,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM subscriptions WHERE status IN ('active', 'trialing')",
        )
        .fetch_one(pool)
        .await?;
        let (trial_orgs,): (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT org_id) FROM subscriptions WHERE status = 'trialing'",
        )
        .fetch_one(pool)
        .await?;

        let (total_requests,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(requests) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())",
        )
        .fetch_one(pool)
        .await?;
        let (total_storage,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(storage_bytes) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())",
        )
        .fetch_one(pool)
        .await?;
        let (total_ai_tokens,): (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(ai_tokens_used) FROM usage_counters WHERE period_start >= DATE_TRUNC('month', NOW())",
        )
        .fetch_one(pool)
        .await?;

        let top_orgs = sqlx::query_as::<_, (Uuid, String, String, i64, i64)>(
            r#"
            SELECT o.id, o.name, o.plan,
                   COALESCE(SUM(uc.requests), 0) as requests,
                   COALESCE(SUM(uc.storage_bytes), 0) as storage_bytes
            FROM organizations o
            LEFT JOIN usage_counters uc ON o.id = uc.org_id
            WHERE uc.period_start >= DATE_TRUNC('month', NOW())
            GROUP BY o.id, o.name, o.plan
            ORDER BY requests DESC
            LIMIT 10
            "#,
        )
        .fetch_all(pool)
        .await?;

        let (hosted_mocks_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM hosted_mocks WHERE deleted_at IS NULL")
                .fetch_one(pool)
                .await?;
        let (hosted_mocks_orgs,): (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT org_id) FROM hosted_mocks WHERE deleted_at IS NULL",
        )
        .fetch_one(pool)
        .await?;
        let (hosted_mocks_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM hosted_mocks WHERE created_at > NOW() - INTERVAL '30 days' AND deleted_at IS NULL",
        )
        .fetch_one(pool)
        .await?;

        let (plugins_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM plugins").fetch_one(pool).await?;
        let (plugins_orgs,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM plugins WHERE org_id IS NOT NULL")
                .fetch_one(pool)
                .await?;
        let (plugins_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM plugins WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(pool)
        .await?;

        let (templates_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM templates").fetch_one(pool).await?;
        let (templates_orgs,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM templates WHERE org_id IS NOT NULL")
                .fetch_one(pool)
                .await?;
        let (templates_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM templates WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(pool)
        .await?;

        let (scenarios_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scenarios").fetch_one(pool).await?;
        let (scenarios_orgs,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM scenarios WHERE org_id IS NOT NULL")
                .fetch_one(pool)
                .await?;
        let (scenarios_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM scenarios WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(pool)
        .await?;

        let (api_tokens_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM api_tokens").fetch_one(pool).await?;
        let (api_tokens_orgs,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT org_id) FROM api_tokens")
                .fetch_one(pool)
                .await?;
        let (api_tokens_30d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM api_tokens WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(pool)
        .await?;

        let user_growth_30d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
            r#"
            SELECT DATE(created_at) as date, COUNT(*) as count
            FROM users
            WHERE created_at > NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let org_growth_30d = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
            r#"
            SELECT DATE(created_at) as date, COUNT(*) as count
            FROM organizations
            WHERE created_at > NOW() - INTERVAL '30 days'
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let (logins_24h,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM login_attempts WHERE success = TRUE AND created_at > NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(pool)
        .await?;
        let (logins_7d,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM login_attempts WHERE success = TRUE AND created_at > NOW() - INTERVAL '7 days'",
        )
        .fetch_one(pool)
        .await?;

        let (api_requests_24h,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(requests), 0) FROM usage_counters WHERE updated_at > NOW() - INTERVAL '24 hours'",
        )
        .fetch_one(pool)
        .await?;
        let (api_requests_7d,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(requests), 0) FROM usage_counters WHERE updated_at > NOW() - INTERVAL '7 days'",
        )
        .fetch_one(pool)
        .await?;

        Ok(AdminAnalyticsSnapshot {
            total_users,
            verified_users,
            auth_providers,
            new_users_7d,
            new_users_30d,
            total_orgs,
            plan_distribution,
            active_subs,
            trial_orgs,
            total_requests,
            total_storage,
            total_ai_tokens,
            top_orgs,
            hosted_mocks_count,
            hosted_mocks_orgs,
            hosted_mocks_30d,
            plugins_count,
            plugins_orgs,
            plugins_30d,
            templates_count,
            templates_orgs,
            templates_30d,
            scenarios_count,
            scenarios_orgs,
            scenarios_30d,
            api_tokens_count,
            api_tokens_orgs,
            api_tokens_30d,
            user_growth_30d,
            org_growth_30d,
            logins_24h,
            logins_7d,
            api_requests_24h,
            api_requests_7d,
        })
    }

    async fn get_conversion_funnel_snapshot(
        &self,
        interval: &str,
    ) -> StoreResult<ConversionFunnelSnapshot> {
        let pool = &self.pool;

        let (signups,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '{}'",
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (verified,): (i64,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM users WHERE is_verified = TRUE AND created_at > NOW() - INTERVAL '{}'",
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (logged_in,): (i64,) = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM users u
            INNER JOIN login_attempts la ON u.email = la.email
            WHERE la.success = TRUE
            AND u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (org_created,): (i64,) = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM users u
            INNER JOIN organization_members om ON u.id = om.user_id
            INNER JOIN organizations o ON om.org_id = o.id
            WHERE om.role = 'admin'
            AND u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (feature_users,): (i64,) = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM users u
            INNER JOIN feature_usage fu ON u.id = fu.user_id
            WHERE u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (checkout_initiated,): (i64,) = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM users u
            INNER JOIN feature_usage fu ON u.id = fu.user_id
            WHERE fu.feature = 'billing_checkout'
            AND u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        let (paid_subscribers,): (i64,) = sqlx::query_as(&format!(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM users u
            INNER JOIN organization_members om ON u.id = om.user_id
            INNER JOIN organizations o ON om.org_id = o.id
            INNER JOIN subscriptions s ON o.id = s.org_id
            WHERE s.status IN ('active', 'trialing')
            AND s.plan IN ('pro', 'team')
            AND u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        let time_to_convert_days: Option<f64> = sqlx::query_scalar::<_, Option<f64>>(&format!(
            r#"
            SELECT AVG(EXTRACT(EPOCH FROM (s.created_at - u.created_at)) / 86400.0) as avg_days
            FROM users u
            INNER JOIN organization_members om ON u.id = om.user_id
            INNER JOIN organizations o ON om.org_id = o.id
            INNER JOIN subscriptions s ON o.id = s.org_id
            WHERE s.status IN ('active', 'trialing')
            AND s.plan IN ('pro', 'team')
            AND u.created_at > NOW() - INTERVAL '{}'
            "#,
            interval
        ))
        .fetch_one(pool)
        .await?;

        Ok(ConversionFunnelSnapshot {
            signups,
            verified,
            logged_in,
            org_created,
            feature_users,
            checkout_initiated,
            paid_subscribers,
            time_to_convert_days,
        })
    }

    // --- GDPR ---

    async fn list_user_settings_raw(&self, user_id: Uuid) -> StoreResult<Vec<UserSettingRow>> {
        let rows = sqlx::query_as::<_, (String, serde_json::Value, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT setting_key, setting_value, created_at, updated_at FROM user_settings WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(key, value, created_at, updated_at)| UserSettingRow {
                key,
                value,
                created_at,
                updated_at,
            })
            .collect())
    }

    async fn list_user_api_tokens(&self, user_id: Uuid) -> StoreResult<Vec<ApiToken>> {
        sqlx::query_as::<_, ApiToken>("SELECT * FROM api_tokens WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn get_org_membership_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> StoreResult<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM org_members WHERE org_id = $1 AND user_id = $2",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(r,)| r))
    }

    async fn list_org_settings_raw(&self, org_id: Uuid) -> StoreResult<Vec<OrgSettingRow>> {
        let rows = sqlx::query_as::<_, (String, serde_json::Value, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT setting_key, setting_value, created_at, updated_at FROM org_settings WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(key, value, created_at, updated_at)| OrgSettingRow {
                key,
                value,
                created_at,
                updated_at,
            })
            .collect())
    }

    async fn list_org_projects_raw(&self, org_id: Uuid) -> StoreResult<Vec<ProjectRow>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>, DateTime<Utc>)>(
            "SELECT id, name, visibility, created_at, updated_at FROM projects WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, name, visibility, created_at, updated_at)| ProjectRow {
                id,
                name,
                visibility,
                created_at,
                updated_at,
            })
            .collect())
    }

    async fn list_org_subscriptions_raw(&self, org_id: Uuid) -> StoreResult<Vec<SubscriptionRow>> {
        let rows = sqlx::query_as::<
            _,
            (Uuid, String, String, DateTime<Utc>, DateTime<Utc>),
        >(
            "SELECT id, plan, status, current_period_end, created_at FROM subscriptions WHERE org_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, plan, status, current_period_end, created_at)| SubscriptionRow {
                id,
                plan,
                status,
                current_period_end,
                created_at,
            })
            .collect())
    }

    async fn list_org_hosted_mocks_raw(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>> {
        sqlx::query_as::<_, HostedMock>("SELECT * FROM hosted_mocks WHERE org_id = $1")
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn delete_user_data_cascade(&self, user_id: Uuid) -> StoreResult<usize> {
        let mut tx = self.pool.begin().await?;

        let owned_orgs =
            sqlx::query_as::<_, (Uuid,)>("SELECT id FROM organizations WHERE owner_id = $1")
                .bind(user_id)
                .fetch_all(&mut *tx)
                .await?;
        let owned_count = owned_orgs.len();

        for (org_id,) in &owned_orgs {
            let (member_count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM org_members WHERE org_id = $1 AND user_id != $2",
            )
            .bind(org_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            if member_count > 0 {
                let new_owner = sqlx::query_as::<_, (Uuid, Uuid)>(
                    "SELECT id, user_id FROM org_members WHERE org_id = $1 AND user_id != $2 ORDER BY CASE role WHEN 'admin' THEN 1 WHEN 'member' THEN 2 END LIMIT 1",
                )
                .bind(org_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

                if let Some((member_id, new_owner_user_id)) = new_owner {
                    sqlx::query("UPDATE organizations SET owner_id = $1 WHERE id = $2")
                        .bind(new_owner_user_id)
                        .bind(org_id)
                        .execute(&mut *tx)
                        .await?;

                    sqlx::query("UPDATE org_members SET role = 'owner' WHERE id = $1")
                        .bind(member_id)
                        .execute(&mut *tx)
                        .await?;
                }
            } else {
                sqlx::query("DELETE FROM organizations WHERE id = $1")
                    .bind(org_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        sqlx::query("DELETE FROM org_members WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM api_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(owned_count)
    }
}
