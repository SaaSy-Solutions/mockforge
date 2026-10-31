//! Internal endpoints for the Cloudflare hosted-mocks Worker.
//!
//! - `GET  /api/v1/internal/hosted-mocks/runtime/{org_id}/{slug}`: the
//!   environment and limits a mock's container starts with.
//! - `POST /api/v1/internal/hosted-mocks/{deployment_id}/usage`: request and
//!   egress counts the Worker has served since its last report.
//!
//! Auth: the shared `MOCKFORGE_INTERNAL_API_TOKEN` bearer token. The Worker is
//! a separate trust principal with no user, so both run on the owner pool.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    deployment::{cloudflare, machine_config::hosted_mock_runtime_env, router},
    error::{ApiError, ApiResult},
    handlers::internal_test_runs::require_internal_auth,
    models::{DeploymentStatus, HostedMock, UsageCounter},
    AppState,
};

#[derive(Debug, Serialize)]
pub struct RuntimeLimits {
    pub max_body_bytes: usize,
    pub rps: i64,
}

#[derive(Debug, Serialize)]
pub struct RuntimeResponse {
    pub deployment_id: Uuid,
    pub env: HashMap<String, String>,
    pub limits: RuntimeLimits,
    /// The org has reached its opt-in overage ceiling; serve 429.
    pub blocked: bool,
}

/// `GET /api/v1/internal/hosted-mocks/runtime/{org_id}/{slug}`
///
/// 404 unless the mock exists, is active and is served from Cloudflare, so a
/// deleted or moved mock stops starting at the edge.
pub async fn get_runtime(
    State(state): State<AppState>,
    Path((org_id, slug)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<RuntimeResponse>> {
    require_internal_auth(&headers)?;
    let pool = state.db.pool();

    let deployment = HostedMock::find_by_slug(pool, org_id, &slug)
        .await
        .map_err(ApiError::Database)?
        .filter(|d| matches!(d.status(), DeploymentStatus::Active) && cloudflare::is_cloudflare(d))
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;

    let env = hosted_mock_runtime_env(&deployment, None).map_err(ApiError::Internal)?;
    let limits = router::resolve_proxy_limits(pool, org_id).await;
    let blocked = router::quota_ceiling_reached(pool, org_id).await.is_some();

    Ok(Json(RuntimeResponse {
        deployment_id: deployment.id,
        env,
        limits: RuntimeLimits {
            max_body_bytes: limits.max_body_bytes,
            rps: limits.rps,
        },
        blocked,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UsageReport {
    pub requests: i64,
    #[serde(default)]
    pub egress_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub blocked: bool,
}

/// `POST /api/v1/internal/hosted-mocks/{deployment_id}/usage`
///
/// Adds the reported counts to the deployment's org and returns whether the
/// org is now over its overage ceiling. The org comes from the deployment
/// row, never from the request.
pub async fn report_usage(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(report): Json<UsageReport>,
) -> ApiResult<Json<UsageResponse>> {
    require_internal_auth(&headers)?;
    if report.requests < 0 || report.egress_bytes < 0 {
        return Err(ApiError::InvalidRequest("counts must be non-negative".into()));
    }
    let pool = state.db.pool();

    let deployment = HostedMock::find_by_id(pool, deployment_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;
    let org_id = deployment.org_id;

    if report.requests > 0 || report.egress_bytes > 0 {
        let mut conn = pool.acquire().await?;
        if report.requests > 0 {
            UsageCounter::increment_requests(&mut conn, org_id, report.requests)
                .await
                .map_err(ApiError::Database)?;
        }
        if report.egress_bytes > 0 {
            UsageCounter::increment_egress(&mut conn, org_id, report.egress_bytes)
                .await
                .map_err(ApiError::Database)?;
        }
    }

    let blocked = router::quota_ceiling_reached(pool, org_id).await.is_some();
    Ok(Json(UsageResponse { blocked }))
}
