//! Federation CRUD handlers

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
    models::{AuditEventType, FeatureType, Federation},
    AppState,
};

/// List all federations for the user's organization
pub async fn list_federations(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Federation>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federations = state.store.list_federations_by_org(org_ctx.org_id).await?;

    Ok(Json(federations))
}

/// Get a single federation by ID
pub async fn get_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let federation = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    Ok(Json(federation))
}

/// Create a new federation
#[derive(Debug, Deserialize)]
pub struct CreateFederationRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub services: serde_json::Value,
}

pub async fn create_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateFederationRequest>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Federation name is required".to_string()));
    }

    // Default services to empty array if null
    let services = if request.services.is_null() {
        serde_json::json!([])
    } else {
        request.services
    };

    let federation = state
        .store
        .create_federation(
            org_ctx.org_id,
            user_id,
            request.name.trim(),
            &request.description,
            &services,
        )
        .await?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationCreate,
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
        )
        .await;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationCreated,
            format!("Federation '{}' created", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(federation))
}

/// Update an existing federation
#[derive(Debug, Deserialize)]
pub struct UpdateFederationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub services: Option<serde_json::Value>,
}

pub async fn update_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateFederationRequest>,
) -> ApiResult<Json<Federation>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify federation exists and belongs to org
    let existing = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    let federation = state
        .store
        .update_federation(
            id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.services.as_ref(),
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationUpdate,
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
        )
        .await;

    // Record audit log
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationUpdated,
            format!("Federation '{}' updated", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
            ip_address,
            user_agent,
        )
        .await;

    Ok(Json(federation))
}

/// Delete a federation
pub async fn delete_federation(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Verify federation exists and belongs to org
    let federation = state
        .store
        .find_federation_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Federation not found".to_string()))?;

    if federation.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Federation does not belong to this organization".to_string(),
        ));
    }

    // Record audit log before deletion
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    state
        .store
        .record_audit_event(
            org_ctx.org_id,
            Some(user_id),
            AuditEventType::FederationDeleted,
            format!("Federation '{}' deleted", federation.name),
            Some(serde_json::json!({
                "federation_id": federation.id,
                "name": federation.name,
            })),
            ip_address,
            user_agent,
        )
        .await;

    // Track feature usage
    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::FederationDelete,
            Some(serde_json::json!({
                "federation_id": federation.id,
            })),
        )
        .await;

    state.store.delete_federation(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
