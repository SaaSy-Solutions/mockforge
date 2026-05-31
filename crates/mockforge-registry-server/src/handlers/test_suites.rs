//! Test suite CRUD handlers (cloud-enablement task #4 / Phase 1).
//!
//! `test_suites` is the user-authored definition; runs, events, schedules,
//! and artifacts come in follow-up slices. The `kind` field is open so other
//! cloud features (#6/#7/#8/#9/#10) reuse the same resource — see
//! `mockforge_registry_core::models::test_execution::TestSuite` for the
//! current vocabulary.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/test-suites
//!   POST   /api/v1/workspaces/{workspace_id}/test-suites
//!   GET    /api/v1/test-suites/{id}
//!   PATCH  /api/v1/test-suites/{id}
//!   DELETE /api/v1/test-suites/{id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::TestSuite,
    AppState,
};
use mockforge_registry_core::models::test_execution::CreateTestSuite;
use mockforge_registry_core::models::CloudWorkspace;

#[derive(Debug, Deserialize)]
pub struct ListSuitesQuery {
    /// Optional kind filter, e.g. ?kind=integration.
    #[serde(default)]
    pub kind: Option<String>,
}

/// `GET /api/v1/workspaces/{workspace_id}/test-suites`
pub async fn list_suites(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<ListSuitesQuery>,
) -> ApiResult<Json<Vec<TestSuite>>> {
    // Verify the workspace belongs to the caller's org before listing — there
    // is no route-level workspace permission layer, so authz is per-handler.
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let suites = TestSuite::list_by_workspace(state.db.pool(), workspace_id, query.kind.as_deref())
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(suites))
}

#[derive(Debug, Deserialize)]
pub struct CreateSuiteRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub target_workspace_id: Option<Uuid>,
}

/// `POST /api/v1/workspaces/{workspace_id}/test-suites`
pub async fn create_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateSuiteRequest>,
) -> ApiResult<Json<TestSuite>> {
    // Reject creating a suite inside a workspace the caller's org doesn't own.
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if request.kind.trim().is_empty() {
        return Err(ApiError::InvalidRequest("kind must not be empty".into()));
    }

    let suite = TestSuite::create(
        state.db.pool(),
        CreateTestSuite {
            workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            kind: &request.kind,
            config: &request.config,
            target_workspace_id: request.target_workspace_id,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(suite))
}

/// `GET /api/v1/test-suites/{id}`
pub async fn get_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestSuite>> {
    let suite = load_authorized_suite(&state, user_id, &headers, id).await?;
    Ok(Json(suite))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSuiteRequest {
    #[serde(default)]
    pub name: Option<String>,
    /// Outer Option = "field present in PATCH"; inner Option = "set to NULL".
    /// `None` leaves the column unchanged; `Some(None)` clears it.
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub target_workspace_id: Option<Option<Uuid>>,
}

/// `PATCH /api/v1/test-suites/{id}`
pub async fn update_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateSuiteRequest>,
) -> ApiResult<Json<TestSuite>> {
    // Confirm the suite belongs to the caller's org before mutating it.
    load_authorized_suite(&state, user_id, &headers, id).await?;
    let updated = TestSuite::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.description.as_ref().map(|d| d.as_deref()),
        request.config.as_ref(),
        request.target_workspace_id,
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/test-suites/{id}`
pub async fn delete_suite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    // Confirm ownership before deleting.
    load_authorized_suite(&state, user_id, &headers, id).await?;
    let deleted = TestSuite::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Test suite not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Load a test suite only if it belongs to the caller's org; otherwise return
/// the same "not found" as a missing suite (no cross-tenant existence oracle).
/// Mirrors `test_schedules::load_authorized_suite`.
async fn load_authorized_suite(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    suite_id: Uuid,
) -> ApiResult<TestSuite> {
    let suite = TestSuite::find_by_id(state.db.pool(), suite_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), suite.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Test suite not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Test suite not found".into()));
    }
    Ok(suite)
}

/// Verify `workspace_id` belongs to the caller's org for workspace-scoped
/// routes. Mirrors the `authorize_workspace` helper used across the handlers.
async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<CloudWorkspace> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(workspace)
}

/// Distinguish "field omitted" vs "field explicitly set to null" during JSON
/// deserialization so PATCH semantics work correctly. Without this helper,
/// serde collapses both into `None`.
fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
