//! Axum middleware that wires every HTTP request through the
//! `CircuitBreakerManager` + `BulkheadManager` state objects.
//!
//! Without this layer, the resilience managers are inert state holders —
//! the `/api/resilience/*` dashboard returns empty data because nothing
//! ever calls `allow_request` / `try_acquire` / `record_success`. This is
//! Phase 2 of #468; Phase 1 shipped the cloud scaffold so the UI had a
//! place to talk to.
//!
//! ## Behaviour
//!
//! * **Circuit breaker** keyed by `"{METHOD} {path}"` so every endpoint
//!   gets its own breaker. An open breaker short-circuits with 503 and
//!   `Retry-After: 1`.
//! * **Bulkhead** keyed by a configurable `service` string (defaults to
//!   `"http"`) so there's a single global bucket; per-route bulkheads can
//!   be added later if anyone needs them.
//! * **Outcome attribution**: only 5xx counts as a circuit-breaker
//!   failure. 4xx — even 429 from rate limits — represents a client
//!   problem, not a backend problem, so it does not trip the breaker.
//!   This matches the convention `record_request` in `resilience.rs` uses
//!   for SLO bookkeeping elsewhere.
//! * **Cost when disabled**: `CircuitBreaker.allow_request` and
//!   `Bulkhead.try_acquire` both short-circuit to "allow" when their
//!   config has `enabled: false`, so installing the layer with the
//!   default (disabled) config is essentially free per-request.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::resilience::{BulkheadError, BulkheadManager, CircuitBreakerManager};

/// Shared state for the resilience middleware. The same `Arc<...Manager>`
/// instances must be passed to `resilience_api::ResilienceApiState` so the
/// `/api/resilience/*` dashboard reads what the middleware writes.
#[derive(Clone)]
pub struct ResilienceMiddlewareState {
    pub circuit_manager: Arc<CircuitBreakerManager>,
    pub bulkhead_manager: Arc<BulkheadManager>,
    /// Bulkhead key for HTTP traffic. Per-route bulkheads are future
    /// work — a single global bucket gives the dashboard something
    /// concrete to show without exploding the breaker/bulkhead count.
    pub default_service: String,
}

impl ResilienceMiddlewareState {
    pub fn new(
        circuit_manager: Arc<CircuitBreakerManager>,
        bulkhead_manager: Arc<BulkheadManager>,
    ) -> Self {
        Self {
            circuit_manager,
            bulkhead_manager,
            default_service: "http".to_string(),
        }
    }
}

/// Build the middleware state + matching dashboard state from explicit
/// `CircuitBreakerConfig` / `BulkheadConfig` values. Both states share the
/// same `Arc<...Manager>` instances backed by a fresh Prometheus registry,
/// so `/api/resilience/*` reflects what the middleware records.
///
/// `None` for either config falls back to that type's `Default` (which has
/// `enabled: false`), preserving the same behaviour `default_resilience_state`
/// shipped with in #468 Phase 2.
pub fn resilience_state_from_configs(
    circuit_config: Option<crate::config::CircuitBreakerConfig>,
    bulkhead_config: Option<crate::config::BulkheadConfig>,
) -> (ResilienceMiddlewareState, crate::resilience_api::ResilienceApiState) {
    use prometheus::Registry;

    let registry = Arc::new(Registry::new());
    let circuit =
        Arc::new(CircuitBreakerManager::new(circuit_config.unwrap_or_default(), registry.clone()));
    let bulkhead = Arc::new(BulkheadManager::new(bulkhead_config.unwrap_or_default(), registry));
    let mw_state = ResilienceMiddlewareState::new(circuit.clone(), bulkhead.clone());
    let api_state = crate::resilience_api::ResilienceApiState {
        circuit_breaker_manager: circuit,
        bulkhead_manager: bulkhead,
    };
    (mw_state, api_state)
}

/// Build the middleware state + matching dashboard state with default
/// (disabled) configs. Thin wrapper around [`resilience_state_from_configs`]
/// for callers that don't have CLI/YAML overrides to plumb through.
pub fn default_resilience_state(
) -> (ResilienceMiddlewareState, crate::resilience_api::ResilienceApiState) {
    resilience_state_from_configs(None, None)
}

/// Axum middleware: gate request through circuit breaker + bulkhead.
pub async fn resilience_middleware(
    State(state): State<ResilienceMiddlewareState>,
    request: Request,
    next: Next,
) -> Response {
    let endpoint = format!("{} {}", request.method().as_str(), request.uri().path());

    // 1. Circuit-breaker gate. `allow_request` returns true when disabled
    //    *or* when the breaker is closed/half-open with budget.
    let breaker = state.circuit_manager.get_breaker(&endpoint).await;
    if !breaker.allow_request().await {
        return circuit_open_response(&endpoint);
    }

    // 2. Bulkhead admit. The guard auto-releases on drop, which we hold
    //    across `next.run(request)` so the slot stays occupied for the
    //    full handler duration.
    let bulkhead = state.bulkhead_manager.get_bulkhead(&state.default_service).await;
    let _guard = match bulkhead.try_acquire().await {
        Ok(g) => g,
        Err(BulkheadError::Rejected) => {
            // Don't record on the breaker — bulkhead saturation is a
            // separate failure mode and counting it as a breaker failure
            // would punish a healthy endpoint for being popular.
            return bulkhead_response(
                &state.default_service,
                "bulkhead_rejected",
                "Bulkhead is full; request rejected.",
            );
        }
        Err(BulkheadError::Timeout) => {
            return bulkhead_response(
                &state.default_service,
                "bulkhead_timeout",
                "Bulkhead queue timeout; request not admitted.",
            );
        }
    };

    // 3. Run handler.
    let response = next.run(request).await;

    // 4. Record outcome. 5xx = backend problem → breaker failure. 4xx (and
    //    the 503 we just generated) belongs to the client / upstream
    //    saturation and stays out of the per-endpoint failure budget.
    if response.status().is_server_error() {
        breaker.record_failure().await;
    } else {
        breaker.record_success().await;
    }

    response
}

fn circuit_open_response(endpoint: &str) -> Response {
    let body = Json(json!({
        "error": "circuit_open",
        "endpoint": endpoint,
        "message": "Circuit breaker is open for this endpoint; refusing the request.",
    }));
    let mut resp = (StatusCode::SERVICE_UNAVAILABLE, body).into_response();
    resp.headers_mut().insert("retry-after", HeaderValue::from_static("1"));
    resp
}

fn bulkhead_response(service: &str, error: &'static str, message: &'static str) -> Response {
    let body = Json(json!({
        "error": error,
        "service": service,
        "message": message,
    }));
    let mut resp = (StatusCode::SERVICE_UNAVAILABLE, body).into_response();
    resp.headers_mut().insert("retry-after", HeaderValue::from_static("1"));
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BulkheadConfig, CircuitBreakerConfig};
    use axum::{body::Body, http::Request, middleware, routing::get, Router};
    use prometheus::Registry;
    use tower::ServiceExt;

    /// Build a tiny app with the middleware mounted in front of a handler
    /// that returns whatever status we tell it to.
    async fn app(state: ResilienceMiddlewareState) -> Router {
        Router::new()
            .route("/ok", get(|| async { (StatusCode::OK, "ok").into_response() }))
            .route(
                "/boom",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response() }),
            )
            .route(
                "/client-error",
                get(|| async { (StatusCode::BAD_REQUEST, "nope").into_response() }),
            )
            .layer(middleware::from_fn_with_state(state, resilience_middleware))
    }

    fn state() -> ResilienceMiddlewareState {
        let registry = Arc::new(Registry::new());
        let circuit = Arc::new(CircuitBreakerManager::new(
            CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 2,
                success_threshold: 1,
                timeout_ms: 60_000,
                half_open_max_requests: 1,
                failure_rate_threshold: 50.0,
                min_requests_for_rate: 100, // high so the test only trips via consecutive count
                rolling_window_ms: 10_000,
            },
            registry.clone(),
        ));
        let bulkhead = Arc::new(BulkheadManager::new(
            BulkheadConfig {
                enabled: false, // off for these tests; bulkhead has its own coverage below
                max_concurrent_requests: 4,
                max_queue_size: 0,
                queue_timeout_ms: 1000,
            },
            registry,
        ));
        ResilienceMiddlewareState::new(circuit, bulkhead)
    }

    async fn call(app: &Router, path: &str) -> StatusCode {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        res.status()
    }

    #[tokio::test]
    async fn success_response_is_passed_through() {
        let s = state();
        let app = app(s.clone()).await;
        assert_eq!(call(&app, "/ok").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn consecutive_5xx_opens_breaker_and_returns_503() {
        let s = state();
        let app = app(s.clone()).await;
        // failure_threshold is 2, so two consecutive 5xx flips the circuit.
        assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        // Now the breaker is OPEN — next request is 503 from the middleware.
        assert_eq!(call(&app, "/boom").await, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn client_4xx_does_not_trip_breaker() {
        let s = state();
        let app = app(s.clone()).await;
        // Same threshold, but all 400s — should never open.
        for _ in 0..5 {
            assert_eq!(call(&app, "/client-error").await, StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn breaker_is_per_endpoint() {
        let s = state();
        let app = app(s.clone()).await;
        // Trip the breaker for /boom.
        assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(call(&app, "/boom").await, StatusCode::SERVICE_UNAVAILABLE);
        // /ok must still be reachable — separate breaker.
        assert_eq!(call(&app, "/ok").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn bulkhead_rejected_returns_503() {
        let registry = Arc::new(Registry::new());
        // Bulkhead with 0 slots, no queue: every request is rejected.
        let bulkhead = Arc::new(BulkheadManager::new(
            BulkheadConfig {
                enabled: true,
                max_concurrent_requests: 0,
                max_queue_size: 0,
                queue_timeout_ms: 100,
            },
            registry.clone(),
        ));
        let circuit = Arc::new(CircuitBreakerManager::new(
            CircuitBreakerConfig::default(), // disabled
            registry,
        ));
        let s = ResilienceMiddlewareState::new(circuit, bulkhead);
        let app = app(s).await;
        assert_eq!(call(&app, "/ok").await, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn from_configs_threshold_drives_breaker_trip() {
        // Hand the builder a 1-failure-threshold breaker and confirm it
        // actually trips after one 5xx — proves the config passed through
        // rather than being ignored / replaced with defaults.
        let cb = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 1,
            success_threshold: 1,
            timeout_ms: 60_000,
            half_open_max_requests: 1,
            failure_rate_threshold: 100.0,
            min_requests_for_rate: 100,
            rolling_window_ms: 10_000,
        };
        let (mw, _api) = resilience_state_from_configs(Some(cb), None);
        let app = app(mw).await;
        assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        // Threshold=1 means the next request hits an open breaker.
        assert_eq!(call(&app, "/boom").await, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn from_configs_bulkhead_capacity_rejects() {
        // 0-slot bulkhead via the new builder rejects unconditionally —
        // proves the BulkheadConfig also threads through correctly.
        let bh = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 0,
            max_queue_size: 0,
            queue_timeout_ms: 100,
        };
        let (mw, _api) = resilience_state_from_configs(None, Some(bh));
        let app = app(mw).await;
        assert_eq!(call(&app, "/ok").await, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn from_configs_with_none_matches_default_state() {
        // Both None should be equivalent to `default_resilience_state` —
        // no surprises in the boundary case.
        let (mw, _api) = resilience_state_from_configs(None, None);
        let app = app(mw).await;
        // Disabled managers must let everything through unchanged.
        assert_eq!(call(&app, "/ok").await, StatusCode::OK);
        for _ in 0..5 {
            assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    #[tokio::test]
    async fn disabled_managers_pass_through_unchanged() {
        let registry = Arc::new(Registry::new());
        let circuit =
            Arc::new(CircuitBreakerManager::new(CircuitBreakerConfig::default(), registry.clone()));
        let bulkhead = Arc::new(BulkheadManager::new(BulkheadConfig::default(), registry));
        let s = ResilienceMiddlewareState::new(circuit, bulkhead);
        let app = app(s).await;
        // Even 5xx responses pass through with their original status when
        // the breaker is disabled — middleware is effectively transparent.
        assert_eq!(call(&app, "/ok").await, StatusCode::OK);
        for _ in 0..10 {
            assert_eq!(call(&app, "/boom").await, StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}
