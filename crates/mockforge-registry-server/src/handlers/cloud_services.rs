//! Service CRUD handlers for cloud mode

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
    models::{cloud_service::CloudService, AuditEventType, FeatureType},
    AppState,
};

pub async fn list_services(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CloudService>>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let services = state.store.list_cloud_services_by_org(org_ctx.org_id).await?;

    Ok(Json(services))
}

pub async fn get_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<CloudService>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let service = state
        .store
        .find_cloud_service_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Service not found".to_string()))?;

    if service.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Service does not belong to this organization".to_string(),
        ));
    }

    Ok(Json(service))
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub base_url: String,
}

pub async fn create_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateServiceRequest>,
) -> ApiResult<Json<CloudService>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("Service name is required".to_string()));
    }

    let service = state
        .store
        .create_cloud_service(
            org_ctx.org_id,
            user_id,
            request.name.trim(),
            &request.description,
            &request.base_url,
        )
        .await?;

    state
        .store
        .record_feature_usage(
            org_ctx.org_id,
            Some(user_id),
            FeatureType::ServiceCreate,
            Some(serde_json::json!({ "service_id": service.id, "name": service.name })),
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
            AuditEventType::ServiceCreated,
            format!("Service '{}' created", service.name),
            Some(serde_json::json!({ "service_id": service.id })),
            ip,
            ua,
        )
        .await;

    Ok(Json(service))
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub base_url: Option<String>,
    pub enabled: Option<bool>,
    pub tags: Option<serde_json::Value>,
    pub routes: Option<serde_json::Value>,
}

pub async fn update_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateServiceRequest>,
) -> ApiResult<Json<CloudService>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let existing = state
        .store
        .find_cloud_service_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Service not found".to_string()))?;

    if existing.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Service does not belong to this organization".to_string(),
        ));
    }

    let service = state
        .store
        .update_cloud_service(
            id,
            request.name.as_deref(),
            request.description.as_deref(),
            request.base_url.as_deref(),
            request.enabled,
            request.tags.as_ref(),
            request.routes.as_ref(),
        )
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Service not found".to_string()))?;

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
            AuditEventType::ServiceUpdated,
            format!("Service '{}' updated", service.name),
            Some(serde_json::json!({ "service_id": service.id })),
            ip,
            ua,
        )
        .await;

    Ok(Json(service))
}

pub async fn delete_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    let service = state
        .store
        .find_cloud_service_by_id(id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Service not found".to_string()))?;

    if service.org_id != org_ctx.org_id {
        return Err(ApiError::InvalidRequest(
            "Service does not belong to this organization".to_string(),
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
            AuditEventType::ServiceDeleted,
            format!("Service '{}' deleted", service.name),
            Some(serde_json::json!({ "service_id": service.id, "name": service.name })),
            ip,
            ua,
        )
        .await;

    state.store.delete_cloud_service(id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
