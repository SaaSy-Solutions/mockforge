//! Learning Hub: tracks, lessons, recipes, progress
//! (cloud-enablement task #12 / Phase 1).
//!
//! Tracks are ordered tutorials; lessons live inside a track. Recipes
//! are standalone short patterns. Progress is per-user lesson-completion
//! tracking, surfaced as a checkbox in the UI and rolled up to a
//! "You've completed N% of this track" indicator.
//!
//! See docs/cloud/CLOUD_SHOWCASE_LEARNING_DESIGN.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningTrack {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    pub is_published: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningLesson {
    pub id: Uuid,
    pub track_id: Uuid,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub sort_order: i32,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningRecipe {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg_attr(feature = "postgres", derive(FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningProgress {
    pub user_id: Uuid,
    pub lesson_id: Uuid,
    pub completed_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl LearningTrack {
    pub async fn list_published(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM learning_tracks WHERE is_published = TRUE ORDER BY sort_order ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM learning_tracks WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl LearningLesson {
    pub async fn list_by_track(pool: &PgPool, track_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM learning_lessons WHERE track_id = $1 ORDER BY sort_order ASC",
        )
        .bind(track_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM learning_lessons WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl LearningRecipe {
    pub async fn list_published(pool: &PgPool, tag: Option<&str>) -> sqlx::Result<Vec<Self>> {
        match tag {
            Some(t) => {
                sqlx::query_as::<_, Self>(
                    "SELECT * FROM learning_recipes WHERE is_published = TRUE AND $1 = ANY(tags) \
                 ORDER BY created_at DESC",
                )
                .bind(t)
                .fetch_all(pool)
                .await
            }
            None => sqlx::query_as::<_, Self>(
                "SELECT * FROM learning_recipes WHERE is_published = TRUE ORDER BY created_at DESC",
            )
            .fetch_all(pool)
            .await,
        }
    }

    pub async fn find_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM learning_recipes WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await
    }
}

#[cfg(feature = "postgres")]
impl LearningProgress {
    /// Mark a lesson complete. Idempotent — re-marking is a no-op (the
    /// PRIMARY KEY conflict is swallowed via ON CONFLICT DO NOTHING).
    pub async fn mark_completed(pool: &PgPool, user_id: Uuid, lesson_id: Uuid) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO learning_progress (user_id, lesson_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(lesson_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// All lessons the user has marked complete.
    pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM learning_progress WHERE user_id = $1 ORDER BY completed_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
}
