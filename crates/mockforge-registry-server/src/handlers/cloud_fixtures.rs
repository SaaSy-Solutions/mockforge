//! Fixture CRUD handlers for cloud mode

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
    models::{cloud_fixture::CloudFixture, AuditEventType, FeatureType},
    AppState,
};

/// Query parameters accepted by `GET /api/v1/fixtures`. Currently only
/// supports filtering by workspace; absence returns every fixture in the org.
#[derive(Debug, Deserialize, Default)]
pub struct ListFixturesQuery {
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
}

pub async fn list_fixtures(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Query(query): Query<ListFixturesQuery>,
) -> ApiResult<Json<Vec<CloudFixture>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixtures = state
        .store
        .list_cloud_fixtures_by_org(org_ctx.org_id, query.workspace_id)
        .await?;

    Ok(Json(fixtures))
}

pub async fn get_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CloudFixture>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixture = state
        .store
        .find_cloud_fixture_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    if fixture.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Fixture does not belong to this organization".to_string(),
        ));
    }

    Ok(Json(fixture))
}

#[derive(Debug, Deserialize)]
pub struct CreateFixtureRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub content: Option<serde_json::Value>,
    pub protocol: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub workspace_id: Option<Uuid>,
    pub route_path: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

pub async fn create_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateFixtureRequest>,
) -> ApiResult<Json<CloudFixture>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Fixture name is required".to_string()));
    }

    let fixture = state
        .store
        .create_cloud_fixture(
            org_ctx.org_id,
            user_id,
            request.name.trim(),
            &request.description,
            &request.path,
            &request.method,
            request.content.as_ref(),
            request.protocol.as_deref(),
            request.tags.as_ref(),
            request.workspace_id,
            request.route_path.as_deref(),
        )
        .await?;

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FixtureCreate,
            Some(serde_json::json!({ "fixture_id": fixture.id, "name": fixture.name })),
        )
        .await;

    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FixtureCreated,
            format!("Fixture '{}' created", fixture.name),
            Some(serde_json::json!({ "fixture_id": fixture.id })),
            ip,
            ua,
        )
        .await;

    Ok(Json(fixture))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFixtureRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
    pub method: Option<String>,
    pub content: Option<serde_json::Value>,
    pub protocol: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub route_path: Option<String>,
    /// Tri-state workspace assignment: omit to leave unchanged, send `null`
    /// to clear, send a UUID to assign/reassign.
    #[serde(default, deserialize_with = "deserialize_optional_optional_uuid")]
    pub workspace_id: Option<Option<Uuid>>,
}

/// Distinguish "field absent from JSON" (`None`) from "explicit `null`"
/// (`Some(None)`) for the tri-state `workspace_id` patch semantics.
fn deserialize_optional_optional_uuid<'de, D>(
    deserializer: D,
) -> Result<Option<Option<Uuid>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Option<Uuid>>::deserialize(deserializer)
}

pub async fn update_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFixtureRequest>,
) -> ApiResult<Json<CloudFixture>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let existing = state
        .store
        .find_cloud_fixture_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Fixture does not belong to this organization".to_string(),
        ));
    }

    let fixture = state
        .store
        .update_cloud_fixture(
            id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.path.as_deref(),
            request.method.as_deref(),
            request.content.as_ref(),
            request.protocol.as_deref(),
            request.tags.as_ref(),
            request.route_path.as_deref(),
            request.workspace_id,
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FixtureUpdated,
            format!("Fixture '{}' updated", fixture.name),
            Some(serde_json::json!({ "fixture_id": fixture.id })),
            ip,
            ua,
        )
        .await;

    Ok(Json(fixture))
}

pub async fn delete_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixture = state
        .store
        .find_cloud_fixture_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    if fixture.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Fixture does not belong to this organization".to_string(),
        ));
    }

    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FixtureDeleted,
            format!("Fixture '{}' deleted", fixture.name),
            Some(serde_json::json!({ "fixture_id": fixture.id, "name": fixture.name })),
            ip,
            ua,
        )
        .await;

    state.store.delete_cloud_fixture(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    pub fixture_ids: Vec<Uuid>,
}

pub async fn delete_fixtures_bulk(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<BulkDeleteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.fixture_ids.is_empty() {
        return Ok(Json(serde_json::json!({ "deleted": [] })));
    }

    let deleted = state
        .store
        .delete_cloud_fixtures_bulk(org_ctx.org_id, &request.fixture_ids)
        .await?;

    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    if !deleted.is_empty() {
        state
            .store
            .record_audit_event(
                org_ctx.org_id,
                Some(user_id),
                AuditEventType::FixtureDeleted,
                format!("Bulk-deleted {} fixture(s)", deleted.len()),
                Some(serde_json::json!({ "fixture_ids": deleted })),
                ip,
                ua,
            )
            .await;
    }

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
