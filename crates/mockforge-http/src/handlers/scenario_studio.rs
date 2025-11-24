//! Scenario Studio API handlers
//!
//! This module provides HTTP handlers for managing business flows in the Scenario Studio.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::scenario_studio::{
    FlowDefinition, FlowExecutionResult, FlowExecutor, FlowType, FlowVariant,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// State for scenario studio handlers
#[derive(Clone)]
pub struct ScenarioStudioState {
    /// In-memory store for flows (can be replaced with database later)
    flows: Arc<RwLock<HashMap<String, FlowDefinition>>>,
    /// In-memory store for flow variants
    variants: Arc<RwLock<HashMap<String, FlowVariant>>>,
}

impl ScenarioStudioState {
    /// Create a new scenario studio state
    pub fn new() -> Self {
        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
            variants: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for ScenarioStudioState {
    fn default() -> Self {
        Self::new()
    }
}

/// Request to create a flow
#[derive(Debug, Deserialize)]
pub struct CreateFlowRequest {
    /// Flow name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Flow type
    pub flow_type: FlowType,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request to update a flow
#[derive(Debug, Deserialize)]
pub struct UpdateFlowRequest {
    /// Flow name
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Flow type
    pub flow_type: Option<FlowType>,
    /// Steps in the flow
    pub steps: Option<Vec<mockforge_core::scenario_studio::FlowStep>>,
    /// Connections between steps
    pub connections: Option<Vec<mockforge_core::scenario_studio::FlowConnection>>,
    /// Variables
    pub variables: Option<HashMap<String, Value>>,
    /// Tags
    pub tags: Option<Vec<String>>,
}

/// Request to execute a flow
#[derive(Debug, Deserialize)]
pub struct ExecuteFlowRequest {
    /// Optional initial variables
    #[serde(default)]
    pub variables: HashMap<String, Value>,
}

/// Request to create a flow variant
#[derive(Debug, Deserialize)]
pub struct CreateFlowVariantRequest {
    /// Variant name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// ID of the base flow
    pub flow_id: String,
}

/// Query parameters for workspace operations
#[derive(Debug, Deserialize)]
pub struct WorkspaceQuery {
    /// Workspace ID (defaults to "default" if not provided)
    #[serde(default = "default_workspace")]
    pub workspace: String,
}

fn default_workspace() -> String {
    "default".to_string()
}

/// Create a new flow
///
/// POST /api/v1/scenario-studio/flows
pub async fn create_flow(
    State(state): State<ScenarioStudioState>,
    Json(request): Json<CreateFlowRequest>,
) -> Result<Json<FlowDefinition>, StatusCode> {
    let mut flow = FlowDefinition::new(request.name, request.flow_type);
    flow.description = request.description;
    flow.tags = request.tags;

    let flow_id = flow.id.clone();
    let mut flows = state.flows.write().await;
    flows.insert(flow_id.clone(), flow.clone());

    info!("Created flow: {}", flow_id);
    Ok(Json(flow))
}

/// List all flows
///
/// GET /api/v1/scenario-studio/flows
pub async fn list_flows(
    State(state): State<ScenarioStudioState>,
) -> Result<Json<Vec<FlowDefinition>>, StatusCode> {
    let flows = state.flows.read().await;
    let flows_list: Vec<FlowDefinition> = flows.values().cloned().collect();
    Ok(Json(flows_list))
}

/// Get a specific flow
///
/// GET /api/v1/scenario-studio/flows/:id
pub async fn get_flow(
    State(state): State<ScenarioStudioState>,
    Path(id): Path<String>,
) -> Result<Json<FlowDefinition>, StatusCode> {
    let flows = state.flows.read().await;
    let flow = flows.get(&id).cloned().ok_or_else(|| {
        error!("Flow not found: {}", id);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(flow))
}

/// Update a flow
///
/// PUT /api/v1/scenario-studio/flows/:id
pub async fn update_flow(
    State(state): State<ScenarioStudioState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateFlowRequest>,
) -> Result<Json<FlowDefinition>, StatusCode> {
    let mut flows = state.flows.write().await;
    let flow = flows.get_mut(&id).ok_or_else(|| {
        error!("Flow not found: {}", id);
        StatusCode::NOT_FOUND
    })?;

    if let Some(name) = request.name {
        flow.name = name;
    }
    if let Some(description) = request.description {
        flow.description = Some(description);
    }
    if let Some(flow_type) = request.flow_type {
        flow.flow_type = flow_type;
    }
    if let Some(steps) = request.steps {
        flow.steps = steps;
    }
    if let Some(connections) = request.connections {
        flow.connections = connections;
    }
    if let Some(variables) = request.variables {
        flow.variables = variables;
    }
    if let Some(tags) = request.tags {
        flow.tags = tags;
    }

    flow.updated_at = chrono::Utc::now();

    let flow_clone = flow.clone();
    info!("Updated flow: {}", id);
    Ok(Json(flow_clone))
}

/// Delete a flow
///
/// DELETE /api/v1/scenario-studio/flows/:id
pub async fn delete_flow(
    State(state): State<ScenarioStudioState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let mut flows = state.flows.write().await;
    if flows.remove(&id).is_none() {
        error!("Flow not found: {}", id);
        return Err(StatusCode::NOT_FOUND);
    }

    // Also remove any variants associated with this flow
    let mut variants = state.variants.write().await;
    variants.retain(|_, v| v.flow_id != id);

    info!("Deleted flow: {}", id);
    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Flow {} deleted", id),
    })))
}

/// Execute a flow
///
/// POST /api/v1/scenario-studio/flows/:id/execute
#[axum::debug_handler]
pub async fn execute_flow(
    Path(id): Path<String>,
    State(state): State<ScenarioStudioState>,
    Json(request): Json<ExecuteFlowRequest>,
) -> Result<Json<FlowExecutionResult>, StatusCode> {
    let flows = state.flows.read().await;
    let flow = flows.get(&id).cloned().ok_or_else(|| {
        error!("Flow not found: {}", id);
        StatusCode::NOT_FOUND
    })?;

    drop(flows); // Release lock before async operation

    let initial_variables = request.variables;
    let mut executor = FlowExecutor::with_variables(initial_variables);
    let result = executor.execute(&flow).await.map_err(|e| {
        error!("Failed to execute flow {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("Executed flow: {}", id);
    Ok(Json(result))
}

/// Create a flow variant
///
/// POST /api/v1/scenario-studio/flows/:id/variants
pub async fn create_flow_variant(
    State(state): State<ScenarioStudioState>,
    Path(id): Path<String>,
    Json(request): Json<CreateFlowVariantRequest>,
) -> Result<Json<FlowVariant>, StatusCode> {
    // Verify that the base flow exists
    let flows = state.flows.read().await;
    if !flows.contains_key(&id) {
        error!("Base flow not found: {}", id);
        return Err(StatusCode::NOT_FOUND);
    }
    drop(flows);

    let mut variant = FlowVariant::new(request.name, id.clone());
    variant.description = request.description;

    let variant_id = variant.id.clone();
    let mut variants = state.variants.write().await;
    variants.insert(variant_id.clone(), variant.clone());

    info!("Created flow variant: {} for flow: {}", variant_id, id);
    Ok(Json(variant))
}

/// List all variants for a flow
///
/// GET /api/v1/scenario-studio/flows/:id/variants
pub async fn list_flow_variants(
    State(state): State<ScenarioStudioState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<FlowVariant>>, StatusCode> {
    let variants = state.variants.read().await;
    let flow_variants: Vec<FlowVariant> =
        variants.values().filter(|v| v.flow_id == id).cloned().collect();
    Ok(Json(flow_variants))
}

/// Create scenario studio router
pub fn scenario_studio_router(state: ScenarioStudioState) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        .route("/api/v1/scenario-studio/flows", post(create_flow).get(list_flows))
        .route(
            "/api/v1/scenario-studio/flows/{id}",
            get(get_flow).put(update_flow).delete(delete_flow),
        )
        .route("/api/v1/scenario-studio/flows/{id}/execute", post(execute_flow))
        .route(
            "/api/v1/scenario-studio/flows/{id}/variants",
            post(create_flow_variant).get(list_flow_variants),
        )
        .with_state(state)
}
