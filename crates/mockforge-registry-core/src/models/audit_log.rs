//! Audit log model for organization admin actions
//!
//! Tracks important administrative actions within organizations for compliance and security

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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
    TemplatePublished,
    ScenarioPublished,
    // Security
    PasswordChanged,
    EmailChanged,
    TwoFactorEnabled,
    TwoFactorDisabled,
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
            "template_published" => Some(Self::TemplatePublished),
            "scenario_published" => Some(Self::ScenarioPublished),
            "password_changed" => Some(Self::PasswordChanged),
            "email_changed" => Some(Self::EmailChanged),
            "two_factor_enabled" => Some(Self::TwoFactorEnabled),
            "two_factor_disabled" => Some(Self::TwoFactorDisabled),
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
            "admin_impersonation" => Some(Self::AdminImpersonation),
            _ => None,
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

#[cfg(feature = "postgres")]
impl AuditLog {
    /// Create a new audit log entry
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
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO audit_logs (org_id, user_id, event_type, description, metadata, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(event_type)
        .bind(description)
        .bind(metadata)
        .bind(ip_address)
        .bind(user_agent)
        .fetch_one(pool)
        .await
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

    /// Clean up old audit logs (older than N days)
    pub async fn cleanup_old(pool: &sqlx::PgPool, days: i64) -> sqlx::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query("DELETE FROM audit_logs WHERE created_at < $1")
            .bind(cutoff)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
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
