//! Unified storage layer for the registry domain.
//!
//! The [`RegistryStore`] trait abstracts over the concrete database backend so
//! the same handlers, middleware, and domain logic can run against either
//! PostgreSQL (for the multi-tenant SaaS binary) or SQLite (for the OSS admin
//! server embedded in `mockforge-ui`).
//!
//! Phase 1a introduces the trait with the API-token domain only. Subsequent
//! phases will add organizations, organization members, settings (BYOK),
//! audit logs, feature usage, users, invitations, and quotas.
//!
//! The initial Postgres implementation delegates to the existing inherent
//! `ApiToken::*` methods so that introducing the trait is a no-op refactor.
//! Later phases will invert this relationship and move the SQL into the trait
//! impls directly.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::api_token::{ApiToken, TokenScope};
use crate::models::audit_log::{AuditEventType, AuditLog};
use crate::models::cloud_fixture::CloudFixture;
use crate::models::cloud_service::CloudService;
use crate::models::cloud_workspace::Workspace as CloudWorkspace;
use crate::models::feature_usage::FeatureType;
use crate::models::federation::Federation;
use crate::models::hosted_mock::{DeploymentStatus, HealthStatus, HostedMock};
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::settings::OrgSetting;
use crate::models::subscription::UsageCounter;
use crate::models::suspicious_activity::{SuspiciousActivity, SuspiciousActivityType};
use crate::models::user::User;
use crate::models::verification_token::VerificationToken;
use crate::models::waitlist::WaitlistSubscriber;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PgRegistryStore;

/// Result alias for all [`RegistryStore`] operations.
pub type StoreResult<T> = Result<T, StoreError>;

/// Backend-agnostic errors surfaced by [`RegistryStore`] implementations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("record not found")]
    NotFound,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("hashing error: {0}")]
    Hash(String),
}

/// Unified storage trait for the registry domain.
///
/// Implementations must be `Send + Sync + 'static` so they can live behind an
/// `Arc<dyn RegistryStore>` inside `AppState` and be cloned across request
/// handlers without extra synchronization.
#[async_trait]
pub trait RegistryStore: Send + Sync + 'static {
    // ---------------------------------------------------------------------
    // Health
    // ---------------------------------------------------------------------

    /// Ping the backing database. Returns `Ok(())` if the store is reachable.
    /// Implementations should issue the cheapest possible liveness check
    /// (`SELECT 1` for SQL backends).
    async fn health_check(&self) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // API tokens
    // ---------------------------------------------------------------------

    /// Create a new API token. Returns the plaintext token (shown once) and
    /// the persisted [`ApiToken`] record.
    async fn create_api_token(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name: &str,
        scopes: &[TokenScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<(String, ApiToken)>;

    /// Look up a token by its database id.
    async fn find_api_token_by_id(&self, token_id: Uuid) -> StoreResult<Option<ApiToken>>;

    /// List every token that belongs to an organization, newest first.
    async fn list_api_tokens_by_org(&self, org_id: Uuid) -> StoreResult<Vec<ApiToken>>;

    /// Look up a token by its public prefix within an organization.
    async fn find_api_token_by_prefix(
        &self,
        org_id: Uuid,
        prefix: &str,
    ) -> StoreResult<Option<ApiToken>>;

    /// Verify a plaintext token string against stored hashes, updating
    /// `last_used_at` on success. Returns `None` for invalid or expired tokens.
    async fn verify_api_token(&self, token: &str) -> StoreResult<Option<ApiToken>>;

    /// Permanently delete a token.
    async fn delete_api_token(&self, token_id: Uuid) -> StoreResult<()>;

    /// Rotate an existing token — create a replacement with the same scopes
    /// and optionally delete the old one. Returns the new plaintext token,
    /// the new record, and the deleted record (when `delete_old` was `true`).
    async fn rotate_api_token(
        &self,
        token_id: Uuid,
        new_name: Option<&str>,
        delete_old: bool,
    ) -> StoreResult<(String, ApiToken, Option<ApiToken>)>;

    /// Find tokens older than `days_old`, optionally scoped to a single org.
    async fn find_api_tokens_needing_rotation(
        &self,
        org_id: Option<Uuid>,
        days_old: i64,
    ) -> StoreResult<Vec<ApiToken>>;

    // ---------------------------------------------------------------------
    // Organization settings (JSON key/value per org)
    // ---------------------------------------------------------------------

    /// Fetch a single org-level setting by key, returning `None` when absent.
    async fn get_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<Option<OrgSetting>>;

    /// Upsert an org-level setting, returning the persisted record.
    async fn set_org_setting(
        &self,
        org_id: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> StoreResult<OrgSetting>;

    /// Delete an org-level setting by key. Idempotent.
    async fn delete_org_setting(&self, org_id: Uuid, key: &str) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Organizations
    // ---------------------------------------------------------------------

    /// Create a new organization and auto-create the owner membership.
    async fn create_organization(
        &self,
        name: &str,
        slug: &str,
        owner_id: Uuid,
        plan: Plan,
    ) -> StoreResult<Organization>;

    /// Look up an organization by id.
    async fn find_organization_by_id(&self, org_id: Uuid) -> StoreResult<Option<Organization>>;

    /// Look up an organization by slug.
    async fn find_organization_by_slug(&self, slug: &str) -> StoreResult<Option<Organization>>;

    /// List all organizations a user belongs to (as owner or member).
    async fn list_organizations_by_user(&self, user_id: Uuid) -> StoreResult<Vec<Organization>>;

    /// Update an organization's display name.
    async fn update_organization_name(&self, org_id: Uuid, name: &str) -> StoreResult<()>;

    /// Update an organization's slug.
    async fn update_organization_slug(&self, org_id: Uuid, slug: &str) -> StoreResult<()>;

    /// Update an organization's plan (and refresh limits).
    async fn update_organization_plan(&self, org_id: Uuid, plan: Plan) -> StoreResult<()>;

    /// Check whether an organization has an active or trialing subscription.
    async fn organization_has_active_subscription(&self, org_id: Uuid) -> StoreResult<bool>;

    /// Permanently delete an organization (cascades to related rows).
    async fn delete_organization(&self, org_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Organization members
    // ---------------------------------------------------------------------

    /// Add a user to an organization with the given role.
    async fn create_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<OrgMember>;

    /// Look up a specific (org, user) membership.
    async fn find_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<Option<OrgMember>>;

    /// List every member of an organization, oldest first.
    async fn list_org_members(&self, org_id: Uuid) -> StoreResult<Vec<OrgMember>>;

    /// Update a member's role.
    async fn update_org_member_role(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: OrgRole,
    ) -> StoreResult<()>;

    /// Remove a member from an organization.
    async fn delete_org_member(&self, org_id: Uuid, user_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Audit logs
    // ---------------------------------------------------------------------

    /// Best-effort audit event recording. Failures are logged and swallowed
    /// so they never block the caller's primary operation.
    #[allow(clippy::too_many_arguments)]
    async fn record_audit_event(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event_type: AuditEventType,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    );

    /// List audit logs for an organization with optional filters.
    async fn list_audit_logs(
        &self,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<Vec<AuditLog>>;

    /// Count audit logs matching the filter (for pagination).
    async fn count_audit_logs(
        &self,
        org_id: Uuid,
        event_type: Option<AuditEventType>,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Feature usage
    // ---------------------------------------------------------------------

    /// Record a feature-usage event. Failures are logged and swallowed.
    async fn record_feature_usage(
        &self,
        org_id: Uuid,
        user_id: Option<Uuid>,
        feature: FeatureType,
        metadata: Option<serde_json::Value>,
    );

    /// Count how many times an org used a feature over the last `days` days.
    async fn count_feature_usage_by_org(
        &self,
        org_id: Uuid,
        feature: FeatureType,
        days: i64,
    ) -> StoreResult<i64>;

    // ---------------------------------------------------------------------
    // Suspicious activity
    // ---------------------------------------------------------------------

    /// Record a suspicious-activity event. Failures are logged and swallowed.
    // ---------------------------------------------------------------------
    // Users
    // ---------------------------------------------------------------------

    /// Create a new user with an already-hashed password.
    async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> StoreResult<User>;

    /// Look up a user by id.
    async fn find_user_by_id(&self, user_id: Uuid) -> StoreResult<Option<User>>;

    /// Look up a user by email.
    async fn find_user_by_email(&self, email: &str) -> StoreResult<Option<User>>;

    /// Look up a user by username.
    async fn find_user_by_username(&self, username: &str) -> StoreResult<Option<User>>;

    /// Batch lookup by id to avoid N+1 queries.
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> StoreResult<Vec<User>>;

    /// Set the persistent API token on a user record.
    async fn set_user_api_token(&self, user_id: Uuid, token: &str) -> StoreResult<()>;

    /// Enable TOTP 2FA for a user with the given secret and hashed backup codes.
    async fn enable_user_2fa(
        &self,
        user_id: Uuid,
        secret: &str,
        backup_codes: &[String],
    ) -> StoreResult<()>;

    /// Disable 2FA and clear stored secret + backup codes.
    async fn disable_user_2fa(&self, user_id: Uuid) -> StoreResult<()>;

    /// Refresh the 2FA verified timestamp (e.g. after a successful TOTP challenge).
    async fn update_user_2fa_verified(&self, user_id: Uuid) -> StoreResult<()>;

    /// Remove a consumed backup code by index.
    async fn remove_user_backup_code(&self, user_id: Uuid, code_index: usize) -> StoreResult<()>;

    /// Look up a user by their GitHub account id.
    async fn find_user_by_github_id(&self, github_id: &str) -> StoreResult<Option<User>>;

    /// Look up a user by their Google account id.
    async fn find_user_by_google_id(&self, google_id: &str) -> StoreResult<Option<User>>;

    /// Link an existing user to a GitHub account (sets github_id, auth_provider, avatar_url).
    async fn link_user_github_account(
        &self,
        user_id: Uuid,
        github_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()>;

    /// Link an existing user to a Google account (sets google_id, auth_provider, avatar_url).
    async fn link_user_google_account(
        &self,
        user_id: Uuid,
        google_id: &str,
        avatar_url: Option<&str>,
    ) -> StoreResult<()>;

    /// Create a new verified user from an OAuth provider (random password hash).
    #[allow(clippy::too_many_arguments)]
    async fn create_oauth_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        auth_provider: &str,
        github_id: Option<&str>,
        google_id: Option<&str>,
        avatar_url: Option<&str>,
    ) -> StoreResult<User>;

    /// Fetch or create a user's personal/default organization.
    async fn get_or_create_personal_org(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> StoreResult<Organization>;

    /// Replace a user's password hash (no-op on verification).
    async fn update_user_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> StoreResult<()>;

    /// Mark a user's email as verified.
    async fn mark_user_verified(&self, user_id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Verification / password-reset tokens
    // ---------------------------------------------------------------------

    /// Create a new verification token for a user (24h default expiry).
    async fn create_verification_token(&self, user_id: Uuid) -> StoreResult<VerificationToken>;

    /// Shorten a verification token's expiry to `hours` from now.
    /// Used by password-reset to override the default 24h window.
    async fn set_verification_token_expiry_hours(
        &self,
        token_id: Uuid,
        hours: i64,
    ) -> StoreResult<()>;

    /// Look up a verification token by its plaintext token string.
    async fn find_verification_token_by_token(
        &self,
        token: &str,
    ) -> StoreResult<Option<VerificationToken>>;

    /// Mark a verification token as consumed.
    async fn mark_verification_token_used(&self, token_id: Uuid) -> StoreResult<()>;

    #[allow(clippy::too_many_arguments)]
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
    );

    // ---------------------------------------------------------------------
    // Federations
    // ---------------------------------------------------------------------

    async fn create_federation(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> StoreResult<Federation>;

    async fn find_federation_by_id(&self, id: Uuid) -> StoreResult<Option<Federation>>;

    async fn list_federations_by_org(&self, org_id: Uuid) -> StoreResult<Vec<Federation>>;

    async fn update_federation(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> StoreResult<Option<Federation>>;

    async fn delete_federation(&self, id: Uuid) -> StoreResult<()>;

    /// List unresolved suspicious activities with optional filters.
    async fn list_unresolved_suspicious_activities(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> StoreResult<Vec<SuspiciousActivity>>;

    /// Count unresolved suspicious activities for an org.
    async fn count_unresolved_suspicious_activities(&self, org_id: Uuid) -> StoreResult<i64>;

    /// Mark a suspicious activity as resolved by the given user.
    async fn resolve_suspicious_activity(
        &self,
        activity_id: Uuid,
        resolved_by: Uuid,
    ) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud workspaces
    // ---------------------------------------------------------------------

    async fn create_cloud_workspace(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
    ) -> StoreResult<CloudWorkspace>;

    async fn find_cloud_workspace_by_id(&self, id: Uuid) -> StoreResult<Option<CloudWorkspace>>;

    async fn list_cloud_workspaces_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudWorkspace>>;

    async fn update_cloud_workspace(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        is_active: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudWorkspace>>;

    async fn delete_cloud_workspace(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud services
    // ---------------------------------------------------------------------

    async fn create_cloud_service(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        base_url: &str,
    ) -> StoreResult<CloudService>;

    async fn find_cloud_service_by_id(&self, id: Uuid) -> StoreResult<Option<CloudService>>;

    async fn list_cloud_services_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudService>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_cloud_service(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        base_url: Option<&str>,
        enabled: Option<bool>,
        tags: Option<&serde_json::Value>,
        routes: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudService>>;

    async fn delete_cloud_service(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Cloud fixtures
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn create_cloud_fixture(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        path: &str,
        method: &str,
        content: Option<&serde_json::Value>,
    ) -> StoreResult<CloudFixture>;

    async fn find_cloud_fixture_by_id(&self, id: Uuid) -> StoreResult<Option<CloudFixture>>;

    async fn list_cloud_fixtures_by_org(&self, org_id: Uuid) -> StoreResult<Vec<CloudFixture>>;

    #[allow(clippy::too_many_arguments)]
    async fn update_cloud_fixture(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        path: Option<&str>,
        method: Option<&str>,
        content: Option<&serde_json::Value>,
        tags: Option<&serde_json::Value>,
    ) -> StoreResult<Option<CloudFixture>>;

    async fn delete_cloud_fixture(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Hosted mocks (deployments)
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
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
    ) -> StoreResult<HostedMock>;

    async fn find_hosted_mock_by_id(&self, id: Uuid) -> StoreResult<Option<HostedMock>>;

    async fn find_hosted_mock_by_slug(
        &self,
        org_id: Uuid,
        slug: &str,
    ) -> StoreResult<Option<HostedMock>>;

    async fn list_hosted_mocks_by_org(&self, org_id: Uuid) -> StoreResult<Vec<HostedMock>>;

    async fn update_hosted_mock_status(
        &self,
        id: Uuid,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> StoreResult<()>;

    async fn update_hosted_mock_urls(
        &self,
        id: Uuid,
        deployment_url: Option<&str>,
        internal_url: Option<&str>,
    ) -> StoreResult<()>;

    async fn update_hosted_mock_health(
        &self,
        id: Uuid,
        health_status: HealthStatus,
        health_check_url: Option<&str>,
    ) -> StoreResult<()>;

    async fn delete_hosted_mock(&self, id: Uuid) -> StoreResult<()>;

    // ---------------------------------------------------------------------
    // Waitlist
    // ---------------------------------------------------------------------

    async fn subscribe_waitlist(
        &self,
        email: &str,
        source: &str,
    ) -> StoreResult<WaitlistSubscriber>;

    async fn unsubscribe_waitlist_by_token(&self, token: Uuid) -> StoreResult<bool>;

    // ---------------------------------------------------------------------
    // Usage counters
    // ---------------------------------------------------------------------

    async fn get_or_create_current_usage_counter(&self, org_id: Uuid) -> StoreResult<UsageCounter>;

    async fn list_usage_counters_by_org(&self, org_id: Uuid) -> StoreResult<Vec<UsageCounter>>;
}
