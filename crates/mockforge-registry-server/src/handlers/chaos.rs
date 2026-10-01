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
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::{ChaosCampaign, ChaosCampaignReport, CloudWorkspace, ResiliencePattern, TestRun},
    store::with_org_context,
    AppState,
};

/// `GET /api/v1/workspaces/{workspace_id}/chaos-campaigns`
pub async fn list_campaigns(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ChaosCampaign>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    let campaigns = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(ChaosCampaign::list_by_workspace(&mut **tx, workspace_id).await?) },
        )
    })
    .await?;
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
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;

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

    let campaign = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(ChaosCampaign::create(
                &mut **tx,
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
            .await?)
        })
    })
    .await?;

    Ok(Json(campaign))
}

/// `GET /api/v1/chaos-campaigns/{id}`
pub async fn get_campaign(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ChaosCampaign>> {
    let (campaign, _) = load_authorized_campaign_with_org(&state, user_id, &headers, id).await?;
    Ok(Json(campaign))
}

/// `DELETE /api/v1/chaos-campaigns/{id}`
pub async fn delete_campaign(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let (_, org_id) = load_authorized_campaign_with_org(&state, user_id, &headers, id).await?;

    let deleted = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(ChaosCampaign::delete(&mut **tx, id).await?) })
    })
    .await?;
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
    let (campaign, org_id) =
        load_authorized_campaign_with_org(&state, user_id, &headers, id).await?;
    let campaign_id = campaign.id;
    let reports = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(
            async move { Ok(ChaosCampaignReport::list_by_campaign(&mut **tx, campaign_id).await?) },
        )
    })
    .await?;
    Ok(Json(reports))
}

/// `POST /api/v1/chaos-campaigns/{id}/runs`
///
/// Triggers a chaos run. Reuses the #4 test_runs lifecycle with
/// kind='chaos_campaign' so it shares the runner pool, concurrency cap,
/// and runner_seconds metering with regular test runs. The runner-side
/// ChaosExecutor picks up the queued job and reports results back via
/// the internal callbacks.
pub async fn trigger_run(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TestRun>> {
    // `org_id` is the campaign's workspace org, verified against the caller.
    let (campaign, org_id) =
        load_authorized_campaign_with_org(&state, user_id, &headers, id).await?;

    // Reuse the test_runs concurrency cap so chaos campaigns and unit
    // tests fight over the same runner-pool slots — a single org can't
    // dodge max_concurrent_runs by mixing kinds.
    let org = mockforge_registry_core::models::Organization::find_by_id(state.db.pool(), org_id)
        .await
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("DB error loading org")))?
        .ok_or_else(|| ApiError::InvalidRequest("Organization not found".into()))?;
    let limits = crate::handlers::usage::effective_limits(&state, &org).await?;
    let max_concurrent = limits.get("max_concurrent_runs").and_then(|v| v.as_i64()).unwrap_or(0);
    if max_concurrent == 0 {
        return Err(ApiError::ResourceLimitExceeded(
            "Test execution / chaos is not enabled on this plan".into(),
        ));
    }
    // Concurrency cap + create the test_runs row in one org-bound
    // transaction — `suite_id` reuses the column for the owning resource
    // id, matching how flows etc. will use it.
    let campaign_id = campaign.id;
    let run = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            if max_concurrent > 0 {
                let inflight = TestRun::count_inflight(tx, org_id).await?;
                if inflight.total() >= max_concurrent {
                    return Ok(Err(inflight.total()));
                }
            }
            Ok(Ok(TestRun::enqueue(
                &mut **tx,
                EnqueueTestRun {
                    suite_id: campaign_id,
                    org_id,
                    kind: "chaos_campaign",
                    triggered_by: "manual",
                    triggered_by_user: Some(user_id),
                    git_ref: None,
                    git_sha: None,
                },
            )
            .await?))
        })
    })
    .await?
    .map_err(|inflight_total| {
        ApiError::ResourceLimitExceeded(format!(
            "Concurrent run limit reached ({}/{}).",
            inflight_total, max_concurrent,
        ))
    })?;

    // Push onto the queue so the runner's ChaosExecutor picks it up.
    // Includes the full campaign config + safety config so the executor
    // can iterate the user's declared faults instead of synthesizing.
    if let Err(e) = crate::run_queue::enqueue(
        state.redis.as_ref(),
        crate::run_queue::EnqueuedJob {
            run_id: run.id,
            org_id: run.org_id,
            source_id: campaign.id,
            kind: "chaos_campaign",
            payload: serde_json::json!({
                "target_kind": campaign.target_kind,
                "target_ref": campaign.target_ref,
                "config": campaign.config,
                "safety_config": campaign.safety_config,
            }),
        },
    )
    .await
    {
        tracing::error!(run_id = %run.id, error = %e, "failed to enqueue chaos run");
    }

    Ok(Json(run))
}

/// `GET /api/v1/workspaces/{workspace_id}/resilience-patterns`
pub async fn list_patterns(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ResiliencePattern>>> {
    let org_id = authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    // Bound to the caller's org: the policy also admits the platform rows
    // (`workspace_id IS NULL`) that the query asks for.
    let patterns = with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(ResiliencePattern::list_visible_to_workspace(&mut **tx, workspace_id).await?)
        })
    })
    .await?;
    Ok(Json(patterns))
}

async fn authorize_workspace(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    workspace_id: Uuid,
) -> ApiResult<Uuid> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let workspace = find_workspace_in_org(state, ctx.org_id, workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Workspace not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Workspace not found".into()));
    }
    Ok(ctx.org_id)
}

/// Load a campaign the caller may access, returning it with the authorized
/// org id. The campaign is read bound to the caller's org, so a campaign in
/// another org reads as `None` and gets the same "Campaign not found".
async fn load_authorized_campaign_with_org(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    id: Uuid,
) -> ApiResult<(ChaosCampaign, Uuid)> {
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let campaign = with_org_context(state.db.runtime_pool(), ctx.org_id, |tx| {
        Box::pin(async move { Ok(ChaosCampaign::find_by_id(&mut **tx, id).await?) })
    })
    .await?
    .ok_or_else(|| ApiError::InvalidRequest("Campaign not found".into()))?;
    let workspace = find_workspace_in_org(state, ctx.org_id, campaign.workspace_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Campaign not found".into()))?;
    if ctx.org_id != workspace.org_id {
        return Err(ApiError::InvalidRequest("Campaign not found".into()));
    }
    Ok((campaign, ctx.org_id))
}

/// Load a workspace bound to `org_id`; a workspace in another org is `None`.
async fn find_workspace_in_org(
    state: &AppState,
    org_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<Option<CloudWorkspace>> {
    Ok(with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move { Ok(CloudWorkspace::find_by_id(&mut **tx, workspace_id).await?) })
    })
    .await?)
}
