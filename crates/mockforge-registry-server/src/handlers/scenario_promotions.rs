//! Scenario promotion handlers
//!
//! Handles scenario promotion workflow between environments (dev → test → prod)
//! with approval support for high-impact changes.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{
        OrgMember, OrgRole, PromotionStatus, Scenario, ScenarioEnvironmentVersion,
        ScenarioPromotion,
    },
    AppState,
};
use mockforge_collab::models::UserRole;
use mockforge_collab::permissions::{Permission, RolePermissions};
use mockforge_core::workspace::MockEnvironmentName;

/// Promote a scenario from one environment to another
///
/// POST /api/v1/workspaces/{workspace_id}/environments/{env}/promote-scenario
pub async fn promote_scenario(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, environment)): Path<(Uuid, String)>,
    Json(request): Json<PromoteScenarioRequest>,
) -> ApiResult<Json<PromoteScenarioResponse>> {
    let pool = state.db.pool();

    // Resolve org context for authorization
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioPromote permission
    let member = OrgMember::find(pool, org_ctx.org_id, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PermissionDenied)?;

    // Map OrgRole to UserRole for permission checking
    let user_role = match member.role() {
        OrgRole::Owner | OrgRole::Admin => UserRole::Admin,
        OrgRole::Member => UserRole::Editor,
    };

    // Check if user has ScenarioPromote permission
    if !RolePermissions::has_permission(user_role, Permission::ScenarioPromote) {
        return Err(ApiError::PermissionDenied);
    }

    // Parse environment names
    let from_env = MockEnvironmentName::from_str(&request.from_environment)
        .ok_or_else(|| ApiError::InvalidRequest("Invalid from_environment".to_string()))?;
    let to_env = MockEnvironmentName::from_str(&request.to_environment)
        .ok_or_else(|| ApiError::InvalidRequest("Invalid to_environment".to_string()))?;

    // Validate promotion path
    mockforge_core::workspace::ScenarioPromotionWorkflow::validate_promotion_path(from_env, to_env)
        .map_err(|e| ApiError::InvalidRequest(e))?;

    // Get scenario
    let scenario = Scenario::find_by_id(pool, request.scenario_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::ScenarioNotFound("Scenario not found".to_string()))?;

    // Determine if approval is required
    let approval_rules = mockforge_core::workspace::ApprovalRules::default();
    let (requires_approval, approval_reason) =
        mockforge_core::workspace::ScenarioPromotionWorkflow::requires_approval(
            &scenario.tags,
            to_env,
            &approval_rules,
        );

    // Create promotion record
    let promotion = ScenarioPromotion::create(
        pool,
        request.scenario_id,
        &request.scenario_version,
        workspace_id,
        from_env.as_str(),
        to_env.as_str(),
        user_id,
        requires_approval,
        approval_reason.as_deref(),
        request.comments.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // If no approval required, auto-complete the promotion
    if !requires_approval {
        // Set the version in the target environment
        ScenarioEnvironmentVersion::set_version(
            pool,
            request.scenario_id,
            workspace_id,
            to_env.as_str(),
            &request.scenario_version,
            user_id,
            Some(promotion.id),
        )
        .await
        .map_err(|e| ApiError::Database(e))?;

        // Mark promotion as completed
        ScenarioPromotion::mark_completed(pool, promotion.id)
            .await
            .map_err(|e| ApiError::Database(e))?;
    }

    Ok(Json(PromoteScenarioResponse {
        promotion_id: promotion.id,
        status: promotion.status_enum().unwrap_or(PromotionStatus::Pending),
        requires_approval,
        approval_reason,
        message: if requires_approval {
            "Promotion created and pending approval".to_string()
        } else {
            "Promotion completed successfully".to_string()
        },
    }))
}

/// List promotion history for a workspace
///
/// GET /api/v1/workspaces/{workspace_id}/promotions
pub async fn list_promotions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<PromotionListQuery>,
) -> ApiResult<Json<PromotionListResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioPromote permission (needed to view promotions)
    let member = OrgMember::find(pool, org_ctx.org_id, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PermissionDenied)?;

    // Map OrgRole to UserRole for permission checking
    let user_role = match member.role() {
        OrgRole::Owner | OrgRole::Admin => UserRole::Admin,
        OrgRole::Member => UserRole::Editor,
    };

    // Check if user has ScenarioPromote permission (needed to view promotions)
    if !RolePermissions::has_permission(user_role, Permission::ScenarioPromote) {
        return Err(ApiError::PermissionDenied);
    }

    // Parse status filter
    let status_filter = params.status.and_then(|s| PromotionStatus::from_str(&s));

    // Get promotions
    let promotions = ScenarioPromotion::list_by_workspace(pool, workspace_id, status_filter)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(PromotionListResponse { promotions }))
}

/// Approve a promotion
///
/// POST /api/v1/workspaces/{workspace_id}/promotions/{promotion_id}/approve
pub async fn approve_promotion(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, promotion_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<ApprovePromotionRequest>,
) -> ApiResult<Json<ApprovePromotionResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioApprove permission
    let member = OrgMember::find(pool, org_ctx.org_id, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PermissionDenied)?;

    // Map OrgRole to UserRole for permission checking
    let user_role = match member.role() {
        OrgRole::Owner | OrgRole::Admin => UserRole::Admin,
        OrgRole::Member => UserRole::Editor,
    };

    // Check if user has ScenarioApprove permission
    if !RolePermissions::has_permission(user_role, Permission::ScenarioApprove) {
        return Err(ApiError::PermissionDenied);
    }

    // Get promotion
    let promotion = ScenarioPromotion::find_by_id(pool, promotion_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Promotion not found".to_string()))?;

    // Verify it's for the correct workspace
    if promotion.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Promotion does not belong to this workspace".to_string(),
        ));
    }

    // Verify it's pending
    if promotion.status != "pending" {
        return Err(ApiError::InvalidRequest(format!(
            "Promotion is not pending (current status: {})",
            promotion.status
        )));
    }

    // Approve the promotion
    let approved = promotion
        .approve(pool, user_id, request.comments.as_deref())
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Complete the promotion by setting the version in the target environment
    ScenarioEnvironmentVersion::set_version(
        pool,
        approved.scenario_id,
        workspace_id,
        &approved.to_environment,
        &approved.scenario_version,
        user_id,
        Some(approved.id),
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Mark promotion as completed
    ScenarioPromotion::mark_completed(pool, approved.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(ApprovePromotionResponse {
        promotion_id: approved.id,
        status: PromotionStatus::Completed,
        message: "Promotion approved and completed".to_string(),
    }))
}

/// Reject a promotion
///
/// POST /api/v1/workspaces/{workspace_id}/promotions/{promotion_id}/reject
pub async fn reject_promotion(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Path((workspace_id, promotion_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<RejectPromotionRequest>,
) -> ApiResult<Json<RejectPromotionResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioApprove permission
    let member = OrgMember::find(pool, org_ctx.org_id, user_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PermissionDenied)?;

    // Map OrgRole to UserRole for permission checking
    let user_role = match member.role() {
        OrgRole::Owner | OrgRole::Admin => UserRole::Admin,
        OrgRole::Member => UserRole::Editor,
    };

    // Check if user has ScenarioApprove permission
    if !RolePermissions::has_permission(user_role, Permission::ScenarioApprove) {
        return Err(ApiError::PermissionDenied);
    }

    // Get promotion
    let promotion = ScenarioPromotion::find_by_id(pool, promotion_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest("Promotion not found".to_string()))?;

    // Verify it's for the correct workspace
    if promotion.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Promotion does not belong to this workspace".to_string(),
        ));
    }

    // Reject the promotion
    let rejected = promotion
        .reject(pool, user_id, &request.reason)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(RejectPromotionResponse {
        promotion_id: rejected.id,
        status: PromotionStatus::Rejected,
        message: "Promotion rejected".to_string(),
    }))
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct PromoteScenarioRequest {
    pub scenario_id: Uuid,
    pub scenario_version: String,
    pub from_environment: String,
    pub to_environment: String,
    pub comments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PromoteScenarioResponse {
    pub promotion_id: Uuid,
    pub status: PromotionStatus,
    pub requires_approval: bool,
    pub approval_reason: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PromotionListQuery {
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PromotionListResponse {
    pub promotions: Vec<ScenarioPromotion>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovePromotionRequest {
    pub comments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovePromotionResponse {
    pub promotion_id: Uuid,
    pub status: PromotionStatus,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectPromotionRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct RejectPromotionResponse {
    pub promotion_id: Uuid,
    pub status: PromotionStatus,
    pub message: String,
}
