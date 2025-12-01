//! World State API handlers
//!
//! This module provides HTTP handlers for querying and visualizing the unified
//! world state of MockForge, including REST API endpoints and WebSocket streaming.

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;
use mockforge_world_state::{
    model::{StateLayer, WorldStateSnapshot},
    WorldStateEngine, WorldStateQuery,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// State for world state handlers
#[derive(Clone)]
pub struct WorldStateState {
    /// World state engine
    pub engine: Arc<RwLock<WorldStateEngine>>,
}

/// Query parameters for world state operations
#[derive(Debug, Deserialize)]
pub struct WorldStateQueryParams {
    /// Workspace ID (optional)
    pub workspace: Option<String>,
    /// Layer filter (comma-separated)
    pub layers: Option<String>,
    /// Node type filter (comma-separated)
    pub node_types: Option<String>,
}

/// Request body for querying world state
#[derive(Debug, Deserialize)]
pub struct WorldStateQueryRequest {
    /// Filter by node types
    pub node_types: Option<Vec<String>>,
    /// Filter by layers
    pub layers: Option<Vec<String>>,
    /// Filter by node IDs
    pub node_ids: Option<Vec<String>>,
    /// Filter by relationship types
    pub relationship_types: Option<Vec<String>>,
    /// Include edges in results
    #[serde(default = "default_true")]
    pub include_edges: bool,
    /// Maximum depth for traversal
    pub max_depth: Option<usize>,
}

fn default_true() -> bool {
    true
}

/// Response for world state snapshot
#[derive(Debug, Serialize)]
pub struct WorldStateSnapshotResponse {
    /// The snapshot
    pub snapshot: WorldStateSnapshot,
    /// Available layers
    pub available_layers: Vec<String>,
}

/// Response for world state graph
#[derive(Debug, Serialize)]
pub struct WorldStateGraphResponse {
    /// Graph nodes
    pub nodes: Vec<Value>,
    /// Graph edges
    pub edges: Vec<Value>,
    /// Metadata
    pub metadata: Value,
}

/// Get current world state snapshot
///
/// GET /api/world-state/snapshot
pub async fn get_current_snapshot(
    State(state): State<WorldStateState>,
) -> Result<Json<WorldStateSnapshotResponse>, StatusCode> {
    let engine = state.engine.read().await;
    let snapshot = engine.get_current_snapshot().await.map_err(|e| {
        error!("Failed to create world state snapshot: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let layers: Vec<String> = engine.get_layers().iter().map(|l| l.name().to_string()).collect();

    Ok(Json(WorldStateSnapshotResponse {
        snapshot,
        available_layers: layers,
    }))
}

/// Get a specific snapshot by ID
///
/// GET /api/world-state/snapshot/{id}
pub async fn get_snapshot(
    State(state): State<WorldStateState>,
    Path(snapshot_id): Path<String>,
) -> Result<Json<WorldStateSnapshot>, StatusCode> {
    let engine = state.engine.read().await;
    let snapshot = engine.get_snapshot(&snapshot_id).await.ok_or_else(|| {
        error!("Snapshot not found: {}", snapshot_id);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(snapshot))
}

/// Get world state as a graph
///
/// GET /api/world-state/graph
pub async fn get_world_state_graph(
    State(state): State<WorldStateState>,
    Query(params): Query<WorldStateQueryParams>,
) -> Result<Json<WorldStateGraphResponse>, StatusCode> {
    let engine = state.engine.read().await;
    let snapshot = engine.get_current_snapshot().await.map_err(|e| {
        error!("Failed to create world state snapshot: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert nodes and edges to JSON values
    let nodes: Vec<Value> = snapshot
        .nodes
        .iter()
        .map(|n| serde_json::to_value(n).unwrap_or_default())
        .collect();

    let edges: Vec<Value> = snapshot
        .edges
        .iter()
        .map(|e| serde_json::to_value(e).unwrap_or_default())
        .collect();

    let metadata = serde_json::json!({
        "node_count": nodes.len(),
        "edge_count": edges.len(),
        "timestamp": snapshot.timestamp.to_rfc3339(),
    });

    Ok(Json(WorldStateGraphResponse {
        nodes,
        edges,
        metadata,
    }))
}

/// Get available layers
///
/// GET /api/world-state/layers
pub async fn get_layers(State(state): State<WorldStateState>) -> Result<Json<Value>, StatusCode> {
    let engine = state.engine.read().await;
    let layers: Vec<Value> = engine
        .get_layers()
        .iter()
        .map(|layer| {
            serde_json::json!({
                "id": format!("{:?}", layer),
                "name": layer.name(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "layers": layers,
        "count": layers.len(),
    })))
}

/// Query world state with filters
///
/// POST /api/world-state/query
pub async fn query_world_state(
    State(state): State<WorldStateState>,
    Json(request): Json<WorldStateQueryRequest>,
) -> Result<Json<WorldStateSnapshot>, StatusCode> {
    let engine = state.engine.read().await;

    // Build query from request
    let mut query = WorldStateQuery::new();

    if let Some(ref node_types) = request.node_types {
        let types: HashSet<_> = node_types
            .iter()
            .filter_map(|s| {
                // Try to parse as NodeType - for now just store as string
                // This would need proper parsing in a real implementation
                Some(s.as_str())
            })
            .collect();
        // Note: This is a simplified version - would need proper NodeType parsing
    }

    if let Some(ref layers) = request.layers {
        let layer_set: HashSet<StateLayer> = layers
            .iter()
            .filter_map(|s| {
                // Parse layer string to StateLayer
                match s.as_str() {
                    "personas" => Some(StateLayer::Personas),
                    "lifecycle" => Some(StateLayer::Lifecycle),
                    "reality" => Some(StateLayer::Reality),
                    "time" => Some(StateLayer::Time),
                    "protocols" => Some(StateLayer::Protocols),
                    "behavior" => Some(StateLayer::Behavior),
                    "schemas" => Some(StateLayer::Schemas),
                    "recorded" => Some(StateLayer::Recorded),
                    "ai_modifiers" => Some(StateLayer::AiModifiers),
                    "system" => Some(StateLayer::System),
                    _ => None,
                }
            })
            .collect();
        if !layer_set.is_empty() {
            query = query.with_layers(layer_set);
        }
    }

    if let Some(ref node_ids) = request.node_ids {
        let id_set: HashSet<String> = node_ids.iter().cloned().collect();
        query = query.with_node_ids(id_set);
    }

    if let Some(ref rel_types) = request.relationship_types {
        let rel_set: HashSet<String> = rel_types.iter().cloned().collect();
        query = query.with_relationship_types(rel_set);
    }

    query = query.include_edges(request.include_edges);

    if let Some(depth) = request.max_depth {
        query = query.with_max_depth(depth);
    }

    let snapshot = engine.query(&query).await.map_err(|e| {
        error!("Failed to query world state: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(snapshot))
}

/// WebSocket handler for real-time world state updates
///
/// WS /api/world-state/stream
pub async fn world_state_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<WorldStateState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_world_state_stream(socket, state))
}

/// Handle WebSocket stream for world state updates
async fn handle_world_state_stream(
    mut socket: axum::extract::ws::WebSocket,
    state: WorldStateState,
) {
    use axum::extract::ws::Message;
    use futures_util::SinkExt;
    use tokio::time::{interval, Duration};

    // Send initial snapshot
    {
        let engine = state.engine.read().await;
        if let Ok(snapshot) = engine.get_current_snapshot().await {
            if let Ok(json) = serde_json::to_string(&snapshot) {
                let _ = socket.send(Message::Text(json.into())).await;
            }
        }
    }

    // Send periodic updates (every 5 seconds)
    let mut interval = interval(Duration::from_secs(5));
    let mut closed = false;

    loop {
        tokio::select! {
            // Handle incoming messages (for now, just acknowledge)
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        info!("Received WebSocket message: {}", text);
                        // Could handle commands like "subscribe to layer X"
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket connection closed");
                        closed = true;
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        closed = true;
                        break;
                    }
                    None => {
                        closed = true;
                        break;
                    }
                    _ => {}
                }
            }
            // Send periodic updates
            _ = interval.tick() => {
                let engine = state.engine.read().await;
                if let Ok(snapshot) = engine.get_current_snapshot().await {
                    if let Ok(json) = serde_json::to_string(&snapshot) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            closed = true;
                            break;
                        }
                    }
                }
            }
        }

        if closed {
            break;
        }
    }
}

/// Create the world state router
pub fn world_state_router() -> Router<WorldStateState> {
    Router::new()
        .route("/snapshot", get(get_current_snapshot))
        .route("/snapshot/{id}", get(get_snapshot))
        .route("/graph", get(get_world_state_graph))
        .route("/layers", get(get_layers))
        .route("/query", post(query_world_state))
        .route("/stream", get(world_state_websocket_handler))
}
