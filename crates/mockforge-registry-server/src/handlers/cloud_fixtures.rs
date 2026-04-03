//! Fixture CRUD handlers for cloud mode

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        cloud_fixture::CloudFixture, record_audit_event, AuditEventType, FeatureType, FeatureUsage,
    },
    AppState,
};

pub async fn list_fixtures(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CloudFixture>>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixtures = CloudFixture::find_by_org(pool, org_ctx.org_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(fixtures))
}

pub async fn get_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CloudFixture>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixture = CloudFixture::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
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
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Fixture name is required".to_string()));
    }

    let fixture = CloudFixture::create(
        pool,
        org_ctx.org_id,
        user_id,
        request.name.trim(),
        &request.description,
        &request.path,
        &request.method,
        request.content.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?;

    let _ = FeatureUsage::record(
        pool,
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

    record_audit_event(
        pool,
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
    pub tags: Option<serde_json::Value>,
}

pub async fn update_fixture(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFixtureRequest>,
) -> ApiResult<Json<CloudFixture>> {
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let existing = CloudFixture::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Fixture does not belong to this organization".to_string(),
        ));
    }

    let fixture = CloudFixture::update(
        pool,
        id,
        request.name.as_deref(),
        request.description.as_deref(),
        request.path.as_deref(),
        request.method.as_deref(),
        request.content.as_ref(),
        request.tags.as_ref(),
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::InvalidRequest("Fixture not found".to_string()))?;

    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let ua = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
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
    let pool = state.db.pool();
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let fixture = CloudFixture::find_by_id(pool, id)
        .await
        .map_err(ApiError::Database)?
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

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(user_id),
        AuditEventType::FixtureDeleted,
        format!("Fixture '{}' deleted", fixture.name),
        Some(serde_json::json!({ "fixture_id": fixture.id, "name": fixture.name })),
        ip,
        ua,
    )
    .await;

    CloudFixture::delete(pool, id).await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({ "success": true })))
}
