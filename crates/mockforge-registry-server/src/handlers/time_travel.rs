//! Cloud time-travel handlers (#466).
//!
//! Live virtual-clock control for a hosted mock. The deployment itself runs
//! `mockforge serve`, which mounts `mockforge_http::time_travel_api::time_travel_router()`
//! at `/__mockforge/time-travel` on the **main HTTP port (3000)** — not the
//! admin port like resilience. The runtime-side router is intentionally a
//! subset of the local admin's full surface: only the 7 clock-control endpoints
//! (status / enable / disable / advance / set / scale / reset). Cron jobs and
//! mutation rules stay local-only because they don't belong to a hosted-mock's
//! single-process clock.
//!
//! ## Wire format
//!
//! GET `/status` is wrapped in `{ runtime_state, data }`:
//! * `"live"` — proxy succeeded; `data` is the deployment's clock state.
//! * `"unreachable"` — proxy failed (connection refused, timeout, non-2xx);
//!   `data` is a synthesized "disabled" state so the UI's existing rendering
//!   path keeps working. Same rationale as `resilience::ResilienceEnvelope`.
//!
//! Mutating POSTs return `{ accepted, runtime_state, status?, reason? }`.
//! `accepted=true` only on a 2xx from the upstream endpoint; the optional
//! `status` field carries the post-mutation state so the UI doesn't need a
//! follow-up GET to refresh.
//!
//! ## Routes
//!
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/time-travel/status`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/enable`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/disable`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/advance`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/set`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/scale`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/time-travel/reset`

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

/// Main HTTP port the hosted mock listens on. Matches the `PORT=3000` that
/// the orchestrator injects on every Fly deploy
/// (`deployment::orchestrator::deploy_to_flyio`). The time-travel routes
/// are mounted on this app by `mockforge serve` — not on the admin port,
/// because the admin port isn't always exposed publicly on a hosted mock.
const RUNTIME_HTTP_PORT: u16 = 3000;

/// Reqwest timeout for one proxy call. Time-travel is interactive — the UI
/// fires these from button clicks — so a slow upstream shouldn't lock the
/// browser. Fail fast and let the user retry.
const PROXY_TIMEOUT: Duration = Duration::from_secs(3);

// --- Wire types -----------------------------------------------------------
//
// These mirror `mockforge_http::time_travel_api::TimeTravelStatus` etc.
// Duplicated here (rather than `pub use`d) so the registry doesn't pull in
// the mockforge-http crate; the upstream shape changes rarely and Deserialize
// keeps us resilient to additive fields.

/// Mirrors the runtime's `TimeTravelStatus` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelStatusResponse {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_time: Option<String>,
    pub scale_factor: f64,
    pub real_time: String,
}

impl TimeTravelStatusResponse {
    /// Synthesized "disabled" state for `unreachable` responses. Matches the
    /// shape the UI already renders when time-travel has never been turned
    /// on, so the dashboard's empty state is the same UX as before.
    fn disabled_placeholder() -> Self {
        Self {
            enabled: false,
            current_time: None,
            scale_factor: 1.0,
            real_time: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Envelope discriminating live proxy vs unreachable upstream.
#[derive(Debug, Serialize)]
pub struct TimeTravelEnvelope<T: Serialize> {
    pub runtime_state: &'static str,
    pub data: T,
}

impl<T: Serialize> TimeTravelEnvelope<T> {
    fn live(data: T) -> Self {
        Self {
            runtime_state: "live",
            data,
        }
    }

    fn unreachable(data: T) -> Self {
        Self {
            runtime_state: "unreachable",
            data,
        }
    }
}

// --- Mutation request bodies ----------------------------------------------
//
// We accept these from the UI then forward verbatim to the runtime — the
// runtime owns the canonical schema. Defining them here keeps axum's
// extractor happy without leaking the runtime types into the registry.

#[derive(Debug, Deserialize, Serialize)]
pub struct EnableRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AdvanceRequest {
    pub duration: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SetTimeRequest {
    pub time: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SetScaleRequest {
    pub scale: f64,
}

// --- GET handlers ---------------------------------------------------------

pub async fn get_status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<TimeTravelEnvelope<TimeTravelStatusResponse>>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/status", runtime_base_url(&deployment));
    Ok(Json(match proxy_get::<TimeTravelStatusResponse>(&url).await {
        Ok(data) => TimeTravelEnvelope::live(data),
        Err(err) => {
            tracing::warn!(%deployment_id, error = %err, "time-travel proxy GET status failed");
            TimeTravelEnvelope::unreachable(TimeTravelStatusResponse::disabled_placeholder())
        }
    }))
}

// --- POST handlers --------------------------------------------------------

pub async fn enable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<EnableRequest>,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/enable", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &body, deployment_id, "enable").await))
}

pub async fn disable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/disable", runtime_base_url(&deployment));
    // `disable` has no body; upstream tolerates empty `{}`.
    Ok(Json(proxy_post_json(&url, &json!({}), deployment_id, "disable").await))
}

pub async fn advance(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<AdvanceRequest>,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/advance", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &body, deployment_id, "advance").await))
}

pub async fn set_time(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SetTimeRequest>,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/set", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &body, deployment_id, "set").await))
}

pub async fn set_scale(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SetScaleRequest>,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/scale", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &body, deployment_id, "scale").await))
}

pub async fn reset(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/__mockforge/time-travel/reset", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &json!({}), deployment_id, "reset").await))
}

// --- helpers --------------------------------------------------------------

/// Build the 6PN base URL for a hosted mock's main HTTP port. Fly resolves
/// `{name}.internal` to a private IPv6 reachable from the registry pod.
fn runtime_base_url(deployment: &HostedMock) -> String {
    format!("http://{}.internal:{RUNTIME_HTTP_PORT}", deployment.fly_app_name())
}

/// GET a JSON object. 2xx + valid JSON → Ok; anything else → Err.
async fn proxy_get<T: for<'de> Deserialize<'de>>(url: &str) -> reqwest::Result<T> {
    reqwest::Client::builder()
        .timeout(PROXY_TIMEOUT)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<T>()
        .await
}

/// POST a JSON body, forward the runtime's response. On 2xx returns the
/// runtime's full payload tagged `accepted: true, runtime_state: "live"`;
/// on any error returns the standard unreachable envelope. The optional
/// pre-existing `status` / `success` fields from the runtime are preserved
/// so the UI doesn't need a follow-up GET to refresh.
async fn proxy_post_json<B: Serialize>(
    url: &str,
    body: &B,
    deployment_id: Uuid,
    op: &'static str,
) -> Value {
    let client = match reqwest::Client::builder().timeout(PROXY_TIMEOUT).build() {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "reqwest client build failed");
            return unreachable_post_body(err.to_string());
        }
    };

    let resp = match client.post(url).json(body).send().await {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "time-travel POST failed");
            return unreachable_post_body(err.to_string());
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::warn!(%deployment_id, op, %status, body = %text, "time-travel POST non-2xx");
        return unreachable_post_body(format!("upstream {status}"));
    }

    // Best-effort: parse upstream JSON to merge into our envelope. If parse
    // fails (e.g. empty body) we still report success — the proxy POST got
    // a 2xx so the runtime accepted the mutation.
    let upstream: Value = resp.json().await.unwrap_or(Value::Null);

    let mut body = json!({
        "accepted": true,
        "runtime_state": "live",
    });
    if let Value::Object(ref mut map) = body {
        if !upstream.is_null() {
            map.insert("upstream".into(), upstream);
        }
    }
    body
}

fn unreachable_post_body(reason: String) -> Value {
    json!({
        "accepted": false,
        "runtime_state": "unreachable",
        "reason": reason,
    })
}

/// Resolve `deployment_id` to a `HostedMock`, after confirming the caller's
/// org matches. Mirrors `resilience::authorize_deployment`.
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
    fn envelope_live_round_trips_status() {
        let env = TimeTravelEnvelope::live(TimeTravelStatusResponse {
            enabled: true,
            current_time: Some("2030-01-01T00:00:00Z".into()),
            scale_factor: 60.0,
            real_time: "2026-05-16T05:00:00Z".into(),
        });
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "live");
        assert_eq!(body["data"]["enabled"], true);
        assert_eq!(body["data"]["current_time"], "2030-01-01T00:00:00Z");
        assert_eq!(body["data"]["scale_factor"], 60.0);
    }

    #[test]
    fn envelope_unreachable_is_disabled_placeholder() {
        let env = TimeTravelEnvelope::unreachable(TimeTravelStatusResponse::disabled_placeholder());
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "unreachable");
        assert_eq!(body["data"]["enabled"], false);
        assert_eq!(body["data"]["scale_factor"], 1.0);
        // current_time is omitted when None — UI treats absence as "no virtual time".
        assert!(
            body["data"]["current_time"].is_null()
                || !body["data"].as_object().unwrap().contains_key("current_time")
        );
    }

    #[test]
    fn unreachable_post_body_shape() {
        let body = unreachable_post_body("connection refused".into());
        assert_eq!(body["accepted"], false);
        assert_eq!(body["runtime_state"], "unreachable");
        assert_eq!(body["reason"], "connection refused");
    }

    #[test]
    fn enable_request_serializes_minimal() {
        let body = serde_json::to_value(EnableRequest {
            time: None,
            scale: None,
        })
        .unwrap();
        // Both fields skip_serializing_if = None — empty object passes through.
        assert_eq!(body, serde_json::json!({}));
    }

    #[test]
    fn enable_request_serializes_partial() {
        let body = serde_json::to_value(EnableRequest {
            time: None,
            scale: Some(60.0),
        })
        .unwrap();
        assert_eq!(body["scale"], 60.0);
        assert!(body.get("time").is_none());
    }
}
