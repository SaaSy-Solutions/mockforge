//! Cloud world-state handlers (#464) — Phase 1 runtime proxy.
//!
//! Live world-state snapshot + graph + layers + query for a hosted mock.
//! Each deployment runs `mockforge serve`, which mounts the world-state
//! router at `/api/world-state` on the **main HTTP port (3000)** — same
//! reachability story as time-travel (#466): the admin port isn't always
//! exposed publicly on hosted mocks, so we target the main HTTP port over
//! Fly 6PN.
//!
//! ## Phase 1 scope
//!
//! The runtime exposes 6 endpoints. We proxy the 5 HTTP ones in this PR.
//! `GET /stream` is a WebSocket (full duplex bidirectional protocol over
//! HTTP/1.1 Upgrade) — proxying that through the registry over 6PN
//! requires a ws-aware tunnel and is intentionally deferred to Phase 2.
//! The local UI polls every 5s as its default refresh anyway (see
//! `useWorldStateLayers` and friends), so the cloud UI is fully functional
//! with polling-only in Phase 1.
//!
//! ## Wire format
//!
//! Every GET returns `{ runtime_state, data }`:
//! * `"live"` — proxy succeeded; `data` is the deployment's response.
//! * `"unreachable"` — proxy failed (connection refused, timeout, non-2xx);
//!   `data` is `null` so the UI can render an explicit empty state.
//!
//! Same `runtime_state` discriminator as [`crate::handlers::resilience`]
//! and [`crate::handlers::time_travel`].
//!
//! ## Routes
//!
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/world-state/snapshot`
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/world-state/snapshot/{snapshot_id}`
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/world-state/graph?layers=...`
//! * `GET  /api/v1/hosted-mocks/{deployment_id}/world-state/layers`
//! * `POST /api/v1/hosted-mocks/{deployment_id}/world-state/query`

use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    models::HostedMock,
    AppState,
};

/// Main HTTP port the hosted mock listens on (PORT=3000 injected by the
/// orchestrator on every Fly deploy).
const RUNTIME_HTTP_PORT: u16 = 3000;

/// Reqwest timeout for one proxy call. World-state graph queries can be
/// larger than time-travel status, so this is a touch more generous than
/// the 3s elsewhere — but still fail-fast: a slow snapshot lookup is the
/// UI's polling problem, not a request-blocking one.
const PROXY_TIMEOUT: Duration = Duration::from_secs(5);

// --- Envelope -------------------------------------------------------------

/// Envelope discriminating live proxy vs unreachable upstream.
///
/// Unlike resilience (`data: Vec<T>`) and time-travel (`data: T` with a
/// synthesized placeholder), the world-state shapes are heterogeneous
/// (snapshot vs graph vs layers vs query result), so we keep `data` as
/// `serde_json::Value` and let the UI deserialize per-route.
#[derive(Debug, Serialize)]
pub struct WorldStateEnvelope {
    pub runtime_state: &'static str,
    pub data: Value,
}

impl WorldStateEnvelope {
    fn live(data: Value) -> Self {
        Self {
            runtime_state: "live",
            data,
        }
    }

    fn unreachable() -> Self {
        Self {
            runtime_state: "unreachable",
            data: Value::Null,
        }
    }
}

// --- Query/body shapes ----------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GraphQueryParams {
    /// Optional comma-separated list of layer ids to include. Forwarded
    /// verbatim to the runtime — the runtime owns the filter semantics.
    pub layers: Option<String>,
}

/// We accept arbitrary JSON for `/query` (the local handler uses
/// `WorldStateQueryRequest` with optional `node_type`, `layer`, `since`
/// fields, but we don't want to leak the runtime's struct into the
/// registry — the runtime is the source of truth). Forward verbatim.
type QueryRequestBody = Value;

// --- GET handlers ---------------------------------------------------------

pub async fn get_snapshot(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<WorldStateEnvelope>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/api/world-state/snapshot", runtime_base_url(&deployment));
    Ok(Json(proxy_get(&url, deployment_id, "snapshot").await))
}

pub async fn get_snapshot_by_id(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((deployment_id, snapshot_id)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> ApiResult<Json<WorldStateEnvelope>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!(
        "{}/api/world-state/snapshot/{}",
        runtime_base_url(&deployment),
        urlencoding::encode(&snapshot_id),
    );
    Ok(Json(proxy_get(&url, deployment_id, "snapshot_by_id").await))
}

pub async fn get_graph(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    Query(params): Query<GraphQueryParams>,
    headers: HeaderMap,
) -> ApiResult<Json<WorldStateEnvelope>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = match params.layers {
        Some(layers) if !layers.is_empty() => format!(
            "{}/api/world-state/graph?layers={}",
            runtime_base_url(&deployment),
            urlencoding::encode(&layers),
        ),
        _ => format!("{}/api/world-state/graph", runtime_base_url(&deployment)),
    };
    Ok(Json(proxy_get(&url, deployment_id, "graph").await))
}

pub async fn get_layers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<WorldStateEnvelope>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/api/world-state/layers", runtime_base_url(&deployment));
    Ok(Json(proxy_get(&url, deployment_id, "layers").await))
}

// --- POST handlers --------------------------------------------------------

pub async fn query(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(deployment_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<QueryRequestBody>,
) -> ApiResult<Json<WorldStateEnvelope>> {
    let deployment = authorize_deployment(&state, user_id, &headers, deployment_id).await?;
    let url = format!("{}/api/world-state/query", runtime_base_url(&deployment));
    Ok(Json(proxy_post_json(&url, &body, deployment_id, "query").await))
}

// --- helpers --------------------------------------------------------------

fn runtime_base_url(deployment: &HostedMock) -> String {
    format!("http://{}.internal:{RUNTIME_HTTP_PORT}", deployment.fly_app_name())
}

/// GET a JSON value, wrap in envelope. 2xx + valid JSON → live; anything
/// else → unreachable + warning log.
async fn proxy_get(url: &str, deployment_id: Uuid, op: &'static str) -> WorldStateEnvelope {
    let client = match reqwest::Client::builder().timeout(PROXY_TIMEOUT).build() {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "reqwest client build failed");
            return WorldStateEnvelope::unreachable();
        }
    };

    match client.get(url).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(body) => WorldStateEnvelope::live(body),
                Err(err) => {
                    tracing::warn!(%deployment_id, op, error = %err, "world-state proxy GET JSON parse failed");
                    WorldStateEnvelope::unreachable()
                }
            },
            Err(err) => {
                tracing::warn!(%deployment_id, op, error = %err, "world-state proxy GET non-2xx");
                WorldStateEnvelope::unreachable()
            }
        },
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "world-state proxy GET failed");
            WorldStateEnvelope::unreachable()
        }
    }
}

/// POST a JSON body, wrap response in envelope. Same error semantics as
/// `proxy_get`.
async fn proxy_post_json<B: Serialize>(
    url: &str,
    body: &B,
    deployment_id: Uuid,
    op: &'static str,
) -> WorldStateEnvelope {
    let client = match reqwest::Client::builder().timeout(PROXY_TIMEOUT).build() {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "reqwest client build failed");
            return WorldStateEnvelope::unreachable();
        }
    };

    match client.post(url).json(body).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(resp) => match resp.json::<Value>().await {
                Ok(body) => WorldStateEnvelope::live(body),
                Err(err) => {
                    tracing::warn!(%deployment_id, op, error = %err, "world-state proxy POST JSON parse failed");
                    WorldStateEnvelope::unreachable()
                }
            },
            Err(err) => {
                tracing::warn!(%deployment_id, op, error = %err, "world-state proxy POST non-2xx");
                WorldStateEnvelope::unreachable()
            }
        },
        Err(err) => {
            tracing::warn!(%deployment_id, op, error = %err, "world-state proxy POST failed");
            WorldStateEnvelope::unreachable()
        }
    }
}

/// Resolve `deployment_id` to a `HostedMock`, after confirming the caller's
/// org matches. Mirrors `resilience::authorize_deployment` and
/// `time_travel::authorize_deployment`.
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
    use serde_json::json;

    #[test]
    fn envelope_live_round_trips_arbitrary_value() {
        let env = WorldStateEnvelope::live(json!({
            "snapshot_id": "snap-1",
            "nodes": [],
            "edges": [],
        }));
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "live");
        assert_eq!(body["data"]["snapshot_id"], "snap-1");
        assert!(body["data"]["nodes"].is_array());
    }

    #[test]
    fn envelope_unreachable_is_null_data() {
        let env = WorldStateEnvelope::unreachable();
        let body = serde_json::to_value(&env).unwrap();
        assert_eq!(body["runtime_state"], "unreachable");
        assert!(body["data"].is_null());
    }

    #[test]
    fn graph_query_params_default_has_no_layers() {
        // Constructing without `layers` mirrors what axum's `Query` extractor
        // produces when the querystring is absent. We can't round-trip via
        // serde_urlencoded here because it isn't a direct dep of this crate;
        // verify the struct shape via JSON instead (Deserialize derives the
        // same field-optional behaviour serde_urlencoded would).
        let params: GraphQueryParams = serde_json::from_str("{}").unwrap();
        assert!(params.layers.is_none());
    }

    #[test]
    fn graph_query_params_carries_comma_list() {
        let params: GraphQueryParams =
            serde_json::from_str(r#"{"layers":"accounts,inventory"}"#).unwrap();
        assert_eq!(params.layers.as_deref(), Some("accounts,inventory"));
    }
}
