//! Consistency engine API handlers
//!
//! This module provides HTTP handlers for managing unified state across protocols.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
// ChaosScenario is now serde_json::Value to avoid circular dependency
use mockforge_core::consistency::{
    enrich_order_response, enrich_response_via_graph, enrich_user_response,
    get_user_orders_via_graph, ConsistencyEngine, EntityState, UnifiedState,
};
use mockforge_core::reality::RealityLevel;
use mockforge_data::{LifecycleState, PersonaLifecycle, PersonaProfile};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
    pub rule: JsonValue, // ChaosScenario as JSON value
}

/// Request to deactivate chaos rule
#[derive(Debug, Deserialize)]
pub struct DeactivateChaosRuleRequest {
    /// Rule name
    pub rule_name: String,
}

/// Request to set persona lifecycle state
#[derive(Debug, Deserialize)]
pub struct SetPersonaLifecycleRequest {
    /// Persona ID
    pub persona_id: String,
    /// Initial lifecycle state
    pub initial_state: String,
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
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
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

    state.engine.set_reality_level(&params.workspace, level).await.map_err(|e| {
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
    let mut entity = EntityState::new(request.entity_type, request.entity_id, request.data);
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
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
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

/// Set persona lifecycle state
///
/// POST /api/v1/consistency/persona/lifecycle?workspace={workspace_id}
pub async fn set_persona_lifecycle(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
    Json(request): Json<SetPersonaLifecycleRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Parse lifecycle state
    let lifecycle_state = match request.initial_state.to_lowercase().as_str() {
        "new" | "new_signup" => LifecycleState::NewSignup,
        "active" => LifecycleState::Active,
        "power_user" | "poweruser" => LifecycleState::PowerUser,
        "churn_risk" | "churnrisk" => LifecycleState::ChurnRisk,
        "churned" => LifecycleState::Churned,
        "upgrade_pending" | "upgradepending" => LifecycleState::UpgradePending,
        "payment_failed" | "paymentfailed" => LifecycleState::PaymentFailed,
        _ => {
            error!("Invalid lifecycle state: {}", request.initial_state);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Get unified state to access active persona
    let unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        error!("State not found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    // Update persona lifecycle if active persona matches
    if let Some(ref persona) = unified_state.active_persona {
        if persona.id == request.persona_id {
            let mut persona_mut = persona.clone();
            let lifecycle = PersonaLifecycle::new(request.persona_id.clone(), lifecycle_state);
            persona_mut.set_lifecycle(lifecycle);

            // Apply lifecycle effects to persona traits
            if let Some(ref lifecycle) = persona_mut.lifecycle {
                let effects = lifecycle.apply_lifecycle_effects();
                for (key, value) in effects {
                    persona_mut.set_trait(key, value);
                }
            }

            // Update the persona in the engine
            state
                .engine
                .set_active_persona(&params.workspace, persona_mut)
                .await
                .map_err(|e| {
                    error!("Failed to set persona lifecycle: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            info!(
                "Set lifecycle state {} for persona {} in workspace: {}",
                request.initial_state, request.persona_id, params.workspace
            );

            return Ok(Json(serde_json::json!({
                "success": true,
                "workspace": params.workspace,
                "persona_id": request.persona_id,
                "lifecycle_state": request.initial_state,
            })));
        }
    }

    error!(
        "Persona {} not found or not active in workspace: {}",
        request.persona_id, params.workspace
    );
    Err(StatusCode::NOT_FOUND)
}

/// Get user by ID with persona graph enrichment
///
/// GET /api/v1/consistency/users/{id}?workspace={workspace_id}
/// This endpoint uses the persona graph to enrich the user response with related entities.
pub async fn get_user_with_graph(
    State(state): State<ConsistencyState>,
    Path(user_id): Path<String>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Get user entity
    let mut user_entity = state
        .engine
        .get_entity(&params.workspace, "user", &user_id)
        .await
        .ok_or_else(|| {
            error!("User not found: {} in workspace: {}", user_id, params.workspace);
            StatusCode::NOT_FOUND
        })?;

    // Enrich response with persona graph data
    let mut response = user_entity.data.clone();
    enrich_user_response(&state.engine, &params.workspace, &user_id, &mut response).await;

    Ok(Json(response))
}

/// Get orders for a user using persona graph
///
/// GET /api/v1/consistency/users/{id}/orders?workspace={workspace_id}
/// This endpoint uses the persona graph to find all orders related to the user.
pub async fn get_user_orders_with_graph(
    State(state): State<ConsistencyState>,
    Path(user_id): Path<String>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Verify user exists
    state
        .engine
        .get_entity(&params.workspace, "user", &user_id)
        .await
        .ok_or_else(|| {
            error!("User not found: {} in workspace: {}", user_id, params.workspace);
            StatusCode::NOT_FOUND
        })?;

    // Get orders via persona graph
    let orders = get_user_orders_via_graph(&state.engine, &params.workspace, &user_id).await;

    // Convert to JSON response
    let orders_json: Vec<Value> = orders.iter().map(|e| e.data.clone()).collect();

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "orders": orders_json,
        "count": orders_json.len(),
    })))
}

/// Get order by ID with persona graph enrichment
///
/// GET /api/v1/consistency/orders/{id}?workspace={workspace_id}
/// This endpoint uses the persona graph to enrich the order response with related entities.
pub async fn get_order_with_graph(
    State(state): State<ConsistencyState>,
    Path(order_id): Path<String>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Get order entity
    let mut order_entity = state
        .engine
        .get_entity(&params.workspace, "order", &order_id)
        .await
        .ok_or_else(|| {
            error!("Order not found: {} in workspace: {}", order_id, params.workspace);
            StatusCode::NOT_FOUND
        })?;

    // Enrich response with persona graph data
    let mut response = order_entity.data.clone();
    enrich_order_response(&state.engine, &params.workspace, &order_id, &mut response).await;

    Ok(Json(response))
}

/// Update persona lifecycle states based on current virtual time
///
/// POST /api/v1/consistency/persona/update-lifecycles?workspace={workspace_id}
/// This endpoint checks all active personas and updates their lifecycle states
/// based on elapsed time since state entry, using virtual time if time travel is enabled.
pub async fn update_persona_lifecycles(
    State(state): State<ConsistencyState>,
    Query(params): Query<WorkspaceQuery>,
) -> Result<Json<Value>, StatusCode> {
    use mockforge_core::time_travel::now as get_virtual_time;

    // Get unified state
    let mut unified_state = state.engine.get_state(&params.workspace).await.ok_or_else(|| {
        error!("State not found for workspace: {}", params.workspace);
        StatusCode::NOT_FOUND
    })?;

    // Get current time (virtual if time travel is enabled, real otherwise)
    let current_time = get_virtual_time();

    // Update lifecycle state for active persona if present
    let mut updated = false;
    if let Some(ref mut persona) = unified_state.active_persona {
        let old_state = persona
            .lifecycle
            .as_ref()
            .map(|l| l.current_state)
            .unwrap_or(mockforge_data::LifecycleState::Active);

        // Update lifecycle state based on elapsed time
        persona.update_lifecycle_state(current_time);

        let new_state = persona
            .lifecycle
            .as_ref()
            .map(|l| l.current_state)
            .unwrap_or(mockforge_data::LifecycleState::Active);

        if old_state != new_state {
            updated = true;
            info!(
                "Persona {} lifecycle state updated: {:?} -> {:?}",
                persona.id, old_state, new_state
            );

            // Update the persona in the engine
            state
                .engine
                .set_active_persona(&params.workspace, persona.clone())
                .await
                .map_err(|e| {
                    error!("Failed to update persona lifecycle: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "workspace": params.workspace,
        "updated": updated,
        "current_time": current_time.to_rfc3339(),
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
        .route("/api/v1/consistency/persona/lifecycle", post(set_persona_lifecycle))
        .route("/api/v1/consistency/persona/update-lifecycles", post(update_persona_lifecycles))
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
        // Persona graph-enabled endpoints
        .route("/api/v1/consistency/users/:id", get(get_user_with_graph))
        .route("/api/v1/consistency/users/:id/orders", get(get_user_orders_with_graph))
        .route("/api/v1/consistency/orders/:id", get(get_order_with_graph))
        // Chaos rule management
        .route("/api/v1/consistency/chaos/activate", post(activate_chaos_rule))
        .route("/api/v1/consistency/chaos/deactivate", post(deactivate_chaos_rule))
        .with_state(state)
}
