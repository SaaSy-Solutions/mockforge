//! Persona graph response enrichment
//!
//! This module provides utilities for enriching HTTP responses using the persona graph,
//! ensuring that related entities are coherently linked across endpoints.

use crate::consistency::{ConsistencyEngine, EntityState};
#[cfg(feature = "persona-graph")]
use mockforge_data::PersonaGraph;
use serde_json::Value;
use tracing::debug;
#[cfg(not(feature = "persona-graph"))]
type PersonaGraph = ();

/// Enrich a user response with related entities from the persona graph
///
/// Given a user ID and response data, this function:
/// 1. Finds the user's persona in the graph
/// 2. Finds related entities (orders, accounts, etc.) via the graph
/// 3. Enriches the response with related entity IDs or summaries
#[cfg(feature = "persona-graph")]
pub async fn enrich_user_response(
    engine: &ConsistencyEngine,
    workspace_id: &str,
    user_id: &str,
    response: &mut Value,
) {
    // Get unified state to access persona graph
    let state = match engine.get_state(workspace_id).await {
        Some(s) => s,
        None => {
            debug!("No unified state found for workspace: {}", workspace_id);
            return;
        }
    };

    let graph = match state.persona_graph() {
        Some(g) => g,
        None => {
            debug!("No persona graph found for workspace: {}", workspace_id);
            return;
        }
    };

    // Find user persona ID
    let user_persona_id = format!("user:{}", user_id);

    // Find related orders via persona graph
    let related_order_ids =
        graph.find_related_by_entity_type(&user_persona_id, "order", Some("has_orders"));

    // Enrich response with related order IDs
    if let Some(obj) = response.as_object_mut() {
        if !related_order_ids.is_empty() {
            // Extract just the order IDs (remove "order:" prefix)
            let order_ids: Vec<String> = related_order_ids
                .iter()
                .filter_map(|pid| pid.strip_prefix("order:").map(|s| s.to_string()))
                .collect();

            obj.insert(
                "order_ids".to_string(),
                Value::Array(order_ids.iter().map(|id| Value::String(id.clone())).collect()),
            );
            obj.insert("order_count".to_string(), Value::Number(order_ids.len().into()));
        }

        // Find related accounts
        let related_account_ids =
            graph.find_related_by_entity_type(&user_persona_id, "account", Some("has_accounts"));

        if !related_account_ids.is_empty() {
            let account_ids: Vec<String> = related_account_ids
                .iter()
                .filter_map(|pid| pid.strip_prefix("account:").map(|s| s.to_string()))
                .collect();

            obj.insert(
                "account_ids".to_string(),
                Value::Array(account_ids.iter().map(|id| Value::String(id.clone())).collect()),
            );
        }
    }
}

/// Get orders for a user using the persona graph
///
/// This function:
/// 1. Finds the user's persona in the graph
/// 2. Traverses the graph to find related order personas
/// 3. Returns entity states for those orders
#[cfg(feature = "persona-graph")]
pub async fn get_user_orders_via_graph(
    engine: &ConsistencyEngine,
    workspace_id: &str,
    user_id: &str,
) -> Vec<EntityState> {
    // Get unified state to access persona graph
    let state = match engine.get_state(workspace_id).await {
        Some(s) => s,
        None => {
            debug!("No unified state found for workspace: {}", workspace_id);
            return Vec::new();
        }
    };

    let graph = match state.persona_graph() {
        Some(g) => g,
        None => {
            debug!("No persona graph found for workspace: {}", workspace_id);
            return Vec::new();
        }
    };

    // Find user persona ID
    let user_persona_id = format!("user:{}", user_id);

    // Find related order personas via graph
    let related_order_persona_ids =
        graph.find_related_by_entity_type(&user_persona_id, "order", Some("has_orders"));

    // Convert persona IDs to entity states
    let mut orders = Vec::new();
    for order_persona_id in related_order_persona_ids {
        // Extract order ID from persona ID (format: "order:order_id")
        if let Some((_, order_id)) = order_persona_id.split_once(':') {
            // Try to get entity from unified state
            if let Some(entity) = state.get_entity("order", order_id) {
                orders.push(entity.clone());
            } else {
                // If entity doesn't exist in unified state, try to get from engine
                if let Some(entity) = engine.get_entity(workspace_id, "order", order_id).await {
                    orders.push(entity);
                }
            }
        }
    }

    orders
}

/// Enrich an order response with related entities from the persona graph
///
/// Given an order ID and response data, this function:
/// 1. Finds the order's persona in the graph
/// 2. Finds related entities (user, payments, etc.) via the graph
/// 3. Enriches the response with related entity information
#[cfg(feature = "persona-graph")]
pub async fn enrich_order_response(
    engine: &ConsistencyEngine,
    workspace_id: &str,
    order_id: &str,
    response: &mut Value,
) {
    // Get unified state to access persona graph
    let state = match engine.get_state(workspace_id).await {
        Some(s) => s,
        None => {
            debug!("No unified state found for workspace: {}", workspace_id);
            return;
        }
    };

    let graph = match state.persona_graph() {
        Some(g) => g,
        None => {
            debug!("No persona graph found for workspace: {}", workspace_id);
            return;
        }
    };

    // Find order persona ID
    let order_persona_id = format!("order:{}", order_id);

    // Find related payments via persona graph
    let related_payment_ids =
        graph.find_related_by_entity_type(&order_persona_id, "payment", Some("has_payments"));

    // Enrich response with related payment IDs
    if let Some(obj) = response.as_object_mut() {
        if !related_payment_ids.is_empty() {
            let payment_ids: Vec<String> = related_payment_ids
                .iter()
                .filter_map(|pid| pid.strip_prefix("payment:").map(|s| s.to_string()))
                .collect();

            obj.insert(
                "payment_ids".to_string(),
                Value::Array(payment_ids.iter().map(|id| Value::String(id.clone())).collect()),
            );
        }

        // Find user by traversing backwards in the graph
        let edges_to = graph.get_edges_to(&order_persona_id);
        for edge in edges_to {
            if edge.relationship_type == "has_orders" {
                // This edge comes from a user
                if let Some((entity_type, user_id)) = edge.from.split_once(':') {
                    if entity_type == "user" {
                        obj.insert("user_id".to_string(), Value::String(user_id.to_string()));
                        // Try to get user entity to include more details
                        if let Some(user_entity) = state.get_entity("user", user_id) {
                            if let Some(user_data) = user_entity.data.as_object() {
                                if let Some(name) = user_data.get("name") {
                                    obj.insert("user_name".to_string(), name.clone());
                                }
                                if let Some(email) = user_data.get("email") {
                                    obj.insert("user_email".to_string(), email.clone());
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// Enrich a response based on endpoint path and entity type
///
/// This is a convenience function that routes to the appropriate enrichment
/// function based on the endpoint path.
#[cfg(feature = "persona-graph")]
pub async fn enrich_response_via_graph(
    engine: &ConsistencyEngine,
    workspace_id: &str,
    path: &str,
    entity_id: Option<&str>,
    response: &mut Value,
) {
    // Extract entity type and ID from path
    let path_lower = path.to_lowercase();

    if path_lower.contains("/users/") {
        if let Some(id) = entity_id {
            enrich_user_response(engine, workspace_id, id, response).await;
        }
    } else if path_lower.contains("/orders/") {
        if let Some(id) = entity_id {
            enrich_order_response(engine, workspace_id, id, response).await;
        }
    }
    // Add more entity types as needed
}

/// Stub implementations when persona-graph feature is disabled
///
/// These functions are no-ops when the persona-graph feature is disabled.
#[cfg(not(feature = "persona-graph"))]
pub async fn enrich_user_response(
    _engine: &ConsistencyEngine,
    _workspace_id: &str,
    _user_id: &str,
    _response: &mut Value,
) {
    // No-op when feature is disabled
}

/// Stub: Get user orders via graph (returns empty when feature disabled)
#[cfg(not(feature = "persona-graph"))]
pub async fn get_user_orders_via_graph(
    _engine: &ConsistencyEngine,
    _workspace_id: &str,
    _user_id: &str,
) -> Vec<EntityState> {
    Vec::new()
}

/// Stub: Enrich order response (no-op when feature disabled)
#[cfg(not(feature = "persona-graph"))]
pub async fn enrich_order_response(
    _engine: &ConsistencyEngine,
    _workspace_id: &str,
    _order_id: &str,
    _response: &mut Value,
) {
    // No-op when feature is disabled
}

/// Stub: Enrich response via graph (no-op when feature disabled)
#[cfg(not(feature = "persona-graph"))]
pub async fn enrich_response_via_graph(
    _engine: &ConsistencyEngine,
    _workspace_id: &str,
    _path: &str,
    _entity_id: Option<&str>,
    _response: &mut Value,
) {
    // No-op when feature is disabled
}
