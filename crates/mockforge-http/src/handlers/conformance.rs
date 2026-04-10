//! Conformance testing API handlers
//!
//! Provides REST endpoints for starting, monitoring, and retrieving
//! OpenAPI 3.0.0 conformance test runs via the native Rust executor.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Json,
    },
    routing::{delete, get, post},
    Router,
};
use dashmap::DashMap;
use futures::stream::{self, Stream};
use mockforge_bench::conformance::{
    ConformanceConfig, ConformanceProgress, NativeConformanceExecutor,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

/// Conformance run status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Run is queued
    Pending,
    /// Run is in progress
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed
    Failed,
}

/// A conformance test run
#[derive(Debug, Clone, Serialize)]
pub struct ConformanceRun {
    /// Unique run ID
    pub id: Uuid,
    /// Current status
    pub status: RunStatus,
    /// Configuration used
    pub config: ConformanceRunRequest,
    /// Report (available when completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<serde_json::Value>,
    /// Error message (available when failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Number of checks completed so far
    pub checks_done: usize,
    /// Total number of checks
    pub total_checks: usize,
}

/// Request body for starting a conformance run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConformanceRunRequest {
    /// Target URL to test against
    pub target_url: String,
    /// Inline OpenAPI spec JSON/YAML (optional)
    #[serde(default)]
    pub spec: Option<String>,
    /// Categories to test (optional filter)
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    /// Custom request headers
    #[serde(default)]
    pub custom_headers: Option<Vec<(String, String)>>,
    /// API key for security tests
    #[serde(default)]
    pub api_key: Option<String>,
    /// Basic auth credentials (user:pass)
    #[serde(default)]
    pub basic_auth: Option<String>,
    /// Skip TLS verification
    #[serde(default)]
    pub skip_tls_verify: Option<bool>,
    /// API base path prefix
    #[serde(default)]
    pub base_path: Option<String>,
    /// Test all operations (not just representative samples)
    #[serde(default)]
    pub all_operations: Option<bool>,
    /// Delay in milliseconds between consecutive requests
    #[serde(default)]
    pub request_delay_ms: Option<u64>,
    /// Inline YAML custom checks
    #[serde(default)]
    pub custom_checks_yaml: Option<String>,
}

/// Shared state for conformance handlers
#[derive(Clone)]
pub struct ConformanceState {
    /// Active and completed runs
    pub runs: Arc<DashMap<Uuid, ConformanceRun>>,
    /// Broadcast channels for progress events
    pub progress_channels: Arc<DashMap<Uuid, broadcast::Sender<ConformanceProgress>>>,
}

impl ConformanceState {
    /// Create new conformance state
    pub fn new() -> Self {
        Self {
            runs: Arc::new(DashMap::new()),
            progress_channels: Arc::new(DashMap::new()),
        }
    }
}

impl Default for ConformanceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the conformance router
pub fn conformance_router(state: ConformanceState) -> Router {
    Router::new()
        .route("/run", post(start_run))
        .route("/run/{id}", get(get_run))
        .route("/run/{id}", delete(delete_run))
        .route("/run/{id}/stream", get(stream_progress))
        .route("/runs", get(list_runs))
        .with_state(state)
}

/// POST /api/conformance/run — Start a new conformance test run
async fn start_run(
    State(state): State<ConformanceState>,
    Json(req): Json<ConformanceRunRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if req.target_url.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4();

    // Create broadcast channel for progress
    let (tx, _) = broadcast::channel(256);
    state.progress_channels.insert(id, tx.clone());

    // Create the run record
    let run = ConformanceRun {
        id,
        status: RunStatus::Pending,
        config: req.clone(),
        report: None,
        error: None,
        checks_done: 0,
        total_checks: 0,
    };
    state.runs.insert(id, run);

    // Spawn the execution task
    let runs = state.runs.clone();
    let channels = state.progress_channels.clone();

    tokio::spawn(async move {
        // Build ConformanceConfig from request
        let config = ConformanceConfig {
            target_url: req.target_url.clone(),
            api_key: req.api_key.clone(),
            basic_auth: req.basic_auth.clone(),
            skip_tls_verify: req.skip_tls_verify.unwrap_or(false),
            categories: req.categories.clone(),
            base_path: req.base_path.clone(),
            custom_headers: req.custom_headers.clone().unwrap_or_default(),
            output_dir: None,
            all_operations: req.all_operations.unwrap_or(false),
            custom_checks_file: None,
            request_delay_ms: req.request_delay_ms.unwrap_or(0),
            custom_filter: None,
            export_requests: false,
        };

        // Build executor
        let executor = match NativeConformanceExecutor::new(config) {
            Ok(e) => e,
            Err(e) => {
                if let Some(mut run) = runs.get_mut(&id) {
                    run.status = RunStatus::Failed;
                    run.error = Some(format!("Failed to create executor: {}", e));
                }
                let _ = tx.send(ConformanceProgress::Error {
                    message: e.to_string(),
                });
                return;
            }
        };

        // Build checks (reference mode for now, spec-driven requires parsing inline spec)
        let executor = executor.with_reference_checks();

        // Update run status
        if let Some(mut run) = runs.get_mut(&id) {
            run.status = RunStatus::Running;
            run.total_checks = executor.check_count();
        }

        // Create progress channel
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(256);

        // Forward progress to broadcast
        let broadcast_tx = tx.clone();
        let runs_for_progress = runs.clone();
        tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                // Update checks_done in the run record
                if let ConformanceProgress::CheckCompleted { checks_done, .. } = &progress {
                    if let Some(mut run) = runs_for_progress.get_mut(&id) {
                        run.checks_done = *checks_done;
                    }
                }
                let _ = broadcast_tx.send(progress);
            }
        });

        // Execute
        match executor.execute_with_progress(progress_tx).await {
            Ok(report) => {
                let report_json = report.to_json();
                if let Some(mut run) = runs.get_mut(&id) {
                    run.status = RunStatus::Completed;
                    run.report = Some(report_json);
                    run.checks_done = run.total_checks;
                }
                info!("Conformance run {} completed", id);
            }
            Err(e) => {
                if let Some(mut run) = runs.get_mut(&id) {
                    run.status = RunStatus::Failed;
                    run.error = Some(format!("{}", e));
                }
                error!("Conformance run {} failed: {}", id, e);
            }
        }

        // Clean up progress channel after a delay
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        channels.remove(&id);
    });

    Ok(Json(serde_json::json!({ "id": id })))
}

/// GET /api/conformance/run/{id} — Get run status and results
async fn get_run(
    State(state): State<ConformanceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConformanceRun>, StatusCode> {
    state.runs.get(&id).map(|run| Json(run.clone())).ok_or(StatusCode::NOT_FOUND)
}

/// GET /api/conformance/runs — List recent runs
async fn list_runs(State(state): State<ConformanceState>) -> Json<Vec<serde_json::Value>> {
    let runs: Vec<serde_json::Value> = state
        .runs
        .iter()
        .map(|entry| {
            let run = entry.value();
            serde_json::json!({
                "id": run.id,
                "status": run.status,
                "checks_done": run.checks_done,
                "total_checks": run.total_checks,
                "target_url": run.config.target_url,
            })
        })
        .collect();
    Json(runs)
}

/// DELETE /api/conformance/run/{id} — Delete a completed run
async fn delete_run(State(state): State<ConformanceState>, Path(id): Path<Uuid>) -> StatusCode {
    if let Some((_, run)) = state.runs.remove(&id) {
        if run.status == RunStatus::Running {
            // Re-insert if still running
            state.runs.insert(id, run);
            return StatusCode::CONFLICT;
        }
        state.progress_channels.remove(&id);
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// GET /api/conformance/run/{id}/stream — SSE stream for live progress
async fn stream_progress(
    State(state): State<ConformanceState>,
    Path(id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let rx = state
        .progress_channels
        .get(&id)
        .map(|entry| entry.subscribe())
        .ok_or(StatusCode::NOT_FOUND)?;

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(progress) => {
                let data = serde_json::to_string(&progress).unwrap_or_default();
                let event = Event::default().event("conformance_progress").data(data);
                Some((Ok(event), rx))
            }
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                let event = Event::default().event("conformance_progress").data(format!(
                    r#"{{"type":"error","message":"lagged, skipped {} events"}}"#,
                    skipped
                ));
                Some((Ok(event), rx))
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
