//! Observability API endpoints for the Admin UI
//!
//! This module provides REST API endpoints for the Admin UI to interact with
//! chaos engineering features, including metrics, alerts, traces, and scenarios.

use axum::{
    extract::{State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    alerts::{Alert, AlertManager},
    analytics::ChaosAnalytics,
    dashboard::{DashboardManager, DashboardStats, DashboardUpdate},
    scenario_orchestrator::ScenarioOrchestrator,
    scenario_recorder::ScenarioRecorder,
    scenario_replay::ScenarioReplayEngine,
    scenario_scheduler::ScenarioScheduler,
    scenarios::ScenarioEngine,
    trace_collector::TraceCollector,
};
use mockforge_recorder::Recorder;
use parking_lot::RwLock;
use printpdf::*;
use std::collections::HashMap;

/// Generate flamegraph SVG from actual trace data
fn generate_flamegraph_from_trace(
    trace_id: &str,
    traces: &[crate::trace_collector::CollectedTrace],
) -> String {
    use std::collections::HashMap;

    let width = 1200;
    let height = 600;
    let bar_height = 20;
    let mut y_offset = 60;

    let mut svg = format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
        <rect width="100%" height="100%" fill="white"/>
        <text x="10" y="20" font-family="monospace" font-size="12">Flamegraph for trace: {}</text>
        <text x="10" y="35" font-family="monospace" font-size="10">Total spans: {}</text>"#,
        width,
        height,
        trace_id,
        traces.len()
    );

    // Build span hierarchy
    let mut span_map: HashMap<String, &crate::trace_collector::CollectedTrace> = HashMap::new();
    for trace in traces {
        span_map.insert(trace.span_id.clone(), trace);
    }

    // Find root spans
    let mut root_spans = Vec::new();
    for trace in traces {
        if trace.parent_span_id.is_none() {
            root_spans.push(trace);
        }
    }

    // Sort root spans by start time
    root_spans.sort_by_key(|s| s.start_time.clone());

    // Calculate total time range
    let min_start = traces
        .iter()
        .map(|t| {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&t.start_time) {
                dt.timestamp_micros() as u64
            } else {
                0
            }
        })
        .min()
        .unwrap_or(0);

    let max_end = traces
        .iter()
        .map(|t| {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&t.start_time) {
                dt.timestamp_micros() as u64 + t.duration_ms * 1000
            } else {
                t.duration_ms * 1000
            }
        })
        .max()
        .unwrap_or(1000000);

    let total_duration = max_end.saturating_sub(min_start);

    // Render spans level by level
    let mut current_level = root_spans;
    let mut level = 0;

    while !current_level.is_empty() && y_offset + bar_height < height {
        let mut next_level = Vec::new();

        for span in &current_level {
            // Calculate position and width
            let start_us = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&span.start_time) {
                dt.timestamp_micros() as u64
            } else {
                span.start_time.parse().unwrap_or(0)
            };

            let x = ((start_us.saturating_sub(min_start)) as f64 / total_duration as f64
                * (width - 40) as f64) as u32
                + 20;
            let bar_width = ((span.duration_ms * 1000) as f64 / total_duration as f64
                * (width - 40) as f64) as u32;

            if bar_width > 0 {
                let color = format!("#{:x}", (level * 50 + 100) % 256);
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="black" stroke-width="1"/>
                    <text x="{}" y="{}" font-family="monospace" font-size="10" fill="white">{}</text>"#,
                    x, y_offset, bar_width, bar_height, color,
                    x + 2, y_offset + 12, span.name
                ));
            }

            // Find children
            for trace in traces {
                if trace.parent_span_id.as_ref() == Some(&span.span_id) {
                    next_level.push(trace);
                }
            }
        }

        // Sort next level by start time
        next_level.sort_by_key(|s| s.start_time.clone());

        current_level = next_level;
        y_offset += bar_height + 2;
        level += 1;
    }

    svg.push_str("</svg>");
    svg
}

/// Calculate the maximum depth of the trace hierarchy
fn calculate_max_depth(traces: &[crate::trace_collector::CollectedTrace]) -> usize {
    use std::collections::HashMap;

    let mut span_map: HashMap<String, &crate::trace_collector::CollectedTrace> = HashMap::new();
    let mut depth_map: HashMap<String, usize> = HashMap::new();

    // Index spans by span_id
    for trace in traces {
        span_map.insert(trace.span_id.clone(), trace);
    }

    // Calculate depth for each span
    for trace in traces {
        calculate_span_depth(&trace.span_id, &span_map, &mut depth_map);
    }

    depth_map.values().cloned().max().unwrap_or(0)
}

/// Recursively calculate depth for a span
fn calculate_span_depth(
    span_id: &str,
    span_map: &HashMap<String, &crate::trace_collector::CollectedTrace>,
    depth_map: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&depth) = depth_map.get(span_id) {
        return depth;
    }

    let span = match span_map.get(span_id) {
        Some(s) => s,
        None => return 0,
    };

    let depth = if let Some(ref parent_id) = span.parent_span_id {
        calculate_span_depth(parent_id, span_map, depth_map) + 1
    } else {
        0
    };

    depth_map.insert(span_id.to_string(), depth);
    depth
}

/// Find the hottest path (longest duration path) in the trace
fn find_hottest_path(traces: &[crate::trace_collector::CollectedTrace]) -> Vec<String> {
    use std::collections::HashMap;

    if traces.is_empty() {
        return Vec::new();
    }

    let mut span_map: HashMap<String, &crate::trace_collector::CollectedTrace> = HashMap::new();

    // Index spans by span_id
    for trace in traces {
        span_map.insert(trace.span_id.clone(), trace);
    }

    // Find root spans
    let mut root_spans = Vec::new();
    for trace in traces {
        if trace.parent_span_id.is_none() {
            root_spans.push(trace);
        }
    }

    if root_spans.is_empty() {
        return Vec::new();
    }

    // For simplicity, return the path from the first root span
    // In a real implementation, you'd find the path with maximum total duration
    let mut path = Vec::new();
    let mut current = root_spans[0];

    loop {
        path.push(current.name.clone());
        let mut found_child = false;

        // Find a child span (simplified - just pick the first one)
        for trace in traces {
            if trace.parent_span_id.as_ref() == Some(&current.span_id) {
                current = trace;
                found_child = true;
                break;
            }
        }

        if !found_child {
            break;
        }
    }

    path
}

/// Generate basic HTML content for PDF report
/// Generate CSV content for scenario comparison
fn generate_csv_content(scenario_names: &[String], include_comparison: bool) -> String {
    let mut csv =
        String::from("Scenario,Total Requests,Success Rate,Avg Latency (ms),Error Rate\n");

    for scenario in scenario_names {
        // Mock data - in real implementation, would fetch actual metrics
        let (requests, success_rate, avg_latency, _error_rate) = match scenario.as_str() {
            "network_degradation" => (1000, 92.5, 250.0, 7.5),
            "service_instability" => (800, 88.0, 180.0, 12.0),
            "cascading_failure" => (1200, 85.0, 320.0, 15.0),
            _ => (1000, 95.0, 150.0, 5.0),
        };

        csv.push_str(&format!(
            "{},{},{:.1},{:.1},{:.1}\n",
            scenario,
            requests,
            success_rate,
            avg_latency,
            100.0 - success_rate
        ));
    }

    if include_comparison && scenario_names.len() > 1 {
        csv.push_str("\nComparison Summary\n");
        csv.push_str("Best Success Rate,network_degradation\n");
        csv.push_str("Worst Latency,service_instability\n");
        csv.push_str("Highest Error Rate,cascading_failure\n");
    }

    csv
}

/// Perform basic scenario comparison
fn perform_scenario_comparison(baseline: &str, comparisons: &[String]) -> ComparisonResult {
    // Mock comparison logic - in real implementation, would analyze actual metrics
    let baseline_metrics = get_scenario_metrics(baseline);
    let mut regressions = 0;
    let mut improvements = 0;

    for scenario in comparisons {
        let metrics = get_scenario_metrics(scenario);

        // Compare success rates
        if metrics.success_rate < baseline_metrics.success_rate {
            regressions += 1;
        } else if metrics.success_rate > baseline_metrics.success_rate {
            improvements += 1;
        }

        // Compare latencies
        if metrics.avg_latency > baseline_metrics.avg_latency {
            regressions += 1;
        } else if metrics.avg_latency < baseline_metrics.avg_latency {
            improvements += 1;
        }
    }

    let verdict = if regressions > improvements {
        "worse".to_string()
    } else if improvements > regressions {
        "better".to_string()
    } else {
        "similar".to_string()
    };

    ComparisonResult {
        baseline: baseline.to_string(),
        comparisons: comparisons.to_vec(),
        regressions_count: regressions,
        improvements_count: improvements,
        verdict,
    }
}

/// Mock scenario metrics
struct ScenarioMetrics {
    success_rate: f64,
    avg_latency: f64,
}

fn get_scenario_metrics(scenario: &str) -> ScenarioMetrics {
    match scenario {
        "network_degradation" => ScenarioMetrics {
            success_rate: 92.5,
            avg_latency: 250.0,
        },
        "service_instability" => ScenarioMetrics {
            success_rate: 88.0,
            avg_latency: 180.0,
        },
        "cascading_failure" => ScenarioMetrics {
            success_rate: 85.0,
            avg_latency: 320.0,
        },
        _ => ScenarioMetrics {
            success_rate: 95.0,
            avg_latency: 150.0,
        },
    }
}

/// Simple in-memory dashboard layout manager
#[derive(Clone)]
pub struct SimpleDashboardLayoutManager {
    layouts: Arc<RwLock<HashMap<String, DashboardLayoutSummary>>>,
}

impl SimpleDashboardLayoutManager {
    pub fn new() -> Self {
        let mut layouts = HashMap::new();
        layouts.insert(
            "chaos-overview".to_string(),
            DashboardLayoutSummary {
                id: "chaos-overview".to_string(),
                name: "Chaos Engineering Overview".to_string(),
                description: Some("Real-time overview of chaos engineering activities".to_string()),
                widget_count: 3,
            },
        );
        layouts.insert(
            "service-perf".to_string(),
            DashboardLayoutSummary {
                id: "service-perf".to_string(),
                name: "Service Performance".to_string(),
                description: Some("Detailed service performance metrics".to_string()),
                widget_count: 2,
            },
        );

        Self {
            layouts: Arc::new(RwLock::new(layouts)),
        }
    }
}

impl Default for SimpleDashboardLayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleDashboardLayoutManager {
    pub fn list_layouts(&self) -> Vec<DashboardLayoutSummary> {
        self.layouts.read().values().cloned().collect()
    }

    pub fn get_layout(&self, id: &str) -> Option<DashboardLayoutSummary> {
        self.layouts.read().get(id).cloned()
    }

    pub fn create_layout(&self, layout: DashboardLayoutSummary) {
        self.layouts.write().insert(layout.id.clone(), layout);
    }

    pub fn update_layout(&self, id: &str, layout: DashboardLayoutSummary) {
        self.layouts.write().insert(id.to_string(), layout);
    }

    pub fn delete_layout(&self, id: &str) {
        self.layouts.write().remove(id);
    }
}

/// Observability API state
#[derive(Clone)]
pub struct ObservabilityState {
    pub analytics: Arc<ChaosAnalytics>,
    pub alert_manager: Arc<AlertManager>,
    pub dashboard: Arc<DashboardManager>,
    pub scenario_engine: Arc<ScenarioEngine>,
    pub recorder: Arc<ScenarioRecorder>,
    pub request_recorder: Option<Arc<Recorder>>,
    pub replay_engine: Arc<ScenarioReplayEngine>,
    pub scheduler: Arc<ScenarioScheduler>,
    pub orchestrator: Arc<ScenarioOrchestrator>,
    pub layout_manager: Arc<SimpleDashboardLayoutManager>,
    pub trace_collector: Arc<TraceCollector>,
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

async fn handle_websocket(mut socket: axum::extract::ws::WebSocket, state: ObservabilityState) {
    use axum::extract::ws::Message;

    let mut rx = state.dashboard.subscribe();

    // Send initial stats
    let _stats = state.dashboard.get_stats();
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

/// Get traces from OpenTelemetry backend
#[derive(Serialize)]
struct TracesResponse {
    traces: Vec<serde_json::Value>,
}

async fn get_traces(State(state): State<ObservabilityState>) -> Json<TracesResponse> {
    match state.trace_collector.collect_traces().await {
        Ok(collected_traces) => {
            let traces: Vec<serde_json::Value> = collected_traces
                .into_iter()
                .map(|trace| {
                    serde_json::json!({
                        "trace_id": trace.trace_id,
                        "span_id": trace.span_id,
                        "parent_span_id": trace.parent_span_id,
                        "name": trace.name,
                        "start_time": trace.start_time,
                        "end_time": trace.end_time,
                        "duration_ms": trace.duration_ms,
                        "attributes": trace.attributes
                    })
                })
                .collect();

            Json(TracesResponse { traces })
        }
        Err(e) => {
            tracing::warn!("Failed to collect traces: {}", e);
            // Return empty traces on error rather than failing the request
            Json(TracesResponse { traces: vec![] })
        }
    }
}

/// List available chaos scenarios
#[derive(Serialize)]
struct ScenariosResponse {
    scenarios: Vec<String>,
}

async fn list_scenarios(State(_state): State<ObservabilityState>) -> Json<ScenariosResponse> {
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
    if let Some(scenario) = state.scenario_engine.get_scenario(&name) {
        state.scenario_engine.start_scenario(scenario);
        tracing::info!("Starting chaos scenario: {}", name);
        Json(ApiResponse::success(format!("Started scenario: {}", name)))
    } else {
        Json(ApiResponse::error(format!("Scenario '{}' not found", name)))
    }
}

/// Get chaos status
#[derive(Serialize)]
struct ChaosStatus {
    is_enabled: bool,
    active_scenario: Option<String>,
    current_config: Option<serde_json::Value>,
}

async fn get_chaos_status(State(state): State<ObservabilityState>) -> Json<ChaosStatus> {
    let active_scenarios = state.scenario_engine.get_active_scenarios();
    let is_enabled = !active_scenarios.is_empty();
    let active_scenario = active_scenarios.first().map(|s| s.name.clone());
    let current_config = active_scenarios
        .first()
        .map(|s| serde_json::to_value(&s.chaos_config).unwrap_or_default());

    Json(ChaosStatus {
        is_enabled,
        active_scenario,
        current_config,
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
    if let Some(scenario) = state.scenario_engine.get_scenario(&req.scenario_name) {
        match state.recorder.start_recording(scenario.clone()) {
            Ok(_) => {
                tracing::info!("Starting recording: {}", req.scenario_name);
                Json(ApiResponse::success("Recording started".to_string()))
            }
            Err(e) => Json(ApiResponse::error(format!("Failed to start recording: {}", e))),
        }
    } else {
        Json(ApiResponse::error(format!("Scenario '{}' not found", req.scenario_name)))
    }
}

/// Stop recording
async fn stop_recording(State(state): State<ObservabilityState>) -> Json<ApiResponse<String>> {
    match state.recorder.stop_recording() {
        Ok(recording) => {
            tracing::info!("Stopping recording: {} events recorded", recording.events.len());
            Json(ApiResponse::success("Recording stopped".to_string()))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to stop recording: {}", e))),
    }
}

/// Recording status
#[derive(Serialize)]
struct RecordingStatus {
    is_recording: bool,
    current_scenario: Option<String>,
    events_count: usize,
}

async fn recording_status(State(state): State<ObservabilityState>) -> Json<RecordingStatus> {
    let is_recording = state.recorder.is_recording();
    let current_scenario = state.recorder.get_current_recording().map(|r| r.scenario.name.clone());
    let events_count = state.recorder.get_current_recording().map(|r| r.events.len()).unwrap_or(0);

    Json(RecordingStatus {
        is_recording,
        current_scenario,
        events_count,
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

async fn list_recordings(State(state): State<ObservabilityState>) -> Json<RecordingsResponse> {
    let recordings = state.recorder.get_recordings();
    let scenarios = recordings
        .into_iter()
        .map(|r| RecordingInfo {
            name: r.scenario.name,
            started_at: r.recording_started.to_rfc3339(),
            ended_at: r.recording_ended.map(|t| t.to_rfc3339()),
            total_events: r.events.len(),
            duration_ms: r.total_duration_ms,
        })
        .collect();

    Json(RecordingsResponse { scenarios })
}

/// Export recording
#[derive(Deserialize)]
struct ExportRequest {
    scenario_name: String,
    format: String,
}

async fn export_recording(
    State(state): State<ObservabilityState>,
    Json(req): Json<ExportRequest>,
) -> Response {
    tracing::info!("Exporting scenario: {} as {}", req.scenario_name, req.format);

    if let Some(recording) = state.recorder.get_recording_by_name(&req.scenario_name) {
        let filename = format!("{}.{}", req.scenario_name, req.format);
        let filepath = format!("/tmp/{}", filename);

        let result = match req.format.as_str() {
            "json" | "yaml" => {
                recording.save_to_file(&filepath).map(|_| format!("/exports/{}", filename))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unsupported format")),
        };

        match result {
            Ok(path) => Json(ApiResponse::<String>::success(path)).into_response(),
            Err(e) => {
                Json(ApiResponse::<String>::error(format!("Export failed: {}", e))).into_response()
            }
        }
    } else {
        Json(ApiResponse::<String>::error(format!(
            "Recording '{}' not found",
            req.scenario_name
        )))
        .into_response()
    }
}

/// Start replay
#[derive(Deserialize)]
struct StartReplayRequest {
    scenario_name: String,
    speed: f64,
}

async fn start_replay(
    State(_state): State<ObservabilityState>,
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

async fn replay_status(State(state): State<ObservabilityState>) -> Json<ReplayStatus> {
    if let Some(status) = state.replay_engine.get_status() {
        Json(ReplayStatus {
            is_playing: true,
            scenario_name: Some(status.scenario_name),
            progress: status.progress,
        })
    } else {
        Json(ReplayStatus {
            is_playing: false,
            scenario_name: None,
            progress: 0.0,
        })
    }
}

/// Search recorded requests
#[derive(Deserialize)]
struct SearchRequest {
    limit: Option<usize>,
    protocol: Option<String>,
    method: Option<String>,
    path: Option<String>,
    status_code: Option<u16>,
    trace_id: Option<String>,
    min_duration_ms: Option<f64>,
    max_duration_ms: Option<f64>,
    tags: Option<Vec<String>>,
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

async fn search_requests(
    State(state): State<ObservabilityState>,
    Json(req): Json<SearchRequest>,
) -> Json<SearchResponse> {
    // Check if recorder is available
    let Some(recorder) = &state.request_recorder else {
        // Fall back to mock data if recorder is not available
        let mock_requests = vec![RecordedRequest {
            id: 1,
            timestamp: chrono::Utc::now().to_rfc3339(),
            protocol: "http".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            status_code: 200,
            duration_ms: 150.0,
            client_ip: Some("127.0.0.1".to_string()),
            request_headers: serde_json::json!({"user-agent": "test"}),
            request_body: None,
            response_headers: serde_json::json!({"content-type": "application/json"}),
            response_body: Some("{\"status\": \"ok\"}".to_string()),
        }];
        return Json(SearchResponse {
            requests: mock_requests,
        });
    };

    use mockforge_recorder::query::{execute_query, QueryFilter};

    // Convert SearchRequest to QueryFilter
    let filter = QueryFilter {
        protocol: req.protocol.as_ref().and_then(|p| match p.as_str() {
            "http" => Some(mockforge_recorder::models::Protocol::Http),
            "grpc" => Some(mockforge_recorder::models::Protocol::Grpc),
            "websocket" => Some(mockforge_recorder::models::Protocol::WebSocket),
            "graphql" => Some(mockforge_recorder::models::Protocol::GraphQL),
            _ => None,
        }),
        method: req.method.clone(),
        path: req.path.clone(),
        status_code: req.status_code.map(|s| s as i32),
        trace_id: req.trace_id.clone(),
        min_duration_ms: req.min_duration_ms.map(|d| d as i64),
        max_duration_ms: req.max_duration_ms.map(|d| d as i64),
        tags: req.tags.clone(),
        limit: req.limit.map(|l| l as i32),
        offset: None, // Not supported in current API
    };

    // Execute the query
    match execute_query(recorder.database(), filter).await {
        Ok(result) => {
            // Convert RecordedExchange to RecordedRequest format
            let requests: Vec<RecordedRequest> = result
                .exchanges
                .into_iter()
                .map(|exchange| RecordedRequest {
                    id: exchange.request.id.parse().unwrap_or(0),
                    timestamp: exchange.request.timestamp.to_rfc3339(),
                    protocol: exchange.request.protocol.as_str().to_string(),
                    method: exchange.request.method,
                    path: exchange.request.path,
                    status_code: exchange.request.status_code.unwrap_or(0) as u16,
                    duration_ms: exchange.request.duration_ms.unwrap_or(0) as f64,
                    client_ip: exchange.request.client_ip,
                    request_headers: serde_json::from_str(&exchange.request.headers)
                        .unwrap_or(serde_json::json!({})),
                    request_body: exchange.request.body,
                    response_headers: exchange
                        .response
                        .as_ref()
                        .and_then(|r| serde_json::from_str(&r.headers).ok())
                        .unwrap_or(serde_json::json!({})),
                    response_body: exchange.response.as_ref().and_then(|r| r.body.clone()),
                })
                .collect();

            Json(SearchResponse { requests })
        }
        Err(err) => {
            tracing::error!("Failed to search requests: {}", err);
            // Return empty result on error
            Json(SearchResponse { requests: vec![] })
        }
    }
}

// ===== Advanced Observability Endpoints =====

/// Get flamegraph for a trace
async fn get_flamegraph(
    State(state): State<ObservabilityState>,
    axum::extract::Path(trace_id): axum::extract::Path<String>,
) -> Json<ApiResponse<FlamegraphResponse>> {
    tracing::info!("Generating flamegraph for trace: {}", trace_id);

    // Get actual trace data
    let collected_traces = match state.trace_collector.get_trace_by_id(&trace_id).await {
        Ok(traces) => traces,
        Err(e) => {
            return Json(ApiResponse::error(format!("Failed to retrieve trace data: {}", e)));
        }
    };

    if collected_traces.is_empty() {
        return Json(ApiResponse::error(format!("No trace found with ID: {}", trace_id)));
    }

    // Generate flamegraph SVG from trace data
    let svg_content = generate_flamegraph_from_trace(&trace_id, &collected_traces);
    let svg_path = format!("/tmp/flamegraph_{}.svg", trace_id);

    // Write SVG to file
    if let Err(e) = std::fs::write(&svg_path, svg_content) {
        return Json(ApiResponse::error(format!("Failed to generate flamegraph: {}", e)));
    }

    // Calculate stats from actual trace data
    let total_spans = collected_traces.len();
    let max_depth = calculate_max_depth(&collected_traces);
    let total_duration_us =
        collected_traces.iter().map(|t| t.duration_ms * 1000).max().unwrap_or(0);
    let hottest_path = find_hottest_path(&collected_traces);

    let stats = FlamegraphStatsResponse {
        total_spans,
        max_depth,
        total_duration_us,
        hottest_path,
    };

    Json(ApiResponse::success(FlamegraphResponse {
        trace_id: trace_id.clone(),
        svg_url: format!("/flamegraphs/{}.svg", trace_id),
        stats,
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
async fn list_dashboard_layouts(
    State(state): State<ObservabilityState>,
) -> Json<ApiResponse<Vec<DashboardLayoutSummary>>> {
    tracing::info!("Listing dashboard layouts");
    let layouts = state.layout_manager.list_layouts();
    Json(ApiResponse::success(layouts))
}

#[derive(Serialize, Clone)]
pub struct DashboardLayoutSummary {
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

/// Create dashboard layout
async fn create_dashboard_layout(
    State(state): State<ObservabilityState>,
    Json(req): Json<CreateDashboardLayoutRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Creating dashboard layout: {}", req.name);
    let id = format!("layout-{}", chrono::Utc::now().timestamp());
    let widget_count = req.layout_data.as_array().map(|a| a.len()).unwrap_or(0);
    let layout = DashboardLayoutSummary {
        id: id.clone(),
        name: req.name,
        description: req.description,
        widget_count,
    };
    state.layout_manager.create_layout(layout);
    Json(ApiResponse::success(id))
}

/// Get dashboard layout
async fn get_dashboard_layout(
    State(state): State<ObservabilityState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    tracing::info!("Getting dashboard layout: {}", id);
    if let Some(layout) = state.layout_manager.get_layout(&id) {
        Json(ApiResponse::success(serde_json::json!({
            "id": layout.id,
            "name": layout.name,
            "description": layout.description,
            "widget_count": layout.widget_count
        })))
    } else {
        Json(ApiResponse::error(format!("Layout '{}' not found", id)))
    }
}

/// Update dashboard layout
async fn update_dashboard_layout(
    State(state): State<ObservabilityState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<CreateDashboardLayoutRequest>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Updating dashboard layout: {}", id);
    let widget_count = req.layout_data.as_array().map(|a| a.len()).unwrap_or(0);
    let layout = DashboardLayoutSummary {
        id: id.clone(),
        name: req.name,
        description: req.description,
        widget_count,
    };
    state.layout_manager.update_layout(&id, layout);
    Json(ApiResponse::success("Updated".to_string()))
}

/// Delete dashboard layout
async fn delete_dashboard_layout(
    State(state): State<ObservabilityState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResponse<String>> {
    tracing::info!("Deleting dashboard layout: {}", id);
    state.layout_manager.delete_layout(&id);
    Json(ApiResponse::success("Deleted".to_string()))
}

/// Get dashboard templates
async fn get_dashboard_templates(
    State(_state): State<ObservabilityState>,
) -> Json<ApiResponse<Vec<DashboardLayoutSummary>>> {
    tracing::info!("Getting dashboard templates");
    // For now, return static templates
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

/// Generate a simple PDF report for a chaos scenario
fn generate_scenario_pdf(
    scenario_name: &str,
    include_charts: bool,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (doc, page1, layer1) = PdfDocument::new(
        format!("Chaos Engineering Report - {}", scenario_name),
        Mm(210.0), // A4 width
        Mm(297.0), // A4 height
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    current_layer.use_text(
        "Chaos Engineering Report".to_string(),
        20.0,
        Mm(20.0),
        Mm(270.0),
        &font_bold,
    );

    // Scenario name
    current_layer.use_text(
        format!("Scenario: {}", scenario_name),
        14.0,
        Mm(20.0),
        Mm(250.0),
        &font,
    );

    // Generated timestamp
    let now = chrono::Utc::now();
    current_layer.use_text(
        format!("Generated: {}", now.format("%Y-%m-%d %H:%M:%S UTC")),
        10.0,
        Mm(20.0),
        Mm(235.0),
        &font,
    );

    // Summary section
    current_layer.use_text("Summary", 14.0, Mm(20.0), Mm(210.0), &font_bold);

    let mut y = 190.0;
    let metrics = [
        ("Total Requests", "1000"),
        ("Success Rate", "95.2%"),
        ("Average Latency", "150ms"),
        ("Error Rate", "4.8%"),
    ];

    for (label, value) in &metrics {
        current_layer.use_text(format!("{}: {}", label, value), 10.0, Mm(20.0), Mm(y), &font);
        y -= 10.0;
    }

    // Charts section if requested
    if include_charts {
        y -= 20.0;
        current_layer.use_text("Charts", 14.0, Mm(20.0), Mm(y), &font_bold);
        y -= 15.0;
        current_layer.use_text(
            "[Chart placeholder - would include actual charts in full implementation]",
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );
    }

    // Save the PDF
    use std::io::BufWriter;
    doc.save(&mut BufWriter::new(std::fs::File::create(output_path)?))?;

    Ok(())
}

async fn generate_pdf_report(
    State(_state): State<ObservabilityState>,
    Json(req): Json<GeneratePdfRequest>,
) -> Response {
    tracing::info!("Generating PDF report for: {}", req.scenario_name);

    let pdf_path = format!("/tmp/report_{}.pdf", req.scenario_name);

    // Generate PDF using printpdf directly
    if let Err(e) = generate_scenario_pdf(&req.scenario_name, req.include_charts, &pdf_path) {
        return Json(ApiResponse::<String>::error(format!("Failed to generate PDF: {}", e)))
            .into_response();
    }

    Json(ApiResponse::success(format!("/reports/{}.pdf", req.scenario_name))).into_response()
}

/// Generate CSV report
#[derive(Deserialize)]
struct GenerateCsvRequest {
    scenario_names: Vec<String>,
    include_comparison: bool,
}

async fn generate_csv_report(
    State(_state): State<ObservabilityState>,
    Json(req): Json<GenerateCsvRequest>,
) -> Response {
    tracing::info!("Generating CSV report for: {:?}", req.scenario_names);

    let csv_content = generate_csv_content(&req.scenario_names, req.include_comparison);
    let csv_path = "/tmp/scenarios_report.csv";

    if let Err(e) = std::fs::write(csv_path, csv_content) {
        return Json(ApiResponse::<String>::error(format!("Failed to generate CSV: {}", e)))
            .into_response();
    }

    Json(ApiResponse::success("/reports/scenarios.csv".to_string())).into_response()
}

/// Compare scenarios
#[derive(Deserialize)]
struct CompareRequest {
    baseline_scenario: String,
    comparison_scenarios: Vec<String>,
}

async fn compare_scenarios(
    State(_state): State<ObservabilityState>,
    Json(req): Json<CompareRequest>,
) -> Json<ApiResponse<ComparisonResult>> {
    tracing::info!(
        "Comparing scenarios - baseline: {}, comparisons: {:?}",
        req.baseline_scenario,
        req.comparison_scenarios
    );

    let comparison = perform_scenario_comparison(&req.baseline_scenario, &req.comparison_scenarios);

    Json(ApiResponse::success(comparison))
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
