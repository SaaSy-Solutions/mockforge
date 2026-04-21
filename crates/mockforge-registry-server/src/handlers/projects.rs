//! Project management handlers
//!
//! Exposes a read-only list endpoint so UI surfaces (e.g. the hosted-mocks
//! create dialog) can populate a project picker scoped to the caller's org.

use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub default_env: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// List projects for the caller's organization
pub async fn list_projects(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ProjectResponse>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let projects = sqlx::query_as::<_, ProjectResponse>(
        r#"
        SELECT id, org_id, slug, name, description, visibility, default_env,
               created_at, updated_at
        FROM projects
        WHERE org_id = $1
        ORDER BY name
        "#,
    )
    .bind(org_ctx.org_id)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(projects))
}
