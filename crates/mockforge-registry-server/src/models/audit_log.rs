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
    // Admin actions
    AdminImpersonation,
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

impl AuditLog {
    /// Create a new audit log entry
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

    /// Get audit logs for an organization
    pub async fn get_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
        event_type: Option<AuditEventType>,
    ) -> sqlx::Result<Vec<Self>> {
        let mut query = sqlx::QueryBuilder::new("SELECT * FROM audit_logs WHERE org_id = $1");

        if let Some(event_type) = event_type {
            query.push(" AND event_type = $2");
            query.push_bind(event_type);
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
        let mut query = sqlx::QueryBuilder::new(
            "SELECT * FROM audit_logs WHERE org_id = $1 AND user_id = $2 ORDER BY created_at DESC",
        );

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
