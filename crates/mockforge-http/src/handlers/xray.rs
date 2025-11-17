//! X-Ray API handlers for frontend debugging
//!
//! This module provides lightweight API endpoints for the browser extension
//! to display current state (scenario, persona, reality level, chaos rules).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::consistency::ConsistencyEngine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// State for X-Ray handlers
#[derive(Clone)]
pub struct XRayState {
    /// Consistency engine
    pub engine: Arc<ConsistencyEngine>,
}

/// Query parameters for X-Ray operations
#[derive(Debug, Deserialize)]
pub struct XRayQuery {
    /// Workspace ID (defaults to "default" if not provided)
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_workspace() -> String {
    "default".to_string()
}

/// Get current state summary (optimized for extension overlay)
///
/// GET /api/v1/xray/state/summary?workspace={workspace_id}
///
/// Returns a lightweight summary suitable for the browser extension overlay.
pub async fn get_state_summary(
    State(state): State<XRayState>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        debug!("No state found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    // Build lightweight summary
    let summary = serde_json::json!({
        "workspace_id": unified_state.workspace_id,
        "scenario": unified_state.active_scenario,
        "persona": unified_state.active_persona.as_ref().map(|p| serde_json::json!({
            "id": p.id,
            "traits": p.traits,
        })),
        "reality_level": unified_state.reality_level.value(),
        "reality_level_name": unified_state.reality_level.name(),
        "reality_ratio": unified_state.reality_continuum_ratio,
        "chaos_rules": unified_state.active_chaos_rules.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
        "timestamp": unified_state.last_updated,
    });

    Ok(Json(summary))
}

/// Get full state (for DevTools panel)
///
/// GET /api/v1/xray/state?workspace={workspace_id}
pub async fn get_state(
    State(state): State<XRayState>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        debug!("No state found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(serde_json::to_value(&unified_state).unwrap()))
}

/// Get request context for a specific request ID
///
/// GET /api/v1/xray/request-context/{request_id}?workspace={workspace_id}
///
/// Returns the state that was active when a specific request was made.
/// Request IDs are provided in X-MockForge-Request-ID headers.
pub async fn get_request_context(
    State(_state): State<XRayState>,
    Path(request_id): Path<String>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    // TODO: Implement request context storage/retrieval
    // For now, return current state
    debug!("Request context lookup for request_id: {} (not yet implemented)", request_id);
    Ok(Json(serde_json::json!({
        "request_id": request_id,
        "workspace": params.workspace,
        "message": "Request context tracking not yet implemented",
    })))
}

/// Get workspace summary
///
/// GET /api/v1/xray/workspace/{workspace_id}/summary
pub async fn get_workspace_summary(
    State(state): State<XRayState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let unified_state = state.engine.get_state(&workspace_id).await.ok_or_else(|| {
        debug!("No state found for workspace: {}", workspace_id);
        StatusCode::NOT_FOUND
    })?;

    let summary = serde_json::json!({
        "workspace_id": unified_state.workspace_id,
        "scenario": unified_state.active_scenario,
        "persona_id": unified_state.active_persona.as_ref().map(|p| p.id.clone()),
        "reality_level": unified_state.reality_level.value(),
        "reality_ratio": unified_state.reality_continuum_ratio,
        "active_chaos_rules_count": unified_state.active_chaos_rules.len(),
        "entity_count": unified_state.entity_state.len(),
        "protocol_count": unified_state.protocol_states.len(),
        "last_updated": unified_state.last_updated,
    });

    Ok(Json(summary))
}

/// List all entities for a workspace
///
/// GET /api/v1/xray/entities?workspace={workspace_id}
pub async fn list_entities(
    State(state): State<XRayState>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        debug!("No state found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    let entities: Vec<&mockforge_core::consistency::EntityState> =
        unified_state.entity_state.values().collect();

    Ok(Json(serde_json::json!({
        "workspace": params.workspace,
        "entities": entities,
        "count": entities.len(),
    })))
}

/// Get specific entity
///
/// GET /api/v1/xray/entities/{entity_type}/{entity_id}?workspace={workspace_id}
pub async fn get_entity(
    State(state): State<XRayState>,
    Path((entity_type, entity_id)): Path<(String, String)>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    let entity = state
        .engine
        .get_entity(&params.workspace, &entity_type, &entity_id)
        .await
        .ok_or_else(|| {
            debug!(
                "Entity not found: {}:{} in workspace: {}",
                entity_type, entity_id, params.workspace
            );
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::to_value(&entity).unwrap()))
}

/// Create X-Ray router
pub fn xray_router(state: XRayState) -> axum::Router {
    use axum::routing::get;

    axum::Router::new()
        .route("/api/v1/xray/state/summary", get(get_state_summary))
        .route("/api/v1/xray/state", get(get_state))
        .route("/api/v1/xray/request-context/:request_id", get(get_request_context))
        .route("/api/v1/xray/workspace/:workspace_id/summary", get(get_workspace_summary))
        .route("/api/v1/xray/entities", get(list_entities))
        .route("/api/v1/xray/entities/:entity_type/:entity_id", get(get_entity))
        .with_state(state)
}
