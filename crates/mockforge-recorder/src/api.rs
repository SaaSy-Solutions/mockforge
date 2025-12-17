//! Management API for recorded requests

use crate::{
    diff::ComparisonResult,
    har_export::export_to_har,
    integration_testing::{IntegrationTestGenerator, IntegrationWorkflow, WorkflowSetup},
    models::RecordedExchange,
    query::{execute_query, QueryFilter, QueryResult},
    recorder::Recorder,
    replay::ReplayEngine,
    stub_mapping::{StubFormat, StubMappingConverter},
    sync::{SyncConfig, SyncService, SyncStatus},
    sync_snapshots::EndpointTimeline,
    test_generation::{LlmConfig, TestFormat, TestGenerationConfig, TestGenerator},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

/// API state
#[derive(Clone)]
pub struct ApiState {
    pub recorder: Arc<Recorder>,
    pub sync_service: Option<Arc<SyncService>>,
}

/// Create the management API router
pub fn create_api_router(
    recorder: Arc<Recorder>,
    sync_service: Option<Arc<SyncService>>,
) -> Router {
    let state = ApiState {
        recorder,
        sync_service,
    };

    Router::new()
        // Query endpoints
        .route("/api/recorder/requests", get(list_requests))
        .route("/api/recorder/requests/{id}", get(get_request))
        .route("/api/recorder/requests/{id}/response", get(get_response))
        .route("/api/recorder/search", post(search_requests))

        // Export endpoints
        .route("/api/recorder/export/har", get(export_har))

        // Control endpoints
        .route("/api/recorder/status", get(get_status))
        .route("/api/recorder/enable", post(enable_recording))
        .route("/api/recorder/disable", post(disable_recording))
        .route("/api/recorder/clear", delete(clear_recordings))

        // Replay endpoints
        .route("/api/recorder/replay/{id}", post(replay_request))
        .route("/api/recorder/compare/{id}", post(compare_responses))

        // Statistics endpoints
        .route("/api/recorder/stats", get(get_statistics))

        // Test generation endpoints
        .route("/api/recorder/generate-tests", post(generate_tests))

        // Integration testing endpoints
        .route("/api/recorder/workflows", post(create_workflow))
        .route("/api/recorder/workflows/{id}", get(get_workflow))
        .route("/api/recorder/workflows/{id}/generate", post(generate_integration_test))

        // Sync endpoints
        .route("/api/recorder/sync/status", get(get_sync_status))
        .route("/api/recorder/sync/config", get(get_sync_config))
        .route("/api/recorder/sync/config", post(update_sync_config))
        .route("/api/recorder/sync/now", post(sync_now))
        .route("/api/recorder/sync/changes", get(get_sync_changes))

        // Sync snapshot endpoints (Shadow Snapshot Mode)
        .route("/api/recorder/sync/snapshots", get(list_snapshots))
        .route("/api/recorder/sync/snapshots/{endpoint}", get(get_endpoint_timeline))
        .route("/api/recorder/sync/snapshots/cycle/{cycle_id}", get(get_snapshots_by_cycle))

        // Stub mapping conversion endpoints
        .route("/api/recorder/convert/{id}", post(convert_to_stub))
        .route("/api/recorder/convert/batch", post(convert_batch))

        .with_state(state)
}

/// List recent requests
async fn list_requests(
    State(state): State<ApiState>,
    Query(params): Query<ListParams>,
) -> Result<Json<QueryResult>, ApiError> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let filter = QueryFilter {
        limit: Some(limit),
        offset: Some(offset),
        ..Default::default()
    };

    let result = execute_query(state.recorder.database(), filter).await?;
    Ok(Json(result))
}

/// Get a single request by ID
async fn get_request(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<RecordedExchange>, ApiError> {
    let exchange = state
        .recorder
        .database()
        .get_exchange(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Request {} not found", id)))?;

    Ok(Json(exchange))
}

/// Get response for a request
async fn get_response(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let response = state
        .recorder
        .database()
        .get_response(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Response for request {} not found", id)))?;

    Ok(Json(serde_json::json!({
        "request_id": response.request_id,
        "status_code": response.status_code,
        "headers": serde_json::from_str::<serde_json::Value>(&response.headers)?,
        "body": response.body,
        "body_encoding": response.body_encoding,
        "size_bytes": response.size_bytes,
        "timestamp": response.timestamp,
    })))
}

/// Search requests with filters
async fn search_requests(
    State(state): State<ApiState>,
    Json(filter): Json<QueryFilter>,
) -> Result<Json<QueryResult>, ApiError> {
    let result = execute_query(state.recorder.database(), filter).await?;
    Ok(Json(result))
}

/// Export recordings to HAR format
async fn export_har(
    State(state): State<ApiState>,
    Query(params): Query<ExportParams>,
) -> Result<Response, ApiError> {
    let limit = params.limit.unwrap_or(1000);

    let filter = QueryFilter {
        limit: Some(limit),
        protocol: Some(crate::models::Protocol::Http), // HAR only supports HTTP
        ..Default::default()
    };

    let result = execute_query(state.recorder.database(), filter).await?;
    let har = export_to_har(&result.exchanges)?;
    let har_json = serde_json::to_string_pretty(&har)?;

    Ok((StatusCode::OK, [("content-type", "application/json")], har_json).into_response())
}

/// Get recording status
async fn get_status(State(state): State<ApiState>) -> Json<StatusResponse> {
    let enabled = state.recorder.is_enabled().await;
    Json(StatusResponse { enabled })
}

/// Enable recording
async fn enable_recording(State(state): State<ApiState>) -> Json<StatusResponse> {
    state.recorder.enable().await;
    debug!("Recording enabled via API");
    Json(StatusResponse { enabled: true })
}

/// Disable recording
async fn disable_recording(State(state): State<ApiState>) -> Json<StatusResponse> {
    state.recorder.disable().await;
    debug!("Recording disabled via API");
    Json(StatusResponse { enabled: false })
}

/// Clear all recordings
async fn clear_recordings(State(state): State<ApiState>) -> Result<Json<ClearResponse>, ApiError> {
    state.recorder.database().clear_all().await?;
    debug!("All recordings cleared via API");
    Ok(Json(ClearResponse {
        message: "All recordings cleared".to_string(),
    }))
}

/// Replay a single request
async fn replay_request(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let engine = ReplayEngine::new((**state.recorder.database()).clone());
    let result = engine.replay_request(&id).await?;

    Ok(Json(serde_json::json!({
        "request_id": result.request_id,
        "success": result.success,
        "message": result.message,
        "original_status": result.original_status,
        "replay_status": result.replay_status,
    })))
}

/// Compare original response with a replayed/new response
async fn compare_responses(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(payload): Json<CompareRequest>,
) -> Result<Json<ComparisonResult>, ApiError> {
    let engine = ReplayEngine::new((**state.recorder.database()).clone());

    let result = engine
        .compare_responses(&id, payload.body.as_bytes(), payload.status_code, &payload.headers)
        .await?;

    Ok(Json(result))
}

/// Get statistics about recordings
async fn get_statistics(
    State(state): State<ApiState>,
) -> Result<Json<StatisticsResponse>, ApiError> {
    let db = state.recorder.database();
    let stats = db.get_statistics().await?;

    Ok(Json(StatisticsResponse {
        total_requests: stats.total_requests,
        by_protocol: stats.by_protocol,
        by_status_code: stats.by_status_code,
        avg_duration_ms: stats.avg_duration_ms,
    }))
}

// Request/Response types

#[derive(Debug, Deserialize)]
struct ListParams {
    limit: Option<i32>,
    offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ExportParams {
    limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CompareRequest {
    status_code: i32,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ClearResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct StatisticsResponse {
    total_requests: i64,
    by_protocol: std::collections::HashMap<String, i64>,
    by_status_code: std::collections::HashMap<i32, i64>,
    avg_duration_ms: Option<f64>,
}

// Error handling

#[derive(Debug)]
enum ApiError {
    Database(sqlx::Error),
    Serialization(serde_json::Error),
    NotFound(String),
    InvalidInput(String),
    Recorder(crate::RecorderError),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Database(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Serialization(err)
    }
}

impl From<crate::RecorderError> for ApiError {
    fn from(err: crate::RecorderError) -> Self {
        ApiError::Recorder(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Database(e) => {
                error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
            }
            ApiError::Serialization(e) => {
                error!("Serialization error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", e))
            }
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Recorder(e) => {
                error!("Recorder error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Recorder error: {}", e))
            }
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

/// Test generation request
#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateTestsRequest {
    /// Test format to generate
    #[serde(default = "default_format")]
    pub format: String,

    /// Filter for query
    #[serde(flatten)]
    pub filter: QueryFilter,

    /// Test suite name
    #[serde(default = "default_suite_name")]
    pub suite_name: String,

    /// Base URL for tests
    pub base_url: Option<String>,

    /// Use AI for test descriptions
    #[serde(default)]
    pub ai_descriptions: bool,

    /// LLM configuration for AI descriptions
    pub llm_config: Option<LlmConfigRequest>,

    /// Include assertions
    #[serde(default = "default_true")]
    pub include_assertions: bool,

    /// Validate response body
    #[serde(default = "default_true")]
    pub validate_body: bool,

    /// Validate status code
    #[serde(default = "default_true")]
    pub validate_status: bool,

    /// Validate headers
    #[serde(default)]
    pub validate_headers: bool,

    /// Validate timing
    #[serde(default)]
    pub validate_timing: bool,

    /// Max duration threshold for timing validation
    pub max_duration_ms: Option<u64>,
}

fn default_format() -> String {
    "rust_reqwest".to_string()
}

fn default_suite_name() -> String {
    "generated_tests".to_string()
}

fn default_true() -> bool {
    true
}

/// LLM configuration request
#[derive(Debug, Serialize, Deserialize)]
pub struct LlmConfigRequest {
    /// LLM provider
    pub provider: String,
    /// API endpoint
    pub api_endpoint: String,
    /// API key
    pub api_key: Option<String>,
    /// Model name
    pub model: String,
    /// Temperature
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

fn default_temperature() -> f64 {
    0.3
}

/// Generate tests from recorded requests
async fn generate_tests(
    State(state): State<ApiState>,
    Json(request): Json<GenerateTestsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!("Generating tests with format: {}", request.format);

    // Parse test format
    let test_format = match request.format.as_str() {
        "rust_reqwest" => TestFormat::RustReqwest,
        "http_file" => TestFormat::HttpFile,
        "curl" => TestFormat::Curl,
        "postman" => TestFormat::Postman,
        "k6" => TestFormat::K6,
        "python_pytest" => TestFormat::PythonPytest,
        "javascript_jest" => TestFormat::JavaScriptJest,
        "go_test" => TestFormat::GoTest,
        _ => {
            return Err(ApiError::NotFound(format!(
                "Invalid test format: {}. Supported: rust_reqwest, http_file, curl, postman, k6, python_pytest, javascript_jest, go_test",
                request.format
            )));
        }
    };

    // Convert LLM config if provided
    let llm_config = request.llm_config.map(|cfg| LlmConfig {
        provider: cfg.provider,
        api_endpoint: cfg.api_endpoint,
        api_key: cfg.api_key,
        model: cfg.model,
        temperature: cfg.temperature,
    });

    // Create test generation config
    let config = TestGenerationConfig {
        format: test_format,
        include_assertions: request.include_assertions,
        validate_body: request.validate_body,
        validate_status: request.validate_status,
        validate_headers: request.validate_headers,
        validate_timing: request.validate_timing,
        max_duration_ms: request.max_duration_ms,
        suite_name: request.suite_name,
        base_url: request.base_url,
        ai_descriptions: request.ai_descriptions,
        llm_config,
        group_by_endpoint: true,
        include_setup_teardown: true,
        generate_fixtures: false,
        suggest_edge_cases: false,
        analyze_test_gaps: false,
        deduplicate_tests: false,
        optimize_test_order: false,
    };

    // Create test generator
    let generator = TestGenerator::from_arc(state.recorder.database().clone(), config);

    // Generate tests
    let result = generator.generate_from_filter(request.filter).await?;

    // Return result
    Ok(Json(serde_json::json!({
        "success": true,
        "metadata": {
            "suite_name": result.metadata.name,
            "test_count": result.metadata.test_count,
            "endpoint_count": result.metadata.endpoint_count,
            "protocols": result.metadata.protocols,
            "format": result.metadata.format,
            "generated_at": result.metadata.generated_at,
        },
        "tests": result.tests.iter().map(|t| serde_json::json!({
            "name": t.name,
            "description": t.description,
            "endpoint": t.endpoint,
            "method": t.method,
        })).collect::<Vec<_>>(),
        "test_file": result.test_file,
    })))
}

// Integration Testing Endpoints

/// Create workflow request
#[derive(Debug, Serialize, Deserialize)]
struct CreateWorkflowRequest {
    workflow: IntegrationWorkflow,
}

/// Create a new integration test workflow
async fn create_workflow(
    State(_state): State<ApiState>,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // For now, just return the workflow with success
    // In a full implementation, this would store in a database
    Ok(Json(serde_json::json!({
        "success": true,
        "workflow": request.workflow,
        "message": "Workflow created successfully"
    })))
}

/// Get workflow by ID
async fn get_workflow(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Mock workflow for demonstration
    // In a full implementation, this would fetch from database
    let workflow = IntegrationWorkflow {
        id: id.clone(),
        name: "Sample Workflow".to_string(),
        description: "A sample integration test workflow".to_string(),
        steps: vec![],
        setup: WorkflowSetup::default(),
        cleanup: vec![],
        created_at: chrono::Utc::now(),
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "workflow": workflow
    })))
}

/// Generate integration test request
#[derive(Debug, Deserialize)]
struct GenerateIntegrationTestRequest {
    workflow: IntegrationWorkflow,
    format: String, // "rust", "python", "javascript"
}

/// Generate integration test code from workflow
async fn generate_integration_test(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
    Json(request): Json<GenerateIntegrationTestRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let generator = IntegrationTestGenerator::new(request.workflow);

    let test_code = match request.format.as_str() {
        "rust" => generator.generate_rust_test(),
        "python" => generator.generate_python_test(),
        "javascript" | "js" => generator.generate_javascript_test(),
        _ => return Err(ApiError::InvalidInput(format!("Unsupported format: {}", request.format))),
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "format": request.format,
        "test_code": test_code,
        "message": "Integration test generated successfully"
    })))
}

// Sync endpoints

/// Get sync status
async fn get_sync_status(State(state): State<ApiState>) -> Result<Json<SyncStatus>, ApiError> {
    let sync_service = state
        .sync_service
        .ok_or_else(|| ApiError::NotFound("Sync service not available".to_string()))?;

    let status = sync_service.get_status().await;
    Ok(Json(status))
}

/// Get sync configuration
async fn get_sync_config(State(state): State<ApiState>) -> Result<Json<SyncConfig>, ApiError> {
    let sync_service = state
        .sync_service
        .ok_or_else(|| ApiError::NotFound("Sync service not available".to_string()))?;

    let config = sync_service.get_config().await;
    Ok(Json(config))
}

/// Update sync configuration
async fn update_sync_config(
    State(state): State<ApiState>,
    Json(config): Json<SyncConfig>,
) -> Result<Json<SyncConfig>, ApiError> {
    let sync_service = state
        .sync_service
        .ok_or_else(|| ApiError::NotFound("Sync service not available".to_string()))?;

    sync_service.update_config(config.clone()).await;
    Ok(Json(config))
}

/// Trigger sync now
async fn sync_now(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, ApiError> {
    let sync_service = state
        .sync_service
        .ok_or_else(|| ApiError::NotFound("Sync service not available".to_string()))?;

    match sync_service.sync_now().await {
        Ok((changes, updated)) => Ok(Json(serde_json::json!({
            "success": true,
            "changes_detected": changes.len(),
            "fixtures_updated": updated,
            "changes": changes,
            "message": format!("Sync complete: {} changes detected, {} fixtures updated", changes.len(), updated)
        }))),
        Err(e) => Err(ApiError::Recorder(e)),
    }
}

/// Get sync changes (from last sync)
async fn get_sync_changes(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sync_service = state
        .sync_service
        .ok_or_else(|| ApiError::NotFound("Sync service not available".to_string()))?;

    let status = sync_service.get_status().await;

    Ok(Json(serde_json::json!({
        "last_sync": status.last_sync,
        "last_changes_detected": status.last_changes_detected,
        "last_fixtures_updated": status.last_fixtures_updated,
        "last_error": status.last_error,
        "total_syncs": status.total_syncs,
        "is_running": status.is_running,
    })))
}

/// Convert a single recording to stub mapping
#[derive(Debug, Deserialize)]
struct ConvertRequest {
    format: Option<String>, // "yaml" or "json"
    detect_dynamic_values: Option<bool>,
}

async fn convert_to_stub(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(req): Json<ConvertRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let exchange = state
        .recorder
        .database()
        .get_exchange(&id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Request {} not found", id)))?;

    let detect_dynamic = req.detect_dynamic_values.unwrap_or(true);
    let converter = StubMappingConverter::new(detect_dynamic);
    let stub = converter.convert(&exchange)?;

    let format = match req.format.as_deref() {
        Some("json") => StubFormat::Json,
        Some("yaml") | None => StubFormat::Yaml,
        _ => StubFormat::Yaml,
    };

    let content = converter.to_string(&stub, format)?;

    Ok(Json(serde_json::json!({
        "request_id": id,
        "format": match format {
            StubFormat::Yaml => "yaml",
            StubFormat::Json => "json",
        },
        "stub": stub,
        "content": content,
    })))
}

/// Convert multiple recordings to stub mappings
#[derive(Debug, Deserialize)]
struct BatchConvertRequest {
    request_ids: Vec<String>,
    format: Option<String>,
    detect_dynamic_values: Option<bool>,
    deduplicate: Option<bool>,
}

async fn convert_batch(
    State(state): State<ApiState>,
    Json(req): Json<BatchConvertRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let detect_dynamic = req.detect_dynamic_values.unwrap_or(true);
    let converter = StubMappingConverter::new(detect_dynamic);

    let format = match req.format.as_deref() {
        Some("json") => StubFormat::Json,
        Some("yaml") | None => StubFormat::Yaml,
        _ => StubFormat::Yaml,
    };

    let mut stubs = Vec::new();
    let mut errors = Vec::new();

    for request_id in &req.request_ids {
        match state.recorder.database().get_exchange(request_id).await {
            Ok(Some(exchange)) => match converter.convert(&exchange) {
                Ok(stub) => {
                    let content = converter.to_string(&stub, format)?;
                    stubs.push(serde_json::json!({
                        "request_id": request_id,
                        "stub": stub,
                        "content": content,
                    }));
                }
                Err(e) => {
                    errors.push(format!("Failed to convert {}: {}", request_id, e));
                }
            },
            Ok(None) => {
                errors.push(format!("Request {} not found", request_id));
            }
            Err(e) => {
                errors.push(format!("Database error for {}: {}", request_id, e));
            }
        }
    }

    // Deduplicate if requested
    if req.deduplicate.unwrap_or(false) {
        // Simple deduplication based on identifier
        let mut seen = std::collections::HashSet::new();
        stubs.retain(|stub| {
            if let Some(id) = stub.get("stub").and_then(|s| s.get("identifier")) {
                if let Some(id_str) = id.as_str() {
                    return seen.insert(id_str.to_string());
                }
            }
            true
        });
    }

    Ok(Json(serde_json::json!({
        "total": req.request_ids.len(),
        "converted": stubs.len(),
        "errors": errors.len(),
        "stubs": stubs,
        "errors_list": errors,
    })))
}

/// List all snapshots
#[derive(Debug, Deserialize)]
struct SnapshotListParams {
    limit: Option<i32>,
    offset: Option<i32>,
}

async fn list_snapshots(
    State(state): State<ApiState>,
    Query(params): Query<SnapshotListParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit.unwrap_or(100);
    let database = state.recorder.database();

    // Get all unique endpoints to list snapshots
    // For simplicity, we'll get snapshots for all endpoints
    // In a real implementation, you might want to paginate differently
    let snapshots = database.get_snapshots_for_endpoint("", None, Some(limit)).await?;

    Ok(Json(serde_json::json!({
        "snapshots": snapshots,
        "total": snapshots.len(),
    })))
}

/// Get timeline for a specific endpoint
#[derive(Debug, Deserialize)]
struct TimelineParams {
    method: Option<String>,
    limit: Option<i32>,
}

async fn get_endpoint_timeline(
    State(state): State<ApiState>,
    Path(endpoint): Path<String>,
    Query(params): Query<TimelineParams>,
) -> Result<Json<EndpointTimeline>, ApiError> {
    let database = state.recorder.database();
    let limit = params.limit.unwrap_or(100);

    // Axum automatically URL-decodes path parameters
    let snapshots = database
        .get_snapshots_for_endpoint(&endpoint, params.method.as_deref(), Some(limit))
        .await?;

    // Build timeline data
    let mut response_time_trends = Vec::new();
    let mut status_code_history = Vec::new();
    let mut error_patterns = std::collections::HashMap::new();

    for snapshot in &snapshots {
        response_time_trends.push((
            snapshot.timestamp,
            snapshot.response_time_after.or(snapshot.response_time_before),
        ));
        status_code_history.push((snapshot.timestamp, snapshot.after.status_code));

        // Track error patterns
        if snapshot.after.status_code >= 400 {
            let key = format!("{}", snapshot.after.status_code);
            let pattern =
                error_patterns
                    .entry(key)
                    .or_insert_with(|| crate::sync_snapshots::ErrorPattern {
                        status_code: snapshot.after.status_code,
                        message_pattern: None,
                        occurrences: 0,
                        first_seen: snapshot.timestamp,
                        last_seen: snapshot.timestamp,
                    });
            pattern.occurrences += 1;
            if snapshot.timestamp < pattern.first_seen {
                pattern.first_seen = snapshot.timestamp;
            }
            if snapshot.timestamp > pattern.last_seen {
                pattern.last_seen = snapshot.timestamp;
            }
        }
    }

    let timeline = EndpointTimeline {
        endpoint,
        method: params.method.unwrap_or_else(|| "ALL".to_string()),
        snapshots,
        response_time_trends,
        status_code_history,
        error_patterns: error_patterns.into_values().collect(),
    };

    Ok(Json(timeline))
}

/// Get snapshots by sync cycle ID
async fn get_snapshots_by_cycle(
    State(state): State<ApiState>,
    Path(cycle_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let database = state.recorder.database();

    let snapshots = database.get_snapshots_by_cycle(&cycle_id).await?;

    Ok(Json(serde_json::json!({
        "sync_cycle_id": cycle_id,
        "snapshots": snapshots,
        "total": snapshots.len(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;
    use crate::models::{Protocol, RecordedRequest, RecordedResponse};
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HttpStatusCode};
    use tower::ServiceExt;

    async fn create_test_db() -> RecorderDatabase {
        RecorderDatabase::new_in_memory().await.unwrap()
    }

    async fn create_test_recorder() -> Arc<Recorder> {
        let db = create_test_db().await;
        Arc::new(Recorder::new(db))
    }

    #[tokio::test]
    async fn test_api_state_creation() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        assert!(state.sync_service.is_none());
    }

    #[tokio::test]
    async fn test_create_api_router() {
        let recorder = create_test_recorder().await;
        let router = create_api_router(recorder, None);

        // Router should be created successfully
        assert!(std::mem::size_of_val(&router) > 0);
    }

    #[tokio::test]
    async fn test_get_status_enabled() {
        let recorder = create_test_recorder().await;
        recorder.enable().await;

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let response = get_status(State(state)).await;
        assert!(response.0.enabled);
    }

    #[tokio::test]
    async fn test_get_status_disabled() {
        let recorder = create_test_recorder().await;
        recorder.disable().await;

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let response = get_status(State(state)).await;
        assert!(!response.0.enabled);
    }

    #[tokio::test]
    async fn test_enable_recording() {
        let recorder = create_test_recorder().await;
        recorder.disable().await;

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let response = enable_recording(State(state)).await;
        assert!(response.0.enabled);
        assert!(recorder.is_enabled().await);
    }

    #[tokio::test]
    async fn test_disable_recording() {
        let recorder = create_test_recorder().await;
        recorder.enable().await;

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let response = disable_recording(State(state)).await;
        assert!(!response.0.enabled);
        assert!(!recorder.is_enabled().await);
    }

    #[tokio::test]
    async fn test_clear_recordings() {
        let recorder = create_test_recorder().await;

        // Add a test request
        let request = RecordedRequest {
            id: "test-1".to_string(),
            protocol: Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            status_code: Some(200),
            tags: None,
        };
        recorder.database().insert_request(&request).await.unwrap();

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let response = clear_recordings(State(state)).await.unwrap();
        assert_eq!(response.0.message, "All recordings cleared");
    }

    #[test]
    fn test_api_error_from_sqlx() {
        let err = sqlx::Error::RowNotFound;
        let api_err = ApiError::from(err);

        match api_err {
            ApiError::Database(_) => {}
            _ => panic!("Expected Database error"),
        }
    }

    #[test]
    fn test_api_error_from_serde() {
        let err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let api_err = ApiError::from(err);

        match api_err {
            ApiError::Serialization(_) => {}
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_api_error_into_response_not_found() {
        let err = ApiError::NotFound("Test not found".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), HttpStatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_error_into_response_invalid_input() {
        let err = ApiError::InvalidInput("Invalid data".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), HttpStatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_list_params_defaults() {
        let params = ListParams {
            limit: None,
            offset: None,
        };

        assert!(params.limit.is_none());
        assert!(params.offset.is_none());
    }

    #[test]
    fn test_export_params_defaults() {
        let params = ExportParams { limit: None };
        assert!(params.limit.is_none());
    }

    #[test]
    fn test_compare_request_creation() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        let req = CompareRequest {
            status_code: 200,
            headers,
            body: "test body".to_string(),
        };

        assert_eq!(req.status_code, 200);
        assert_eq!(req.body, "test body");
        assert_eq!(req.headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_status_response_serialization() {
        let response = StatusResponse { enabled: true };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("enabled"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_clear_response_serialization() {
        let response = ClearResponse {
            message: "All cleared".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("All cleared"));
    }

    #[test]
    fn test_statistics_response_creation() {
        let mut by_protocol = std::collections::HashMap::new();
        by_protocol.insert("http".to_string(), 100);

        let mut by_status_code = std::collections::HashMap::new();
        by_status_code.insert(200, 80);
        by_status_code.insert(404, 20);

        let response = StatisticsResponse {
            total_requests: 100,
            by_protocol,
            by_status_code,
            avg_duration_ms: Some(150.5),
        };

        assert_eq!(response.total_requests, 100);
        assert_eq!(response.by_protocol.get("http").unwrap(), &100);
        assert_eq!(response.by_status_code.get(&200).unwrap(), &80);
        assert_eq!(response.avg_duration_ms, Some(150.5));
    }

    #[test]
    fn test_default_format() {
        assert_eq!(default_format(), "rust_reqwest");
    }

    #[test]
    fn test_default_suite_name() {
        assert_eq!(default_suite_name(), "generated_tests");
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_default_temperature() {
        assert_eq!(default_temperature(), 0.3);
    }

    #[test]
    fn test_generate_tests_request_defaults() {
        let request = GenerateTestsRequest {
            format: default_format(),
            filter: QueryFilter::default(),
            suite_name: default_suite_name(),
            base_url: None,
            ai_descriptions: false,
            llm_config: None,
            include_assertions: default_true(),
            validate_body: default_true(),
            validate_status: default_true(),
            validate_headers: false,
            validate_timing: false,
            max_duration_ms: None,
        };

        assert_eq!(request.format, "rust_reqwest");
        assert_eq!(request.suite_name, "generated_tests");
        assert!(request.include_assertions);
        assert!(request.validate_body);
        assert!(request.validate_status);
        assert!(!request.validate_headers);
    }

    #[test]
    fn test_llm_config_request_creation() {
        let config = LlmConfigRequest {
            provider: "openai".to_string(),
            api_endpoint: "https://api.openai.com".to_string(),
            api_key: Some("secret".to_string()),
            model: "gpt-4".to_string(),
            temperature: default_temperature(),
        };

        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.3);
    }

    #[test]
    fn test_create_workflow_request_serialization() {
        let workflow = IntegrationWorkflow {
            id: "wf-1".to_string(),
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            steps: vec![],
            setup: WorkflowSetup::default(),
            cleanup: vec![],
            created_at: chrono::Utc::now(),
        };

        let request = CreateWorkflowRequest { workflow };
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("Test Workflow"));
    }

    #[test]
    fn test_generate_integration_test_request_creation() {
        let workflow = IntegrationWorkflow {
            id: "wf-1".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            steps: vec![],
            setup: WorkflowSetup::default(),
            cleanup: vec![],
            created_at: chrono::Utc::now(),
        };

        let request = GenerateIntegrationTestRequest {
            workflow,
            format: "rust".to_string(),
        };

        assert_eq!(request.format, "rust");
    }

    #[test]
    fn test_convert_request_defaults() {
        let req = ConvertRequest {
            format: None,
            detect_dynamic_values: None,
        };

        assert!(req.format.is_none());
        assert!(req.detect_dynamic_values.is_none());
    }

    #[test]
    fn test_batch_convert_request_creation() {
        let request = BatchConvertRequest {
            request_ids: vec!["req-1".to_string(), "req-2".to_string()],
            format: Some("json".to_string()),
            detect_dynamic_values: Some(true),
            deduplicate: Some(false),
        };

        assert_eq!(request.request_ids.len(), 2);
        assert_eq!(request.format, Some("json".to_string()));
        assert_eq!(request.detect_dynamic_values, Some(true));
        assert_eq!(request.deduplicate, Some(false));
    }

    #[test]
    fn test_snapshot_list_params() {
        let params = SnapshotListParams {
            limit: Some(50),
            offset: Some(10),
        };

        assert_eq!(params.limit, Some(50));
        assert_eq!(params.offset, Some(10));
    }

    #[test]
    fn test_timeline_params() {
        let params = TimelineParams {
            method: Some("GET".to_string()),
            limit: Some(100),
        };

        assert_eq!(params.method, Some("GET".to_string()));
        assert_eq!(params.limit, Some(100));
    }

    #[tokio::test]
    async fn test_get_request_not_found() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let result = get_request(State(state), Path("non-existent".to_string())).await;

        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_response_not_found() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let result = get_response(State(state), Path("non-existent".to_string())).await;

        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_sync_status_no_service() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let result = get_sync_status(State(state)).await;

        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_sync_config_no_service() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let result = get_sync_config(State(state)).await;

        assert!(result.is_err());
        match result {
            Err(ApiError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_search_requests_empty() {
        let recorder = create_test_recorder().await;
        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let filter = QueryFilter::default();
        let result = search_requests(State(state), Json(filter)).await.unwrap();

        assert_eq!(result.0.total, 0);
        assert!(result.0.exchanges.is_empty());
    }

    #[tokio::test]
    async fn test_list_requests_with_params() {
        let recorder = create_test_recorder().await;

        // Add some test requests
        for i in 0..5 {
            let request = RecordedRequest {
                id: format!("req-{}", i),
                protocol: Protocol::Http,
                timestamp: chrono::Utc::now(),
                method: "GET".to_string(),
                path: "/test".to_string(),
                query_params: None,
                headers: "{}".to_string(),
                body: None,
                body_encoding: "utf8".to_string(),
                client_ip: None,
                trace_id: None,
                span_id: None,
                duration_ms: None,
                status_code: Some(200),
                tags: None,
            };
            recorder.database().insert_request(&request).await.unwrap();
        }

        let state = ApiState {
            recorder: recorder.clone(),
            sync_service: None,
        };

        let params = ListParams {
            limit: Some(3),
            offset: Some(0),
        };

        let result = list_requests(State(state), Query(params)).await.unwrap();

        assert_eq!(result.0.exchanges.len(), 3);
    }

    #[test]
    fn test_generate_tests_request_serialization() {
        let request = GenerateTestsRequest {
            format: "rust_reqwest".to_string(),
            filter: QueryFilter::default(),
            suite_name: "my_tests".to_string(),
            base_url: Some("http://localhost:8080".to_string()),
            ai_descriptions: true,
            llm_config: None,
            include_assertions: true,
            validate_body: true,
            validate_status: true,
            validate_headers: true,
            validate_timing: false,
            max_duration_ms: Some(1000),
        };

        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("rust_reqwest"));
        assert!(json.contains("my_tests"));
        assert!(json.contains("http://localhost:8080"));
    }
}
