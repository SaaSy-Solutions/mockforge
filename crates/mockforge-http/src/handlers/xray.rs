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
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Request context snapshot - stores the state that was active when a request was made
#[derive(Debug, Clone)]
pub(crate) struct RequestContextSnapshot {
    /// Workspace ID
    workspace_id: String,
    /// Snapshot of unified state at request time
    state_snapshot: serde_json::Value,
    /// Timestamp when request was made
    timestamp: i64,
}

/// State for X-Ray handlers
#[derive(Clone)]
pub struct XRayState {
    /// Consistency engine
    pub engine: Arc<ConsistencyEngine>,
    /// Request context storage (request_id -> snapshot)
    pub request_contexts: Arc<RwLock<HashMap<String, RequestContextSnapshot>>>,
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
        // Note: ChaosScenario is now serde_json::Value, so we extract the name field
        "chaos_rules": unified_state
            .active_chaos_rules
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect::<Vec<_>>(),
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
    State(state): State<XRayState>,
    Path(request_id): Path<String>,
    Query(params): Query<XRayQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Try to retrieve stored context snapshot
    let contexts = state.request_contexts.read().await;
    if let Some(snapshot) = contexts.get(&request_id) {
        // Verify workspace matches (if provided)
        if snapshot.workspace_id == params.workspace {
            return Ok(Json(serde_json::json!({
                "request_id": request_id,
                "workspace": snapshot.workspace_id,
                "state_snapshot": snapshot.state_snapshot,
                "timestamp": snapshot.timestamp,
                "cached": true,
            })));
        }
    }
    drop(contexts);

    // Fallback: return current state if snapshot not found
    debug!(
        "Request context not found for request_id: {}, returning current state",
        request_id
    );
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        debug!("No state found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(serde_json::json!({
        "request_id": request_id,
        "workspace": params.workspace,
        "state_snapshot": serde_json::to_value(&unified_state).unwrap(),
        "timestamp": unified_state.last_updated,
        "cached": false,
        "note": "Snapshot not found, returning current state",
    })))
}

/// Store request context snapshot
///
/// This is called by the middleware when a request is processed.
/// It stores a snapshot of the unified state at the time of the request.
pub async fn store_request_context(
    state: &XRayState,
    request_id: String,
    workspace_id: String,
    unified_state: &mockforge_core::consistency::types::UnifiedState,
) {
    let state_snapshot = serde_json::to_value(unified_state).unwrap_or_default();
    let snapshot = RequestContextSnapshot {
        workspace_id: workspace_id.clone(),
        state_snapshot,
        timestamp: unified_state.last_updated.timestamp(),
    };

    let mut contexts = state.request_contexts.write().await;

    // Limit storage to last 1000 requests per workspace to prevent memory bloat
    // Remove oldest entries if we exceed the limit
    let workspace_entries: Vec<_> = contexts
        .iter()
        .filter(|(_, s)| s.workspace_id == workspace_id)
        .map(|(k, _)| k.clone())
        .collect();

    if workspace_entries.len() >= 1000 {
        // Remove oldest 100 entries for this workspace
        let mut timestamps: Vec<_> = workspace_entries
            .iter()
            .filter_map(|id| contexts.get(id).map(|s| (id.clone(), s.timestamp)))
            .collect();
        timestamps.sort_by_key(|(_, ts)| *ts);

        for (id, _) in timestamps.iter().take(100) {
            contexts.remove(id);
        }
    }

    contexts.insert(request_id, snapshot);
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
        .route("/api/v1/xray/request-context/{request_id}", get(get_request_context))
        .route("/api/v1/xray/workspace/{workspace_id}/summary", get(get_workspace_summary))
        .route("/api/v1/xray/entities", get(list_entities))
        .route("/api/v1/xray/entities/{entity_type}/{entity_id}", get(get_entity))
        .with_state(state)
}
