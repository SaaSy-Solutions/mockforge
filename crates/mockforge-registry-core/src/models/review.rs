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
    /// Public response from the plugin's author. NULL until the author posts
    /// one through `POST /api/v1/plugins/{name}/reviews/{review_id}/respond`.
    /// Stored alongside the review (rather than a separate response table)
    /// because there's at most one response per review.
    #[sqlx(default)]
    pub author_response_text: Option<String>,
    #[sqlx(default)]
    pub author_response_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
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
            "#,
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
            "#,
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
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reviews WHERE plugin_id = $1")
            .bind(plugin_id)
            .fetch_one(pool)
            .await?;

        Ok(count)
    }

    /// Find a single review by id within a plugin. Used by the author-response
    /// endpoint to verify the review actually belongs to the plugin in the
    /// path before letting the plugin author post a response.
    pub async fn find_in_plugin(
        pool: &sqlx::PgPool,
        plugin_id: Uuid,
        review_id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM reviews WHERE plugin_id = $1 AND id = $2")
            .bind(plugin_id)
            .bind(review_id)
            .fetch_optional(pool)
            .await
    }

    /// Set or clear the plugin author's response on a review. Passing `None`
    /// clears both the text and the timestamp.
    pub async fn set_author_response(
        pool: &sqlx::PgPool,
        review_id: Uuid,
        text: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE reviews
            SET author_response_text = $2,
                author_response_at = CASE WHEN $2 IS NULL THEN NULL ELSE NOW() END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(review_id)
        .bind(text)
        .execute(pool)
        .await?;
        Ok(())
    }
}
