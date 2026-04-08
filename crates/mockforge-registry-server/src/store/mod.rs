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
use crate::models::organization::{OrgMember, OrgRole, Organization, Plan};
use crate::models::settings::OrgSetting;

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
}
