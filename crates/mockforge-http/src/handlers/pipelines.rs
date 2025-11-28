//! Pipeline management API handlers
//!
//! This module provides HTTP handlers for managing MockOps pipelines,
//! including CRUD operations and execution monitoring.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_pipelines::{
    pipeline::{Pipeline, PipelineDefinition, PipelineExecution, PipelineExecutor},
    PipelineEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// State for pipeline handlers
#[derive(Clone)]
pub struct PipelineState {
    /// Pipeline executor
    pub executor: Arc<PipelineExecutor>,
    /// Pipeline storage (in-memory for now, can be extended to database)
    pub storage: Arc<dashmap::DashMap<Uuid, Pipeline>>,
    /// Execution history (in-memory for now)
    pub executions: Arc<dashmap::DashMap<Uuid, PipelineExecution>>,
}

impl PipelineState {
    /// Create a new pipeline state
    pub fn new() -> Self {
        Self {
            executor: Arc::new(PipelineExecutor::new()),
            storage: Arc::new(dashmap::DashMap::new()),
            executions: Arc::new(dashmap::DashMap::new()),
        }
    }
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Request to create a pipeline
#[derive(Debug, Deserialize)]
pub struct CreatePipelineRequest {
    /// Pipeline name
    pub name: String,
    /// Pipeline definition
    pub definition: PipelineDefinition,
    /// Optional workspace ID
    pub workspace_id: Option<Uuid>,
    /// Optional organization ID
    pub org_id: Option<Uuid>,
}

/// Request to update a pipeline
#[derive(Debug, Deserialize)]
pub struct UpdatePipelineRequest {
    /// Pipeline name (optional)
    pub name: Option<String>,
    /// Pipeline definition (optional)
    pub definition: Option<PipelineDefinition>,
    /// Whether pipeline is enabled (optional)
    pub enabled: Option<bool>,
}

/// Query parameters for listing pipelines
#[derive(Debug, Deserialize)]
pub struct ListPipelinesQuery {
    /// Filter by workspace ID
    pub workspace_id: Option<Uuid>,
    /// Filter by organization ID
    pub org_id: Option<Uuid>,
    /// Filter by enabled status
    pub enabled: Option<bool>,
}

/// Query parameters for listing executions
#[derive(Debug, Deserialize)]
pub struct ListExecutionsQuery {
    /// Filter by pipeline ID
    pub pipeline_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Create a new pipeline
///
/// POST /api/v1/pipelines
pub async fn create_pipeline(
    State(state): State<PipelineState>,
    Json(request): Json<CreatePipelineRequest>,
) -> Result<Json<Pipeline>, StatusCode> {
    info!("Creating pipeline: {}", request.name);

    let pipeline =
        Pipeline::new(request.name, request.definition, request.workspace_id, request.org_id);

    state.storage.insert(pipeline.id, pipeline.clone());

    info!("Pipeline created: {}", pipeline.id);
    Ok(Json(pipeline))
}

/// List pipelines
///
/// GET /api/v1/pipelines
pub async fn list_pipelines(
    State(state): State<PipelineState>,
    Query(params): Query<ListPipelinesQuery>,
) -> Result<Json<Vec<Pipeline>>, StatusCode> {
    let mut pipelines: Vec<Pipeline> =
        state.storage.iter().map(|entry| entry.value().clone()).collect();

    // Apply filters
    if let Some(workspace_id) = params.workspace_id {
        pipelines.retain(|p| p.workspace_id == Some(workspace_id));
    }
    if let Some(org_id) = params.org_id {
        pipelines.retain(|p| p.org_id == Some(org_id));
    }
    if let Some(enabled) = params.enabled {
        pipelines.retain(|p| p.definition.enabled == enabled);
    }

    pipelines.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(pipelines))
}

/// Get a specific pipeline
///
/// GET /api/v1/pipelines/{id}
pub async fn get_pipeline(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> Result<Json<Pipeline>, StatusCode> {
    let pipeline_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let pipeline = state
        .storage
        .get(&pipeline_id)
        .map(|entry| entry.value().clone())
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(pipeline))
}

/// Update a pipeline
///
/// PATCH /api/v1/pipelines/{id}
pub async fn update_pipeline(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePipelineRequest>,
) -> Result<Json<Pipeline>, StatusCode> {
    let pipeline_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut pipeline = state.storage.get_mut(&pipeline_id).ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = request.name {
        pipeline.name = name;
    }
    if let Some(definition) = request.definition {
        pipeline.definition = definition;
    }
    if let Some(enabled) = request.enabled {
        pipeline.definition.enabled = enabled;
    }
    pipeline.updated_at = chrono::Utc::now();

    info!("Pipeline updated: {}", pipeline_id);
    Ok(Json(pipeline.clone()))
}

/// Delete a pipeline
///
/// DELETE /api/v1/pipelines/{id}
pub async fn delete_pipeline(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let pipeline_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    if state.storage.remove(&pipeline_id).is_some() {
        info!("Pipeline deleted: {}", pipeline_id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Trigger a pipeline manually
///
/// POST /api/v1/pipelines/{id}/trigger
pub async fn trigger_pipeline(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(event): Json<PipelineEvent>,
) -> Result<Json<PipelineExecution>, StatusCode> {
    let pipeline_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let pipeline = state
        .storage
        .get(&pipeline_id)
        .map(|entry| entry.value().clone())
        .ok_or(StatusCode::NOT_FOUND)?;

    if !pipeline.definition.enabled {
        return Err(StatusCode::BAD_REQUEST);
    }

    info!("Manually triggering pipeline: {}", pipeline_id);

    // Execute pipeline
    match state.executor.execute(&pipeline, event.clone()).await {
        Ok(execution) => {
            state.executions.insert(execution.id, execution.clone());
            Ok(Json(execution))
        }
        Err(e) => {
            error!("Pipeline execution failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List pipeline executions
///
/// GET /api/v1/pipelines/executions
pub async fn list_executions(
    State(state): State<PipelineState>,
    Query(params): Query<ListExecutionsQuery>,
) -> Result<Json<Vec<PipelineExecution>>, StatusCode> {
    let mut executions: Vec<PipelineExecution> =
        state.executions.iter().map(|entry| entry.value().clone()).collect();

    // Apply filters
    if let Some(pipeline_id) = params.pipeline_id {
        executions.retain(|e| e.pipeline_id == pipeline_id);
    }
    if let Some(status_str) = params.status {
        executions
            .retain(|e| format!("{:?}", e.status).to_lowercase() == status_str.to_lowercase());
    }

    // Sort by started_at (most recent first)
    executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    // Apply pagination
    let offset = params.offset.unwrap_or(0) as usize;
    let limit = params.limit.unwrap_or(100) as usize;
    let end = (offset + limit).min(executions.len());
    let paginated = executions.into_iter().skip(offset).take(end - offset).collect();

    Ok(Json(paginated))
}

/// Get a specific execution
///
/// GET /api/v1/pipelines/executions/{id}
pub async fn get_execution(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> Result<Json<PipelineExecution>, StatusCode> {
    let execution_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let execution = state
        .executions
        .get(&execution_id)
        .map(|entry| entry.value().clone())
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(execution))
}

/// Get pipeline statistics
///
/// GET /api/v1/pipelines/{id}/stats
pub async fn get_pipeline_stats(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pipeline_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get all executions for this pipeline
    let executions: Vec<PipelineExecution> = state
        .executions
        .iter()
        .filter(|entry| entry.value().pipeline_id == pipeline_id)
        .map(|entry| entry.value().clone())
        .collect();

    let total = executions.len();
    let completed = executions
        .iter()
        .filter(|e| {
            matches!(e.status, mockforge_pipelines::pipeline::PipelineExecutionStatus::Completed)
        })
        .count();
    let failed = executions
        .iter()
        .filter(|e| {
            matches!(e.status, mockforge_pipelines::pipeline::PipelineExecutionStatus::Failed)
        })
        .count();
    let running = executions
        .iter()
        .filter(|e| {
            matches!(e.status, mockforge_pipelines::pipeline::PipelineExecutionStatus::Running)
        })
        .count();

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "total_executions": total,
        "completed": completed,
        "failed": failed,
        "running": running,
        "success_rate": if total > 0 { (completed as f64 / total as f64) * 100.0 } else { 0.0 },
    })))
}

/// Create pipeline router
pub fn pipeline_router(state: PipelineState) -> axum::Router {
    use axum::routing::{delete, get, patch, post};

    axum::Router::new()
        // Pipeline CRUD
        .route("/api/v1/pipelines", post(create_pipeline))
        .route("/api/v1/pipelines", get(list_pipelines))
        .route("/api/v1/pipelines/{id}", get(get_pipeline))
        .route("/api/v1/pipelines/{id}", patch(update_pipeline))
        .route("/api/v1/pipelines/{id}", delete(delete_pipeline))
        .route("/api/v1/pipelines/{id}/trigger", post(trigger_pipeline))
        .route("/api/v1/pipelines/{id}/stats", get(get_pipeline_stats))
        // Execution management
        .route("/api/v1/pipelines/executions", get(list_executions))
        .route("/api/v1/pipelines/executions/{id}", get(get_execution))
        .with_state(state)
}
