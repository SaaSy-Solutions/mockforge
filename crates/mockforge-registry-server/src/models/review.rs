//! Review model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Review {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub version: String,
    pub user_id: Uuid,
    pub rating: i16,
    pub title: Option<String>,
    pub comment: String,
    pub helpful_count: i32,
    pub unhelpful_count: i32,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Review {
    /// Get reviews for a plugin
    pub async fn get_by_plugin(
        pool: &sqlx::PgPool,
        plugin_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM reviews
            WHERE plugin_id = $1
            ORDER BY helpful_count DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(plugin_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Create a new review
    pub async fn create(
        pool: &sqlx::PgPool,
        plugin_id: Uuid,
        user_id: Uuid,
        version: &str,
        rating: i16,
        title: Option<&str>,
        comment: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO reviews (plugin_id, user_id, version, rating, title, comment)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(plugin_id)
        .bind(user_id)
        .bind(version)
        .bind(rating)
        .bind(title)
        .bind(comment)
        .fetch_one(pool)
        .await
    }

    /// Count reviews for a plugin
    pub async fn count_by_plugin(pool: &sqlx::PgPool, plugin_id: Uuid) -> sqlx::Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reviews WHERE plugin_id = $1"
        )
        .bind(plugin_id)
        .fetch_one(pool)
        .await?;

        Ok(count)
    }
}
