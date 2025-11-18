//! Organization template handlers
//!
//! Handles org-level templates for workspace creation with blueprint
//! and security baseline configurations.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, resolve_org_context},
    models::{OrgMember, OrgRole, OrgTemplate},
    AppState,
};

/// List all templates for an organization
///
/// GET /api/v1/organizations/{org_id}/templates
pub async fn list_templates(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
) -> ApiResult<Json<TemplateListResponse>> {
    let pool = state.db.pool();

    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Get templates
    let templates = OrgTemplate::list_by_org(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(TemplateListResponse { templates }))
}

/// Get a specific template
///
/// GET /api/v1/organizations/{org_id}/templates/{template_id}
pub async fn get_template(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((org_id, template_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<OrgTemplate>> {
    let pool = state.db.pool();

    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Get template
    let template = OrgTemplate::find_by_id(pool, template_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Template not found".to_string()))?;

    // Verify template belongs to org
    if template.org_id != org_ctx.org_id {
        return Err(ApiError::PermissionDenied);
    }

    Ok(Json(template))
}

/// Create a new organization template
///
/// POST /api/v1/organizations/{org_id}/templates
pub async fn create_template(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(request): Json<CreateTemplateRequest>,
) -> ApiResult<Json<OrgTemplate>> {
    let pool = state.db.pool();

    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Verify user has permission (owner or admin)
    let is_owner = org_ctx.org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Create template
    let template = OrgTemplate::create(
        pool,
        org_ctx.org_id,
        &request.name,
        request.description.as_deref(),
        request.blueprint_config,
        request.security_baseline,
        user_id,
        request.is_default.unwrap_or(false),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(Json(template))
}

/// Update an organization template
///
/// PATCH /api/v1/organizations/{org_id}/templates/{template_id}
pub async fn update_template(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((org_id, template_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateTemplateRequest>,
) -> ApiResult<Json<OrgTemplate>> {
    let pool = state.db.pool();

    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Verify user has permission (owner or admin)
    let is_owner = org_ctx.org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Get template
    let template = OrgTemplate::find_by_id(pool, template_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Template not found".to_string()))?;

    // Verify template belongs to org
    if template.org_id != org_ctx.org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Update template
    let updated = template
        .update(
            pool,
            request.name.as_deref(),
            request.description.as_deref(),
            request.blueprint_config,
            request.security_baseline,
            request.is_default,
        )
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(updated))
}

/// Delete an organization template
///
/// DELETE /api/v1/organizations/{org_id}/templates/{template_id}
pub async fn delete_template(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((org_id, template_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<DeleteTemplateResponse>> {
    let pool = state.db.pool();

    // Resolve org context and verify access
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Verify the resolved org matches the requested org_id
    if org_ctx.org_id != org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Verify user has permission (owner or admin)
    let is_owner = org_ctx.org.owner_id == user_id;
    let is_admin = if !is_owner {
        if let Ok(Some(member)) = OrgMember::find(pool, org_ctx.org_id, user_id).await {
            matches!(member.role(), OrgRole::Admin | OrgRole::Owner)
        } else {
            false
        }
    } else {
        false
    };

    if !is_owner && !is_admin {
        return Err(ApiError::PermissionDenied);
    }

    // Get template to verify it belongs to org
    let template = OrgTemplate::find_by_id(pool, template_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Template not found".to_string()))?;

    if template.org_id != org_ctx.org_id {
        return Err(ApiError::PermissionDenied);
    }

    // Delete template
    OrgTemplate::delete(pool, template_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(DeleteTemplateResponse {
        success: true,
        message: "Template deleted successfully".to_string(),
    }))
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub blueprint_config: Option<serde_json::Value>,
    pub security_baseline: Option<serde_json::Value>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub blueprint_config: Option<serde_json::Value>,
    pub security_baseline: Option<serde_json::Value>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<OrgTemplate>,
}

#[derive(Debug, Serialize)]
pub struct DeleteTemplateResponse {
    pub success: bool,
    pub message: String,
}
