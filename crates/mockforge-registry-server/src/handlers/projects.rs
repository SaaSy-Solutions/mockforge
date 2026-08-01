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

    // #832: run on the runtime pool inside the org GUC so the projects RLS
    // policy enforces isolation even though this handler queries directly
    // (not via the store). The `WHERE org_id` stays as defense-in-depth.
    let org_id = org_ctx.org_id;
    let projects = crate::store::with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move {
            sqlx::query_as::<_, ProjectResponse>(
                r#"
                SELECT id, org_id, slug, name, description, visibility, default_env,
                       created_at, updated_at
                FROM projects
                WHERE org_id = $1
                ORDER BY name
                "#,
            )
            .bind(org_id)
            .fetch_all(&mut **tx)
            .await
            .map_err(Into::into)
        })
    })
    .await?;

    Ok(Json(projects))
}
