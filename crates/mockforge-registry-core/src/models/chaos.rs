//! Chaos campaigns + reports + resilience patterns
//! (cloud-enablement task #7 / Phase 1).
//!
//! Run execution reuses the #4 worker pool with kind='chaos_campaign';
//! reports are written by the worker on run completion.
//!
//! See docs/cloud/CLOUD_CHAOS_RESILIENCE_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosCampaign {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub target_kind: String,
    pub target_ref: String,
    pub config: serde_json::Value,
    pub safety_config: serde_json::Value,
    #[serde(default)]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosCampaignReport {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub run_id: Uuid,
    pub fault_count: i32,
    pub aborted: bool,
    #[serde(default)]
    pub abort_reason: Option<String>,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
    #[serde(default)]
    pub recommendations: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResiliencePattern {
    pub id: Uuid,
    /// NULL = platform-provided, available to every workspace.
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateChaosCampaign<'a> {
    pub workspace_id: Uuid,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub target_kind: &'a str,
    pub target_ref: &'a str,
    pub config: &'a serde_json::Value,
    pub safety_config: &'a serde_json::Value,
    pub created_by: Option<Uuid>,
}

#[cfg(feature = "postgres")]
impl ChaosCampaign {
    pub const VALID_TARGET_KINDS: &'static [&'static str] = &["hosted_mock", "external"];

    pub fn is_valid_target_kind(kind: &str) -> bool {
        Self::VALID_TARGET_KINDS.contains(&kind)
    }

    pub async fn list_by_workspace(pool: &PgPool, workspace_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM chaos_campaigns WHERE workspace_id = $1 ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM chaos_campaigns WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateChaosCampaign<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO chaos_campaigns
                (workspace_id, name, description, target_kind, target_ref,
                 config, safety_config, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.target_kind)
        .bind(input.target_ref)
        .bind(input.config)
        .bind(input.safety_config)
        .bind(input.created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
        let rows = sqlx::query("DELETE FROM chaos_campaigns WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();
        Ok(rows > 0)
    }
}

#[cfg(feature = "postgres")]
impl ChaosCampaignReport {
    pub async fn list_by_campaign(pool: &PgPool, campaign_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM chaos_campaign_reports WHERE campaign_id = $1 \
             ORDER BY created_at DESC",
        )
        .bind(campaign_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM chaos_campaign_reports WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl ResiliencePattern {
    pub const VALID_KINDS: &'static [&'static str] =
        &["circuit_breaker", "retry", "bulkhead", "rate_limit"];

    pub fn is_valid_kind(kind: &str) -> bool {
        Self::VALID_KINDS.contains(&kind)
    }

    /// Patterns visible to a workspace = platform patterns (workspace_id
    /// IS NULL) ∪ this workspace's own patterns.
    pub async fn list_visible_to_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM resilience_patterns \
             WHERE workspace_id IS NULL OR workspace_id = $1 \
             ORDER BY workspace_id NULLS FIRST, created_at",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_kinds_recognized() {
        assert!(ChaosCampaign::is_valid_target_kind("hosted_mock"));
        assert!(ChaosCampaign::is_valid_target_kind("external"));
    }

    #[test]
    fn target_kinds_rejected() {
        assert!(!ChaosCampaign::is_valid_target_kind(""));
        assert!(!ChaosCampaign::is_valid_target_kind("HOSTED_MOCK"));
        assert!(!ChaosCampaign::is_valid_target_kind("internal"));
    }

    #[test]
    fn pattern_kinds_recognized() {
        assert!(ResiliencePattern::is_valid_kind("circuit_breaker"));
        assert!(ResiliencePattern::is_valid_kind("retry"));
        assert!(ResiliencePattern::is_valid_kind("bulkhead"));
        assert!(ResiliencePattern::is_valid_kind("rate_limit"));
    }
}
