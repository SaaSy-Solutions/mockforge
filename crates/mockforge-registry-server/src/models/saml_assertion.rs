//! SAML assertion tracking model for replay attack prevention

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// SAML assertion ID record for preventing replay attacks
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SAMLAssertionId {
    pub id: Uuid,
    pub assertion_id: String,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name_id: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl SAMLAssertionId {
    /// Check if an assertion ID has been used (replay attack prevention)
    pub async fn is_used(
        pool: &sqlx::PgPool,
        assertion_id: &str,
        org_id: Uuid,
    ) -> sqlx::Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM saml_assertion_ids WHERE assertion_id = $1 AND org_id = $2"
        )
        .bind(assertion_id)
        .bind(org_id)
        .fetch_one(pool)
        .await?;

        Ok(count.0 > 0)
    }

    /// Find assertion ID record by ID and org
    pub async fn find(
        pool: &sqlx::PgPool,
        assertion_id: &str,
        org_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM saml_assertion_ids WHERE assertion_id = $1 AND org_id = $2"
        )
        .bind(assertion_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await
    }

    /// Record a used assertion ID to prevent replay attacks
    pub async fn record_used(
        pool: &sqlx::PgPool,
        assertion_id: &str,
        org_id: Uuid,
        user_id: Option<Uuid>,
        name_id: Option<&str>,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO saml_assertion_ids (assertion_id, org_id, user_id, name_id, issued_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(assertion_id)
        .bind(org_id)
        .bind(user_id)
        .bind(name_id)
        .bind(issued_at)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    /// Cleanup expired assertion IDs (older than 24 hours)
    /// Should be called periodically via a scheduled task
    pub async fn cleanup_expired(pool: &sqlx::PgPool) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "DELETE FROM saml_assertion_ids WHERE expires_at < NOW() - INTERVAL '24 hours'"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
