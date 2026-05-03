//! Showcase + Learning Hub handlers (cloud-enablement task #12 / Phase 1).
//!
//! Public read paths for the showcase gallery, learning tracks, and
//! recipes. Like-toggle + lesson-completion are auth-required and live
//! at separate endpoints. Admin authoring of tracks/lessons/recipes is
//! a follow-up slice (uses /api/v1/admin/* paths).
//!
//! Routes (this slice):
//!   GET    /api/v1/showcase/entries[?tag=&limit=]
//!   GET    /api/v1/showcase/entries/{slug}
//!   POST   /api/v1/showcase/entries/{id}/like-toggle
//!
//!   GET    /api/v1/learning/tracks
//!   GET    /api/v1/learning/tracks/{slug}                  (track + ordered lessons)
//!   GET    /api/v1/learning/recipes[?tag=]
//!   GET    /api/v1/learning/recipes/{slug}
//!   POST   /api/v1/learning/lessons/{lesson_id}/complete
//!   GET    /api/v1/learning/progress

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    models::{LearningLesson, LearningProgress, LearningRecipe, LearningTrack, ShowcaseEntry},
    AppState,
};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

#[derive(Debug, Deserialize)]
pub struct ListShowcaseQuery {
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/showcase/entries`
pub async fn list_showcase_entries(
    State(state): State<AppState>,
    Query(query): Query<ListShowcaseQuery>,
) -> ApiResult<Json<Vec<ShowcaseEntry>>> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let entries = ShowcaseEntry::list_published(state.db.pool(), query.tag.as_deref(), limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(entries))
}

/// `GET /api/v1/showcase/entries/{slug}`
pub async fn get_showcase_entry(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<ShowcaseEntry>> {
    let entry = ShowcaseEntry::find_by_slug(state.db.pool(), &slug)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Showcase entry not found".into()))?;
    if !entry.is_published {
        return Err(ApiError::InvalidRequest("Showcase entry not found".into()));
    }
    Ok(Json(entry))
}

#[derive(Debug, Serialize)]
pub struct LikeToggleResponse {
    pub liked: bool,
    pub likes_count: i32,
}

/// `POST /api/v1/showcase/entries/{id}/like-toggle`
pub async fn toggle_showcase_like(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LikeToggleResponse>> {
    // Verify the entry exists + is published before allowing a like.
    let entry = ShowcaseEntry::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Showcase entry not found".into()))?;
    if !entry.is_published {
        return Err(ApiError::InvalidRequest("Showcase entry not found".into()));
    }

    let (liked, likes_count) = ShowcaseEntry::toggle_like(state.db.pool(), id, user_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(LikeToggleResponse { liked, likes_count }))
}

// --- Learning Hub ----------------------------------------------------------

/// `GET /api/v1/learning/tracks`
pub async fn list_learning_tracks(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<LearningTrack>>> {
    let tracks = LearningTrack::list_published(state.db.pool())
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(tracks))
}

#[derive(Debug, Serialize)]
pub struct TrackDetail {
    #[serde(flatten)]
    pub track: LearningTrack,
    pub lessons: Vec<LearningLesson>,
}

/// `GET /api/v1/learning/tracks/{slug}`
pub async fn get_learning_track(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<TrackDetail>> {
    let track = LearningTrack::find_by_slug(state.db.pool(), &slug)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Learning track not found".into()))?;
    if !track.is_published {
        return Err(ApiError::InvalidRequest("Learning track not found".into()));
    }
    let lessons = LearningLesson::list_by_track(state.db.pool(), track.id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(TrackDetail { track, lessons }))
}

#[derive(Debug, Deserialize)]
pub struct ListRecipesQuery {
    #[serde(default)]
    pub tag: Option<String>,
}

/// `GET /api/v1/learning/recipes`
pub async fn list_learning_recipes(
    State(state): State<AppState>,
    Query(query): Query<ListRecipesQuery>,
) -> ApiResult<Json<Vec<LearningRecipe>>> {
    let recipes = LearningRecipe::list_published(state.db.pool(), query.tag.as_deref())
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(recipes))
}

/// `GET /api/v1/learning/recipes/{slug}`
pub async fn get_learning_recipe(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<LearningRecipe>> {
    let recipe = LearningRecipe::find_by_slug(state.db.pool(), &slug)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Recipe not found".into()))?;
    if !recipe.is_published {
        return Err(ApiError::InvalidRequest("Recipe not found".into()));
    }
    Ok(Json(recipe))
}

/// `POST /api/v1/learning/lessons/{lesson_id}/complete`
pub async fn complete_learning_lesson(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(lesson_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Verify the lesson exists before marking — avoids storing progress
    // rows that point at deleted lessons.
    let lesson = LearningLesson::find_by_id(state.db.pool(), lesson_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Lesson not found".into()))?;
    LearningProgress::mark_completed(state.db.pool(), user_id, lesson.id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(serde_json::json!({ "completed": true, "lesson_id": lesson.id })))
}

/// `GET /api/v1/learning/progress`
pub async fn list_learning_progress(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> ApiResult<Json<Vec<LearningProgress>>> {
    let rows = LearningProgress::list_for_user(state.db.pool(), user_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}
