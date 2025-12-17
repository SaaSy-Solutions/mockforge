//! Scenario review model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScenarioReview {
    pub id: Uuid,
    pub scenario_id: Uuid,
    pub reviewer_id: Uuid,
    pub rating: i32,
    pub title: Option<String>,
    pub comment: String,
    pub helpful_count: i32,
    pub verified_purchase: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScenarioReview {
    /// Get reviews for a scenario
    pub async fn get_by_scenario(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM scenario_reviews
            WHERE scenario_id = $1
            ORDER BY helpful_count DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(scenario_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Create a new review
    pub async fn create(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO scenario_reviews (scenario_id, reviewer_id, rating, title, comment)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(scenario_id)
        .bind(reviewer_id)
        .bind(rating)
        .bind(title)
        .bind(comment)
        .fetch_one(pool)
        .await
    }

    /// Count reviews for a scenario
    pub async fn count_by_scenario(pool: &sqlx::PgPool, scenario_id: Uuid) -> sqlx::Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM scenario_reviews WHERE scenario_id = $1")
                .bind(scenario_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Update rating stats for scenario
    pub async fn update_scenario_stats(pool: &sqlx::PgPool, scenario_id: Uuid) -> sqlx::Result<()> {
        let stats = sqlx::query_as::<_, (f64, i64)>(
            r#"
            SELECT COALESCE(AVG(rating), 0.0)::float8, COUNT(*)
            FROM scenario_reviews
            WHERE scenario_id = $1
            "#,
        )
        .bind(scenario_id)
        .fetch_one(pool)
        .await?;

        // Update scenario rating_avg and rating_count
        sqlx::query(
            r#"
            UPDATE scenarios
            SET rating_avg = $1, rating_count = $2
            WHERE id = $3
            "#,
        )
        .bind(rust_decimal::Decimal::try_from(stats.0).unwrap_or(rust_decimal::Decimal::ZERO))
        .bind(stats.1 as i32)
        .bind(scenario_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
