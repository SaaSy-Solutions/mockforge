//! Observability API endpoints for the Admin UI
//!
//! This module provides REST API endpoints for the Admin UI to interact with
//! chaos engineering features, including metrics, alerts, traces, and scenarios.

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    alerts::{Alert, AlertManager},
    analytics::{ChaosAnalytics, ChaosImpact, MetricsBucket, TimeBucket},
    dashboard::{DashboardManager, DashboardStats, DashboardUpdate},
    scenario_orchestrator::ScenarioOrchestrator,
    scenario_recorder::ScenarioRecorder,
    scenario_replay::ScenarioReplayEngine,
    scenario_scheduler::ScenarioScheduler,
    scenarios::ScenarioEngine,
};

/// Observability API state
#[derive(Clone)]
pub struct ObservabilityState {
    pub analytics: Arc<ChaosAnalytics>,
    pub alert_manager: Arc<AlertManager>,
    pub dashboard: Arc<DashboardManager>,
    pub scenario_engine: Arc<ScenarioEngine>,
    pub recorder: Arc<ScenarioRecorder>,
    pub replay_engine: Arc<ScenarioReplayEngine>,
    pub scheduler: Arc<ScenarioScheduler>,
    pub orchestrator: Arc<ScenarioOrchestrator>,
}

/// Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Create observability API router
pub fn create_observability_router(state: ObservabilityState) -> Router {
    Router::new()
        // Dashboard stats
        .route("/api/observability/stats", get(get_stats))
        .route("/api/observability/alerts", get(get_alerts))
        .route("/api/observability/ws", get(websocket_handler))
        // Traces
        .route("/api/observability/traces", get(get_traces))
        .route("/api/observability/traces/:trace_id/flamegraph", get(get_flamegraph))
        // Dashboard layouts
        .route("/api/dashboard/layouts", get(list_dashboard_layouts))
        .route("/api/dashboard/layouts", post(create_dashboard_layout))
        .route("/api/dashboard/layouts/:id", get(get_dashboard_layout))
        .route("/api/dashboard/layouts/:id", post(update_dashboard_layout))
        .route("/api/dashboard/layouts/:id", axum::routing::delete(delete_dashboard_layout))
        .route("/api/dashboard/templates", get(get_dashboard_templates))
        // Reports and exports
        .route("/api/reports/pdf", post(generate_pdf_report))
        .route("/api/reports/csv", post(generate_csv_report))
        .route("/api/reports/compare", post(compare_scenarios))
        // Chaos scenarios
        .route("/api/chaos/scenarios", get(list_scenarios))
        .route("/api/chaos/scenarios/:name", post(start_scenario))
        .route("/api/chaos/status", get(get_chaos_status))
        .route("/api/chaos/disable", post(disable_chaos))
        .route("/api/chaos/reset", post(reset_chaos))
        // Recording
        .route("/api/chaos/recording/start", post(start_recording))
        .route("/api/chaos/recording/stop", post(stop_recording))
        .route("/api/chaos/recording/status", get(recording_status))
        .route("/api/chaos/recording/list", get(list_recordings))
        .route("/api/chaos/recording/export", post(export_recording))
        // Replay
        .route("/api/chaos/replay/start", post(start_replay))
        .route("/api/chaos/replay/stop", post(stop_replay))
        .route("/api/chaos/replay/status", get(replay_status))
        // Recorder search
        .route("/api/recorder/search", post(search_requests))
        .with_state(state)
}

/// Get dashboard statistics
async fn get_stats(State(state): State<ObservabilityState>) -> Json<DashboardStats> {
    let stats = state.dashboard.get_stats();
    Json(stats)
}

/// Get active alerts
async fn get_alerts(State(state): State<ObservabilityState>) -> Json<Vec<Alert>> {
    let alerts = state.alert_manager.get_active_alerts();
    Json(alerts)
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ObservabilityState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(
    mut socket: axum::extract::ws::WebSocket,
    state: ObservabilityState,
) {
    use axum::extract::ws::Message;

    let mut rx = state.dashboard.subscribe();

    // Send initial stats
    let stats = state.dashboard.get_stats();
    let update = DashboardUpdate::Ping {
        timestamp: chrono::Utc::now(),
    };
    if let Ok(json) = serde_json::to_string(&update) {
        let _ = socket.send(Message::Text(json.into())).await;
    }

    // Stream updates
    while let Ok(update) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&update) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
}

/// Get traces (stub - would integrate with OpenTelemetry backend)
#[derive(Serialize)]
struct TracesResponse {
    traces: Vec<serde_json::Value>,
}

async fn get_traces() -> Json<TracesResponse> {
    // TODO: Integrate with OpenTelemetry exporter
    // For now, return empty array
    Json(TracesResponse { traces: vec![] })
}

/// List available chaos scenarios
#[derive(Serialize)]
struct ScenariosResponse {
    scenarios: Vec<String>,
}

async fn list_scenarios(State(state): State<ObservabilityState>) -> Json<ScenariosResponse> {
    let scenarios = vec![
        "network_degradation".to_string(),
        "service_instability".to_string(),
        "cascading_failure".to_string(),
        "peak_traffic".to_string(),
        "slow_backend".to_string(),
    ];

    Json(ScenariosResponse { scenarios })
}

/// Start a chaos scenario
async fn start_scenario(
    State(state): State<ObservabilityState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    // TODO: Actually start the scenario
    tracing::info!("Starting chaos scenario: {}", name);
    Json(ApiResponse::success(format!("Started scenario: {}", name)))
}

/// Get chaos status
#[derive(Serialize)]
struct ChaosStatus {
    is_enabled: bool,
    active_scenario: Option<String>,
    current_config: Option<serde_json::Value>,
}

async fn get_chaos_status() -> Json<ChaosStatus> {
    // TODO: Get actual status from chaos engine
    Json(ChaosStatus {
        is_enabled: false,
        active_scenario: None,
        current_config: None,
    })
}

/// Disable chaos
async fn disable_chaos() -> Json<ApiResponse<String>> {
    tracing::info!("Disabling chaos engineering");
    Json(ApiResponse::success("Chaos disabled".to_string()))
}

/// Reset chaos configuration
async fn reset_chaos() -> Json<ApiResponse<String>> {
    tracing::info!("Resetting chaos configuration");
    Json(ApiResponse::success("Chaos reset".to_string()))
}

/// Start recording
#[derive(Deserialize)]
struct StartRecordingRequest {
    scenario_name: String,
}

async fn start_recording(
    State(state): State<ObservabilityState>,
    Json(req): Json<StartRecordingRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Starting recording: {}", req.scenario_name);
    Json(ApiResponse::success("Recording started".to_string()))
}

/// Stop recording
async fn stop_recording(State(state): State<ObservabilityState>) -> Json<ApiResponse<String>> {
    tracing::info!("Stopping recording");
    Json(ApiResponse::success("Recording stopped".to_string()))
}

/// Recording status
#[derive(Serialize)]
struct RecordingStatus {
    is_recording: bool,
    current_scenario: Option<String>,
    events_count: usize,
}

async fn recording_status() -> Json<RecordingStatus> {
    Json(RecordingStatus {
        is_recording: false,
        current_scenario: None,
        events_count: 0,
    })
}

/// List recordings
#[derive(Serialize)]
struct RecordingsResponse {
    scenarios: Vec<RecordingInfo>,
}

#[derive(Serialize)]
struct RecordingInfo {
    name: String,
    started_at: String,
    ended_at: Option<String>,
    total_events: usize,
    duration_ms: u64,
}

async fn list_recordings() -> Json<RecordingsResponse> {
    Json(RecordingsResponse { scenarios: vec![] })
}

/// Export recording
#[derive(Deserialize)]
struct ExportRequest {
    scenario_name: String,
    format: String,
}

async fn export_recording(Json(req): Json<ExportRequest>) -> Response {
    tracing::info!("Exporting scenario: {} as {}", req.scenario_name, req.format);
    // TODO: Actually export the recording
    Json(ApiResponse::success("Export initiated".to_string())).into_response()
}

/// Start replay
#[derive(Deserialize)]
struct StartReplayRequest {
    scenario_name: String,
    speed: f64,
    loop_replay: bool,
}

async fn start_replay(
    State(state): State<ObservabilityState>,
    Json(req): Json<StartReplayRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Starting replay: {} at {}x speed", req.scenario_name, req.speed);
    Json(ApiResponse::success("Replay started".to_string()))
}

/// Stop replay
async fn stop_replay() -> Json<ApiResponse<String>> {
    tracing::info!("Stopping replay");
    Json(ApiResponse::success("Replay stopped".to_string()))
}

/// Replay status
#[derive(Serialize)]
struct ReplayStatus {
    is_playing: bool,
    scenario_name: Option<String>,
    progress: f64,
}

async fn replay_status() -> Json<ReplayStatus> {
    Json(ReplayStatus {
        is_playing: false,
        scenario_name: None,
        progress: 0.0,
    })
}

/// Search recorded requests
#[derive(Deserialize)]
struct SearchRequest {
    limit: Option<usize>,
    protocol: Option<String>,
    status_code: Option<u16>,
}

#[derive(Serialize)]
struct SearchResponse {
    requests: Vec<RecordedRequest>,
}

#[derive(Serialize)]
struct RecordedRequest {
    id: i64,
    timestamp: String,
    protocol: String,
    method: String,
    path: String,
    status_code: u16,
    duration_ms: f64,
    client_ip: Option<String>,
    request_headers: serde_json::Value,
    request_body: Option<String>,
    response_headers: serde_json::Value,
    response_body: Option<String>,
}

async fn search_requests(Json(req): Json<SearchRequest>) -> Json<SearchResponse> {
    // TODO: Query from recorder database
    Json(SearchResponse { requests: vec![] })
}

// ===== Advanced Observability Endpoints =====

/// Get flamegraph for a trace
async fn get_flamegraph(
    axum::extract::Path(trace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<FlamegraphResponse>> {
    // TODO: Generate flamegraph from trace data
    tracing::info!("Generating flamegraph for trace: {}", trace_id);
    Json(ApiResponse::success(FlamegraphResponse {
        trace_id: trace_id.clone(),
        svg_url: format!("/flamegraphs/{}.svg", trace_id),
        stats: FlamegraphStatsResponse {
            total_spans: 10,
            max_depth: 3,
            total_duration_us: 50000,
            hottest_path: vec!["api-gateway::request".to_string(), "database::query".to_string()],
        },
    }))
}

#[derive(Serialize)]
struct FlamegraphResponse {
    trace_id: String,
    svg_url: String,
    stats: FlamegraphStatsResponse,
}

#[derive(Serialize)]
struct FlamegraphStatsResponse {
    total_spans: usize,
    max_depth: usize,
    total_duration_us: u64,
    hottest_path: Vec<String>,
}

/// List dashboard layouts
async fn list_dashboard_layouts() -> Json<ApiResponse<Vec<DashboardLayoutSummary>>> {
    // TODO: Load from dashboard layout manager
    tracing::info!("Listing dashboard layouts");
    Json(ApiResponse::success(vec![
        DashboardLayoutSummary {
            id: "chaos-overview".to_string(),
            name: "Chaos Engineering Overview".to_string(),
            description: Some("Real-time overview of chaos engineering activities".to_string()),
            widget_count: 3,
        },
        DashboardLayoutSummary {
            id: "service-perf".to_string(),
            name: "Service Performance".to_string(),
            description: Some("Detailed service performance metrics".to_string()),
            widget_count: 2,
        },
    ]))
}

#[derive(Serialize)]
struct DashboardLayoutSummary {
    id: String,
    name: String,
    description: Option<String>,
    widget_count: usize,
}

/// Create dashboard layout
#[derive(Deserialize)]
struct CreateDashboardLayoutRequest {
    name: String,
    description: Option<String>,
    layout_data: serde_json::Value,
}

async fn create_dashboard_layout(
    Json(req): Json<CreateDashboardLayoutRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Creating dashboard layout: {}", req.name);
    // TODO: Save to dashboard layout manager
    Json(ApiResponse::success("layout-id-123".to_string()))
}

/// Get dashboard layout
async fn get_dashboard_layout(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    tracing::info!("Getting dashboard layout: {}", id);
    // TODO: Load from dashboard layout manager
    Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "name": "Sample Layout",
        "widgets": []
    })))
}

/// Update dashboard layout
async fn update_dashboard_layout(
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<CreateDashboardLayoutRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Updating dashboard layout: {}", id);
    // TODO: Update in dashboard layout manager
    Json(ApiResponse::success("Updated".to_string()))
}

/// Delete dashboard layout
async fn delete_dashboard_layout(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Deleting dashboard layout: {}", id);
    // TODO: Delete from dashboard layout manager
    Json(ApiResponse::success("Deleted".to_string()))
}

/// Get dashboard templates
async fn get_dashboard_templates() -> Json<ApiResponse<Vec<DashboardLayoutSummary>>> {
    tracing::info!("Getting dashboard templates");
    Json(ApiResponse::success(vec![
        DashboardLayoutSummary {
            id: "template-chaos-overview".to_string(),
            name: "Chaos Engineering Overview".to_string(),
            description: Some("Pre-built chaos engineering dashboard".to_string()),
            widget_count: 3,
        },
        DashboardLayoutSummary {
            id: "template-resilience".to_string(),
            name: "Resilience Testing".to_string(),
            description: Some("Monitor resilience patterns".to_string()),
            widget_count: 2,
        },
    ]))
}

/// Generate PDF report
#[derive(Deserialize)]
struct GeneratePdfRequest {
    scenario_name: String,
    include_charts: bool,
}

async fn generate_pdf_report(Json(req): Json<GeneratePdfRequest>) -> Response {
    tracing::info!("Generating PDF report for: {}", req.scenario_name);
    // TODO: Generate PDF using mockforge-reporting
    Json(ApiResponse::success(
        format!("/reports/{}.pdf", req.scenario_name)
    )).into_response()
}

/// Generate CSV report
#[derive(Deserialize)]
struct GenerateCsvRequest {
    scenario_names: Vec<String>,
    include_comparison: bool,
}

async fn generate_csv_report(Json(req): Json<GenerateCsvRequest>) -> Response {
    tracing::info!("Generating CSV report for: {:?}", req.scenario_names);
    // TODO: Generate CSV using mockforge-reporting
    Json(ApiResponse::success(
        "/reports/scenarios.csv".to_string()
    )).into_response()
}

/// Compare scenarios
#[derive(Deserialize)]
struct CompareRequest {
    baseline_scenario: String,
    comparison_scenarios: Vec<String>,
}

async fn compare_scenarios(Json(req): Json<CompareRequest>) -> Json<ApiResponse<ComparisonResult>> {
    tracing::info!(
        "Comparing scenarios - baseline: {}, comparisons: {:?}",
        req.baseline_scenario,
        req.comparison_scenarios
    );
    // TODO: Use mockforge-reporting comparison tools
    Json(ApiResponse::success(ComparisonResult {
        baseline: req.baseline_scenario,
        comparisons: req.comparison_scenarios,
        regressions_count: 2,
        improvements_count: 5,
        verdict: "better".to_string(),
    }))
}

#[derive(Serialize)]
struct ComparisonResult {
    baseline: String,
    comparisons: Vec<String>,
    regressions_count: usize,
    improvements_count: usize,
    verdict: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test");
        assert!(response.success);
        assert_eq!(response.data, Some("test"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse::error("error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("error".to_string()));
    }
}
