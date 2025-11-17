//! Snapshot management API handlers
//!
//! This module provides HTTP handlers for managing snapshots via REST API.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::consistency::ConsistencyEngine;
use mockforge_core::snapshots::{SnapshotComponents, SnapshotManager, SnapshotMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info};

/// State for snapshot handlers
#[derive(Clone)]
pub struct SnapshotState {
    /// Snapshot manager
    pub manager: Arc<SnapshotManager>,
    /// Consistency engine (optional, for saving/loading unified state)
    pub consistency_engine: Option<Arc<ConsistencyEngine>>,
}

/// Request to save a snapshot
#[derive(Debug, Deserialize)]
pub struct SaveSnapshotRequest {
    /// Snapshot name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Components to include
    pub components: Option<SnapshotComponents>,
}

/// Request to load a snapshot
#[derive(Debug, Deserialize)]
pub struct LoadSnapshotRequest {
    /// Components to restore (optional, defaults to all)
    pub components: Option<SnapshotComponents>,
}

/// Query parameters for snapshot operations
#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    /// Workspace ID (defaults to "default" if not provided)
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_workspace() -> String {
    "default".to_string()
}

/// Save a snapshot
///
/// POST /api/v1/snapshots?workspace={workspace_id}
pub async fn save_snapshot(
    State(state): State<SnapshotState>,
    Query(params): Query<SnapshotQuery>,
    Json(request): Json<SaveSnapshotRequest>,
) -> Result<Json<Value>, StatusCode> {
    let components = request.components.unwrap_or_else(SnapshotComponents::all);

    let consistency_engine = state.consistency_engine.as_deref();
    let manifest = state
        .manager
        .save_snapshot(
            request.name.clone(),
            request.description,
            params.workspace.clone(),
            components,
            consistency_engine,
        )
        .await
        .map_err(|e| {
            error!("Failed to save snapshot: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Saved snapshot '{}' for workspace '{}'", request.name, params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "manifest": manifest,
    })))
}

/// Load a snapshot
///
/// POST /api/v1/snapshots/{name}/load?workspace={workspace_id}
pub async fn load_snapshot(
    State(state): State<SnapshotState>,
    Path(name): Path<String>,
    Query(params): Query<SnapshotQuery>,
    Json(request): Json<LoadSnapshotRequest>,
) -> Result<Json<Value>, StatusCode> {
    let consistency_engine = state.consistency_engine.as_deref();
    let manifest = state
        .manager
        .load_snapshot(
            name.clone(),
            params.workspace.clone(),
            request.components,
            consistency_engine,
        )
        .await
        .map_err(|e| {
            error!("Failed to load snapshot: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Loaded snapshot '{}' for workspace '{}'", name, params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "manifest": manifest,
    })))
}

/// List all snapshots
///
/// GET /api/v1/snapshots?workspace={workspace_id}
pub async fn list_snapshots(
    State(state): State<SnapshotState>,
    Query(params): Query<SnapshotQuery>,
) -> Result<Json<Value>, StatusCode> {
    let snapshots = state.manager.list_snapshots(&params.workspace).await.map_err(|e| {
        error!("Failed to list snapshots: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "workspace": params.workspace,
        "snapshots": snapshots,
        "count": snapshots.len(),
    })))
}

/// Get snapshot information
///
/// GET /api/v1/snapshots/{name}?workspace={workspace_id}
pub async fn get_snapshot_info(
    State(state): State<SnapshotState>,
    Path(name): Path<String>,
    Query(params): Query<SnapshotQuery>,
) -> Result<Json<Value>, StatusCode> {
    let manifest = state
        .manager
        .get_snapshot_info(name.clone(), params.workspace.clone())
        .await
        .map_err(|e| {
            error!("Failed to get snapshot info: {}", e);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "manifest": manifest,
    })))
}

/// Delete a snapshot
///
/// DELETE /api/v1/snapshots/{name}?workspace={workspace_id}
pub async fn delete_snapshot(
    State(state): State<SnapshotState>,
    Path(name): Path<String>,
    Query(params): Query<SnapshotQuery>,
) -> Result<Json<Value>, StatusCode> {
    state
        .manager
        .delete_snapshot(name.clone(), params.workspace.clone())
        .await
        .map_err(|e| {
            error!("Failed to delete snapshot: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Deleted snapshot '{}' for workspace '{}'", name, params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Snapshot '{}' deleted successfully", name),
    })))
}

/// Validate snapshot integrity
///
/// GET /api/v1/snapshots/{name}/validate?workspace={workspace_id}
pub async fn validate_snapshot(
    State(state): State<SnapshotState>,
    Path(name): Path<String>,
    Query(params): Query<SnapshotQuery>,
) -> Result<Json<Value>, StatusCode> {
    let is_valid = state
        .manager
        .validate_snapshot(name.clone(), params.workspace.clone())
        .await
        .map_err(|e| {
            error!("Failed to validate snapshot: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "valid": is_valid,
        "snapshot": name,
        "workspace": params.workspace,
    })))
}

/// Create snapshot router
pub fn snapshot_router(state: SnapshotState) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/api/v1/snapshots", get(list_snapshots).post(save_snapshot))
        .route("/api/v1/snapshots/{name}", get(get_snapshot_info).delete(delete_snapshot))
        .route("/api/v1/snapshots/{name}/load", post(load_snapshot))
        .route("/api/v1/snapshots/{name}/validate", get(validate_snapshot))
        .with_state(state)
}
