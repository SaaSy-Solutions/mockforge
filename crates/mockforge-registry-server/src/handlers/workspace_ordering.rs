//! Activate a workspace (mark it as the caller's active one) and reorder workspaces.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudWorkspace,
    AppState,
};

/// POST /api/v1/workspaces/{workspace_id}/activate
///
/// Marks the target workspace `is_active = true` and every other workspace in the org
/// `is_active = false`. Mirrors the self-hosted notion of an "active workspace."
pub async fn activate_workspace(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;
    if workspace.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Workspace does not belong to this organization".to_string(),
        ));
    }

    let pool = state.db.pool();
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE workspaces SET is_active = FALSE, updated_at = NOW() WHERE org_id = $1")
        .bind(org_ctx.org_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE workspaces SET is_active = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({ "message": "Active workspace updated" })))
}

#[derive(Debug, Deserialize)]
pub struct WorkspaceOrderRequest {
    pub workspace_ids: Vec<Uuid>,
}

/// PUT /api/v1/workspaces/order
///
/// Rewrites `sort_order` on each workspace within the caller's org to match the supplied
/// list. Ids not belonging to the org are ignored silently (they can't be updated).
pub async fn reorder_workspaces(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<WorkspaceOrderRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let pool = state.db.pool();
    let mut tx = pool.begin().await?;
    for (idx, id) in request.workspace_ids.iter().enumerate() {
        sqlx::query(
            r#"UPDATE workspaces
               SET sort_order = $3, updated_at = NOW()
               WHERE id = $1 AND org_id = $2"#,
        )
        .bind(id)
        .bind(org_ctx.org_id)
        .bind(idx as i32)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Json(json!({ "message": "Workspace order updated" })))
}
