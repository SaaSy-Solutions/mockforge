//! Cloud resilience handlers (#468) — Phase 2 (runtime proxy).
//!
//! Live circuit-breaker + bulkhead state from a hosted mock. The deployment
//! itself runs `mockforge serve` with the resilience middleware installed
//! (#518) and exposes `/api/resilience/*` on its admin port. We reach it
//! over Fly.io's private network (`{fly-app}.internal:9080`).
//!
//! Phase 1 of #468 (PR #517) shipped a workspace-scoped envelope that
//! always returned `runtime_state: "pending"`. Phase 2 (this PR) corrects
//! the scope — resilience state is per-deployment, not per-workspace —
//! and replaces the placeholder with a real reqwest-driven proxy.
//!
//! ## Wire format
//!
//! Every GET returns `{ runtime_state, data }`. `runtime_state` is one of:
//! * `"live"` — proxy succeeded; `data` is whatever the deployment reported.
//! * `"unreachable"` — proxy failed (connection refused, timeout, non-2xx
//!   upstream, etc.); `data` is empty. The UI shows a "deployment not
//!   reachable" banner. Picked over `"pending"` because it covers every
//!   non-live state with one label and is honest about *why* the page is
//!   empty: the registry tried and the deployment did not answer.
//!
//! POST resets return `{ accepted, runtime_state, reason }`. `accepted=true`
//! is only set on a 2xx from the upstream reset endpoint.
//!
//! ## Routes
//!
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/resilience/circuit-breakers`
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/resilience/bulkheads`
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/resilience/summary`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/resilience/circuit-breakers/{endpoint}/reset`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/resilience/bulkheads/{service}/reset`

use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::HostedMock,
    AppState,
};

/// Admin port the hosted mock exposes `/api/resilience/*` on. Matches the
/// default in `mockforge-core::config::protocol::AdminConfig`. Fly machines
/// expose ports bound to `0.0.0.0` on the private 6PN network, so the
/// admin server is reachable from the registry pod even though no
/// `[[services]]` entry publishes it.
const ADMIN_PORT: u16 = 9080;

/// Reqwest timeout for one proxy call. The dashboard polls every few
/// seconds, so a slow call shouldn't block UX — fail fast and let the
/// next poll retry.
const PROXY_TIMEOUT: Duration = Duration::from_secs(3);

// --- Wire types -----------------------------------------------------------
//
// These mirror `mockforge_chaos::resilience_api::*Response`. Duplicated here
// (rather than `pub use`d) so the registry doesn't pull in the whole chaos
// crate; the upstream shape changes rarely and Deserialize keeps us
// resilient to additive fields.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStateResponse {
    pub endpoint: String,
    pub state: String,
    pub stats: CircuitStatsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStateResponse {
    pub service: String,
    pub stats: BulkheadStatsResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStatsResponse {
    pub active_requests: u32,
    pub queued_requests: u32,
    pub total_requests: u64,
    pub rejected_requests: u64,
    pub timeout_requests: u64,
    pub utilization_percent: f64,
}

/// Envelope wrapping each GET response with a `runtime_state` discriminator.
#[derive(Debug, Serialize)]
pub struct ResilienceEnvelope<T: Serialize> {
    pub runtime_state: &'static str,
    pub data: Vec<T>,
}

impl<T: Serialize> ResilienceEnvelope<T> {
    fn live(data: Vec<T>) -> Self {
        Self {
            runtime_state: "live",
            data,
        }
    }

    fn unreachable() -> Self {
        Self {
            runtime_state: "unreachable",
            data: Vec::new(),
        }
    }
}

// --- GET handlers ---------------------------------------------------------

pub async fn list_circuit_breakers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ResilienceEnvelope<CircuitBreakerStateResponse>>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let base_url = admin_base_url(&deployment);
    let url = format!("{base_url}/api/resilience/circuit-breakers");
    Ok(Json(match proxy_get_vec::<CircuitBreakerStateResponse>(&url).await {
        Ok(data) => ResilienceEnvelope::live(data),
        Err(err) => {
            tracing::warn!(%deployment_id, error = %err, "resilience proxy GET failed");
            ResilienceEnvelope::unreachable()
        }
    }))
}

pub async fn list_bulkheads(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<ResilienceEnvelope<BulkheadStateResponse>>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let base_url = admin_base_url(&deployment);
    let url = format!("{base_url}/api/resilience/bulkheads");
    Ok(Json(match proxy_get_vec::<BulkheadStateResponse>(&url).await {
        Ok(data) => ResilienceEnvelope::live(data),
        Err(err) => {
            tracing::warn!(%deployment_id, error = %err, "resilience proxy GET failed");
            ResilienceEnvelope::unreachable()
        }
    }))
}

pub async fn get_summary(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let base_url = admin_base_url(&deployment);
    let url = format!("{base_url}/api/resilience/dashboard/summary");
    Ok(Json(match proxy_get_value(&url).await {
        Ok(mut body) => {
            // The hosted mock's summary endpoint returns the raw stats
            // object without a runtime_state tag; tag it here so the UI's
            // single discriminator path keeps working.
            if let Value::Object(ref mut map) = body {
                map.insert("runtime_state".into(), Value::String("live".into()));
            }
            body
        }
        Err(err) => {
            tracing::warn!(%deployment_id, error = %err, "resilience proxy GET summary failed");
            json!({
                "runtime_state": "unreachable",
                "circuit_breakers": { "total": 0, "open": 0, "half_open": 0, "closed": 0 },
                "bulkheads": { "total": 0, "active_requests": 0, "queued_requests": 0 },
            })
        }
    }))
}

// --- POST handlers --------------------------------------------------------

pub async fn reset_circuit_breaker(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((deployment_id, endpoint)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let base_url = admin_base_url(&deployment);
    // The hosted mock's endpoint segment is itself a path component that
    // may contain `/`, so urlencode it.
    let url = format!(
        "{base_url}/api/resilience/circuit-breakers/{}/reset",
        urlencoding::encode(&endpoint),
    );
    Ok(Json(match proxy_post_empty(&url).await {
        Ok(()) => json!({ "accepted": true, "runtime_state": "live" }),
        Err(err) => {
            tracing::warn!(%deployment_id, %endpoint, error = %err, "resilience proxy POST reset failed");
            json!({
                "accepted": false,
                "runtime_state": "unreachable",
                "reason": err.to_string(),
            })
        }
    }))
}

pub async fn reset_bulkhead(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((deployment_id, service)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let base_url = admin_base_url(&deployment);
    let url =
        format!("{base_url}/api/resilience/bulkheads/{}/reset", urlencoding::encode(&service),);
    Ok(Json(match proxy_post_empty(&url).await {
        Ok(()) => json!({ "accepted": true, "runtime_state": "live" }),
        Err(err) => {
            tracing::warn!(%deployment_id, %service, error = %err, "resilience proxy POST reset failed");
            json!({
                "accepted": false,
                "runtime_state": "unreachable",
                "reason": err.to_string(),
            })
        }
    }))
}

// --- helpers --------------------------------------------------------------

/// Build the 6PN admin URL for a hosted mock. The deployment is a Fly app
/// named via [`HostedMock::fly_app_name`]; Fly resolves `{name}.internal`
/// to a private IPv6 reachable from the registry pod.
fn admin_base_url(deployment: &HostedMock) -> String {
    format!("http://{}.internal:{ADMIN_PORT}", deployment.fly_app_name())
}

/// GET a JSON list. 2xx + valid JSON → Ok; anything else → Err.
async fn proxy_get_vec<T: for<'de> Deserialize<'de>>(url: &str) -> reqwest::Result<Vec<T>> {
    reqwest::Client::builder()
        .timeout(PROXY_TIMEOUT)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<T>>()
        .await
}

/// GET an arbitrary JSON object (used for the summary endpoint, which has
/// a fixed shape but the registry doesn't model it strongly).
async fn proxy_get_value(url: &str) -> reqwest::Result<Value> {
    reqwest::Client::builder()
        .timeout(PROXY_TIMEOUT)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await
}

/// POST with no body, discard response. Used for reset endpoints; 2xx is
/// the only signal we care about.
async fn proxy_post_empty(url: &str) -> reqwest::Result<()> {
    reqwest::Client::builder()
        .timeout(PROXY_TIMEOUT)
        .build()?
        .post(url)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// Resolve `deployment_id` to a `HostedMock`, after confirming the caller's
/// org matches. Returns the deployment so callers can build the admin URL
/// without a second DB hit.
async fn authorize_deployment(
    state: &AppState,
    user_id: Uuid,
    headers: &HeaderMap,
    deployment_id: Uuid,
) -> ApiResult<HostedMock> {
    let deployment = HostedMock::find_by_id(state.db.pool(), deployment_id)
        .await?
        .ok_or_else(|| ApiError::InvalidRequest("Deployment not found".into()))?;
    let ctx = resolve_org_context(state, user_id, headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    if ctx.org_id != deployment.org_id {
        return Err(ApiError::InvalidRequest("Deployment not found".into()));
    }
    Ok(deployment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_live_round_trips_data() {
        let env = ResilienceEnvelope::live(vec![BulkheadStateResponse {
            service: "http".into(),
            stats: BulkheadStatsResponse {
                active_requests: 3,
                queued_requests: 0,
                total_requests: 17,
                rejected_requests: 0,
                timeout_requests: 0,
                utilization_percent: 3.0,
            },
        }]);
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "live");
        assert_eq!(body["data"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"][0]["service"], "http");
        assert_eq!(body["data"][0]["stats"]["active_requests"], 3);
    }

    #[test]
    fn envelope_unreachable_is_empty() {
        let env: ResilienceEnvelope<CircuitBreakerStateResponse> =
            ResilienceEnvelope::unreachable();
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "unreachable");
        assert_eq!(body["data"].as_array().unwrap().len(), 0);
    }
}
