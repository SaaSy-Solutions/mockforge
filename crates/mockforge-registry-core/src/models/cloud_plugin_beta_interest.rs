//! Cloud Plugins beta interest signups (Phase 0 demand validation).
//!
//! Each row captures one user's "Request beta access" submission for the
//! cloud-hosted plugin runtime. Unique on `user_id` so repeat submissions
//! UPSERT (latest `use_case` wins). See migration
//! 20250101000073_cloud_plugin_beta_interest.sql.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPluginBetaInterest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub org_id: Option<Uuid>,
    pub use_case: Option<String>,
    pub plan_at_signup: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct UpsertCloudPluginBetaInterest<'a> {
    pub user_id: Uuid,
    pub org_id: Option<Uuid>,
    pub use_case: Option<&'a str>,
    pub plan_at_signup: Option<&'a str>,
}

#[cfg(feature = "postgres")]
impl CloudPluginBetaInterest {
    pub async fn upsert(
        pool: &PgPool,
        input: UpsertCloudPluginBetaInterest<'_>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO cloud_plugin_beta_interest (user_id, org_id, use_case, plan_at_signup)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
                org_id = EXCLUDED.org_id,
                use_case = EXCLUDED.use_case,
                plan_at_signup = EXCLUDED.plan_at_signup,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(input.user_id)
        .bind(input.org_id)
        .bind(input.use_case)
        .bind(input.plan_at_signup)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_user(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM cloud_plugin_beta_interest WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
    }

    /// Aggregate counts for the go/no-go review. Returns
    /// `(total_signups, distinct_orgs)`.
    pub async fn aggregate_counts(pool: &PgPool) -> sqlx::Result<(i64, i64)> {
        let row: (Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::BIGINT AS total,
                COUNT(DISTINCT org_id)::BIGINT AS distinct_orgs
            FROM cloud_plugin_beta_interest
            "#,
        )
        .fetch_one(pool)
        .await?;
        Ok((row.0.unwrap_or(0), row.1.unwrap_or(0)))
    }
}
