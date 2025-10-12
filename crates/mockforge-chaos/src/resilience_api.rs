//! API endpoints for resilience management

use crate::resilience::{
    BulkheadManager, BulkheadStats, CircuitBreakerManager, CircuitState, CircuitStats,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Circuit breaker state for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStateResponse {
    pub endpoint: String,
    pub state: String,
    pub stats: CircuitStatsResponse,
}

/// Circuit breaker statistics for API response
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

impl From<CircuitStats> for CircuitStatsResponse {
    fn from(stats: CircuitStats) -> Self {
        let success_rate = if stats.total_requests > 0 {
            (stats.successful_requests as f64 / stats.total_requests as f64) * 100.0
        } else {
            0.0
        };

        let failure_rate = if stats.total_requests > 0 {
            (stats.failed_requests as f64 / stats.total_requests as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_requests: stats.total_requests,
            successful_requests: stats.successful_requests,
            failed_requests: stats.failed_requests,
            rejected_requests: stats.rejected_requests,
            consecutive_failures: stats.consecutive_failures,
            consecutive_successes: stats.consecutive_successes,
            success_rate,
            failure_rate,
        }
    }
}

/// Bulkhead state for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStateResponse {
    pub service: String,
    pub stats: BulkheadStatsResponse,
}

/// Bulkhead statistics for API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadStatsResponse {
    pub active_requests: u32,
    pub queued_requests: u32,
    pub total_requests: u64,
    pub rejected_requests: u64,
    pub timeout_requests: u64,
    pub utilization_percent: f64,
}

impl BulkheadStatsResponse {
    fn from_stats(stats: BulkheadStats, max_concurrent: u32) -> Self {
        let utilization_percent = if max_concurrent > 0 {
            (stats.active_requests as f64 / max_concurrent as f64) * 100.0
        } else {
            0.0
        };

        Self {
            active_requests: stats.active_requests,
            queued_requests: stats.queued_requests,
            total_requests: stats.total_requests,
            rejected_requests: stats.rejected_requests,
            timeout_requests: stats.timeout_requests,
            utilization_percent,
        }
    }
}

/// Shared state for resilience API
#[derive(Clone)]
pub struct ResilienceApiState {
    pub circuit_breaker_manager: Arc<CircuitBreakerManager>,
    pub bulkhead_manager: Arc<BulkheadManager>,
}

/// Get all circuit breaker states
async fn get_all_circuit_breakers(State(state): State<ResilienceApiState>) -> impl IntoResponse {
    let states = state.circuit_breaker_manager.get_all_states().await;
    let mut responses = Vec::new();

    for (endpoint, cb_state) in states {
        let breaker = state.circuit_breaker_manager.get_breaker(&endpoint).await;
        let stats = breaker.stats().await;

        responses.push(CircuitBreakerStateResponse {
            endpoint: endpoint.clone(),
            state: format!("{:?}", cb_state),
            stats: stats.into(),
        });
    }

    Json(responses)
}

/// Get circuit breaker state for specific endpoint
async fn get_circuit_breaker(
    State(state): State<ResilienceApiState>,
    Path(endpoint): Path<String>,
) -> impl IntoResponse {
    let breaker = state.circuit_breaker_manager.get_breaker(&endpoint).await;
    let cb_state = breaker.state().await;
    let stats = breaker.stats().await;

    Json(CircuitBreakerStateResponse {
        endpoint: endpoint.clone(),
        state: format!("{:?}", cb_state),
        stats: stats.into(),
    })
}

/// Reset circuit breaker for specific endpoint
async fn reset_circuit_breaker(
    State(state): State<ResilienceApiState>,
    Path(endpoint): Path<String>,
) -> impl IntoResponse {
    let breaker = state.circuit_breaker_manager.get_breaker(&endpoint).await;
    breaker.reset().await;

    (StatusCode::OK, "Circuit breaker reset")
}

/// Get all bulkhead states
async fn get_all_bulkheads(State(state): State<ResilienceApiState>) -> impl IntoResponse {
    let stats_map = state.bulkhead_manager.get_all_stats().await;
    let mut responses = Vec::new();

    for (service, stats) in stats_map {
        let bulkhead = state.bulkhead_manager.get_bulkhead(&service).await;
        let config = bulkhead.config().await;

        responses.push(BulkheadStateResponse {
            service: service.clone(),
            stats: BulkheadStatsResponse::from_stats(stats, config.max_concurrent_requests),
        });
    }

    Json(responses)
}

/// Get bulkhead state for specific service
async fn get_bulkhead(
    State(state): State<ResilienceApiState>,
    Path(service): Path<String>,
) -> impl IntoResponse {
    let bulkhead = state.bulkhead_manager.get_bulkhead(&service).await;
    let stats = bulkhead.stats().await;
    let config = bulkhead.config().await;

    Json(BulkheadStateResponse {
        service: service.clone(),
        stats: BulkheadStatsResponse::from_stats(stats, config.max_concurrent_requests),
    })
}

/// Reset bulkhead statistics for specific service
async fn reset_bulkhead(
    State(state): State<ResilienceApiState>,
    Path(service): Path<String>,
) -> impl IntoResponse {
    let bulkhead = state.bulkhead_manager.get_bulkhead(&service).await;
    bulkhead.reset().await;

    (StatusCode::OK, "Bulkhead statistics reset")
}

/// Get dashboard summary
async fn get_dashboard_summary(State(state): State<ResilienceApiState>) -> impl IntoResponse {
    let circuit_states = state.circuit_breaker_manager.get_all_states().await;
    let bulkhead_stats = state.bulkhead_manager.get_all_stats().await;

    let mut open_circuits = 0;
    let mut half_open_circuits = 0;
    let mut closed_circuits = 0;

    for cb_state in circuit_states.values() {
        match cb_state {
            CircuitState::Open => open_circuits += 1,
            CircuitState::HalfOpen => half_open_circuits += 1,
            CircuitState::Closed => closed_circuits += 1,
        }
    }

    let total_active_requests: u32 = bulkhead_stats.values().map(|s| s.active_requests).sum();
    let total_queued_requests: u32 = bulkhead_stats.values().map(|s| s.queued_requests).sum();

    let summary = serde_json::json!({
        "circuit_breakers": {
            "total": circuit_states.len(),
            "open": open_circuits,
            "half_open": half_open_circuits,
            "closed": closed_circuits,
        },
        "bulkheads": {
            "total": bulkhead_stats.len(),
            "active_requests": total_active_requests,
            "queued_requests": total_queued_requests,
        },
    });

    Json(summary)
}

/// Create resilience API router
pub fn create_resilience_router(state: ResilienceApiState) -> Router {
    Router::new()
        .route("/circuit-breakers", get(get_all_circuit_breakers))
        .route("/circuit-breakers/:endpoint", get(get_circuit_breaker))
        .route("/circuit-breakers/:endpoint/reset", post(reset_circuit_breaker))
        .route("/bulkheads", get(get_all_bulkheads))
        .route("/bulkheads/:service", get(get_bulkhead))
        .route("/bulkheads/:service/reset", post(reset_bulkhead))
        .route("/dashboard/summary", get(get_dashboard_summary))
        .with_state(state)
}
