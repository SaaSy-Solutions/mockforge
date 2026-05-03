//! Chaos campaign handlers (cloud-enablement task #7 / Phase 1).
//!
//! Phase 1 surface: campaign CRUD + report read + resilience-pattern
//! read. Run trigger / abort / target-authorization / kill-switch worker
//! land in follow-up slices once #4 worker pool is up.
//!
//! Routes:
//!   GET    /api/v1/workspaces/{workspace_id}/chaos-campaigns
//!   POST   /api/v1/workspaces/{workspace_id}/chaos-campaigns
//!   GET    /api/v1/chaos-campaigns/{id}
//!   DELETE /api/v1/chaos-campaigns/{id}
//!   GET    /api/v1/chaos-campaigns/{id}/reports
//!   GET    /api/v1/workspaces/{workspace_id}/resilience-patterns

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use mockforge_registry_core::models::chaos::CreateChaosCampaign;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{ChaosCampaign, ChaosCampaignReport, CloudWorkspace, ResiliencePattern},
    AppState,
};

/// `GET /api/v1/workspaces/{workspace_id}/chaos-campaigns`
pub async fn list_campaigns(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ChaosCampaign>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let campaigns = ChaosCampaign::list_by_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(campaigns))
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub target_kind: String,
    pub target_ref: String,
    pub config: serde_json::Value,
    pub safety_config: serde_json::Value,
}

/// `POST /api/v1/workspaces/{workspace_id}/chaos-campaigns`
pub async fn create_campaign(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateCampaignRequest>,
) -> ApiResult<Json<ChaosCampaign>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;

    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidRequest("name must not be empty".into()));
    }
    if !ChaosCampaign::is_valid_target_kind(&request.target_kind) {
        return Err(ApiError::InvalidRequest(
            "target_kind must be 'hosted_mock' or 'external'".into(),
        ));
    }
    if request.target_ref.trim().is_empty() {
        return Err(ApiError::InvalidRequest("target_ref must not be empty".into()));
    }

    let campaign = ChaosCampaign::create(
        state.db.pool(),
        CreateChaosCampaign {
            workspace_id,
            name: &request.name,
            description: request.description.as_deref(),
            target_kind: &request.target_kind,
            target_ref: &request.target_ref,
            config: &request.config,
            safety_config: &request.safety_config,
            created_by: Some(user_id),
        },
    )
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(campaign))
}

/// `GET /api/v1/chaos-campaigns/{id}`
pub async fn get_campaign(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ChaosCampaign>> {
    let campaign = load_authorized_campaign(&state, user_id, &headers, id).await?;
    Ok(Json(campaign))
}

/// `DELETE /api/v1/chaos-campaigns/{id}`
pub async fn delete_campaign(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    load_authorized_campaign(&state, user_id, &headers, id).await?;

    let deleted = ChaosCampaign::delete(state.db.pool(), id).await.map_err(ApiError::Database)?;
    if !deleted {
        return Err(ApiError::InvalidRequest("Campaign not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// `GET /api/v1/chaos-campaigns/{id}/reports`
pub async fn list_campaign_reports(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ChaosCampaignReport>>> {
    let campaign = load_authorized_campaign(&state, user_id, &headers, id).await?;
    let reports = ChaosCampaignReport::list_by_campaign(state.db.pool(), campaign.id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(reports))
}

/// `GET /api/v1/workspaces/{workspace_id}/resilience-patterns`
pub async fn list_patterns(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ResiliencePattern>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let patterns = ResiliencePattern::list_visible_to_workspace(state.db.pool(), workspace_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(patterns))
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(())
}

async fn load_authorized_campaign(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<ChaosCampaign> {
    let campaign = ChaosCampaign::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Campaign not found".into()))?;
    let workspace = CloudWorkspace::find_by_id(state.db.pool(), campaign.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Campaign not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Campaign not found".into()));
    }
    Ok(campaign)
}
