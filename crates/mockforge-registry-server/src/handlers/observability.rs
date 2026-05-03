//! Observability saved-queries + dashboards handlers
//! (cloud-enablement task #2 / Phase 1).
//!
//! Cross-deployment query handlers themselves live in a follow-up
//! slice — this file owns the persistence layer for users' named
//! filters and dashboard layouts, which is enough for the UI to render
//! the "saved searches" sidebar and the dashboard list.
//!
//! Routes:
//!   GET    /api/v1/organizations/{org_id}/observability/saved-queries[?kind=]
//!   POST   /api/v1/organizations/{org_id}/observability/saved-queries
//!   PATCH  /api/v1/observability/saved-queries/{id}
//!   DELETE /api/v1/observability/saved-queries/{id}
//!
//!   GET    /api/v1/organizations/{org_id}/observability/dashboards
//!   POST   /api/v1/organizations/{org_id}/observability/dashboards
//!   PATCH  /api/v1/observability/dashboards/{id}
//!   DELETE /api/v1/observability/dashboards/{id}

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::observability_query::{CreateDashboard, CreateSavedQuery};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{ObservabilityDashboard, ObservabilitySavedQuery},
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ListQueriesQuery {
    #[serde(default)]
    pub kind: Option<String>,
}

/// `GET /api/v1/organizations/{org_id}/observability/saved-queries`
pub async fn list_saved_queries(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListQueriesQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ObservabilitySavedQuery>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let rows = ObservabilitySavedQuery::list_by_org(state.db.pool(), org_id, query.kind.as_deref())
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateSavedQueryRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub kind: String,
    pub filters: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/observability/saved-queries`
pub async fn create_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateSavedQueryRequest>,
) -> ApiResult<Json<ObservabilitySavedQuery>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !ObservabilitySavedQuery::is_valid_kind(&request.kind) {
        return Err(ApiError::InvalidRequest(format!(
            "kind must be one of: {}",
            ObservabilitySavedQuery::VALID_KINDS.join(", ")
        )));
    }

    let row = ObservabilitySavedQuery::create(
        state.db.pool(),
        CreateSavedQuery {
            org_id,
            workspace_id: request.workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            kind: &request.kind,
            filters: &request.filters,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSavedQueryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub filters: Option<serde_json::Value>,
}

/// `PATCH /api/v1/observability/saved-queries/{id}`
pub async fn update_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateSavedQueryRequest>,
) -> ApiResult<Json<ObservabilitySavedQuery>> {
    let existing = load_authorized_query(&state, user_id, &headers, id).await?;
    let _ = existing;
    let updated = ObservabilitySavedQuery::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.filters.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Saved query not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/observability/saved-queries/{id}`
pub async fn delete_saved_query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_query(&state, user_id, &headers, id).await?;
    let deleted = ObservabilitySavedQuery::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Saved query not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

// --- dashboards ------------------------------------------------------------

/// `GET /api/v1/organizations/{org_id}/observability/dashboards`
pub async fn list_dashboards(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ObservabilityDashboard>>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    let rows = ObservabilityDashboard::list_by_org(state.db.pool(), org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateDashboardRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub layout: serde_json::Value,
    pub queries: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

/// `POST /api/v1/organizations/{org_id}/observability/dashboards`
pub async fn create_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(org_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateDashboardRequest>,
) -> ApiResult<Json<ObservabilityDashboard>> {
    authorize_org(&state, user_id, &headers, org_id).await?;
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    let row = ObservabilityDashboard::create(
        state.db.pool(),
        CreateDashboard {
            org_id,
            workspace_id: request.workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            layout: &request.layout,
            queries: &request.queries,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDashboardRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub layout: Option<serde_json::Value>,
    #[serde(default)]
    pub queries: Option<serde_json::Value>,
}

/// `PATCH /api/v1/observability/dashboards/{id}`
pub async fn update_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateDashboardRequest>,
) -> ApiResult<Json<ObservabilityDashboard>> {
    load_authorized_dashboard(&state, user_id, &headers, id).await?;
    let updated = ObservabilityDashboard::update(
        state.db.pool(),
        id,
        request.name.as_deref(),
        request.layout.as_ref(),
        request.queries.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Dashboard not found".into()))?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/observability/dashboards/{id}`
pub async fn delete_dashboard(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_dashboard(&state, user_id, &headers, id).await?;
    let deleted = ObservabilityDashboard::delete(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Dashboard not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn authorize_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    org_id: Uuid,
) -> ApiResult<()> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != org_id {
        return Err(ApiError::InvalidRequest(
            "Cannot access observability for a different org".into(),
        ));
    }
    Ok(())
}

async fn load_authorized_query(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<ObservabilitySavedQuery> {
    let row = ObservabilitySavedQuery::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Saved query not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != row.org_id {
        return Err(ApiError::InvalidRequest("Saved query not found".into()));
    }
    Ok(row)
}

async fn load_authorized_dashboard(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<ObservabilityDashboard> {
    let row = ObservabilityDashboard::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Dashboard not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != row.org_id {
        return Err(ApiError::InvalidRequest("Dashboard not found".into()));
    }
    Ok(row)
}
