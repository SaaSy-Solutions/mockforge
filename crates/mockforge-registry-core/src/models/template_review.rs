//! Template review model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TemplateReview {
    pub id: Uuid,
    pub template_id: Uuid,
    pub reviewer_id: Uuid,
    pub rating: i32,
    pub title: Option<String>,
    pub comment: String,
    pub helpful_count: i32,
    pub verified_use: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TemplateReview {
    /// Get reviews for a template
    pub async fn get_by_template(
        pool: &sqlx::PgPool,
        template_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM template_reviews
            WHERE template_id = $1
            ORDER BY helpful_count DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(template_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Create a new review
    pub async fn create(
        pool: &sqlx::PgPool,
        template_id: Uuid,
        reviewer_id: Uuid,
        rating: i32,
        title: Option<&str>,
        comment: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO template_reviews (template_id, reviewer_id, rating, title, comment)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(template_id)
        .bind(reviewer_id)
        .bind(rating)
        .bind(title)
        .bind(comment)
        .fetch_one(pool)
        .await
    }

    /// Count reviews for a template
    pub async fn count_by_template(pool: &sqlx::PgPool, template_id: Uuid) -> sqlx::Result<i64> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM template_reviews WHERE template_id = $1")
                .bind(template_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Update rating stats for template
    pub async fn update_template_stats(pool: &sqlx::PgPool, template_id: Uuid) -> sqlx::Result<()> {
        let stats = sqlx::query_as::<_, (f64, i64)>(
            r#"
            SELECT COALESCE(AVG(rating), 0.0)::float8, COUNT(*)
            FROM template_reviews
            WHERE template_id = $1
            "#,
        )
        .bind(template_id)
        .fetch_one(pool)
        .await?;

        // Update template stats_json
        sqlx::query(
            r#"
            UPDATE templates
            SET stats_json = jsonb_set(
                COALESCE(stats_json, '{}'::jsonb),
                '{rating}',
                to_jsonb($1::float8)
            )
            WHERE id = $2
            "#,
        )
        .bind(stats.0)
        .bind(template_id)
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE templates
            SET stats_json = jsonb_set(
                COALESCE(stats_json, '{}'::jsonb),
                '{rating_count}',
                to_jsonb($1::bigint)
            )
            WHERE id = $2
            "#,
        )
        .bind(stats.1)
        .bind(template_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
