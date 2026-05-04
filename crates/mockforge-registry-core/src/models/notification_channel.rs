//! Notification channels for the incidents subsystem (cloud-enablement
//! task #3 / Phase 1, follow-up slice).
//!
//! Each row is a place an incident notification can be delivered to:
//! email recipients, a Slack webhook, a PagerDuty integration key, or a
//! generic outbound webhook. The `config` JSONB carries per-kind
//! settings; secrets in there ride encrypted via the same path as
//! BYOK API keys (handlers::settings::encrypt_api_key).
//!
//! Schema lives in migration 20250101000060_incidents.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    /// `email` | `slack` | `pagerduty` | `webhook`
    pub kind: String,
    /// Per-kind settings (recipients, webhook URL, integration key, …).
    /// Sensitive fields are stored encrypted; callers handling them
    /// should round-trip through settings::encrypt_api_key.
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateNotificationChannel<'a> {
    pub org_id: Uuid,
    pub name: &'a str,
    pub kind: &'a str,
    pub config: &'a serde_json::Value,
    pub enabled: bool,
}

#[cfg(feature = "postgres")]
impl NotificationChannel {
    pub async fn list_by_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM notification_channels WHERE org_id = $1 ORDER BY created_at ASC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM notification_channels WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateNotificationChannel<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO notification_channels (org_id, name, kind, config, enabled)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(input.org_id)
        .bind(input.name)
        .bind(input.kind)
        .bind(input.config)
        .bind(input.enabled)
        .fetch_one(pool)
        .await
    }

    /// PATCH-style update. Any `Some(_)` field overwrites; `None` leaves it.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        name: Option<&str>,
        config: Option<&serde_json::Value>,
        enabled: Option<bool>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE notification_channels SET
                name = COALESCE($2, name),
                config = COALESCE($3, config),
                enabled = COALESCE($4, enabled),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(config)
        .bind(enabled)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM notification_channels WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}
