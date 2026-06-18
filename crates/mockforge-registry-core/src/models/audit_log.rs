//! Audit log model for organization admin actions
//!
//! Tracks important administrative actions within organizations for compliance and security

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sha2::{Digest, Sha256};

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_event_type", rename_all = "snake_case")]
pub enum AuditEventType {
    // Member management
    MemberAdded,
    MemberRemoved,
    MemberRoleChanged,
    // Organization management
    OrgCreated,
    OrgUpdated,
    OrgDeleted,
    OrgPlanChanged,
    // Billing
    BillingCheckout,
    BillingUpgrade,
    BillingDowngrade,
    BillingCanceled,
    // API tokens
    ApiTokenCreated,
    ApiTokenDeleted,
    ApiTokenRotated,
    // Settings
    SettingsUpdated,
    ByokConfigUpdated,
    ByokConfigDeleted,
    // Deployments
    DeploymentCreated,
    DeploymentDeleted,
    DeploymentUpdated,
    // Marketplace
    PluginPublished,
    PluginTakenDown,
    PluginRestored,
    PluginReviewResponsePosted,
    TemplatePublished,
    ScenarioPublished,
    // Publisher attestation keys (user-scoped — recorded with org_id=nil)
    PublisherKeyCreated,
    PublisherKeyRevoked,
    PublisherKeyRotated,
    // Authentication (#871) — login/logout visibility for SOC2 / brute-force forensics.
    LoginSucceeded,
    LoginFailed,
    Logout,
    // Security
    PasswordChanged,
    EmailChanged,
    TwoFactorEnabled,
    TwoFactorDisabled,
    // GDPR / data egress (#872) — bulk personal-data export.
    DataExported,
    // Federation
    FederationCreated,
    FederationUpdated,
    FederationDeleted,
    FederationScenarioActivated,
    FederationScenarioDeactivated,
    // Workspaces
    WorkspaceCreated,
    WorkspaceUpdated,
    WorkspaceDeleted,
    // Services
    ServiceCreated,
    ServiceUpdated,
    ServiceDeleted,
    // Fixtures
    FixtureCreated,
    FixtureUpdated,
    FixtureDeleted,
    // Invitations
    InvitationCreated,
    InvitationRevoked,
    InvitationAccepted,
    // Cloud Plugins (Phase 1) — see migration
    // 20250101000074_cloud_plugin_attachments.sql.
    PluginAttached,
    PluginDetached,
    PluginRevoked,
    PluginBlocklistHit,
    OrgTrustRootCreated,
    OrgTrustRootRevoked,
    // Platform signing-root rotation (RFC §8.2 / §9, Issue #550).
    // Operator-scoped events — recorded with the operator's org_id (or
    // the platform operator's tenant when SaaSy Solutions itself runs
    // the rotation).
    PlatformSigningRotationStarted,
    PlatformSigningKeyRetired,
    PlatformSigningKeyRevoked,
    // Admin actions
    AdminImpersonation,
}

impl AuditEventType {
    /// Parse the snake_case string representation back into an enum.
    ///
    /// Mirrors the `rename_all = "snake_case"` serde encoding used for the
    /// sqlx `audit_event_type` Postgres enum and the SQLite TEXT column.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "member_added" => Some(Self::MemberAdded),
            "member_removed" => Some(Self::MemberRemoved),
            "member_role_changed" => Some(Self::MemberRoleChanged),
            "org_created" => Some(Self::OrgCreated),
            "org_updated" => Some(Self::OrgUpdated),
            "org_deleted" => Some(Self::OrgDeleted),
            "org_plan_changed" => Some(Self::OrgPlanChanged),
            "billing_checkout" => Some(Self::BillingCheckout),
            "billing_upgrade" => Some(Self::BillingUpgrade),
            "billing_downgrade" => Some(Self::BillingDowngrade),
            "billing_canceled" => Some(Self::BillingCanceled),
            "api_token_created" => Some(Self::ApiTokenCreated),
            "api_token_deleted" => Some(Self::ApiTokenDeleted),
            "api_token_rotated" => Some(Self::ApiTokenRotated),
            "settings_updated" => Some(Self::SettingsUpdated),
            "byok_config_updated" => Some(Self::ByokConfigUpdated),
            "byok_config_deleted" => Some(Self::ByokConfigDeleted),
            "deployment_created" => Some(Self::DeploymentCreated),
            "deployment_deleted" => Some(Self::DeploymentDeleted),
            "deployment_updated" => Some(Self::DeploymentUpdated),
            "plugin_published" => Some(Self::PluginPublished),
            "plugin_taken_down" => Some(Self::PluginTakenDown),
            "plugin_restored" => Some(Self::PluginRestored),
            "plugin_review_response_posted" => Some(Self::PluginReviewResponsePosted),
            "template_published" => Some(Self::TemplatePublished),
            "scenario_published" => Some(Self::ScenarioPublished),
            "publisher_key_created" => Some(Self::PublisherKeyCreated),
            "publisher_key_revoked" => Some(Self::PublisherKeyRevoked),
            "publisher_key_rotated" => Some(Self::PublisherKeyRotated),
            "login_succeeded" => Some(Self::LoginSucceeded),
            "login_failed" => Some(Self::LoginFailed),
            "logout" => Some(Self::Logout),
            "password_changed" => Some(Self::PasswordChanged),
            "email_changed" => Some(Self::EmailChanged),
            "two_factor_enabled" => Some(Self::TwoFactorEnabled),
            "two_factor_disabled" => Some(Self::TwoFactorDisabled),
            "data_exported" => Some(Self::DataExported),
            "federation_created" => Some(Self::FederationCreated),
            "federation_updated" => Some(Self::FederationUpdated),
            "federation_deleted" => Some(Self::FederationDeleted),
            "federation_scenario_activated" => Some(Self::FederationScenarioActivated),
            "federation_scenario_deactivated" => Some(Self::FederationScenarioDeactivated),
            "workspace_created" => Some(Self::WorkspaceCreated),
            "workspace_updated" => Some(Self::WorkspaceUpdated),
            "workspace_deleted" => Some(Self::WorkspaceDeleted),
            "service_created" => Some(Self::ServiceCreated),
            "service_updated" => Some(Self::ServiceUpdated),
            "service_deleted" => Some(Self::ServiceDeleted),
            "fixture_created" => Some(Self::FixtureCreated),
            "fixture_updated" => Some(Self::FixtureUpdated),
            "fixture_deleted" => Some(Self::FixtureDeleted),
            "invitation_created" => Some(Self::InvitationCreated),
            "invitation_revoked" => Some(Self::InvitationRevoked),
            "invitation_accepted" => Some(Self::InvitationAccepted),
            "plugin_attached" => Some(Self::PluginAttached),
            "plugin_detached" => Some(Self::PluginDetached),
            "plugin_revoked" => Some(Self::PluginRevoked),
            "plugin_blocklist_hit" => Some(Self::PluginBlocklistHit),
            "org_trust_root_created" => Some(Self::OrgTrustRootCreated),
            "org_trust_root_revoked" => Some(Self::OrgTrustRootRevoked),
            "platform_signing_rotation_started" => Some(Self::PlatformSigningRotationStarted),
            "platform_signing_key_retired" => Some(Self::PlatformSigningKeyRetired),
            "platform_signing_key_revoked" => Some(Self::PlatformSigningKeyRevoked),
            "admin_impersonation" => Some(Self::AdminImpersonation),
            _ => None,
        }
    }

    /// Canonical snake_case wire form. Round-trips with [`Self::from_str`].
    ///
    /// Use this when serializing for HTTP responses; the `Debug` form
    /// produces PascalCase like `OrgUpdated` which doesn't match the input
    /// the filter accepts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemberAdded => "member_added",
            Self::MemberRemoved => "member_removed",
            Self::MemberRoleChanged => "member_role_changed",
            Self::OrgCreated => "org_created",
            Self::OrgUpdated => "org_updated",
            Self::OrgDeleted => "org_deleted",
            Self::OrgPlanChanged => "org_plan_changed",
            Self::BillingCheckout => "billing_checkout",
            Self::BillingUpgrade => "billing_upgrade",
            Self::BillingDowngrade => "billing_downgrade",
            Self::BillingCanceled => "billing_canceled",
            Self::ApiTokenCreated => "api_token_created",
            Self::ApiTokenDeleted => "api_token_deleted",
            Self::ApiTokenRotated => "api_token_rotated",
            Self::SettingsUpdated => "settings_updated",
            Self::ByokConfigUpdated => "byok_config_updated",
            Self::ByokConfigDeleted => "byok_config_deleted",
            Self::DeploymentCreated => "deployment_created",
            Self::DeploymentDeleted => "deployment_deleted",
            Self::DeploymentUpdated => "deployment_updated",
            Self::PluginPublished => "plugin_published",
            Self::PluginTakenDown => "plugin_taken_down",
            Self::PluginRestored => "plugin_restored",
            Self::PluginReviewResponsePosted => "plugin_review_response_posted",
            Self::TemplatePublished => "template_published",
            Self::ScenarioPublished => "scenario_published",
            Self::PublisherKeyCreated => "publisher_key_created",
            Self::PublisherKeyRevoked => "publisher_key_revoked",
            Self::PublisherKeyRotated => "publisher_key_rotated",
            Self::LoginSucceeded => "login_succeeded",
            Self::LoginFailed => "login_failed",
            Self::Logout => "logout",
            Self::PasswordChanged => "password_changed",
            Self::EmailChanged => "email_changed",
            Self::TwoFactorEnabled => "two_factor_enabled",
            Self::TwoFactorDisabled => "two_factor_disabled",
            Self::DataExported => "data_exported",
            Self::FederationCreated => "federation_created",
            Self::FederationUpdated => "federation_updated",
            Self::FederationDeleted => "federation_deleted",
            Self::FederationScenarioActivated => "federation_scenario_activated",
            Self::FederationScenarioDeactivated => "federation_scenario_deactivated",
            Self::WorkspaceCreated => "workspace_created",
            Self::WorkspaceUpdated => "workspace_updated",
            Self::WorkspaceDeleted => "workspace_deleted",
            Self::ServiceCreated => "service_created",
            Self::ServiceUpdated => "service_updated",
            Self::ServiceDeleted => "service_deleted",
            Self::FixtureCreated => "fixture_created",
            Self::FixtureUpdated => "fixture_updated",
            Self::FixtureDeleted => "fixture_deleted",
            Self::InvitationCreated => "invitation_created",
            Self::InvitationRevoked => "invitation_revoked",
            Self::InvitationAccepted => "invitation_accepted",
            Self::PluginAttached => "plugin_attached",
            Self::PluginDetached => "plugin_detached",
            Self::PluginRevoked => "plugin_revoked",
            Self::PluginBlocklistHit => "plugin_blocklist_hit",
            Self::OrgTrustRootCreated => "org_trust_root_created",
            Self::OrgTrustRootRevoked => "org_trust_root_revoked",
            Self::PlatformSigningRotationStarted => "platform_signing_rotation_started",
            Self::PlatformSigningKeyRetired => "platform_signing_key_retired",
            Self::PlatformSigningKeyRevoked => "platform_signing_key_revoked",
            Self::AdminImpersonation => "admin_impersonation",
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>, // User who performed the action
    pub event_type: AuditEventType,
    pub description: String,
    pub metadata: Option<serde_json::Value>, // Additional context
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Build the canonical, order-stable string that the hash chain commits to for
/// a single audit row. Field order and the `|` separator are part of the
/// on-disk contract — changing either breaks verification of existing chains.
///
/// `metadata` is rendered via its compact JSON form (`Value::to_string`), which
/// `serde_json` emits with sorted-by-insertion keys; we feed the already-parsed
/// `serde_json::Value` so the bytes are stable regardless of inbound whitespace.
#[cfg(feature = "postgres")]
fn canonical_entry(
    org_id: Uuid,
    user_id: Option<Uuid>,
    event_type: AuditEventType,
    description: &str,
    metadata: Option<&serde_json::Value>,
    ip_address: Option<&str>,
    created_at: DateTime<Utc>,
) -> String {
    format!(
        "{org}|{user}|{event}|{desc}|{meta}|{ip}|{ts}",
        org = org_id,
        user = user_id.map(|u| u.to_string()).unwrap_or_default(),
        event = event_type.as_str(),
        desc = description,
        meta = metadata.map(|m| m.to_string()).unwrap_or_default(),
        ip = ip_address.unwrap_or_default(),
        // RFC3339 with nanos — matches what Postgres TIMESTAMPTZ round-trips
        // back into chrono, so recomputation in verify_chain is byte-identical.
        ts = created_at.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
    )
}

/// Truncate a timestamp to microsecond resolution to match Postgres
/// `TIMESTAMPTZ` storage precision (see [`AuditLog::create`]).
#[cfg(feature = "postgres")]
fn truncate_to_micros(ts: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::SubsecRound;
    ts.trunc_subsecs(6)
}

/// Compute `sha256_hex(prev_hash_or_empty || "|" || canonical(...))`.
#[cfg(feature = "postgres")]
fn chain_hash(prev_hash: Option<&str>, canonical: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// Minimal projection used to recompute the hash chain in [`AuditLog::verify_chain`].
/// Mirrors the fields fed into [`canonical_entry`] plus the stored chain hashes.
#[cfg(feature = "postgres")]
#[derive(FromRow)]
struct AuditChainRow {
    org_id: Uuid,
    user_id: Option<Uuid>,
    event_type: AuditEventType,
    description: String,
    metadata: Option<serde_json::Value>,
    ip_address: Option<String>,
    created_at: DateTime<Utc>,
    prev_hash: Option<String>,
    entry_hash: Option<String>,
}

#[cfg(feature = "postgres")]
impl AuditLog {
    /// Create a new audit log entry, extending the org's tamper-evident hash
    /// chain (#872).
    ///
    /// The whole operation runs in a transaction: we take a `FOR UPDATE` lock
    /// on the org's current latest row so two concurrent inserts can't both
    /// read the same `prev_hash` and fork the chain. `entry_hash` commits to
    /// `prev_hash` plus the canonical row contents, so any later mutation,
    /// deletion, or reordering is detectable by [`Self::verify_chain`].
    ///
    /// The first row of each org's chain has `prev_hash = NULL`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Option<Uuid>,
        event_type: AuditEventType,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> sqlx::Result<Self> {
        let mut tx = pool.begin().await?;

        // Lock + read the previous entry's hash for this org. FOR UPDATE
        // serializes concurrent inserts on the same org chain.
        let prev_hash: Option<String> = sqlx::query_scalar(
            r#"
            SELECT entry_hash FROM audit_logs
            WHERE org_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(org_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        // created_at is generated here (not by the DB DEFAULT) so it is part of
        // the hashed canonical form. Truncate to microseconds *before* hashing:
        // Postgres TIMESTAMPTZ has microsecond resolution, so a nanosecond-
        // precision `Utc::now()` would be truncated on store and the read-back
        // value in verify_chain would no longer match what we hashed. Truncating
        // up-front keeps insert-time and verify-time canonical bytes identical.
        let created_at = truncate_to_micros(Utc::now());
        let canonical = canonical_entry(
            org_id,
            user_id,
            event_type,
            &description,
            metadata.as_ref(),
            ip_address,
            created_at,
        );
        let entry_hash = chain_hash(prev_hash.as_deref(), &canonical);

        let row = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO audit_logs
                (org_id, user_id, event_type, description, metadata,
                 ip_address, user_agent, created_at, prev_hash, entry_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, org_id, user_id, event_type, description, metadata,
                      ip_address, user_agent, created_at
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(event_type)
        .bind(&description)
        .bind(metadata)
        .bind(ip_address)
        .bind(user_agent)
        .bind(created_at)
        .bind(prev_hash)
        .bind(&entry_hash)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    /// Recompute the per-org hash chain and report whether it is intact.
    ///
    /// Returns `Ok(true)` when every row's stored `entry_hash` matches a fresh
    /// recomputation from its predecessor, and `Ok(false)` on the first break
    /// (mutated field, deleted row, reordered row, or forged hash). Rows whose
    /// `entry_hash` is NULL (pre-#872 history) are treated as a fresh chain
    /// start: the chain is validated from the first hashed row onward.
    pub async fn verify_chain(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query_as::<_, AuditChainRow>(
            r#"
            SELECT org_id, user_id, event_type, description, metadata,
                   ip_address, created_at, prev_hash, entry_hash
            FROM audit_logs
            WHERE org_id = $1
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        let mut prev_hash: Option<String> = None;
        for row in rows {
            // Skip un-chained historical rows but keep walking forward.
            let Some(stored) = row.entry_hash.as_deref() else {
                prev_hash = None;
                continue;
            };

            // The stored prev_hash must match the running chain head.
            if row.prev_hash.as_deref() != prev_hash.as_deref() {
                return Ok(false);
            }

            let canonical = canonical_entry(
                row.org_id,
                row.user_id,
                row.event_type,
                &row.description,
                row.metadata.as_ref(),
                row.ip_address.as_deref(),
                row.created_at,
            );
            let recomputed = chain_hash(row.prev_hash.as_deref(), &canonical);
            if recomputed != stored {
                return Ok(false);
            }
            prev_hash = Some(stored.to_string());
        }

        Ok(true)
    }

    /// Get audit logs for an organization, optionally filtered to one or more event types.
    pub async fn get_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_types: &[AuditEventType],
    ) -> sqlx::Result<Vec<Self>> {
        let mut query = sqlx::QueryBuilder::new("SELECT * FROM audit_logs WHERE org_id = ");
        query.push_bind(org_id);

        if !event_types.is_empty() {
            query.push(" AND event_type IN (");
            let mut sep = query.separated(", ");
            for et in event_types {
                sep.push_bind(*et);
            }
            query.push(")");
        }

        query.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }

        if let Some(offset) = offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }

        query.build_query_as::<Self>().fetch_all(pool).await
    }

    /// Get audit logs for a specific user within an organization
    pub async fn get_by_user_in_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> sqlx::Result<Vec<Self>> {
        let mut query = sqlx::QueryBuilder::new("SELECT * FROM audit_logs WHERE org_id = ");
        query.push_bind(org_id);
        query.push(" AND user_id = ");
        query.push_bind(user_id);
        query.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }

        query.build_query_as::<Self>().fetch_all(pool).await
    }

    /// Audit-log retention is disabled for tamper-evidence (#872).
    ///
    /// `audit_logs` is append-only at the DB level (a trigger raises on any
    /// DELETE/UPDATE — see migration `20250101000080_audit_log_integrity.sql`),
    /// and deleting rows would also break the per-org hash chain. SOC2 / ISO
    /// retention is "keep", not "prune", so this is intentionally a no-op that
    /// logs a warning and reports 0 rows removed. The `_pool`/`_days` arguments
    /// are kept so the call sites and trait shape are unchanged.
    pub async fn cleanup_old(_pool: &sqlx::PgPool, _days: i64) -> sqlx::Result<u64> {
        tracing::warn!("audit retention disabled for immutability (#872)");
        Ok(0)
    }
}

/// Helper function to record audit events from request context
#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
pub async fn record_audit_event(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    user_id: Option<Uuid>,
    event_type: AuditEventType,
    description: String,
    metadata: Option<serde_json::Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) {
    // Don't fail the request if audit logging fails
    if let Err(e) = AuditLog::create(
        pool,
        org_id,
        user_id,
        event_type,
        description,
        metadata,
        ip_address,
        user_agent,
    )
    .await
    {
        tracing::warn!("Failed to record audit event: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant must round-trip through as_str → from_str. A new variant
    /// added without updating either method will fail this test.
    #[test]
    fn audit_event_type_round_trips_via_as_str() {
        let variants = [
            AuditEventType::MemberAdded,
            AuditEventType::MemberRemoved,
            AuditEventType::MemberRoleChanged,
            AuditEventType::OrgCreated,
            AuditEventType::OrgUpdated,
            AuditEventType::OrgDeleted,
            AuditEventType::OrgPlanChanged,
            AuditEventType::BillingCheckout,
            AuditEventType::BillingUpgrade,
            AuditEventType::BillingDowngrade,
            AuditEventType::BillingCanceled,
            AuditEventType::ApiTokenCreated,
            AuditEventType::ApiTokenDeleted,
            AuditEventType::ApiTokenRotated,
            AuditEventType::SettingsUpdated,
            AuditEventType::ByokConfigUpdated,
            AuditEventType::ByokConfigDeleted,
            AuditEventType::DeploymentCreated,
            AuditEventType::DeploymentDeleted,
            AuditEventType::DeploymentUpdated,
            AuditEventType::PluginPublished,
            AuditEventType::PluginTakenDown,
            AuditEventType::PluginRestored,
            AuditEventType::PluginReviewResponsePosted,
            AuditEventType::TemplatePublished,
            AuditEventType::ScenarioPublished,
            AuditEventType::PublisherKeyCreated,
            AuditEventType::PublisherKeyRevoked,
            AuditEventType::PublisherKeyRotated,
            AuditEventType::LoginSucceeded,
            AuditEventType::LoginFailed,
            AuditEventType::Logout,
            AuditEventType::PasswordChanged,
            AuditEventType::EmailChanged,
            AuditEventType::TwoFactorEnabled,
            AuditEventType::TwoFactorDisabled,
            AuditEventType::DataExported,
            AuditEventType::FederationCreated,
            AuditEventType::FederationUpdated,
            AuditEventType::FederationDeleted,
            AuditEventType::FederationScenarioActivated,
            AuditEventType::FederationScenarioDeactivated,
            AuditEventType::WorkspaceCreated,
            AuditEventType::WorkspaceUpdated,
            AuditEventType::WorkspaceDeleted,
            AuditEventType::ServiceCreated,
            AuditEventType::ServiceUpdated,
            AuditEventType::ServiceDeleted,
            AuditEventType::FixtureCreated,
            AuditEventType::FixtureUpdated,
            AuditEventType::FixtureDeleted,
            AuditEventType::InvitationCreated,
            AuditEventType::InvitationRevoked,
            AuditEventType::InvitationAccepted,
            AuditEventType::PlatformSigningRotationStarted,
            AuditEventType::PlatformSigningKeyRetired,
            AuditEventType::PlatformSigningKeyRevoked,
            AuditEventType::AdminImpersonation,
        ];
        for variant in variants {
            let s = variant.as_str();
            assert_eq!(
                AuditEventType::from_str(s),
                Some(variant),
                "round-trip failed for {variant:?} (got {s:?})"
            );
        }
    }

    #[test]
    fn audit_event_type_as_str_examples() {
        assert_eq!(AuditEventType::ApiTokenCreated.as_str(), "api_token_created");
        assert_eq!(AuditEventType::OrgUpdated.as_str(), "org_updated");
        assert_eq!(AuditEventType::MemberRoleChanged.as_str(), "member_role_changed");
    }
}
