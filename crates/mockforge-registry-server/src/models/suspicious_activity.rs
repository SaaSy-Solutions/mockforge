//! Suspicious activity detection model
//!
//! Tracks and detects potentially suspicious security events

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Suspicious activity types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "suspicious_activity_type", rename_all = "snake_case")]
pub enum SuspiciousActivityType {
    MultipleFailedLogins,
    LoginFromNewLocation,
    RapidApiTokenCreation,
    UnusualApiUsage,
    RapidSettingsChanges,
    UnusualBillingActivity,
    MultipleIpAddresses,
    AccountTakeoverAttempt,
    BruteForceAttempt,
    UnusualDeploymentPattern,
}

/// Suspicious activity record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    pub id: Uuid,
    pub org_id: Option<Uuid>, // None for user-level activities
    pub user_id: Option<Uuid>,
    pub activity_type: SuspiciousActivityType,
    pub severity: String, // "low", "medium", "high", "critical"
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl SuspiciousActivity {
    /// Create a new suspicious activity record
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        activity_type: SuspiciousActivityType,
        severity: &str,
        description: String,
        metadata: Option<serde_json::Value>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO suspicious_activities (org_id, user_id, activity_type, severity, description, metadata, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(activity_type)
        .bind(severity)
        .bind(description)
        .bind(metadata)
        .bind(ip_address)
        .bind(user_agent)
        .fetch_one(pool)
        .await
    }

    /// Get unresolved suspicious activities
    pub async fn get_unresolved(
        pool: &sqlx::PgPool,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> sqlx::Result<Vec<Self>> {
        let mut query =
            sqlx::QueryBuilder::new("SELECT * FROM suspicious_activities WHERE resolved = FALSE");

        if let Some(org_id) = org_id {
            query.push(" AND org_id = ");
            query.push_bind(org_id);
        }

        if let Some(user_id) = user_id {
            query.push(" AND user_id = ");
            query.push_bind(user_id);
        }

        if let Some(severity) = severity {
            query.push(" AND severity = ");
            query.push_bind(severity);
        }

        query.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }

        query.build_query_as::<Self>().fetch_all(pool).await
    }

    /// Mark activity as resolved
    pub async fn resolve(
        pool: &sqlx::PgPool,
        activity_id: Uuid,
        resolved_by: Uuid,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE suspicious_activities SET resolved = TRUE, resolved_at = NOW(), resolved_by = $1 WHERE id = $2"
        )
        .bind(resolved_by)
        .bind(activity_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Clean up old resolved activities (older than N days)
    pub async fn cleanup_old(pool: &sqlx::PgPool, days: i64) -> sqlx::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query(
            "DELETE FROM suspicious_activities WHERE resolved = TRUE AND resolved_at < $1",
        )
        .bind(cutoff)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

/// Helper function to record suspicious activity
pub async fn record_suspicious_activity(
    pool: &sqlx::PgPool,
    org_id: Option<Uuid>,
    user_id: Option<Uuid>,
    activity_type: SuspiciousActivityType,
    severity: &str,
    description: String,
    metadata: Option<serde_json::Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) {
    // Don't fail the request if suspicious activity logging fails
    if let Err(e) = SuspiciousActivity::create(
        pool,
        org_id,
        user_id,
        activity_type,
        severity,
        description,
        metadata,
        ip_address,
        user_agent,
    )
    .await
    {
        tracing::warn!("Failed to record suspicious activity: {}", e);
    }
}
