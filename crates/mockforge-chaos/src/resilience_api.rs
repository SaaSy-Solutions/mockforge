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
        .route("/circuit-breakers/{endpoint}", get(get_circuit_breaker))
        .route("/circuit-breakers/{endpoint}/reset", post(reset_circuit_breaker))
        .route("/bulkheads", get(get_all_bulkheads))
        .route("/bulkheads/{service}", get(get_bulkhead))
        .route("/bulkheads/{service}/reset", post(reset_bulkhead))
        .route("/dashboard/summary", get(get_dashboard_summary))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BulkheadConfig, CircuitBreakerConfig};
    use crate::resilience::CircuitState;
    use prometheus::Registry;
    use std::time::Instant;

    fn create_test_circuit_stats() -> CircuitStats {
        CircuitStats {
            total_requests: 100,
            successful_requests: 80,
            failed_requests: 20,
            rejected_requests: 5,
            state: CircuitState::Closed,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 2,
            consecutive_successes: 3,
        }
    }

    fn create_test_bulkhead_stats() -> BulkheadStats {
        BulkheadStats {
            active_requests: 10,
            queued_requests: 5,
            total_requests: 100,
            rejected_requests: 10,
            timeout_requests: 5,
        }
    }

    #[test]
    fn test_circuit_stats_response_conversion() {
        let stats = create_test_circuit_stats();
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.total_requests, 100);
        assert_eq!(response.successful_requests, 80);
        assert_eq!(response.failed_requests, 20);
        assert_eq!(response.rejected_requests, 5);
        assert_eq!(response.consecutive_failures, 2);
        assert_eq!(response.consecutive_successes, 3);
        assert_eq!(response.success_rate, 80.0);
        assert_eq!(response.failure_rate, 20.0);
    }

    #[test]
    fn test_circuit_stats_response_zero_requests() {
        let stats = CircuitStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            rejected_requests: 0,
            state: CircuitState::Closed,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 0,
            consecutive_successes: 0,
        };
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.success_rate, 0.0);
        assert_eq!(response.failure_rate, 0.0);
    }

    #[test]
    fn test_circuit_stats_response_all_successes() {
        let stats = CircuitStats {
            total_requests: 100,
            successful_requests: 100,
            failed_requests: 0,
            rejected_requests: 0,
            state: CircuitState::Closed,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 0,
            consecutive_successes: 10,
        };
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.success_rate, 100.0);
        assert_eq!(response.failure_rate, 0.0);
    }

    #[test]
    fn test_circuit_stats_response_all_failures() {
        let stats = CircuitStats {
            total_requests: 50,
            successful_requests: 0,
            failed_requests: 50,
            rejected_requests: 0,
            state: CircuitState::Open,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 50,
            consecutive_successes: 0,
        };
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.success_rate, 0.0);
        assert_eq!(response.failure_rate, 100.0);
    }

    #[test]
    fn test_bulkhead_stats_response_conversion() {
        let stats = create_test_bulkhead_stats();
        let max_concurrent = 50;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        assert_eq!(response.active_requests, 10);
        assert_eq!(response.queued_requests, 5);
        assert_eq!(response.total_requests, 100);
        assert_eq!(response.rejected_requests, 10);
        assert_eq!(response.timeout_requests, 5);
        assert_eq!(response.utilization_percent, 20.0);
    }

    #[test]
    fn test_bulkhead_stats_response_zero_max() {
        let stats = create_test_bulkhead_stats();
        let max_concurrent = 0;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        assert_eq!(response.utilization_percent, 0.0);
    }

    #[test]
    fn test_bulkhead_stats_response_full_utilization() {
        let stats = BulkheadStats {
            active_requests: 50,
            queued_requests: 0,
            total_requests: 100,
            rejected_requests: 0,
            timeout_requests: 0,
        };
        let max_concurrent = 50;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        assert_eq!(response.utilization_percent, 100.0);
    }

    #[test]
    fn test_circuit_breaker_state_response_serialize() {
        let stats = create_test_circuit_stats();
        let response = CircuitBreakerStateResponse {
            endpoint: "/api/test".to_string(),
            state: "Closed".to_string(),
            stats: stats.into(),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["endpoint"], "/api/test");
        assert_eq!(json["state"], "Closed");
        assert_eq!(json["stats"]["total_requests"], 100);
    }

    #[test]
    fn test_circuit_breaker_state_response_deserialize() {
        let json = serde_json::json!({
            "endpoint": "/api/users",
            "state": "Open",
            "stats": {
                "total_requests": 50,
                "successful_requests": 20,
                "failed_requests": 30,
                "rejected_requests": 10,
                "consecutive_failures": 5,
                "consecutive_successes": 0,
                "success_rate": 40.0,
                "failure_rate": 60.0
            }
        });

        let response: CircuitBreakerStateResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.endpoint, "/api/users");
        assert_eq!(response.state, "Open");
        assert_eq!(response.stats.total_requests, 50);
        assert_eq!(response.stats.success_rate, 40.0);
    }

    #[test]
    fn test_bulkhead_state_response_serialize() {
        let stats = create_test_bulkhead_stats();
        let response = BulkheadStateResponse {
            service: "auth-service".to_string(),
            stats: BulkheadStatsResponse::from_stats(stats, 50),
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["service"], "auth-service");
        assert_eq!(json["stats"]["active_requests"], 10);
        assert_eq!(json["stats"]["utilization_percent"], 20.0);
    }

    #[test]
    fn test_bulkhead_state_response_deserialize() {
        let json = serde_json::json!({
            "service": "payment-service",
            "stats": {
                "active_requests": 15,
                "queued_requests": 3,
                "total_requests": 200,
                "rejected_requests": 5,
                "timeout_requests": 2,
                "utilization_percent": 30.0
            }
        });

        let response: BulkheadStateResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.service, "payment-service");
        assert_eq!(response.stats.active_requests, 15);
        assert_eq!(response.stats.utilization_percent, 30.0);
    }

    #[test]
    fn test_resilience_api_state_creation() {
        let cb_config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 60000,
            half_open_max_requests: 3,
            failure_rate_threshold: 50.0,
            min_requests_for_rate: 10,
            rolling_window_ms: 60000,
        };
        let registry = Arc::new(Registry::new());
        let cb_manager = Arc::new(CircuitBreakerManager::new(cb_config, registry.clone()));

        let bh_config = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 50,
            max_queue_size: 100,
            queue_timeout_ms: 5000,
        };
        let bh_manager = Arc::new(BulkheadManager::new(bh_config, registry));

        let state = ResilienceApiState {
            circuit_breaker_manager: cb_manager,
            bulkhead_manager: bh_manager,
        };

        // Verify state was created successfully - managers are accessible
        assert!(Arc::strong_count(&state.circuit_breaker_manager) >= 1);
        assert!(Arc::strong_count(&state.bulkhead_manager) >= 1);
    }

    #[test]
    fn test_resilience_api_state_clone() {
        let cb_config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 60000,
            half_open_max_requests: 3,
            failure_rate_threshold: 50.0,
            min_requests_for_rate: 10,
            rolling_window_ms: 60000,
        };
        let registry = Arc::new(Registry::new());
        let cb_manager = Arc::new(CircuitBreakerManager::new(cb_config, registry.clone()));

        let bh_config = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 50,
            max_queue_size: 100,
            queue_timeout_ms: 5000,
        };
        let bh_manager = Arc::new(BulkheadManager::new(bh_config, registry));

        let state1 = ResilienceApiState {
            circuit_breaker_manager: cb_manager,
            bulkhead_manager: bh_manager,
        };

        let state2 = state1.clone();
        assert!(Arc::ptr_eq(&state1.circuit_breaker_manager, &state2.circuit_breaker_manager));
        assert!(Arc::ptr_eq(&state1.bulkhead_manager, &state2.bulkhead_manager));
    }

    #[test]
    fn test_create_resilience_router() {
        let cb_config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 60000,
            half_open_max_requests: 3,
            failure_rate_threshold: 50.0,
            min_requests_for_rate: 10,
            rolling_window_ms: 60000,
        };
        let registry = Arc::new(Registry::new());
        let cb_manager = Arc::new(CircuitBreakerManager::new(cb_config, registry.clone()));

        let bh_config = BulkheadConfig {
            enabled: true,
            max_concurrent_requests: 50,
            max_queue_size: 100,
            queue_timeout_ms: 5000,
        };
        let bh_manager = Arc::new(BulkheadManager::new(bh_config, registry));

        let state = ResilienceApiState {
            circuit_breaker_manager: cb_manager,
            bulkhead_manager: bh_manager,
        };

        let mut router = create_resilience_router(state);
        // Router is created successfully - verify it's a valid router
        let _service = router.as_service::<axum::body::Body>();
    }

    #[test]
    fn test_circuit_stats_response_partial_success() {
        let stats = CircuitStats {
            total_requests: 75,
            successful_requests: 45,
            failed_requests: 30,
            rejected_requests: 8,
            state: CircuitState::Closed,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 1,
            consecutive_successes: 0,
        };
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.success_rate, 60.0);
        assert_eq!(response.failure_rate, 40.0);
    }

    #[test]
    fn test_bulkhead_stats_response_no_active() {
        let stats = BulkheadStats {
            active_requests: 0,
            queued_requests: 0,
            total_requests: 100,
            rejected_requests: 0,
            timeout_requests: 0,
        };
        let max_concurrent = 50;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        assert_eq!(response.utilization_percent, 0.0);
        assert_eq!(response.active_requests, 0);
    }

    #[test]
    fn test_bulkhead_stats_response_with_queue() {
        let stats = BulkheadStats {
            active_requests: 50,
            queued_requests: 25,
            total_requests: 150,
            rejected_requests: 10,
            timeout_requests: 5,
        };
        let max_concurrent = 50;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        assert_eq!(response.utilization_percent, 100.0);
        assert_eq!(response.queued_requests, 25);
    }

    #[test]
    fn test_circuit_stats_edge_case_single_request() {
        let stats = CircuitStats {
            total_requests: 1,
            successful_requests: 1,
            failed_requests: 0,
            rejected_requests: 0,
            state: CircuitState::Closed,
            last_state_change: Some(Instant::now()),
            consecutive_failures: 0,
            consecutive_successes: 1,
        };
        let response: CircuitStatsResponse = stats.into();

        assert_eq!(response.success_rate, 100.0);
        assert_eq!(response.failure_rate, 0.0);
    }

    #[test]
    fn test_bulkhead_stats_edge_case_overload() {
        let stats = BulkheadStats {
            active_requests: 100,
            queued_requests: 50,
            total_requests: 1000,
            rejected_requests: 200,
            timeout_requests: 50,
        };
        let max_concurrent = 50;
        let response = BulkheadStatsResponse::from_stats(stats, max_concurrent);

        // Utilization can exceed 100% when active_requests > max_concurrent
        assert_eq!(response.utilization_percent, 200.0);
    }

    #[test]
    fn test_circuit_breaker_state_response_formats() {
        let states = vec!["Closed", "Open", "HalfOpen"];

        for state_str in states {
            let stats = create_test_circuit_stats();
            let response = CircuitBreakerStateResponse {
                endpoint: "/test".to_string(),
                state: state_str.to_string(),
                stats: stats.into(),
            };

            let json = serde_json::to_value(&response).unwrap();
            assert_eq!(json["state"], state_str);
        }
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_circuit_breaker() {
        let stats = create_test_circuit_stats();
        let original = CircuitBreakerStateResponse {
            endpoint: "/api/roundtrip".to_string(),
            state: "Closed".to_string(),
            stats: stats.into(),
        };

        let json = serde_json::to_value(&original).unwrap();
        let deserialized: CircuitBreakerStateResponse = serde_json::from_value(json).unwrap();

        assert_eq!(original.endpoint, deserialized.endpoint);
        assert_eq!(original.state, deserialized.state);
        assert_eq!(original.stats.total_requests, deserialized.stats.total_requests);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_bulkhead() {
        let stats = create_test_bulkhead_stats();
        let original = BulkheadStateResponse {
            service: "test-service".to_string(),
            stats: BulkheadStatsResponse::from_stats(stats, 50),
        };

        let json = serde_json::to_value(&original).unwrap();
        let deserialized: BulkheadStateResponse = serde_json::from_value(json).unwrap();

        assert_eq!(original.service, deserialized.service);
        assert_eq!(original.stats.active_requests, deserialized.stats.active_requests);
    }
}
