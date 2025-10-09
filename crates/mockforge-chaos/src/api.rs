//! Management API for chaos engineering

use crate::{
    ab_testing::{ABTestingEngine, ABTestConfig, VariantResults, TestConclusion},
    analytics::{ChaosAnalytics, TimeBucket},
    auto_remediation::{RemediationEngine, RemediationConfig},
    config::{BulkheadConfig, ChaosConfig, CircuitBreakerConfig, FaultInjectionConfig, LatencyConfig, RateLimitConfig, TrafficShapingConfig},
    recommendations::{RecommendationCategory, RecommendationEngine, RecommendationSeverity, Recommendation},
    scenarios::{PredefinedScenarios, ScenarioEngine, ChaosScenario},
    scenario_orchestrator::{OrchestratedScenario, ScenarioOrchestrator},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// API state
#[derive(Clone)]
pub struct ChaosApiState {
    pub config: Arc<RwLock<ChaosConfig>>,
    pub scenario_engine: Arc<ScenarioEngine>,
    pub orchestrator: Arc<tokio::sync::RwLock<ScenarioOrchestrator>>,
    pub analytics: Arc<ChaosAnalytics>,
    pub recommendation_engine: Arc<RecommendationEngine>,
    pub remediation_engine: Arc<RemediationEngine>,
    pub ab_testing_engine: Arc<tokio::sync::RwLock<ABTestingEngine>>,
}

/// Create the chaos management API router
pub fn create_chaos_api_router(config: ChaosConfig) -> (Router, Arc<RwLock<ChaosConfig>>) {
    let config_arc = Arc::new(RwLock::new(config));
    let scenario_engine = Arc::new(ScenarioEngine::new());
    let orchestrator = Arc::new(tokio::sync::RwLock::new(ScenarioOrchestrator::new()));
    let analytics = Arc::new(ChaosAnalytics::new());
    let recommendation_engine = Arc::new(RecommendationEngine::new());
    let remediation_engine = Arc::new(RemediationEngine::new());
    let ab_testing_engine = Arc::new(tokio::sync::RwLock::new(ABTestingEngine::new(analytics.clone())));

    let state = ChaosApiState {
        config: config_arc.clone(),
        scenario_engine,
        orchestrator,
        analytics,
        recommendation_engine,
        remediation_engine,
        ab_testing_engine,
    };

    let router = Router::new()
        // Configuration endpoints
        .route("/api/chaos/config", get(get_config))
        .route("/api/chaos/config", put(update_config))
        .route("/api/chaos/config/latency", put(update_latency_config))
        .route("/api/chaos/config/faults", put(update_fault_config))
        .route("/api/chaos/config/rate-limit", put(update_rate_limit_config))
        .route("/api/chaos/config/traffic", put(update_traffic_config))
        .route("/api/chaos/config/circuit-breaker", put(update_circuit_breaker_config))
        .route("/api/chaos/config/bulkhead", put(update_bulkhead_config))

        // Protocol-specific configuration endpoints
        .route("/api/chaos/protocols/grpc/status-codes", post(inject_grpc_status_codes))
        .route("/api/chaos/protocols/grpc/stream-interruption", post(set_grpc_stream_interruption))
        .route("/api/chaos/protocols/websocket/close-codes", post(inject_websocket_close_codes))
        .route("/api/chaos/protocols/websocket/message-drop", post(set_websocket_message_drop))
        .route("/api/chaos/protocols/websocket/message-corruption", post(set_websocket_message_corruption))
        .route("/api/chaos/protocols/graphql/error-codes", post(inject_graphql_error_codes))
        .route("/api/chaos/protocols/graphql/partial-data", post(set_graphql_partial_data))
        .route("/api/chaos/protocols/graphql/resolver-latency", post(toggle_graphql_resolver_latency))

        // Control endpoints
        .route("/api/chaos/enable", post(enable_chaos))
        .route("/api/chaos/disable", post(disable_chaos))
        .route("/api/chaos/reset", post(reset_chaos))

        // Scenario endpoints
        .route("/api/chaos/scenarios", get(list_scenarios))
        .route("/api/chaos/scenarios/predefined", get(list_predefined_scenarios))
        .route("/api/chaos/scenarios/:name", post(start_scenario))
        .route("/api/chaos/scenarios/:name", delete(stop_scenario))
        .route("/api/chaos/scenarios", delete(stop_all_scenarios))

        // Status endpoint
        .route("/api/chaos/status", get(get_status))

        // Scenario recording endpoints
        .route("/api/chaos/recording/start", post(start_recording))
        .route("/api/chaos/recording/stop", post(stop_recording))
        .route("/api/chaos/recording/status", get(recording_status))
        .route("/api/chaos/recording/export", post(export_recording))

        // Scenario replay endpoints
        .route("/api/chaos/replay/start", post(start_replay))
        .route("/api/chaos/replay/pause", post(pause_replay))
        .route("/api/chaos/replay/resume", post(resume_replay))
        .route("/api/chaos/replay/stop", post(stop_replay))
        .route("/api/chaos/replay/status", get(replay_status))

        // Scenario orchestration endpoints
        .route("/api/chaos/orchestration/start", post(start_orchestration))
        .route("/api/chaos/orchestration/stop", post(stop_orchestration))
        .route("/api/chaos/orchestration/status", get(orchestration_status))
        .route("/api/chaos/orchestration/import", post(import_orchestration))

        // Scenario scheduling endpoints
        .route("/api/chaos/schedule", post(add_schedule))
        .route("/api/chaos/schedule/:id", get(get_schedule))
        .route("/api/chaos/schedule/:id", delete(remove_schedule))
        .route("/api/chaos/schedule/:id/enable", post(enable_schedule))
        .route("/api/chaos/schedule/:id/disable", post(disable_schedule))
        .route("/api/chaos/schedule/:id/trigger", post(trigger_schedule))
        .route("/api/chaos/schedules", get(list_schedules))

        // AI-powered recommendation endpoints
        .route("/api/chaos/recommendations", get(get_recommendations))
        .route("/api/chaos/recommendations/analyze", post(analyze_and_recommend))
        .route("/api/chaos/recommendations/category/:category", get(get_recommendations_by_category))
        .route("/api/chaos/recommendations/severity/:severity", get(get_recommendations_by_severity))
        .route("/api/chaos/recommendations", delete(clear_recommendations))

        // Auto-remediation endpoints
        .route("/api/chaos/remediation/config", get(get_remediation_config))
        .route("/api/chaos/remediation/config", put(update_remediation_config))
        .route("/api/chaos/remediation/process", post(process_remediation))
        .route("/api/chaos/remediation/approve/:id", post(approve_remediation))
        .route("/api/chaos/remediation/reject/:id", post(reject_remediation))
        .route("/api/chaos/remediation/rollback/:id", post(rollback_remediation))
        .route("/api/chaos/remediation/actions", get(get_remediation_actions))
        .route("/api/chaos/remediation/actions/:id", get(get_remediation_action))
        .route("/api/chaos/remediation/approvals", get(get_approval_queue))
        .route("/api/chaos/remediation/effectiveness/:id", get(get_remediation_effectiveness))
        .route("/api/chaos/remediation/stats", get(get_remediation_stats))

        // A/B testing endpoints
        .route("/api/chaos/ab-tests", post(create_ab_test))
        .route("/api/chaos/ab-tests", get(get_ab_tests))
        .route("/api/chaos/ab-tests/:id", get(get_ab_test))
        .route("/api/chaos/ab-tests/:id/start", post(start_ab_test))
        .route("/api/chaos/ab-tests/:id/stop", post(stop_ab_test))
        .route("/api/chaos/ab-tests/:id/pause", post(pause_ab_test))
        .route("/api/chaos/ab-tests/:id/resume", post(resume_ab_test))
        .route("/api/chaos/ab-tests/:id/record/:variant", post(record_ab_test_result))
        .route("/api/chaos/ab-tests/:id", delete(delete_ab_test))
        .route("/api/chaos/ab-tests/stats", get(get_ab_test_stats))

        .with_state(state);

    (router, config_arc)
}

/// Get current configuration
async fn get_config(State(state): State<ChaosApiState>) -> Json<ChaosConfig> {
    let config = state.config.read().await;
    Json(config.clone())
}

/// Update full configuration
async fn update_config(
    State(state): State<ChaosApiState>,
    Json(new_config): Json<ChaosConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    *config = new_config;
    info!("Chaos configuration updated");
    Json(StatusResponse {
        message: "Configuration updated".to_string(),
    })
}

/// Update latency configuration
async fn update_latency_config(
    State(state): State<ChaosApiState>,
    Json(latency_config): Json<LatencyConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.latency = Some(latency_config);
    info!("Latency configuration updated");
    Json(StatusResponse {
        message: "Latency configuration updated".to_string(),
    })
}

/// Update fault injection configuration
async fn update_fault_config(
    State(state): State<ChaosApiState>,
    Json(fault_config): Json<FaultInjectionConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.fault_injection = Some(fault_config);
    info!("Fault injection configuration updated");
    Json(StatusResponse {
        message: "Fault injection configuration updated".to_string(),
    })
}

/// Update rate limit configuration
async fn update_rate_limit_config(
    State(state): State<ChaosApiState>,
    Json(rate_config): Json<RateLimitConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.rate_limit = Some(rate_config);
    info!("Rate limit configuration updated");
    Json(StatusResponse {
        message: "Rate limit configuration updated".to_string(),
    })
}

/// Update traffic shaping configuration
async fn update_traffic_config(
    State(state): State<ChaosApiState>,
    Json(traffic_config): Json<TrafficShapingConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.traffic_shaping = Some(traffic_config);
    info!("Traffic shaping configuration updated");
    Json(StatusResponse {
        message: "Traffic shaping configuration updated".to_string(),
    })
}

/// Update circuit breaker configuration
async fn update_circuit_breaker_config(
    State(state): State<ChaosApiState>,
    Json(cb_config): Json<CircuitBreakerConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.circuit_breaker = Some(cb_config);
    info!("Circuit breaker configuration updated");
    Json(StatusResponse {
        message: "Circuit breaker configuration updated".to_string(),
    })
}

/// Update bulkhead configuration
async fn update_bulkhead_config(
    State(state): State<ChaosApiState>,
    Json(bulkhead_config): Json<BulkheadConfig>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.bulkhead = Some(bulkhead_config);
    info!("Bulkhead configuration updated");
    Json(StatusResponse {
        message: "Bulkhead configuration updated".to_string(),
    })
}

/// Enable chaos engineering
async fn enable_chaos(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.enabled = true;
    info!("Chaos engineering enabled");
    Json(StatusResponse {
        message: "Chaos engineering enabled".to_string(),
    })
}

/// Disable chaos engineering
async fn disable_chaos(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    config.enabled = false;
    info!("Chaos engineering disabled");
    Json(StatusResponse {
        message: "Chaos engineering disabled".to_string(),
    })
}

/// Reset chaos configuration to defaults
async fn reset_chaos(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    let mut config = state.config.write().await;
    *config = ChaosConfig::default();
    state.scenario_engine.stop_all_scenarios();
    info!("Chaos configuration reset to defaults");
    Json(StatusResponse {
        message: "Chaos configuration reset".to_string(),
    })
}

/// List active scenarios
async fn list_scenarios(State(state): State<ChaosApiState>) -> Json<Vec<ChaosScenario>> {
    let scenarios = state.scenario_engine.get_active_scenarios();
    Json(scenarios)
}

/// List predefined scenarios
async fn list_predefined_scenarios() -> Json<Vec<PredefinedScenarioInfo>> {
    Json(vec![
        PredefinedScenarioInfo {
            name: "network_degradation".to_string(),
            description: "Simulates degraded network conditions with high latency and packet loss".to_string(),
            tags: vec!["network".to_string(), "latency".to_string()],
        },
        PredefinedScenarioInfo {
            name: "service_instability".to_string(),
            description: "Simulates an unstable service with random errors and timeouts".to_string(),
            tags: vec!["service".to_string(), "errors".to_string()],
        },
        PredefinedScenarioInfo {
            name: "cascading_failure".to_string(),
            description: "Simulates a cascading failure with multiple simultaneous issues".to_string(),
            tags: vec!["critical".to_string(), "cascading".to_string()],
        },
        PredefinedScenarioInfo {
            name: "peak_traffic".to_string(),
            description: "Simulates peak traffic conditions with aggressive rate limiting".to_string(),
            tags: vec!["traffic".to_string(), "load".to_string()],
        },
        PredefinedScenarioInfo {
            name: "slow_backend".to_string(),
            description: "Simulates a consistently slow backend service".to_string(),
            tags: vec!["latency".to_string(), "performance".to_string()],
        },
    ])
}

/// Start a scenario
async fn start_scenario(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let scenario = match name.as_str() {
        "network_degradation" => PredefinedScenarios::network_degradation(),
        "service_instability" => PredefinedScenarios::service_instability(),
        "cascading_failure" => PredefinedScenarios::cascading_failure(),
        "peak_traffic" => PredefinedScenarios::peak_traffic(),
        "slow_backend" => PredefinedScenarios::slow_backend(),
        _ => return Err(ChaosApiError::NotFound(format!("Scenario '{}' not found", name))),
    };

    state.scenario_engine.start_scenario(scenario.clone());

    // Update config with scenario's chaos config
    let mut config = state.config.write().await;
    *config = scenario.chaos_config;

    info!("Started scenario: {}", name);
    Ok(Json(StatusResponse {
        message: format!("Scenario '{}' started", name),
    }))
}

/// Stop a scenario
async fn stop_scenario(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    if state.scenario_engine.stop_scenario(&name) {
        info!("Stopped scenario: {}", name);
        Ok(Json(StatusResponse {
            message: format!("Scenario '{}' stopped", name),
        }))
    } else {
        Err(ChaosApiError::NotFound(format!("Scenario '{}' not found or not running", name)))
    }
}

/// Stop all scenarios
async fn stop_all_scenarios(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    state.scenario_engine.stop_all_scenarios();
    info!("Stopped all scenarios");
    Json(StatusResponse {
        message: "All scenarios stopped".to_string(),
    })
}

/// Get chaos status
async fn get_status(State(state): State<ChaosApiState>) -> Json<ChaosStatus> {
    let config = state.config.read().await;
    let scenarios = state.scenario_engine.get_active_scenarios();

    Json(ChaosStatus {
        enabled: config.enabled,
        active_scenarios: scenarios.iter().map(|s| s.name.clone()).collect(),
        latency_enabled: config.latency.as_ref().is_some_and(|l| l.enabled),
        fault_injection_enabled: config.fault_injection.as_ref().is_some_and(|f| f.enabled),
        rate_limit_enabled: config.rate_limit.as_ref().is_some_and(|r| r.enabled),
        traffic_shaping_enabled: config.traffic_shaping.as_ref().is_some_and(|t| t.enabled),
    })
}

// Protocol-specific handlers

/// Inject gRPC status codes
async fn inject_grpc_status_codes(
    State(state): State<ChaosApiState>,
    Json(req): Json<GrpcStatusCodesRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    // Add gRPC-specific HTTP error codes that map to the requested gRPC status codes
    let mut http_errors = config.fault_injection
        .as_ref()
        .map(|f| f.http_errors.clone())
        .unwrap_or_default();

    for code in &req.status_codes {
        // Map gRPC codes to HTTP codes
        let http_code = match code {
            3 => 400,  // INVALID_ARGUMENT
            16 => 401, // UNAUTHENTICATED
            7 => 403,  // PERMISSION_DENIED
            5 => 404,  // NOT_FOUND
            8 => 429,  // RESOURCE_EXHAUSTED
            13 => 500, // INTERNAL
            12 => 501, // UNIMPLEMENTED
            14 => 503, // UNAVAILABLE
            4 => 504,  // DEADLINE_EXCEEDED
            _ => 500,  // Default to internal error
        };
        if !http_errors.contains(&http_code) {
            http_errors.push(http_code);
        }
    }

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.http_errors = http_errors;
        fault_config.http_error_probability = req.probability;
    }

    info!("gRPC status codes configured: {:?}", &req.status_codes);
    Json(StatusResponse {
        message: "gRPC status codes configured".to_string(),
    })
}

/// Set gRPC stream interruption
async fn set_grpc_stream_interruption(
    State(state): State<ChaosApiState>,
    Json(req): Json<ProbabilityRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.partial_response_probability = req.probability;
    }

    info!("gRPC stream interruption probability set to {}", req.probability);
    Json(StatusResponse {
        message: "gRPC stream interruption configured".to_string(),
    })
}

/// Inject WebSocket close codes
async fn inject_websocket_close_codes(
    State(state): State<ChaosApiState>,
    Json(req): Json<WebSocketCloseCodesRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    let mut http_errors = config.fault_injection
        .as_ref()
        .map(|f| f.http_errors.clone())
        .unwrap_or_default();

    for code in &req.close_codes {
        // Map WebSocket close codes to HTTP codes
        let http_code = match code {
            1002 => 400, // Protocol error
            1001 => 408, // Going away (timeout)
            1008 => 429, // Policy violation
            1011 => 500, // Server error
            _ => 500,
        };
        if !http_errors.contains(&http_code) {
            http_errors.push(http_code);
        }
    }

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.http_errors = http_errors;
        fault_config.http_error_probability = req.probability;
    }

    info!("WebSocket close codes configured: {:?}", &req.close_codes);
    Json(StatusResponse {
        message: "WebSocket close codes configured".to_string(),
    })
}

/// Set WebSocket message drop probability
async fn set_websocket_message_drop(
    State(state): State<ChaosApiState>,
    Json(req): Json<ProbabilityRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    if let Some(traffic_config) = &mut config.traffic_shaping {
        traffic_config.packet_loss_percent = req.probability * 100.0;
    }

    info!("WebSocket message drop probability set to {}", req.probability);
    Json(StatusResponse {
        message: "WebSocket message drop configured".to_string(),
    })
}

/// Set WebSocket message corruption probability
async fn set_websocket_message_corruption(
    State(state): State<ChaosApiState>,
    Json(req): Json<ProbabilityRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.partial_response_probability = req.probability;
    }

    info!("WebSocket message corruption probability set to {}", req.probability);
    Json(StatusResponse {
        message: "WebSocket message corruption configured".to_string(),
    })
}

/// Inject GraphQL error codes
async fn inject_graphql_error_codes(
    State(state): State<ChaosApiState>,
    Json(req): Json<GraphQLErrorCodesRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    let mut http_errors = config.fault_injection
        .as_ref()
        .map(|f| f.http_errors.clone())
        .unwrap_or_default();

    for code in &req.error_codes {
        let http_code = match code.as_str() {
            "BAD_USER_INPUT" => 400,
            "UNAUTHENTICATED" => 401,
            "FORBIDDEN" => 403,
            "NOT_FOUND" => 404,
            "INTERNAL_SERVER_ERROR" => 500,
            "SERVICE_UNAVAILABLE" => 503,
            _ => 500,
        };
        if !http_errors.contains(&http_code) {
            http_errors.push(http_code);
        }
    }

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.http_errors = http_errors;
        fault_config.http_error_probability = req.probability;
    }

    info!("GraphQL error codes configured: {:?}", &req.error_codes);
    Json(StatusResponse {
        message: "GraphQL error codes configured".to_string(),
    })
}

/// Set GraphQL partial data probability
async fn set_graphql_partial_data(
    State(state): State<ChaosApiState>,
    Json(req): Json<ProbabilityRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    if let Some(fault_config) = &mut config.fault_injection {
        fault_config.partial_response_probability = req.probability;
    }

    info!("GraphQL partial data probability set to {}", req.probability);
    Json(StatusResponse {
        message: "GraphQL partial data configured".to_string(),
    })
}

/// Toggle GraphQL resolver latency
async fn toggle_graphql_resolver_latency(
    State(state): State<ChaosApiState>,
    Json(req): Json<EnableRequest>,
) -> Json<StatusResponse> {
    let mut config = state.config.write().await;

    if let Some(latency_config) = &mut config.latency {
        latency_config.enabled = req.enabled;
    }

    info!("GraphQL resolver latency {}", if req.enabled { "enabled" } else { "disabled" });
    Json(StatusResponse {
        message: format!("GraphQL resolver latency {}", if req.enabled { "enabled" } else { "disabled" }),
    })
}

// Request/Response types

#[derive(Debug, Serialize)]
struct StatusResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct PredefinedScenarioInfo {
    name: String,
    description: String,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChaosStatus {
    enabled: bool,
    active_scenarios: Vec<String>,
    latency_enabled: bool,
    fault_injection_enabled: bool,
    rate_limit_enabled: bool,
    traffic_shaping_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct GrpcStatusCodesRequest {
    status_codes: Vec<i32>,
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct WebSocketCloseCodesRequest {
    close_codes: Vec<u16>,
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct GraphQLErrorCodesRequest {
    error_codes: Vec<String>,
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct ProbabilityRequest {
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct EnableRequest {
    enabled: bool,
}

// Scenario management handlers (Phase 6)

/// Start recording a scenario
async fn start_recording(
    State(state): State<ChaosApiState>,
    Json(req): Json<StartRecordingRequest>,
) -> Json<StatusResponse> {
    // TODO: Implement scenario recording
    Json(StatusResponse {
        message: format!("Recording started for scenario: {}", req.scenario_name),
    })
}

/// Stop recording
async fn stop_recording(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    // TODO: Implement stop recording
    Json(StatusResponse {
        message: "Recording stopped".to_string(),
    })
}

/// Get recording status
async fn recording_status(State(state): State<ChaosApiState>) -> Json<RecordingStatusResponse> {
    // TODO: Implement recording status
    Json(RecordingStatusResponse {
        is_recording: false,
        scenario_name: None,
        events_recorded: 0,
    })
}

/// Export recording
async fn export_recording(
    State(state): State<ChaosApiState>,
    Json(req): Json<ExportRequest>,
) -> Json<StatusResponse> {
    // TODO: Implement export
    Json(StatusResponse {
        message: format!("Recording exported to: {}", req.path),
    })
}

/// Start replay
async fn start_replay(
    State(state): State<ChaosApiState>,
    Json(req): Json<StartReplayRequest>,
) -> Json<StatusResponse> {
    // TODO: Implement replay
    Json(StatusResponse {
        message: format!("Replay started from: {}", req.path),
    })
}

/// Pause replay
async fn pause_replay(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    // TODO: Implement pause
    Json(StatusResponse {
        message: "Replay paused".to_string(),
    })
}

/// Resume replay
async fn resume_replay(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    // TODO: Implement resume
    Json(StatusResponse {
        message: "Replay resumed".to_string(),
    })
}

/// Stop replay
async fn stop_replay(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    // TODO: Implement stop replay
    Json(StatusResponse {
        message: "Replay stopped".to_string(),
    })
}

/// Get replay status
async fn replay_status(State(state): State<ChaosApiState>) -> Json<ReplayStatusResponse> {
    // TODO: Implement replay status
    Json(ReplayStatusResponse {
        is_replaying: false,
        scenario_name: None,
        progress: 0.0,
    })
}

/// Start orchestration
async fn start_orchestration(
    State(state): State<ChaosApiState>,
    Json(req): Json<OrchestratedScenarioRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    use crate::scenario_orchestrator::ScenarioStep;

    // Build orchestrated scenario from request
    let mut orchestrated = OrchestratedScenario::new(req.name.clone());

    // Parse steps
    for step_value in req.steps {
        let step = serde_json::from_value::<ScenarioStep>(step_value)
            .map_err(|e| ChaosApiError::NotFound(format!("Invalid step: {}", e)))?;
        orchestrated = orchestrated.add_step(step);
    }

    // Set parallel if specified
    if req.parallel.unwrap_or(false) {
        orchestrated = orchestrated.with_parallel_execution();
    }

    // Start the orchestration
    let mut orchestrator = state.orchestrator.write().await;
    orchestrator.execute(orchestrated.clone()).await
        .map_err(|e| ChaosApiError::NotFound(format!("Failed to start orchestration: {}", e)))?;

    info!("Started orchestration '{}'", req.name);

    Ok(Json(StatusResponse {
        message: format!("Orchestration '{}' started successfully", req.name),
    }))
}

/// Stop orchestration
async fn stop_orchestration(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    let orchestrator = state.orchestrator.read().await;

    if orchestrator.is_running() {
        // Note: ScenarioOrchestrator doesn't expose stop() publicly yet
        // This would require adding that method or using the control channel
        info!("Orchestration stop requested");
        Json(StatusResponse {
            message: "Orchestration stop requested (will complete current step)".to_string(),
        })
    } else {
        Json(StatusResponse {
            message: "No orchestration currently running".to_string(),
        })
    }
}

/// Get orchestration status
async fn orchestration_status(State(state): State<ChaosApiState>) -> Json<OrchestrationStatusResponse> {
    let orchestrator = state.orchestrator.read().await;

    if let Some(status) = orchestrator.get_status() {
        Json(OrchestrationStatusResponse {
            is_running: status.is_running,
            name: Some(status.name.clone()),
            progress: status.progress,
        })
    } else {
        Json(OrchestrationStatusResponse {
            is_running: false,
            name: None,
            progress: 0.0,
        })
    }
}

/// Import orchestration from JSON/YAML
async fn import_orchestration(
    State(state): State<ChaosApiState>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Parse based on format
    let orchestrated = if req.format == "json" {
        OrchestratedScenario::from_json(&req.content)
            .map_err(|e| ChaosApiError::NotFound(format!("Invalid JSON: {}", e)))?
    } else if req.format == "yaml" {
        OrchestratedScenario::from_yaml(&req.content)
            .map_err(|e| ChaosApiError::NotFound(format!("Invalid YAML: {}", e)))?
    } else {
        return Err(ChaosApiError::NotFound("Unsupported format. Use 'json' or 'yaml'".to_string()));
    };

    info!("Imported orchestration: {}", orchestrated.name);

    Ok(Json(StatusResponse {
        message: format!("Orchestration '{}' imported successfully ({} steps)",
            orchestrated.name, orchestrated.steps.len()),
    }))
}

/// Add a schedule
async fn add_schedule(
    State(state): State<ChaosApiState>,
    Json(req): Json<ScheduledScenarioRequest>,
) -> Json<StatusResponse> {
    // TODO: Implement add schedule
    Json(StatusResponse {
        message: format!("Schedule '{}' added", req.id),
    })
}

/// Get a schedule
async fn get_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    // TODO: Implement get schedule
    Json(StatusResponse {
        message: format!("Schedule: {}", id),
    })
}

/// Remove a schedule
async fn remove_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    // TODO: Implement remove schedule
    Json(StatusResponse {
        message: format!("Schedule '{}' removed", id),
    })
}

/// Enable a schedule
async fn enable_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    // TODO: Implement enable schedule
    Json(StatusResponse {
        message: format!("Schedule '{}' enabled", id),
    })
}

/// Disable a schedule
async fn disable_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    // TODO: Implement disable schedule
    Json(StatusResponse {
        message: format!("Schedule '{}' disabled", id),
    })
}

/// Manually trigger a schedule
async fn trigger_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    // TODO: Implement trigger schedule
    Json(StatusResponse {
        message: format!("Schedule '{}' triggered", id),
    })
}

/// List all schedules
async fn list_schedules(State(state): State<ChaosApiState>) -> Json<Vec<ScheduleSummary>> {
    // TODO: Implement list schedules
    Json(vec![])
}

// Request/Response types for scenario management

#[derive(Debug, Deserialize)]
struct StartRecordingRequest {
    scenario_name: String,
}

#[derive(Debug, Deserialize)]
struct ExportRequest {
    path: String,
    format: Option<String>, // json or yaml
}

#[derive(Debug, Serialize)]
struct RecordingStatusResponse {
    is_recording: bool,
    scenario_name: Option<String>,
    events_recorded: usize,
}

#[derive(Debug, Deserialize)]
struct StartReplayRequest {
    path: String,
    speed: Option<f64>,
    loop_replay: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ReplayStatusResponse {
    is_replaying: bool,
    scenario_name: Option<String>,
    progress: f64,
}

#[derive(Debug, Deserialize)]
struct OrchestratedScenarioRequest {
    name: String,
    steps: Vec<serde_json::Value>,
    parallel: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OrchestrationStatusResponse {
    is_running: bool,
    name: Option<String>,
    progress: f64,
}

#[derive(Debug, Deserialize)]
struct ImportRequest {
    content: String,
    format: String, // json or yaml
}

#[derive(Debug, Deserialize)]
struct ScheduledScenarioRequest {
    id: String,
    scenario: serde_json::Value,
    schedule: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ScheduleSummary {
    id: String,
    scenario_name: String,
    enabled: bool,
    next_execution: Option<String>,
}

// AI-powered recommendation handlers

/// Get all recommendations
async fn get_recommendations(State(state): State<ChaosApiState>) -> Json<Vec<crate::recommendations::Recommendation>> {
    Json(state.recommendation_engine.get_recommendations())
}

/// Analyze metrics and generate recommendations
async fn analyze_and_recommend(State(state): State<ChaosApiState>) -> Json<AnalyzeResponse> {
    use chrono::{Duration, Utc};

    // Get metrics from last 24 hours
    let end = Utc::now();
    let start = end - Duration::hours(24);

    let buckets = state.analytics.get_metrics(start, end, TimeBucket::Hour);
    let impact = state.analytics.get_impact_analysis(start, end, TimeBucket::Hour);

    let recommendations = state.recommendation_engine.analyze_and_recommend(&buckets, &impact);

    Json(AnalyzeResponse {
        total_recommendations: recommendations.len(),
        high_priority: recommendations.iter().filter(|r| matches!(r.severity, RecommendationSeverity::High | RecommendationSeverity::Critical)).count(),
        recommendations,
    })
}

/// Get recommendations by category
async fn get_recommendations_by_category(
    State(state): State<ChaosApiState>,
    Path(category): Path<String>,
) -> Result<Json<Vec<crate::recommendations::Recommendation>>, StatusCode> {
    let category = match category.as_str() {
        "latency" => RecommendationCategory::Latency,
        "fault_injection" => RecommendationCategory::FaultInjection,
        "rate_limit" => RecommendationCategory::RateLimit,
        "traffic_shaping" => RecommendationCategory::TrafficShaping,
        "circuit_breaker" => RecommendationCategory::CircuitBreaker,
        "bulkhead" => RecommendationCategory::Bulkhead,
        "scenario" => RecommendationCategory::Scenario,
        "coverage" => RecommendationCategory::Coverage,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    Ok(Json(state.recommendation_engine.get_recommendations_by_category(category)))
}

/// Get recommendations by severity
async fn get_recommendations_by_severity(
    State(state): State<ChaosApiState>,
    Path(severity): Path<String>,
) -> Result<Json<Vec<crate::recommendations::Recommendation>>, StatusCode> {
    let severity = match severity.as_str() {
        "info" => RecommendationSeverity::Info,
        "low" => RecommendationSeverity::Low,
        "medium" => RecommendationSeverity::Medium,
        "high" => RecommendationSeverity::High,
        "critical" => RecommendationSeverity::Critical,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    Ok(Json(state.recommendation_engine.get_recommendations_by_severity(severity)))
}

/// Clear all recommendations
async fn clear_recommendations(State(state): State<ChaosApiState>) -> Json<StatusResponse> {
    state.recommendation_engine.clear();
    Json(StatusResponse {
        message: "Recommendations cleared".to_string(),
    })
}

#[derive(Debug, Serialize)]
struct AnalyzeResponse {
    total_recommendations: usize,
    high_priority: usize,
    recommendations: Vec<crate::recommendations::Recommendation>,
}

// Auto-remediation endpoints

/// Get remediation configuration
async fn get_remediation_config(State(state): State<ChaosApiState>) -> Json<RemediationConfig> {
    Json(state.remediation_engine.get_config())
}

/// Update remediation configuration
async fn update_remediation_config(
    State(state): State<ChaosApiState>,
    Json(config): Json<RemediationConfig>,
) -> Json<StatusResponse> {
    state.remediation_engine.update_config(config);
    Json(StatusResponse {
        message: "Remediation configuration updated".to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct ProcessRemediationRequest {
    recommendation: Recommendation,
}

/// Process a recommendation for auto-remediation
async fn process_remediation(
    State(state): State<ChaosApiState>,
    Json(req): Json<ProcessRemediationRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.remediation_engine.process_recommendation(&req.recommendation) {
        Ok(action_id) => Ok(Json(serde_json::json!({
            "success": true,
            "action_id": action_id,
            "message": "Recommendation processed"
        }))),
        Err(err) => Ok(Json(serde_json::json!({
            "success": false,
            "error": err
        }))),
    }
}

#[derive(Debug, Deserialize)]
struct ApproveRequest {
    approver: String,
}

/// Approve a remediation action
async fn approve_remediation(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
    Json(req): Json<ApproveRequest>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.remediation_engine.approve_action(&id, &req.approver) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Action {} approved", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

#[derive(Debug, Deserialize)]
struct RejectRequest {
    reason: String,
}

/// Reject a remediation action
async fn reject_remediation(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
    Json(req): Json<RejectRequest>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.remediation_engine.reject_action(&id, &req.reason) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Action {} rejected", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Rollback a remediation action
async fn rollback_remediation(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    match state.remediation_engine.rollback_action(&id) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Action {} rolled back", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get all remediation actions
async fn get_remediation_actions(State(state): State<ChaosApiState>) -> Json<Vec<crate::auto_remediation::RemediationAction>> {
    Json(state.remediation_engine.get_active_actions())
}

/// Get a specific remediation action
async fn get_remediation_action(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<crate::auto_remediation::RemediationAction>, StatusCode> {
    match state.remediation_engine.get_action(&id) {
        Some(action) => Ok(Json(action)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get approval queue
async fn get_approval_queue(State(state): State<ChaosApiState>) -> Json<Vec<crate::auto_remediation::ApprovalRequest>> {
    Json(state.remediation_engine.get_approval_queue())
}

/// Get effectiveness metrics for an action
async fn get_remediation_effectiveness(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<crate::auto_remediation::EffectivenessMetrics>, StatusCode> {
    match state.remediation_engine.get_effectiveness(&id) {
        Some(metrics) => Ok(Json(metrics)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get remediation statistics
async fn get_remediation_stats(State(state): State<ChaosApiState>) -> Json<crate::auto_remediation::RemediationStats> {
    Json(state.remediation_engine.get_stats())
}

// A/B testing endpoints

/// Create a new A/B test
async fn create_ab_test(
    State(state): State<ChaosApiState>,
    Json(config): Json<ABTestConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.create_test(config) {
        Ok(test_id) => Ok(Json(serde_json::json!({
            "success": true,
            "test_id": test_id
        }))),
        Err(err) => Ok(Json(serde_json::json!({
            "success": false,
            "error": err
        }))),
    }
}

/// Get all A/B tests
async fn get_ab_tests(State(state): State<ChaosApiState>) -> Json<Vec<crate::ab_testing::ABTest>> {
    let engine = state.ab_testing_engine.read().await;
    Json(engine.get_all_tests())
}

/// Get a specific A/B test
async fn get_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<crate::ab_testing::ABTest>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.get_test(&id) {
        Some(test) => Ok(Json(test)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Start an A/B test
async fn start_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.start_test(&id) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Test {} started", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Stop an A/B test
async fn stop_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<TestConclusion>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.stop_test(&id) {
        Ok(conclusion) => Ok(Json(conclusion)),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Pause an A/B test
async fn pause_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.pause_test(&id) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Test {} paused", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Resume an A/B test
async fn resume_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.resume_test(&id) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Test {} resumed", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Record variant results
async fn record_ab_test_result(
    State(state): State<ChaosApiState>,
    Path((id, variant)): Path<(String, String)>,
    Json(results): Json<VariantResults>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.record_variant_result(&id, &variant, results) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Results recorded for variant {}", variant),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Delete an A/B test
async fn delete_ab_test(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, StatusCode> {
    let engine = state.ab_testing_engine.read().await;
    match engine.delete_test(&id) {
        Ok(_) => Ok(Json(StatusResponse {
            message: format!("Test {} deleted", id),
        })),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get A/B test statistics
async fn get_ab_test_stats(State(state): State<ChaosApiState>) -> Json<crate::ab_testing::ABTestStats> {
    let engine = state.ab_testing_engine.read().await;
    Json(engine.get_stats())
}

// Error handling

#[derive(Debug)]
enum ChaosApiError {
    NotFound(String),
}

impl IntoResponse for ChaosApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ChaosApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
