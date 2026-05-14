//! Cloud resilience handlers (#468) — Phase 1 (cloud scaffold).
//!
//! The local `ResiliencePage` reads circuit-breaker + bulkhead state from
//! `/api/resilience/*`. The state managers
//! (`mockforge_chaos::resilience::CircuitBreakerManager` / `BulkheadManager`)
//! exist, but no part of `mockforge-cli serve`, the admin-UI binary, or the
//! hosted-mock runtime daemon currently installs them as middleware in the
//! request path. That gap is the actual #468 work; the runtime wire-up is
//! tracked separately in the #468 description after the scope correction.
//!
//! This Phase 1 ships the *cloud-side API surface* so:
//!   - The UI can branch on `isCloudMode()` and call the registry instead
//!     of the never-mounted local routes.
//!   - The future runtime PR has a stable contract to wire its state into.
//!   - The `resilience` nav item is unblocked from `cloudHiddenNavItemIds`.
//!
//! Endpoints return empty payloads with a top-level `runtime_state` field
//! carrying `"pending"` so the UI can render an honest empty state instead
//! of a generic spinner. Once the runtime ingest channel lands, this module
//! reads from a `runtime_resilience_state` table (or equivalent) and
//! flips `runtime_state` to `"live"`.
//!
//! Routes:
//!   GET  /api/v1/workspaces/{workspace_id}/resilience/circuit-breakers
//!   GET  /api/v1/workspaces/{workspace_id}/resilience/bulkheads
//!   GET  /api/v1/workspaces/{workspace_id}/resilience/summary
//!   POST /api/v1/workspaces/{workspace_id}/resilience/circuit-breakers/{endpoint}/reset
//!   POST /api/v1/workspaces/{workspace_id}/resilience/bulkheads/{service}/reset

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::CloudWorkspace,
    AppState,
};

/// Cloud circuit-breaker state. Mirrors
/// `mockforge_chaos::resilience_api::CircuitBreakerStateResponse` so the UI
/// can swap services without changing its types.
#[derive(Debug, Clone, Serialize)]
pub struct CircuitBreakerStateResponse {
    pub endpoint: String,
    pub state: String,
    pub stats: CircuitStatsResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct CircuitStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub consecutive_failures: u64,
    pub consecutive_successes: u64,
    pub success_rate: f64,
    pub failure_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkheadStateResponse {
    pub service: String,
    pub stats: BulkheadStatsResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct BulkheadStatsResponse {
    pub active_requests: u32,
    pub queued_requests: u32,
    pub total_requests: u64,
    pub rejected_requests: u64,
    pub timeout_requests: u64,
    pub utilization_percent: f64,
}

/// Envelope wrapping each GET response with a `runtime_state` discriminator.
///
/// The UI checks this field to decide between "no breakers configured yet"
/// (empty + state=`live`) and "runtime instrumentation not yet wired"
/// (empty + state=`pending`). Once runtime ingest lands, the inner `data`
/// list populates and `runtime_state` stays `live`.
#[derive(Debug, Serialize)]
pub struct ResilienceEnvelope<T: Serialize> {
    pub runtime_state: &'static str,
    pub data: Vec<T>,
}

impl<T: Serialize> ResilienceEnvelope<T> {
    fn pending() -> Self {
        Self {
            runtime_state: "pending",
            data: Vec::new(),
        }
    }
}

// --- GET handlers ---------------------------------------------------------

pub async fn list_circuit_breakers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ResilienceEnvelope<CircuitBreakerStateResponse>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    // Runtime wire-up is pending; return empty + pending discriminator.
    Ok(Json(ResilienceEnvelope::pending()))
}

pub async fn list_bulkheads(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ResilienceEnvelope<BulkheadStateResponse>>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    Ok(Json(ResilienceEnvelope::pending()))
}

pub async fn get_summary(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(workspace_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    // Same shape as `mockforge_chaos::resilience_api::get_dashboard_summary`
    // so the UI's summary cards render identically; `runtime_state` flags
    // the data as scaffold rather than live.
    Ok(Json(json!({
        "runtime_state": "pending",
        "circuit_breakers": {
            "total": 0,
            "open": 0,
            "half_open": 0,
            "closed": 0,
        },
        "bulkheads": {
            "total": 0,
            "active_requests": 0,
            "queued_requests": 0,
        },
    })))
}

// --- POST handlers --------------------------------------------------------

/// Reset a circuit breaker. No-ops while the runtime ingest channel is
/// pending; returns the same JSON envelope shape `{accepted, runtime_state,
/// reason}` for both states so the UI doesn't need conditional parsing once
/// the live runtime lands.
pub async fn reset_circuit_breaker(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((workspace_id, endpoint)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    tracing::info!(
        %workspace_id,
        endpoint = %endpoint,
        "circuit-breaker reset requested while runtime wire-up is pending",
    );
    Ok(Json(json!({
        "accepted": false,
        "runtime_state": "pending",
        "reason": "Resilience runtime wire-up is pending; this reset is a no-op until it lands.",
    })))
}

pub async fn reset_bulkhead(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((workspace_id, service)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    authorize_workspace(&state, user_id, &headers, workspace_id).await?;
    tracing::info!(
        %workspace_id,
        service = %service,
        "bulkhead reset requested while runtime wire-up is pending",
    );
    Ok(Json(json!({
        "accepted": false,
        "runtime_state": "pending",
        "reason": "Resilience runtime wire-up is pending; this reset is a no-op until it lands.",
    })))
}

// --- helpers --------------------------------------------------------------

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_pending_is_empty() {
        let env: ResilienceEnvelope<CircuitBreakerStateResponse> = ResilienceEnvelope::pending();
        assert_eq!(env.runtime_state, "pending");
        assert!(env.data.is_empty());
    }

    #[test]
    fn envelope_serialization_matches_contract() {
        // The UI keys off `runtime_state` and `data`. Lock the JSON shape
        // so a refactor doesn't silently break the client.
        let env: ResilienceEnvelope<CircuitBreakerStateResponse> = ResilienceEnvelope::pending();
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "pending");
        assert!(body["data"].is_array());
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
    }
}
