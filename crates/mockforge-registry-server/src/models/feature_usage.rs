//! Feature usage tracking model
//!
//! Tracks when specific features are used by organizations for analytics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Feature types that can be tracked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "feature_type", rename_all = "snake_case")]
pub enum FeatureType {
    HostedMockDeploy,
    HostedMockRequest,
    PluginPublish,
    PluginInstall,
    TemplatePublish,
    TemplateInstall,
    ScenarioPublish,
    ScenarioInstall,
    ApiTokenCreate,
    ApiTokenUse,
    BillingCheckout,
    BillingUpgrade,
    BillingDowngrade,
    OrgCreate,
    OrgInvite,
    MarketplaceSearch,
    MarketplaceDownload,
}

/// Feature usage event
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FeatureUsage {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub feature: FeatureType,
    pub metadata: Option<serde_json::Value>, // Additional context (e.g., plugin name, deployment ID)
    pub created_at: DateTime<Utc>,
}

impl FeatureUsage {
    /// Record a feature usage event
    pub async fn record(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        user_id: Option<Uuid>,
        feature: FeatureType,
        metadata: Option<serde_json::Value>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO feature_usage (org_id, user_id, feature, metadata)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(feature)
        .bind(metadata)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Count feature usage for an org in a time period
    pub async fn count_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        feature: FeatureType,
        days: i64,
    ) -> sqlx::Result<i64> {
        let since = Utc::now() - chrono::Duration::days(days);
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM feature_usage
            WHERE org_id = $1 AND feature = $2 AND created_at > $3
            "#,
        )
        .bind(org_id)
        .bind(feature)
        .bind(since)
        .fetch_one(pool)
        .await?;
        Ok(count.0)
    }

    /// Get feature usage stats across all orgs
    pub async fn get_global_stats(
        pool: &sqlx::PgPool,
        feature: FeatureType,
        days: i64,
    ) -> sqlx::Result<(i64, i64)> {
        let since = Utc::now() - chrono::Duration::days(days);
        let stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(DISTINCT org_id) as unique_orgs
            FROM feature_usage
            WHERE feature = $1 AND created_at > $2
            "#,
        )
        .bind(feature)
        .bind(since)
        .fetch_one(pool)
        .await?;
        Ok(stats)
    }

    /// Get feature adoption timeline (daily counts)
    pub async fn get_adoption_timeline(
        pool: &sqlx::PgPool,
        feature: FeatureType,
        days: i64,
    ) -> sqlx::Result<Vec<(chrono::NaiveDate, i64)>> {
        let since = Utc::now() - chrono::Duration::days(days);
        let timeline = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
            r#"
            SELECT
                DATE(created_at) as date,
                COUNT(*) as count
            FROM feature_usage
            WHERE feature = $1 AND created_at > $2
            GROUP BY DATE(created_at)
            ORDER BY date ASC
            "#,
        )
        .bind(feature)
        .bind(since)
        .fetch_all(pool)
        .await?;
        Ok(timeline)
    }

    /// Clean up old feature usage events (older than N days)
    pub async fn cleanup_old(
        pool: &sqlx::PgPool,
        days: i64,
    ) -> sqlx::Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let result = sqlx::query("DELETE FROM feature_usage WHERE created_at < $1")
            .bind(cutoff)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
