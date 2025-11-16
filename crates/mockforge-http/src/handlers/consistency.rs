//! Consistency engine API handlers
//!
//! This module provides HTTP handlers for managing unified state across protocols.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::consistency::{ConsistencyEngine, EntityState, UnifiedState};
use mockforge_core::reality::RealityLevel;
use mockforge_chaos::ChaosScenario;
use mockforge_data::PersonaProfile;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

/// State for consistency handlers
#[derive(Clone)]
pub struct ConsistencyState {
    /// Consistency engine
    pub engine: Arc<ConsistencyEngine>,
}

/// Request to set active persona
#[derive(Debug, Deserialize)]
pub struct SetPersonaRequest {
    /// Persona profile
    pub persona: PersonaProfile,
}

/// Request to set active scenario
#[derive(Debug, Deserialize)]
pub struct SetScenarioRequest {
    /// Scenario ID
    pub scenario_id: String,
}

/// Request to set reality level
#[derive(Debug, Deserialize)]
pub struct SetRealityLevelRequest {
    /// Reality level (1-5)
    pub level: u8,
}

/// Request to set reality ratio
#[derive(Debug, Deserialize)]
pub struct SetRealityRatioRequest {
    /// Reality ratio (0.0-1.0)
    pub ratio: f64,
}

/// Request to register an entity
#[derive(Debug, Deserialize)]
pub struct RegisterEntityRequest {
    /// Entity type
    pub entity_type: String,
    /// Entity ID
    pub entity_id: String,
    /// Entity data (JSON)
    pub data: Value,
    /// Optional persona ID
    pub persona_id: Option<String>,
}

/// Request to activate chaos rule
#[derive(Debug, Deserialize)]
pub struct ActivateChaosRuleRequest {
    /// Chaos scenario
    pub rule: ChaosScenario,
}

/// Request to deactivate chaos rule
#[derive(Debug, Deserialize)]
pub struct DeactivateChaosRuleRequest {
    /// Rule name
    pub rule_name: String,
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

/// Get unified state for a workspace
///
/// GET /api/v1/consistency/state?workspace={workspace_id}
pub async fn get_state(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<UnifiedState>, StatusCode> {
    let unified_state = state
        .engine
        .get_state(&params.workspace)
        .await
        .ok_or_else(|| {
            error!("State not found for workspace: {}", params.workspace);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(unified_state))
}

/// Set active persona for a workspace
///
/// POST /api/v1/consistency/persona?workspace={workspace_id}
pub async fn set_persona(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<SetPersonaRequest>,
) -> Result<Json<Value>, StatusCode> {
    state
        .engine
        .set_active_persona(&params.workspace, request.persona)
        .await
        .map_err(|e| {
            error!("Failed to set persona: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Set persona for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
    })))
}

/// Set active scenario for a workspace
///
/// POST /api/v1/consistency/scenario?workspace={workspace_id}
pub async fn set_scenario(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<SetScenarioRequest>,
) -> Result<Json<Value>, StatusCode> {
    state
        .engine
        .set_active_scenario(&params.workspace, request.scenario_id)
        .await
        .map_err(|e| {
            error!("Failed to set scenario: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Set scenario for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
    })))
}

/// Set reality level for a workspace
///
/// POST /api/v1/consistency/reality-level?workspace={workspace_id}
pub async fn set_reality_level(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<SetRealityLevelRequest>,
) -> Result<Json<Value>, StatusCode> {
    let level = RealityLevel::from_value(request.level).ok_or_else(|| {
        error!("Invalid reality level: {}", request.level);
        StatusCode::BAD_REQUEST
    })?;

    state
        .engine
        .set_reality_level(&params.workspace, level)
        .await
        .map_err(|e| {
            error!("Failed to set reality level: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Set reality level for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
        "level": request.level,
    })))
}

/// Set reality continuum ratio for a workspace
///
/// POST /api/v1/consistency/reality-ratio?workspace={workspace_id}
pub async fn set_reality_ratio(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<SetRealityRatioRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !(0.0..=1.0).contains(&request.ratio) {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .engine
        .set_reality_ratio(&params.workspace, request.ratio)
        .await
        .map_err(|e| {
            error!("Failed to set reality ratio: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Set reality ratio for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
        "ratio": request.ratio,
    })))
}

/// Register or update an entity
///
/// POST /api/v1/consistency/entities?workspace={workspace_id}
pub async fn register_entity(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<RegisterEntityRequest>,
) -> Result<Json<Value>, StatusCode> {
    let mut entity = EntityState::new(
        request.entity_type,
        request.entity_id,
        request.data,
    );
    if let Some(persona_id) = request.persona_id {
        entity.persona_id = Some(persona_id);
    }

    state
        .engine
        .register_entity(&params.workspace, entity.clone())
        .await
        .map_err(|e| {
            error!("Failed to register entity: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(
        "Registered entity {}:{} for workspace: {}",
        entity.entity_type, entity.entity_id, params.workspace
    );
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
        "entity": entity,
    })))
}

/// Get entity by type and ID
///
/// GET /api/v1/consistency/entities/{entity_type}/{entity_id}?workspace={workspace_id}
pub async fn get_entity(
    State(state): State<ConsistencyState>,
    Path((entity_type, entity_id)): Path<(String, String)>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<EntityState>, StatusCode> {
    let entity = state
        .engine
        .get_entity(&params.workspace, &entity_type, &entity_id)
        .await
        .ok_or_else(|| {
            error!(
                "Entity not found: {}:{} in workspace: {}",
                entity_type, entity_id, params.workspace
            );
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(entity))
}

/// List all entities for a workspace
///
/// GET /api/v1/consistency/entities?workspace={workspace_id}
pub async fn list_entities(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<Value>, StatusCode> {
    let unified_state = state
        .engine
        .get_state(&params.workspace)
        .await
        .ok_or_else(|| {
            error!("State not found for workspace: {}", params.workspace);
            StatusCode::NOT_FOUND
        })?;

    let entities: Vec<&EntityState> = unified_state.entity_state.values().collect();
    Ok(Json(serde_json::json!({
        "workspace": params.workspace,
        "entities": entities,
        "count": entities.len(),
    })))
}

/// Activate a chaos rule
///
/// POST /api/v1/consistency/chaos/activate?workspace={workspace_id}
pub async fn activate_chaos_rule(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<ActivateChaosRuleRequest>,
) -> Result<Json<Value>, StatusCode> {
    state
        .engine
        .activate_chaos_rule(&params.workspace, request.rule)
        .await
        .map_err(|e| {
            error!("Failed to activate chaos rule: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Activated chaos rule for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
    })))
}

/// Deactivate a chaos rule
///
/// POST /api/v1/consistency/chaos/deactivate?workspace={workspace_id}
pub async fn deactivate_chaos_rule(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<DeactivateChaosRuleRequest>,
) -> Result<Json<Value>, StatusCode> {
    state
        .engine
        .deactivate_chaos_rule(&params.workspace, &request.rule_name)
        .await
        .map_err(|e| {
            error!("Failed to deactivate chaos rule: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Deactivated chaos rule for workspace: {}", params.workspace);
    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
        "rule_name": request.rule_name,
    })))
}

/// Create consistency router
pub fn consistency_router(state: ConsistencyState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        // State management
        .route("/api/v1/consistency/state", get(get_state))
        // Persona management
        .route("/api/v1/consistency/persona", post(set_persona))
        // Scenario management
        .route("/api/v1/consistency/scenario", post(set_scenario))
        // Reality level management
        .route("/api/v1/consistency/reality-level", post(set_reality_level))
        // Reality ratio management
        .route("/api/v1/consistency/reality-ratio", post(set_reality_ratio))
        // Entity management
        .route("/api/v1/consistency/entities", get(list_entities).post(register_entity))
        .route(
            "/api/v1/consistency/entities/:entity_type/:entity_id",
            get(get_entity),
        )
        // Chaos rule management
        .route("/api/v1/consistency/chaos/activate", post(activate_chaos_rule))
        .route("/api/v1/consistency/chaos/deactivate", post(deactivate_chaos_rule))
        .with_state(state)
}

