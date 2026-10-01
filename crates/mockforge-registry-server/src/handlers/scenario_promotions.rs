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
        CloudWorkspace, OrgMember, OrgRole, PromotionStatus, Scenario, ScenarioEnvironmentVersion,
        ScenarioPromotion,
    },
    store::with_org_context,
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
    Path((workspace_id, _environment)): Path<(Uuid, String)>,
    Json(request): Json<PromoteScenarioRequest>,
) -> ApiResult<Json<PromoteScenarioResponse>> {
    // Resolve org context for authorization
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioPromote permission
    let member = find_member(&state, org_ctx.org_id, user_id)
        .await?
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
        .map_err(ApiError::InvalidRequest)?;

    // Get scenario. Any scenario the caller could see in the marketplace may
    // be promoted: their own org's, a public one, or one another org has
    // published. Authorization is the workspace check below, not scenario
    // ownership.
    let scenario = find_promotable_scenario(&state, org_ctx.org_id, request.scenario_id)
        .await?
        .ok_or_else(|| ApiError::ScenarioNotFound("Scenario not found".to_string()))?;

    // Determine if approval is required
    let approval_rules = mockforge_core::workspace::ApprovalRules::default();
    let (requires_approval, approval_reason) =
        mockforge_core::workspace::ScenarioPromotionWorkflow::requires_approval(
            &scenario.tags,
            to_env,
            &approval_rules,
        );

    // Create promotion record (and auto-complete it when no approval is
    // required), bound to the caller's org. Promotion rows are keyed by
    // workspace, so the workspace must belong to that org; a workspace in
    // another org reads as absent under RLS.
    let reason = approval_reason.clone();
    let promotion = with_org_context(state.db.runtime_pool(), org_ctx.org_id, move |tx| {
        Box::pin(async move {
            if !workspace_in_org(tx, workspace_id, org_ctx.org_id).await? {
                return Ok(None);
            }
            let promotion = ScenarioPromotion::create(
                &mut **tx,
                request.scenario_id,
                &request.scenario_version,
                workspace_id,
                from_env.as_str(),
                to_env.as_str(),
                user_id,
                requires_approval,
                reason.as_deref(),
                request.comments.as_deref(),
            )
            .await?;

            // If no approval required, auto-complete the promotion
            if !requires_approval {
                // Set the version in the target environment
                ScenarioEnvironmentVersion::set_version(
                    &mut **tx,
                    request.scenario_id,
                    workspace_id,
                    to_env.as_str(),
                    &request.scenario_version,
                    user_id,
                    Some(promotion.id),
                )
                .await?;

                // Mark promotion as completed
                ScenarioPromotion::mark_completed(&mut **tx, promotion.id).await?;
            }
            Ok(Some(promotion))
        })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".to_string()))?;

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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioPromote permission (needed to view promotions)
    let member = find_member(&state, org_ctx.org_id, user_id)
        .await?
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

    require_workspace_in_org(&state, org_ctx.org_id, workspace_id).await?;

    // Parse status filter
    let status_filter = params.status.and_then(|s| PromotionStatus::from_str(&s));

    // Get promotions, bound to the caller's org: a workspace in another org
    // has no visible promotions.
    let promotions =
        with_org_context(state.db.runtime_pool(), org_ctx.org_id, move |tx| {
            Box::pin(async move {
                Ok(ScenarioPromotion::list_by_workspace(&mut **tx, workspace_id, status_filter)
                    .await?)
            })
        })
        .await?;

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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioApprove permission
    let member = find_member(&state, org_ctx.org_id, user_id)
        .await?
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

    // Get promotion, bound to the caller's org: a promotion in another org's
    // workspace reads as absent ("Promotion not found").
    let org_id = org_ctx.org_id;
    require_workspace_in_org(&state, org_id, workspace_id).await?;
    let promotion = find_promotion(&state, org_id, promotion_id).await?;

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

    let approved = with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move {
            // Approve the promotion
            let approved =
                promotion.approve(&mut **tx, user_id, request.comments.as_deref()).await?;

            // Complete the promotion by setting the version in the target environment
            ScenarioEnvironmentVersion::set_version(
                &mut **tx,
                approved.scenario_id,
                workspace_id,
                &approved.to_environment,
                &approved.scenario_version,
                user_id,
                Some(approved.id),
            )
            .await?;

            // Mark promotion as completed
            ScenarioPromotion::mark_completed(&mut **tx, approved.id).await?;
            Ok(approved)
        })
    })
    .await?;

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
    // Resolve org context
    let org_ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::AuthRequired)?;

    // Check fine-grained RBAC for ScenarioApprove permission
    let member = find_member(&state, org_ctx.org_id, user_id)
        .await?
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

    // Get promotion, bound to the caller's org: a promotion in another org's
    // workspace reads as absent ("Promotion not found").
    let org_id = org_ctx.org_id;
    require_workspace_in_org(&state, org_id, workspace_id).await?;
    let promotion = find_promotion(&state, org_id, promotion_id).await?;

    // Verify it's for the correct workspace
    if promotion.workspace_id != workspace_id {
        return Err(ApiError::InvalidRequest(
            "Promotion does not belong to this workspace".to_string(),
        ));
    }

    // Reject the promotion
    let rejected = with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move { Ok(promotion.reject(&mut **tx, user_id, &request.reason).await?) })
    })
    .await?;

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

/// Membership lookup for the RBAC check, on the request-path (RLS) pool bound
/// to the org being authorized against (#1087).
async fn find_member(
    state: &AppState,
    org_id: Uuid,
    user_id: Uuid,
) -> ApiResult<Option<OrgMember>> {
    Ok(with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move { Ok(OrgMember::find(&mut **tx, org_id, user_id).await?) })
    })
    .await?)
}

/// Load a promotion bound to `org_id` (already authorized via membership).
/// A promotion in another org's workspace is invisible under RLS and surfaces
/// as the same "Promotion not found" as a missing row.
async fn find_promotion(
    state: &AppState,
    org_id: Uuid,
    promotion_id: Uuid,
) -> ApiResult<ScenarioPromotion> {
    with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move { Ok(ScenarioPromotion::find_by_id(&mut **tx, promotion_id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Promotion not found".to_string()))
}

/// Load a scenario the caller's org may promote: its own, a public one
/// (`org_id IS NULL`), or another org's scenario that is published in the
/// marketplace (has at least one non-yanked version).
///
/// Owner pool on purpose: the `scenarios` policy only admits the bound org's
/// rows and public ones, but promoting another org's published marketplace
/// scenario is a supported flow (on main it ran on the owner pool). The
/// visibility rule above is applied in SQL, and the promotion itself is
/// authorized by `workspace_in_org` and written on the runtime pool.
async fn find_promotable_scenario(
    state: &AppState,
    org_id: Uuid,
    scenario_id: Uuid,
) -> ApiResult<Option<Scenario>> {
    sqlx::query_as::<_, Scenario>(
        r#"
        SELECT s.* FROM scenarios s
        WHERE s.id = $1
          AND (s.org_id = $2
               OR s.org_id IS NULL
               OR EXISTS (SELECT 1 FROM scenario_versions v
                          WHERE v.scenario_id = s.id AND NOT v.yanked))
        "#,
    )
    .bind(scenario_id)
    .bind(org_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(ApiError::Database)
}

/// App-layer twin of the RLS policy: reject a workspace outside the caller's
/// org before touching its promotions. Without it the owner pool (RLS inert)
/// would list or act on another org's promotions.
async fn require_workspace_in_org(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<()> {
    let in_org = with_org_context(state.db.runtime_pool(), org_id, move |tx| {
        Box::pin(async move { Ok(workspace_in_org(tx, workspace_id, org_id).await?) })
    })
    .await?;
    if in_org {
        Ok(())
    } else {
        Err(ApiError::InvalidRequest("Workspace not found".to_string()))
    }
}

/// Whether `workspace_id` belongs to `org_id`, read on the caller's bound
/// transaction (a workspace in another org reads as absent).
async fn workspace_in_org(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    org_id: Uuid,
) -> sqlx::Result<bool> {
    Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id)
        .await?
        .is_some_and(|ws| ws.org_id == org_id))
}
