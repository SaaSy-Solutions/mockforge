//! Community showcase entries (cloud-enablement task #12 / Phase 1).
//!
//! Each row is a customer-built mock/scenario/integration on display in
//! the public gallery. Read access is unauthenticated; submissions and
//! likes require login. Counts (`likes_count`) are denormalized onto the
//! row and updated in the same transaction as the like/unlike to avoid
//! a join on every list-fetch.
//!
//! See docs/cloud/CLOUD_SHOWCASE_LEARNING_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowcaseEntry {
    pub id: Uuid,
    pub slug: String,
    #[serde(default)]
    pub org_id: Option<Uuid>,
    #[serde(default)]
    pub submitted_by: Option<Uuid>,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub body: Option<String>,
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub demo_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    pub tags: Vec<String>,
    pub is_featured: bool,
    pub is_published: bool,
    pub likes_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
pub struct CreateShowcaseEntry<'a> {
    pub slug: &'a str,
    pub org_id: Option<Uuid>,
    pub submitted_by: Option<Uuid>,
    pub title: &'a str,
    pub description: &'a str,
    pub body: Option<&'a str>,
    pub screenshots: &'a [String],
    pub demo_url: Option<&'a str>,
    pub source_url: Option<&'a str>,
    pub tags: &'a [String],
}

#[cfg(feature = "postgres")]
impl ShowcaseEntry {
    /// Public list. Defaults to published only; tag filter is optional.
    /// Featured entries surface first, then by likes_count, then recency.
    pub async fn list_published(
        pool: &PgPool,
        tag: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        match tag {
            Some(t) => {
                sqlx::query_as::<_, Self>(
                    r#"
                SELECT * FROM showcase_entries
                WHERE is_published = TRUE AND $1 = ANY(tags)
                ORDER BY is_featured DESC, likes_count DESC, created_at DESC
                LIMIT $2
                "#,
                )
                .bind(t)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as::<_, Self>(
                    r#"
                SELECT * FROM showcase_entries
                WHERE is_published = TRUE
                ORDER BY is_featured DESC, likes_count DESC, created_at DESC
                LIMIT $1
                "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        }
    }

    pub async fn find_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM showcase_entries WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM showcase_entries WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &PgPool, input: CreateShowcaseEntry<'_>) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO showcase_entries
                (slug, org_id, submitted_by, title, description, body,
                 screenshots, demo_url, source_url, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(input.slug)
        .bind(input.org_id)
        .bind(input.submitted_by)
        .bind(input.title)
        .bind(input.description)
        .bind(input.body)
        .bind(input.screenshots)
        .bind(input.demo_url)
        .bind(input.source_url)
        .bind(input.tags)
        .fetch_one(pool)
        .await
    }

    /// Toggle a like. Returns `(now_liked, new_count)`. The count is
    /// authoritative — computed in the same transaction as the
    /// insert/delete so it never lies about the state the client just set.
    /// Same shape as `ScenarioStar::toggle`.
    pub async fn toggle_like(
        pool: &PgPool,
        entry_id: Uuid,
        user_id: Uuid,
    ) -> sqlx::Result<(bool, i32)> {
        let mut tx = pool.begin().await?;

        let already: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM showcase_likes WHERE entry_id = $1 AND user_id = $2")
                .bind(entry_id)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

        let now_liked = if already.is_some() {
            sqlx::query("DELETE FROM showcase_likes WHERE entry_id = $1 AND user_id = $2")
                .bind(entry_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE showcase_entries SET likes_count = GREATEST(likes_count - 1, 0) \
                 WHERE id = $1",
            )
            .bind(entry_id)
            .execute(&mut *tx)
            .await?;
            false
        } else {
            sqlx::query("INSERT INTO showcase_likes (entry_id, user_id) VALUES ($1, $2)")
                .bind(entry_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE showcase_entries SET likes_count = likes_count + 1 WHERE id = $1")
                .bind(entry_id)
                .execute(&mut *tx)
                .await?;
            true
        };

        let updated: (i32,) =
            sqlx::query_as("SELECT likes_count FROM showcase_entries WHERE id = $1")
                .bind(entry_id)
                .fetch_one(&mut *tx)
                .await?;

        tx.commit().await?;
        Ok((now_liked, updated.0))
    }
}
