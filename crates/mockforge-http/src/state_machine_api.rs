//! State machine API handlers
//!
//! Provides REST endpoints for managing scenario state machines, including
//! CRUD operations, execution, and import/export functionality.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use mockforge_core::intelligent_behavior::{rules::StateMachine, visual_layout::VisualLayout};
use mockforge_scenarios::ScenarioManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::error;

// Re-export ManagementState for use in handlers
use crate::management::ManagementState;

// ===== Request/Response Types =====

/// Request to create or update a state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineRequest {
    /// State machine definition
    pub state_machine: StateMachine,
    /// Optional visual layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_layout: Option<VisualLayout>,
}

/// Request to execute a state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRequest {
    /// Resource ID to transition
    pub resource_id: String,
    /// Target state
    pub to_state: String,
    /// Optional context variables for condition evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, Value>>,
}

/// Request to create a state instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    /// Resource ID
    pub resource_id: String,
    /// Resource type (must match a state machine resource_type)
    pub resource_type: String,
}

/// Response for state machine operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineResponse {
    /// State machine definition
    pub state_machine: StateMachine,
    /// Optional visual layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_layout: Option<VisualLayout>,
}

/// Response for listing state machines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineListResponse {
    /// List of state machines
    pub state_machines: Vec<StateMachineInfo>,
    /// Total count
    pub total: usize,
}

/// Information about a state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineInfo {
    /// Resource type
    pub resource_type: String,
    /// Number of states
    pub state_count: usize,
    /// Number of transitions
    pub transition_count: usize,
    /// Number of sub-scenarios
    pub sub_scenario_count: usize,
    /// Whether it has a visual layout
    pub has_visual_layout: bool,
}

/// Response for state instance operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInstanceResponse {
    /// Resource ID
    pub resource_id: String,
    /// Current state
    pub current_state: String,
    /// Resource type
    pub resource_type: String,
    /// State history count
    pub history_count: usize,
    /// State data
    pub state_data: HashMap<String, Value>,
}

/// Response for listing state instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInstanceListResponse {
    /// List of instances
    pub instances: Vec<StateInstanceResponse>,
    /// Total count
    pub total: usize,
}

/// Response for next states query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextStatesResponse {
    /// List of possible next states
    pub next_states: Vec<String>,
}

/// Response for import/export operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExportResponse {
    /// State machines
    pub state_machines: Vec<StateMachine>,
    /// Visual layouts by resource type
    pub visual_layouts: HashMap<String, VisualLayout>,
}

// ===== Handlers =====

/// List all state machines
pub async fn list_state_machines(
    State(state): State<ManagementState>,
) -> Result<Json<StateMachineListResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    // Get all state machines
    let machines = manager.list_state_machines().await;

    // Check visual layouts separately for each state machine
    // We need to check if a visual layout exists for each state machine
    let mut state_machine_list = Vec::new();
    for (resource_type, sm) in machines.iter() {
        let has_visual_layout = manager.get_visual_layout(resource_type).await.is_some();
        state_machine_list.push(StateMachineInfo {
            resource_type: resource_type.clone(),
            state_count: sm.states.len(),
            transition_count: sm.transitions.len(),
            sub_scenario_count: sm.sub_scenarios.len(),
            has_visual_layout,
        });
    }

    Ok(Json(StateMachineListResponse {
        state_machines: state_machine_list.clone(),
        total: state_machine_list.len(),
    }))
}

/// Get a state machine by resource type
pub async fn get_state_machine(
    State(state): State<ManagementState>,
    Path(resource_type): Path<String>,
) -> Result<Json<StateMachineResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    let state_machine =
        manager.get_state_machine(&resource_type).await.ok_or(StatusCode::NOT_FOUND)?;

    let visual_layout = manager.get_visual_layout(&resource_type).await;

    // Convert types from mockforge-scenarios' dependency version to local version
    // by serializing and deserializing through JSON
    let state_machine_json =
        serde_json::to_value(&state_machine).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let state_machine: StateMachine = serde_json::from_value(state_machine_json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let visual_layout: Option<VisualLayout> = visual_layout
        .map(|layout| {
            let layout_json =
                serde_json::to_value(&layout).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            serde_json::from_value(layout_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
        .transpose()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StateMachineResponse {
        state_machine,
        visual_layout,
    }))
}

/// Create or update a state machine
pub async fn create_state_machine(
    State(state): State<ManagementState>,
    Json(request): Json<StateMachineRequest>,
) -> Result<Json<StateMachineResponse>, StatusCode> {
    let manager = state.state_machine_manager.write().await;

    // Convert types from local version to mockforge-scenarios' dependency version
    // by serializing and deserializing through JSON
    // The ScenarioManifest uses types from mockforge-scenarios' mockforge-core dependency (0.2.9)
    // We need to convert our local StateMachine to that version
    let state_machine_json = serde_json::to_value(&request.state_machine)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create manifest with JSON values - serde will deserialize into the correct types
    // We need to provide all required fields for ScenarioManifest
    let mut manifest_json = serde_json::json!({
        "manifest_version": "1.0",
        "name": "api",
        "version": "1.0.0",
        "title": "API State Machine",
        "description": "State machine created via API",
        "author": "api",
        "category": "other",
        "compatibility": {
            "min_version": "0.1.0",
            "max_version": null
        },
        "files": [],
        "state_machines": [state_machine_json],
        "state_machine_graphs": {}
    });

    if let Some(layout) = &request.visual_layout {
        let layout_json =
            serde_json::to_value(layout).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        manifest_json["state_machine_graphs"][&request.state_machine.resource_type] = layout_json;
    }

    let manifest: ScenarioManifest =
        serde_json::from_value(manifest_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Validate the first state machine from manifest
    if let Some(sm) = manifest.state_machines.first() {
        if let Err(e) = manager.validate_state_machine(sm) {
            error!("Invalid state machine: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Err(e) = manager.load_from_manifest(&manifest).await {
        error!("Failed to load state machine: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Visual layout is already set in the manifest, no need to set separately

    // Broadcast WebSocket event
    if let Some(ref ws_tx) = state.ws_broadcast {
        let event = crate::management_ws::MockEvent::state_machine_updated(
            request.state_machine.resource_type.clone(),
            request.state_machine.clone(),
        );
        let _ = ws_tx.send(event);
    }

    // Get state machine and layout back after loading (returns version from mockforge-scenarios' dependency)
    let state_machine_from_manager = manager
        .get_state_machine(&request.state_machine.resource_type)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let visual_layout_from_manager =
        manager.get_visual_layout(&request.state_machine.resource_type).await;

    // Convert back to local types
    let state_machine_json = serde_json::to_value(&state_machine_from_manager)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let state_machine: StateMachine = serde_json::from_value(state_machine_json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let visual_layout: Option<VisualLayout> = visual_layout_from_manager
        .map(|layout| {
            let layout_json =
                serde_json::to_value(&layout).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            serde_json::from_value(layout_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
        .transpose()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StateMachineResponse {
        state_machine,
        visual_layout,
    }))
}

/// Delete a state machine
pub async fn delete_state_machine(
    State(state): State<ManagementState>,
    Path(resource_type): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let manager = state.state_machine_manager.write().await;

    // Delete the state machine
    let deleted = manager.delete_state_machine(&resource_type).await;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    // Broadcast WebSocket event
    if let Some(ref ws_tx) = state.ws_broadcast {
        let event = crate::management_ws::MockEvent::state_machine_deleted(resource_type);
        let _ = ws_tx.send(event);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// List all state instances
pub async fn list_instances(
    State(state): State<ManagementState>,
) -> Result<Json<StateInstanceListResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    let instances = manager.list_instances().await;

    let instance_responses: Vec<StateInstanceResponse> = instances
        .iter()
        .map(|i| StateInstanceResponse {
            resource_id: i.resource_id.clone(),
            current_state: i.current_state.clone(),
            resource_type: i.resource_type.clone(),
            history_count: i.state_history.len(),
            state_data: i.state_data.clone(),
        })
        .collect();

    Ok(Json(StateInstanceListResponse {
        instances: instance_responses,
        total: instances.len(),
    }))
}

/// Get a state instance by resource ID
pub async fn get_instance(
    State(state): State<ManagementState>,
    Path(resource_id): Path<String>,
) -> Result<Json<StateInstanceResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    let instance = manager.get_instance(&resource_id).await.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(StateInstanceResponse {
        resource_id: instance.resource_id,
        current_state: instance.current_state,
        resource_type: instance.resource_type,
        history_count: instance.state_history.len(),
        state_data: instance.state_data,
    }))
}

/// Create a new state instance
pub async fn create_instance(
    State(state): State<ManagementState>,
    Json(request): Json<CreateInstanceRequest>,
) -> Result<Json<StateInstanceResponse>, StatusCode> {
    let manager = state.state_machine_manager.write().await;

    if let Err(e) = manager.create_instance(&request.resource_id, &request.resource_type).await {
        error!("Failed to create instance: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    let instance = manager
        .get_instance(&request.resource_id)
        .await
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Broadcast WebSocket event
    if let Some(ref ws_tx) = state.ws_broadcast {
        let event = crate::management_ws::MockEvent::state_instance_created(
            instance.resource_id.clone(),
            instance.resource_type.clone(),
            instance.current_state.clone(),
        );
        let _ = ws_tx.send(event);
    }

    Ok(Json(StateInstanceResponse {
        resource_id: instance.resource_id,
        current_state: instance.current_state,
        resource_type: instance.resource_type,
        history_count: instance.state_history.len(),
        state_data: instance.state_data,
    }))
}

/// Execute a state transition
pub async fn execute_transition(
    State(state): State<ManagementState>,
    Json(request): Json<TransitionRequest>,
) -> Result<Json<StateInstanceResponse>, StatusCode> {
    let manager = state.state_machine_manager.write().await;

    if let Err(e) = manager
        .execute_transition(&request.resource_id, &request.to_state, request.context)
        .await
    {
        error!("Failed to execute transition: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    let instance = manager.get_instance(&request.resource_id).await.ok_or(StatusCode::NOT_FOUND)?;

    // Get the previous state from history if available
    let from_state = instance
        .state_history
        .last()
        .map(|h| h.from_state.clone())
        .unwrap_or_else(|| instance.current_state.clone());

    // Broadcast WebSocket event
    if let Some(ref ws_tx) = state.ws_broadcast {
        let event = crate::management_ws::MockEvent::state_transitioned(
            instance.resource_id.clone(),
            instance.resource_type.clone(),
            from_state,
            instance.current_state.clone(),
            instance.state_data.clone(),
        );
        let _ = ws_tx.send(event);
    }

    Ok(Json(StateInstanceResponse {
        resource_id: instance.resource_id,
        current_state: instance.current_state,
        resource_type: instance.resource_type,
        history_count: instance.state_history.len(),
        state_data: instance.state_data,
    }))
}

/// Get next possible states for a resource
pub async fn get_next_states(
    State(state): State<ManagementState>,
    Path(resource_id): Path<String>,
) -> Result<Json<NextStatesResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    let next_states =
        manager.get_next_states(&resource_id).await.map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(NextStatesResponse { next_states }))
}

/// Get current state of a resource
pub async fn get_current_state(
    State(state): State<ManagementState>,
    Path(resource_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    let current_state =
        manager.get_current_state(&resource_id).await.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(serde_json::json!({
        "resource_id": resource_id,
        "current_state": current_state
    })))
}

/// Export state machines as JSON
pub async fn export_state_machines(
    State(state): State<ManagementState>,
) -> Result<Json<ImportExportResponse>, StatusCode> {
    let manager = state.state_machine_manager.read().await;

    // Export all state machines and visual layouts (returns versions from mockforge-scenarios' dependency)
    let (state_machines_from_manager, visual_layouts_from_manager) = manager.export_all().await;

    // Convert to local types by serializing and deserializing
    let state_machines: Vec<StateMachine> = state_machines_from_manager
        .into_iter()
        .map(|sm| {
            let json = serde_json::to_value(&sm).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            serde_json::from_value(json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;

    let visual_layouts: HashMap<String, VisualLayout> = visual_layouts_from_manager
        .into_iter()
        .map(|(k, v)| {
            let json = serde_json::to_value(&v).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let layout: VisualLayout =
                serde_json::from_value(json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok((k, layout))
        })
        .collect::<Result<HashMap<_, _>, StatusCode>>()?;

    Ok(Json(ImportExportResponse {
        state_machines,
        visual_layouts,
    }))
}

/// Import state machines from JSON
pub async fn import_state_machines(
    State(state): State<ManagementState>,
    Json(request): Json<ImportExportResponse>,
) -> Result<StatusCode, StatusCode> {
    let manager = state.state_machine_manager.write().await;

    // Create manifest from JSON to let serde handle type conversion
    // We need to provide all required fields for ScenarioManifest
    let manifest_json = serde_json::json!({
        "manifest_version": "1.0",
        "name": "imported",
        "version": "1.0.0",
        "title": "Imported State Machines",
        "description": "State machines imported via API",
        "author": "api",
        "category": "other",
        "compatibility": {
            "min_version": "0.1.0",
            "max_version": null
        },
        "files": [],
        "state_machines": request.state_machines,
        "state_machine_graphs": request.visual_layouts
    });

    let manifest: ScenarioManifest =
        serde_json::from_value(manifest_json).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(e) = manager.load_from_manifest(&manifest).await {
        error!("Failed to import state machines: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Visual layouts are already set in the manifest, no need to set separately

    Ok(StatusCode::CREATED)
}

/// Create the state machine API router
///
/// This function creates routes that use ManagementState, so they can be
/// nested within the management router.
pub fn create_state_machine_routes() -> axum::Router<ManagementState> {
    use axum::{
        routing::{delete, get, post, put},
        Router,
    };

    Router::new()
        // State machine CRUD
        .route("/", get(list_state_machines))
        .route("/", post(create_state_machine))
        .route("/{resource_type}", get(get_state_machine))
        .route("/{resource_type}", put(create_state_machine))
        .route("/{resource_type}", delete(delete_state_machine))

        // State instance operations
        .route("/instances", get(list_instances))
        .route("/instances", post(create_instance))
        .route("/instances/{resource_id}", get(get_instance))
        .route("/instances/{resource_id}/state", get(get_current_state))
        .route("/instances/{resource_id}/next-states", get(get_next_states))
        .route("/instances/{resource_id}/transition", post(execute_transition))

        // Import/Export
        .route("/export", get(export_state_machines))
        .route("/import", post(import_state_machines))
}
