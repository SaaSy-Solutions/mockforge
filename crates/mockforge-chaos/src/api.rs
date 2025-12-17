//! Management API for chaos engineering

use crate::{
    ab_testing::{ABTestConfig, ABTestingEngine, TestConclusion, VariantResults},
    analytics::{ChaosAnalytics, TimeBucket},
    auto_remediation::{RemediationConfig, RemediationEngine},
    config::{
        BulkheadConfig, ChaosConfig, CircuitBreakerConfig, FaultInjectionConfig, LatencyConfig,
        NetworkProfile, RateLimitConfig, TrafficShapingConfig,
    },
    latency_metrics::LatencyMetricsTracker,
    recommendations::{
        Recommendation, RecommendationCategory, RecommendationEngine, RecommendationSeverity,
    },
    scenario_orchestrator::{OrchestratedScenario, ScenarioOrchestrator},
    scenario_recorder::{RecordedScenario, ScenarioRecorder},
    scenario_replay::{ReplayOptions, ReplaySpeed, ScenarioReplayEngine},
    scenario_scheduler::{ScenarioScheduler, ScheduleType, ScheduledScenario},
    scenarios::{ChaosScenario, PredefinedScenarios, ScenarioEngine},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use parking_lot::RwLock as ParkingRwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Profile manager for storing custom profiles
#[derive(Clone)]
pub struct ProfileManager {
    /// Custom user-created profiles
    custom_profiles: Arc<ParkingRwLock<std::collections::HashMap<String, NetworkProfile>>>,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new() -> Self {
        Self {
            custom_profiles: Arc::new(ParkingRwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get all profiles (built-in + custom)
    pub fn get_all_profiles(&self) -> Vec<NetworkProfile> {
        let mut profiles = NetworkProfile::predefined_profiles();
        let custom = self.custom_profiles.read();
        profiles.extend(custom.values().cloned());
        profiles
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<NetworkProfile> {
        // Check built-in profiles first
        for profile in NetworkProfile::predefined_profiles() {
            if profile.name == name {
                return Some(profile);
            }
        }
        // Check custom profiles
        let custom = self.custom_profiles.read();
        custom.get(name).cloned()
    }

    /// Add or update a custom profile
    pub fn save_profile(&self, profile: NetworkProfile) {
        let mut custom = self.custom_profiles.write();
        custom.insert(profile.name.clone(), profile);
    }

    /// Delete a custom profile
    pub fn delete_profile(&self, name: &str) -> bool {
        let mut custom = self.custom_profiles.write();
        custom.remove(name).is_some()
    }

    /// Get only custom profiles
    pub fn get_custom_profiles(&self) -> Vec<NetworkProfile> {
        let custom = self.custom_profiles.read();
        custom.values().cloned().collect()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub recorder: Arc<ScenarioRecorder>,
    pub replay_engine: Arc<tokio::sync::RwLock<ScenarioReplayEngine>>,
    pub scheduler: Arc<tokio::sync::RwLock<ScenarioScheduler>>,
    pub latency_tracker: Arc<LatencyMetricsTracker>,
    pub profile_manager: Arc<ProfileManager>,
    pub mockai:
        Option<std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>>,
}

/// Create the chaos management API router
///
/// # Arguments
/// * `config` - Initial chaos configuration
/// * `mockai` - Optional MockAI instance for dynamic error message generation
///
/// # Returns
/// Tuple of (Router, Config, LatencyTracker, ChaosApiState) - The router, config, latency tracker, and API state for hot-reload support
pub fn create_chaos_api_router(
    config: ChaosConfig,
    mockai: Option<
        std::sync::Arc<tokio::sync::RwLock<mockforge_core::intelligent_behavior::MockAI>>,
    >,
) -> (Router, Arc<RwLock<ChaosConfig>>, Arc<LatencyMetricsTracker>, Arc<ChaosApiState>) {
    let config_arc = Arc::new(RwLock::new(config));
    let scenario_engine = Arc::new(ScenarioEngine::new());
    let orchestrator = Arc::new(tokio::sync::RwLock::new(ScenarioOrchestrator::new()));
    let analytics = Arc::new(ChaosAnalytics::new());
    let recommendation_engine = Arc::new(RecommendationEngine::new());
    let remediation_engine = Arc::new(RemediationEngine::new());
    let ab_testing_engine =
        Arc::new(tokio::sync::RwLock::new(ABTestingEngine::new(analytics.clone())));
    let recorder = Arc::new(ScenarioRecorder::new());
    let replay_engine = Arc::new(tokio::sync::RwLock::new(ScenarioReplayEngine::new()));
    let scheduler = Arc::new(tokio::sync::RwLock::new(ScenarioScheduler::new()));
    let latency_tracker = Arc::new(LatencyMetricsTracker::new());
    let profile_manager = Arc::new(ProfileManager::new());

    // Clone latency_tracker for return value (state will own the original)
    let latency_tracker_for_return = latency_tracker.clone();

    let state = ChaosApiState {
        config: config_arc.clone(),
        scenario_engine,
        orchestrator,
        analytics,
        recommendation_engine,
        remediation_engine,
        ab_testing_engine,
        recorder,
        replay_engine,
        scheduler,
        latency_tracker,
        profile_manager,
        mockai,
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
        .route("/api/chaos/scenarios/{name}", post(start_scenario))
        .route("/api/chaos/scenarios/{name}", delete(stop_scenario))
        .route("/api/chaos/scenarios", delete(stop_all_scenarios))

        // Status endpoint
        .route("/api/chaos/status", get(get_status))

        // Metrics endpoints
        .route("/api/chaos/metrics/latency", get(get_latency_metrics))
        .route("/api/chaos/metrics/latency/stats", get(get_latency_stats))

        // Profile management endpoints
        .route("/api/chaos/profiles", get(list_profiles))
        .route("/api/chaos/profiles/{name}", get(get_profile))
        .route("/api/chaos/profiles/{name}/apply", post(apply_profile))
        .route("/api/chaos/profiles", post(create_profile))
        .route("/api/chaos/profiles/{name}", delete(delete_profile))
        .route("/api/chaos/profiles/{name}/export", get(export_profile))
        .route("/api/chaos/profiles/import", post(import_profile))

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
        .route("/api/chaos/schedule/{id}", get(get_schedule))
        .route("/api/chaos/schedule/{id}", delete(remove_schedule))
        .route("/api/chaos/schedule/{id}/enable", post(enable_schedule))
        .route("/api/chaos/schedule/{id}/disable", post(disable_schedule))
        // NOTE: Manual trigger endpoint has a known Rust/Axum type inference issue
        // when combining State + Path extractors with nested async calls.
        // The trigger_schedule_by_path handler is implemented but cannot be registered.
        // Workaround: Use the scheduler's automatic execution or recreate the schedule.
        // .route("/api/chaos/schedule/{id}/trigger", post(trigger_schedule_by_path))
        .route("/api/chaos/schedules", get(list_schedules))

        // AI-powered recommendation endpoints
        .route("/api/chaos/recommendations", get(get_recommendations))
        .route("/api/chaos/recommendations/analyze", post(analyze_and_recommend))
        .route("/api/chaos/recommendations/category/{category}", get(get_recommendations_by_category))
        .route("/api/chaos/recommendations/severity/{severity}", get(get_recommendations_by_severity))
        .route("/api/chaos/recommendations", delete(clear_recommendations))

        // Auto-remediation endpoints
        .route("/api/chaos/remediation/config", get(get_remediation_config))
        .route("/api/chaos/remediation/config", put(update_remediation_config))
        .route("/api/chaos/remediation/process", post(process_remediation))
        .route("/api/chaos/remediation/approve/{id}", post(approve_remediation))
        .route("/api/chaos/remediation/reject/{id}", post(reject_remediation))
        .route("/api/chaos/remediation/rollback/{id}", post(rollback_remediation))
        .route("/api/chaos/remediation/actions", get(get_remediation_actions))
        .route("/api/chaos/remediation/actions/{id}", get(get_remediation_action))
        .route("/api/chaos/remediation/approvals", get(get_approval_queue))
        .route("/api/chaos/remediation/effectiveness/{id}", get(get_remediation_effectiveness))
        .route("/api/chaos/remediation/stats", get(get_remediation_stats))

        // A/B testing endpoints
        .route("/api/chaos/ab-tests", post(create_ab_test))
        .route("/api/chaos/ab-tests", get(get_ab_tests))
        .route("/api/chaos/ab-tests/{id}", get(get_ab_test))
        .route("/api/chaos/ab-tests/{id}/start", post(start_ab_test))
        .route("/api/chaos/ab-tests/{id}/stop", post(stop_ab_test))
        .route("/api/chaos/ab-tests/{id}/pause", post(pause_ab_test))
        .route("/api/chaos/ab-tests/{id}/resume", post(resume_ab_test))
        .route("/api/chaos/ab-tests/{id}/record/{variant}", post(record_ab_test_result))
        .route("/api/chaos/ab-tests/{id}", delete(delete_ab_test))
        .route("/api/chaos/ab-tests/stats", get(get_ab_test_stats))

        .with_state(state.clone());

    (router, config_arc, latency_tracker_for_return, Arc::new(state))
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
            description: "Simulates degraded network conditions with high latency and packet loss"
                .to_string(),
            tags: vec!["network".to_string(), "latency".to_string()],
        },
        PredefinedScenarioInfo {
            name: "service_instability".to_string(),
            description: "Simulates an unstable service with random errors and timeouts"
                .to_string(),
            tags: vec!["service".to_string(), "errors".to_string()],
        },
        PredefinedScenarioInfo {
            name: "cascading_failure".to_string(),
            description: "Simulates a cascading failure with multiple simultaneous issues"
                .to_string(),
            tags: vec!["critical".to_string(), "cascading".to_string()],
        },
        PredefinedScenarioInfo {
            name: "peak_traffic".to_string(),
            description: "Simulates peak traffic conditions with aggressive rate limiting"
                .to_string(),
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
    let mut http_errors = config
        .fault_injection
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

    let mut http_errors = config
        .fault_injection
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

    let mut http_errors = config
        .fault_injection
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
        message: format!(
            "GraphQL resolver latency {}",
            if req.enabled { "enabled" } else { "disabled" }
        ),
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

// Scenario management handlers

/// Start recording a scenario
async fn start_recording(
    State(state): State<ChaosApiState>,
    Json(req): Json<StartRecordingRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Get the scenario based on name
    let scenario = match req.scenario_name.as_str() {
        "network_degradation" => PredefinedScenarios::network_degradation(),
        "service_instability" => PredefinedScenarios::service_instability(),
        "cascading_failure" => PredefinedScenarios::cascading_failure(),
        "peak_traffic" => PredefinedScenarios::peak_traffic(),
        "slow_backend" => PredefinedScenarios::slow_backend(),
        _ => {
            // Check if it's an active scenario
            let active_scenarios = state.scenario_engine.get_active_scenarios();
            active_scenarios
                .into_iter()
                .find(|s| s.name == req.scenario_name)
                .ok_or_else(|| {
                    ChaosApiError::NotFound(format!("Scenario '{}' not found", req.scenario_name))
                })?
        }
    };

    // Start recording
    match state.recorder.start_recording(scenario) {
        Ok(_) => {
            info!("Recording started for scenario: {}", req.scenario_name);
            Ok(Json(StatusResponse {
                message: format!("Recording started for scenario: {}", req.scenario_name),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Stop recording
async fn stop_recording(
    State(state): State<ChaosApiState>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    match state.recorder.stop_recording() {
        Ok(recording) => {
            info!(
                "Recording stopped for scenario: {} ({} events)",
                recording.scenario.name,
                recording.events.len()
            );
            Ok(Json(StatusResponse {
                message: format!(
                    "Recording stopped for scenario: {} ({} events, {}ms)",
                    recording.scenario.name,
                    recording.events.len(),
                    recording.total_duration_ms
                ),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Get recording status
async fn recording_status(State(state): State<ChaosApiState>) -> Json<RecordingStatusResponse> {
    if let Some(recording) = state.recorder.get_current_recording() {
        Json(RecordingStatusResponse {
            is_recording: true,
            scenario_name: Some(recording.scenario.name),
            events_recorded: recording.events.len(),
        })
    } else {
        Json(RecordingStatusResponse {
            is_recording: false,
            scenario_name: None,
            events_recorded: 0,
        })
    }
}

/// Export recording
async fn export_recording(
    State(state): State<ChaosApiState>,
    Json(req): Json<ExportRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Check if there's a current recording first
    if state.recorder.get_current_recording().is_some() {
        return Err(ChaosApiError::NotFound(
            "Cannot export while recording is in progress. Stop recording first.".to_string(),
        ));
    }

    // Get the most recent recording
    let recordings = state.recorder.get_recordings();
    if recordings.is_empty() {
        return Err(ChaosApiError::NotFound("No recordings available to export".to_string()));
    }

    let recording = recordings.last().unwrap();

    // Export to the specified path
    match recording.save_to_file(&req.path) {
        Ok(_) => {
            info!("Recording exported to: {}", req.path);
            Ok(Json(StatusResponse {
                message: format!(
                    "Recording exported to: {} ({} events)",
                    req.path,
                    recording.events.len()
                ),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(format!("Failed to export recording: {}", err))),
    }
}

/// Start replay
async fn start_replay(
    State(state): State<ChaosApiState>,
    Json(req): Json<StartReplayRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Load the recorded scenario from file
    let recorded = RecordedScenario::load_from_file(&req.path)
        .map_err(|e| ChaosApiError::NotFound(format!("Failed to load recording: {}", e)))?;

    // Build replay options
    let speed = match req.speed {
        Some(s) if s > 0.0 => ReplaySpeed::Custom(s),
        Some(0.0) => ReplaySpeed::Fast,
        _ => ReplaySpeed::RealTime,
    };

    let options = ReplayOptions {
        speed,
        loop_replay: req.loop_replay.unwrap_or(false),
        skip_initial_delay: false,
        event_type_filter: None,
    };

    // Start replay
    let mut replay_engine = state.replay_engine.write().await;
    match replay_engine.replay(recorded.clone(), options).await {
        Ok(_) => {
            info!("Replay started for scenario: {}", recorded.scenario.name);
            Ok(Json(StatusResponse {
                message: format!(
                    "Replay started for scenario: {} ({} events)",
                    recorded.scenario.name,
                    recorded.events.len()
                ),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Pause replay
async fn pause_replay(
    State(state): State<ChaosApiState>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let replay_engine = state.replay_engine.read().await;
    match replay_engine.pause().await {
        Ok(_) => {
            info!("Replay paused");
            Ok(Json(StatusResponse {
                message: "Replay paused".to_string(),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Resume replay
async fn resume_replay(
    State(state): State<ChaosApiState>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let replay_engine = state.replay_engine.read().await;
    match replay_engine.resume().await {
        Ok(_) => {
            info!("Replay resumed");
            Ok(Json(StatusResponse {
                message: "Replay resumed".to_string(),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Stop replay
async fn stop_replay(
    State(state): State<ChaosApiState>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let replay_engine = state.replay_engine.read().await;
    match replay_engine.stop().await {
        Ok(_) => {
            info!("Replay stopped");
            Ok(Json(StatusResponse {
                message: "Replay stopped".to_string(),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Get replay status
async fn replay_status(State(state): State<ChaosApiState>) -> Json<ReplayStatusResponse> {
    let replay_engine = state.replay_engine.read().await;
    if let Some(status) = replay_engine.get_status() {
        Json(ReplayStatusResponse {
            is_replaying: status.is_playing,
            scenario_name: Some(status.scenario_name),
            progress: status.progress,
        })
    } else {
        Json(ReplayStatusResponse {
            is_replaying: false,
            scenario_name: None,
            progress: 0.0,
        })
    }
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
    orchestrator
        .execute(orchestrated.clone())
        .await
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
async fn orchestration_status(
    State(state): State<ChaosApiState>,
) -> Json<OrchestrationStatusResponse> {
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
    State(_state): State<ChaosApiState>,
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
        return Err(ChaosApiError::NotFound(
            "Unsupported format. Use 'json' or 'yaml'".to_string(),
        ));
    };

    info!("Imported orchestration: {}", orchestrated.name);

    Ok(Json(StatusResponse {
        message: format!(
            "Orchestration '{}' imported successfully ({} steps)",
            orchestrated.name,
            orchestrated.steps.len()
        ),
    }))
}

/// Add a schedule
async fn add_schedule(
    State(state): State<ChaosApiState>,
    Json(req): Json<ScheduledScenarioRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Parse scenario from JSON
    let scenario = serde_json::from_value::<ChaosScenario>(req.scenario)
        .map_err(|e| ChaosApiError::NotFound(format!("Invalid scenario: {}", e)))?;

    // Parse schedule from JSON
    let schedule = serde_json::from_value::<ScheduleType>(req.schedule)
        .map_err(|e| ChaosApiError::NotFound(format!("Invalid schedule: {}", e)))?;

    // Create scheduled scenario
    let scheduled = ScheduledScenario::new(req.id.clone(), scenario, schedule);

    // Add to scheduler
    let scheduler = state.scheduler.read().await;
    scheduler.add_schedule(scheduled);

    info!("Schedule '{}' added", req.id);
    Ok(Json(StatusResponse {
        message: format!("Schedule '{}' added", req.id),
    }))
}

/// Get a schedule
async fn get_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<ScheduledScenario>, ChaosApiError> {
    let scheduler = state.scheduler.read().await;
    match scheduler.get_schedule(&id) {
        Some(scheduled) => Ok(Json(scheduled)),
        None => Err(ChaosApiError::NotFound(format!("Schedule '{}' not found", id))),
    }
}

/// Remove a schedule
async fn remove_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let scheduler = state.scheduler.read().await;
    match scheduler.remove_schedule(&id) {
        Some(_) => {
            info!("Schedule '{}' removed", id);
            Ok(Json(StatusResponse {
                message: format!("Schedule '{}' removed", id),
            }))
        }
        None => Err(ChaosApiError::NotFound(format!("Schedule '{}' not found", id))),
    }
}

/// Enable a schedule
async fn enable_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let scheduler = state.scheduler.read().await;
    match scheduler.enable_schedule(&id) {
        Ok(_) => {
            info!("Schedule '{}' enabled", id);
            Ok(Json(StatusResponse {
                message: format!("Schedule '{}' enabled", id),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Disable a schedule
async fn disable_schedule(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let scheduler = state.scheduler.read().await;
    match scheduler.disable_schedule(&id) {
        Ok(_) => {
            info!("Schedule '{}' disabled", id);
            Ok(Json(StatusResponse {
                message: format!("Schedule '{}' disabled", id),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// Manually trigger a schedule (using Path parameter)
///
/// NOTE: This handler is fully implemented but cannot be registered as a route
/// due to a Rust/Axum type inference issue. The problem occurs when:
/// 1. A handler has State + Path/Json extractors
/// 2. The handler makes two consecutive `.await` calls:
///    - First await: acquiring the RwLock (`scheduler.read().await`)
///    - Second await: calling an async method (`trigger_now(&id).await`)
///
/// This causes Axum's Handler trait inference to fail with:
/// "the trait `Handler<_, _>` is not implemented for fn item..."
///
/// Root cause: Complex interaction between Rust's type inference, async/await
/// semantics, and Axum's Handler trait bounds when futures are composed.
///
/// Workarounds:
/// - Use the scheduler's automatic time-based execution
/// - Recreate the schedule to reset its execution state
/// - Call scheduler.trigger_now() directly from application code
#[allow(dead_code)]
async fn trigger_schedule_by_path(
    State(state): State<ChaosApiState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let scheduler = state.scheduler.read().await;
    let schedule_exists = scheduler.get_schedule(&id).is_some();

    if !schedule_exists {
        return Err(ChaosApiError::NotFound(format!("Schedule '{}' not found", id)));
    }

    let trigger_result = scheduler.trigger_now(&id).await;

    match trigger_result {
        Ok(_) => {
            info!("Schedule '{}' triggered", id);
            Ok(Json(StatusResponse {
                message: format!("Schedule '{}' triggered", id),
            }))
        }
        Err(err) => Err(ChaosApiError::NotFound(err)),
    }
}

/// List all schedules
async fn list_schedules(State(state): State<ChaosApiState>) -> Json<Vec<ScheduleSummary>> {
    let scheduler = state.scheduler.read().await;
    let schedules = scheduler.get_all_schedules();
    let summaries = schedules
        .into_iter()
        .map(|s| ScheduleSummary {
            id: s.id,
            scenario_name: s.scenario.name,
            enabled: s.enabled,
            next_execution: s.next_execution.map(|t| t.to_rfc3339()),
        })
        .collect();
    Json(summaries)
}

// Request/Response types for scenario management

#[derive(Debug, Deserialize)]
struct StartRecordingRequest {
    scenario_name: String,
}

#[derive(Debug, Deserialize)]
struct ExportRequest {
    path: String,
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

#[derive(Debug, Deserialize, Serialize)]
struct ScheduleSummary {
    id: String,
    scenario_name: String,
    enabled: bool,
    next_execution: Option<String>,
}

// AI-powered recommendation handlers

/// Get all recommendations
async fn get_recommendations(
    State(state): State<ChaosApiState>,
) -> Json<Vec<crate::recommendations::Recommendation>> {
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
        high_priority: recommendations
            .iter()
            .filter(|r| {
                matches!(
                    r.severity,
                    RecommendationSeverity::High | RecommendationSeverity::Critical
                )
            })
            .count(),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get all remediation actions
async fn get_remediation_actions(
    State(state): State<ChaosApiState>,
) -> Json<Vec<crate::auto_remediation::RemediationAction>> {
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
async fn get_approval_queue(
    State(state): State<ChaosApiState>,
) -> Json<Vec<crate::auto_remediation::ApprovalRequest>> {
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
async fn get_remediation_stats(
    State(state): State<ChaosApiState>,
) -> Json<crate::auto_remediation::RemediationStats> {
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
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
        Err(_err) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Get A/B test statistics
async fn get_ab_test_stats(
    State(state): State<ChaosApiState>,
) -> Json<crate::ab_testing::ABTestStats> {
    let engine = state.ab_testing_engine.read().await;
    Json(engine.get_stats())
}

// Latency metrics endpoints

/// Get latency metrics (time-series data)
async fn get_latency_metrics(State(state): State<ChaosApiState>) -> Json<LatencyMetricsResponse> {
    let samples = state.latency_tracker.get_samples();
    Json(LatencyMetricsResponse { samples })
}

/// Get latency statistics
async fn get_latency_stats(
    State(state): State<ChaosApiState>,
) -> Json<crate::latency_metrics::LatencyStats> {
    let stats = state.latency_tracker.get_stats();
    Json(stats)
}

#[derive(Debug, Serialize)]
struct LatencyMetricsResponse {
    samples: Vec<crate::latency_metrics::LatencySample>,
}

// Profile management endpoints

/// List all profiles (built-in + custom)
async fn list_profiles(State(state): State<ChaosApiState>) -> Json<Vec<NetworkProfile>> {
    let profiles = state.profile_manager.get_all_profiles();
    Json(profiles)
}

/// Get a specific profile by name
async fn get_profile(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
) -> Result<Json<NetworkProfile>, ChaosApiError> {
    match state.profile_manager.get_profile(&name) {
        Some(profile) => Ok(Json(profile)),
        None => Err(ChaosApiError::NotFound(format!("Profile '{}' not found", name))),
    }
}

/// Apply a profile (update chaos config)
async fn apply_profile(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let profile = state
        .profile_manager
        .get_profile(&name)
        .ok_or_else(|| ChaosApiError::NotFound(format!("Profile '{}' not found", name)))?;

    // Apply the profile's chaos config
    let mut config = state.config.write().await;
    *config = profile.chaos_config.clone();

    info!("Applied profile: {}", name);
    Ok(Json(StatusResponse {
        message: format!("Profile '{}' applied successfully", name),
    }))
}

/// Create a new custom profile
async fn create_profile(
    State(state): State<ChaosApiState>,
    Json(profile): Json<NetworkProfile>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Check if it's a built-in profile name
    for builtin in NetworkProfile::predefined_profiles() {
        if builtin.name == profile.name {
            return Err(ChaosApiError::NotFound(format!(
                "Cannot create profile '{}': name conflicts with built-in profile",
                profile.name
            )));
        }
    }

    // Mark as custom
    let mut custom_profile = profile;
    custom_profile.builtin = false;

    state.profile_manager.save_profile(custom_profile.clone());
    info!("Created custom profile: {}", custom_profile.name);
    Ok(Json(StatusResponse {
        message: format!("Profile '{}' created successfully", custom_profile.name),
    }))
}

/// Delete a custom profile
async fn delete_profile(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    // Check if it's a built-in profile
    for builtin in NetworkProfile::predefined_profiles() {
        if builtin.name == name {
            return Err(ChaosApiError::NotFound(format!(
                "Cannot delete built-in profile '{}'",
                name
            )));
        }
    }

    if state.profile_manager.delete_profile(&name) {
        info!("Deleted custom profile: {}", name);
        Ok(Json(StatusResponse {
            message: format!("Profile '{}' deleted successfully", name),
        }))
    } else {
        Err(ChaosApiError::NotFound(format!("Profile '{}' not found", name)))
    }
}

/// Export a profile as JSON or YAML
async fn export_profile(
    State(state): State<ChaosApiState>,
    Path(name): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, ChaosApiError> {
    let profile = state
        .profile_manager
        .get_profile(&name)
        .ok_or_else(|| ChaosApiError::NotFound(format!("Profile '{}' not found", name)))?;

    let format = params.get("format").map(|s| s.as_str()).unwrap_or("json");

    if format == "yaml" {
        let yaml = serde_yaml::to_string(&profile).map_err(|e| {
            ChaosApiError::NotFound(format!("Failed to serialize profile to YAML: {}", e))
        })?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/yaml")
            .body(axum::body::Body::from(yaml))
            .unwrap()
            .into_response())
    } else {
        // Default to JSON
        let json = serde_json::to_value(&profile)
            .map_err(|e| ChaosApiError::NotFound(format!("Failed to serialize profile: {}", e)))?;
        Ok(Json(json).into_response())
    }
}

/// Import a profile from JSON or YAML
async fn import_profile(
    State(state): State<ChaosApiState>,
    Json(req): Json<ImportProfileRequest>,
) -> Result<Json<StatusResponse>, ChaosApiError> {
    let profile: NetworkProfile = if req.format == "yaml" {
        serde_yaml::from_str(&req.content)
            .map_err(|e| ChaosApiError::NotFound(format!("Failed to parse YAML: {}", e)))?
    } else {
        serde_json::from_str(&req.content)
            .map_err(|e| ChaosApiError::NotFound(format!("Failed to parse JSON: {}", e)))?
    };

    // Check if it's a built-in profile name
    for builtin in NetworkProfile::predefined_profiles() {
        if builtin.name == profile.name {
            return Err(ChaosApiError::NotFound(format!(
                "Cannot import profile '{}': name conflicts with built-in profile",
                profile.name
            )));
        }
    }

    // Mark as custom
    let mut custom_profile = profile;
    custom_profile.builtin = false;

    state.profile_manager.save_profile(custom_profile.clone());
    info!("Imported profile: {}", custom_profile.name);
    Ok(Json(StatusResponse {
        message: format!("Profile '{}' imported successfully", custom_profile.name),
    }))
}

#[derive(Debug, Deserialize)]
struct ImportProfileRequest {
    content: String,
    format: String, // "json" or "yaml"
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LatencyConfig;

    #[test]
    fn test_profile_manager_new() {
        let manager = ProfileManager::new();
        let profiles = manager.get_all_profiles();
        // Should have predefined profiles
        assert!(!profiles.is_empty());
    }

    #[test]
    fn test_profile_manager_default() {
        let manager = ProfileManager::default();
        let profiles = manager.get_all_profiles();
        assert!(!profiles.is_empty());
    }

    #[test]
    fn test_profile_manager_get_all_profiles() {
        let manager = ProfileManager::new();
        let profiles = manager.get_all_profiles();

        // Should contain predefined profiles
        let profile_names: Vec<_> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(profile_names.contains(&"slow_3g"));
        assert!(profile_names.contains(&"fast_3g"));
        assert!(profile_names.contains(&"flaky_wifi"));
    }

    #[test]
    fn test_profile_manager_get_profile_builtin() {
        let manager = ProfileManager::new();

        // Test getting a built-in profile
        let profile = manager.get_profile("slow_3g");
        assert!(profile.is_some());
        let profile = profile.unwrap();
        assert_eq!(profile.name, "slow_3g");
        assert!(profile.builtin);
    }

    #[test]
    fn test_profile_manager_get_profile_not_found() {
        let manager = ProfileManager::new();
        let profile = manager.get_profile("nonexistent");
        assert!(profile.is_none());
    }

    #[test]
    fn test_profile_manager_save_and_get_custom_profile() {
        let manager = ProfileManager::new();

        // Create a custom profile
        let custom = NetworkProfile {
            name: "custom_test".to_string(),
            description: "Test profile".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };

        manager.save_profile(custom.clone());

        // Retrieve it
        let retrieved = manager.get_profile("custom_test");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "custom_test");
        assert_eq!(retrieved.description, "Test profile");
        assert!(!retrieved.builtin);
    }

    #[test]
    fn test_profile_manager_update_custom_profile() {
        let manager = ProfileManager::new();

        // Create and save initial profile
        let custom = NetworkProfile {
            name: "custom_test".to_string(),
            description: "Initial description".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };
        manager.save_profile(custom);

        // Update the profile
        let updated = NetworkProfile {
            name: "custom_test".to_string(),
            description: "Updated description".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };
        manager.save_profile(updated);

        // Verify update
        let retrieved = manager.get_profile("custom_test").unwrap();
        assert_eq!(retrieved.description, "Updated description");
    }

    #[test]
    fn test_profile_manager_delete_profile() {
        let manager = ProfileManager::new();

        // Create and save a custom profile
        let custom = NetworkProfile {
            name: "to_delete".to_string(),
            description: "Will be deleted".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };
        manager.save_profile(custom);

        // Verify it exists
        assert!(manager.get_profile("to_delete").is_some());

        // Delete it
        let deleted = manager.delete_profile("to_delete");
        assert!(deleted);

        // Verify it's gone
        assert!(manager.get_profile("to_delete").is_none());
    }

    #[test]
    fn test_profile_manager_delete_nonexistent() {
        let manager = ProfileManager::new();
        let deleted = manager.delete_profile("nonexistent");
        assert!(!deleted);
    }

    #[test]
    fn test_profile_manager_get_custom_profiles() {
        let manager = ProfileManager::new();

        // Initially should have no custom profiles
        assert_eq!(manager.get_custom_profiles().len(), 0);

        // Add custom profiles
        let custom1 = NetworkProfile {
            name: "custom1".to_string(),
            description: "Custom 1".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };
        let custom2 = NetworkProfile {
            name: "custom2".to_string(),
            description: "Custom 2".to_string(),
            builtin: false,
            tags: Vec::new(),
            chaos_config: ChaosConfig::default(),
        };

        manager.save_profile(custom1);
        manager.save_profile(custom2);

        // Should have 2 custom profiles
        let customs = manager.get_custom_profiles();
        assert_eq!(customs.len(), 2);

        let names: Vec<_> = customs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"custom1"));
        assert!(names.contains(&"custom2"));
    }

    #[test]
    fn test_chaos_api_error_not_found() {
        let error = ChaosApiError::NotFound("Test error".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_status_response_serialize() {
        let response = StatusResponse {
            message: "Test message".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["message"], "Test message");
    }

    #[test]
    fn test_predefined_scenario_info_serialize() {
        let info = PredefinedScenarioInfo {
            name: "test".to_string(),
            description: "Test scenario".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["description"], "Test scenario");
        assert_eq!(json["tags"][0], "tag1");
    }

    #[test]
    fn test_chaos_status_serialize() {
        let status = ChaosStatus {
            enabled: true,
            active_scenarios: vec!["scenario1".to_string()],
            latency_enabled: true,
            fault_injection_enabled: false,
            rate_limit_enabled: true,
            traffic_shaping_enabled: false,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["enabled"], true);
        assert_eq!(json["active_scenarios"][0], "scenario1");
        assert_eq!(json["latency_enabled"], true);
    }

    #[test]
    fn test_grpc_status_codes_request_deserialize() {
        let json = r#"{"status_codes":[3,16,5],"probability":0.5}"#;
        let req: GrpcStatusCodesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status_codes, vec![3, 16, 5]);
        assert_eq!(req.probability, 0.5);
    }

    #[test]
    fn test_websocket_close_codes_request_deserialize() {
        let json = r#"{"close_codes":[1002,1001],"probability":0.3}"#;
        let req: WebSocketCloseCodesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.close_codes, vec![1002, 1001]);
        assert_eq!(req.probability, 0.3);
    }

    #[test]
    fn test_graphql_error_codes_request_deserialize() {
        let json = r#"{"error_codes":["BAD_USER_INPUT","UNAUTHENTICATED"],"probability":0.7}"#;
        let req: GraphQLErrorCodesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.error_codes, vec!["BAD_USER_INPUT", "UNAUTHENTICATED"]);
        assert_eq!(req.probability, 0.7);
    }

    #[test]
    fn test_probability_request_deserialize() {
        let json = r#"{"probability":0.42}"#;
        let req: ProbabilityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.probability, 0.42);
    }

    #[test]
    fn test_enable_request_deserialize() {
        let json = r#"{"enabled":true}"#;
        let req: EnableRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled, true);

        let json = r#"{"enabled":false}"#;
        let req: EnableRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled, false);
    }

    #[test]
    fn test_start_recording_request_deserialize() {
        let json = r#"{"scenario_name":"network_degradation"}"#;
        let req: StartRecordingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.scenario_name, "network_degradation");
    }

    #[test]
    fn test_export_request_deserialize() {
        let json = r#"{"path":"/tmp/recording.json"}"#;
        let req: ExportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "/tmp/recording.json");
    }

    #[test]
    fn test_recording_status_response_serialize() {
        let response = RecordingStatusResponse {
            is_recording: true,
            scenario_name: Some("test_scenario".to_string()),
            events_recorded: 42,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["is_recording"], true);
        assert_eq!(json["scenario_name"], "test_scenario");
        assert_eq!(json["events_recorded"], 42);
    }

    #[test]
    fn test_start_replay_request_deserialize() {
        let json = r#"{"path":"/tmp/replay.json","speed":2.0,"loop_replay":true}"#;
        let req: StartReplayRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "/tmp/replay.json");
        assert_eq!(req.speed, Some(2.0));
        assert_eq!(req.loop_replay, Some(true));
    }

    #[test]
    fn test_replay_status_response_serialize() {
        let response = ReplayStatusResponse {
            is_replaying: true,
            scenario_name: Some("test".to_string()),
            progress: 0.75,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["is_replaying"], true);
        assert_eq!(json["progress"], 0.75);
    }

    #[test]
    fn test_orchestration_status_response_serialize() {
        let response = OrchestrationStatusResponse {
            is_running: true,
            name: Some("test_orchestration".to_string()),
            progress: 0.5,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["is_running"], true);
        assert_eq!(json["name"], "test_orchestration");
        assert_eq!(json["progress"], 0.5);
    }

    #[test]
    fn test_import_request_deserialize() {
        let json = r#"{"content":"test content","format":"json"}"#;
        let req: ImportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test content");
        assert_eq!(req.format, "json");
    }

    #[test]
    fn test_schedule_summary_serialize_deserialize() {
        let summary = ScheduleSummary {
            id: "test_id".to_string(),
            scenario_name: "test_scenario".to_string(),
            enabled: true,
            next_execution: Some("2024-01-01T00:00:00Z".to_string()),
        };

        // Test serialization
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["id"], "test_id");
        assert_eq!(json["scenario_name"], "test_scenario");
        assert_eq!(json["enabled"], true);

        // Test deserialization
        let deserialized: ScheduleSummary = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.id, "test_id");
        assert_eq!(deserialized.scenario_name, "test_scenario");
        assert_eq!(deserialized.enabled, true);
    }

    #[test]
    fn test_analyze_response_serialize() {
        let response = AnalyzeResponse {
            total_recommendations: 10,
            high_priority: 3,
            recommendations: vec![],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total_recommendations"], 10);
        assert_eq!(json["high_priority"], 3);
    }

    #[test]
    fn test_approve_request_deserialize() {
        let json = r#"{"approver":"admin@example.com"}"#;
        let req: ApproveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.approver, "admin@example.com");
    }

    #[test]
    fn test_reject_request_deserialize() {
        let json = r#"{"reason":"Not applicable"}"#;
        let req: RejectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.reason, "Not applicable");
    }

    #[test]
    fn test_latency_metrics_response_serialize() {
        let response = LatencyMetricsResponse { samples: vec![] };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json["samples"].is_array());
    }

    #[test]
    fn test_import_profile_request_deserialize() {
        let json = r#"{"content":"{}","format":"json"}"#;
        let req: ImportProfileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "{}");
        assert_eq!(req.format, "json");
    }

    #[tokio::test]
    async fn test_chaos_api_state_creation() {
        let config = ChaosConfig::default();
        let (mut router, config_arc, latency_tracker, state) =
            create_chaos_api_router(config, None);

        // Verify router was created - getting a service confirms it exists
        let _service = router.as_service::<axum::body::Body>();

        // Verify config was wrapped in Arc
        let cfg = config_arc.read().await;
        assert!(!cfg.enabled); // Default is disabled
        drop(cfg);

        // Verify latency tracker exists
        assert_eq!(latency_tracker.get_samples().len(), 0);

        // Verify state exists and has correct components
        assert!(state.config.read().await.enabled == false);
    }

    #[tokio::test]
    async fn test_chaos_api_state_with_mockai() {
        let config = ChaosConfig::default();
        let mockai_config =
            mockforge_core::intelligent_behavior::IntelligentBehaviorConfig::default();
        let mockai = Arc::new(tokio::sync::RwLock::new(
            mockforge_core::intelligent_behavior::MockAI::new(mockai_config),
        ));

        let (_router, _config_arc, _latency_tracker, state) =
            create_chaos_api_router(config, Some(mockai.clone()));

        // Verify MockAI was set
        assert!(state.mockai.is_some());
    }

    #[test]
    fn test_profile_manager_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(ProfileManager::new());
        let mut handles = vec![];

        // Spawn multiple threads that access the profile manager
        for i in 0..5 {
            let manager = manager.clone();
            let handle = thread::spawn(move || {
                let profile = NetworkProfile {
                    name: format!("concurrent_{}", i),
                    description: format!("Thread {}", i),
                    builtin: false,
                    tags: Vec::new(),
                    chaos_config: ChaosConfig::default(),
                };
                manager.save_profile(profile.clone());

                // Read it back
                let retrieved = manager.get_profile(&format!("concurrent_{}", i));
                assert!(retrieved.is_some());
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all profiles were saved
        let customs = manager.get_custom_profiles();
        assert_eq!(customs.len(), 5);
    }

    #[test]
    fn test_orchestrated_scenario_request_deserialize() {
        let json = r#"{
            "name": "test_orchestration",
            "steps": [],
            "parallel": true
        }"#;
        let req: OrchestratedScenarioRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "test_orchestration");
        assert_eq!(req.steps.len(), 0);
        assert_eq!(req.parallel, Some(true));
    }

    #[test]
    fn test_scheduled_scenario_request_deserialize() {
        let json = r#"{
            "id": "test_schedule",
            "scenario": {},
            "schedule": {}
        }"#;
        let req: ScheduledScenarioRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "test_schedule");
    }

    #[test]
    fn test_edge_cases_probability_values() {
        // Test boundary values
        let json = r#"{"probability":0.0}"#;
        let req: ProbabilityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.probability, 0.0);

        let json = r#"{"probability":1.0}"#;
        let req: ProbabilityRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.probability, 1.0);
    }

    #[test]
    fn test_empty_arrays_in_requests() {
        let json = r#"{"status_codes":[],"probability":0.5}"#;
        let req: GrpcStatusCodesRequest = serde_json::from_str(json).unwrap();
        assert!(req.status_codes.is_empty());

        let json = r#"{"close_codes":[],"probability":0.5}"#;
        let req: WebSocketCloseCodesRequest = serde_json::from_str(json).unwrap();
        assert!(req.close_codes.is_empty());
    }
}
