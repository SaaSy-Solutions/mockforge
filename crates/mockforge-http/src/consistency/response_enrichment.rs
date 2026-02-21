//! Response enrichment utilities for cross-protocol consistency
//!
//! This module provides utilities for enriching HTTP responses with persona graph
//! data and lifecycle effects based on the unified state from the consistency engine.

use axum::extract::Request;
use mockforge_core::consistency::UnifiedState;
#[cfg(feature = "persona-graph")]
use mockforge_data::apply_lifecycle_effects;
use serde_json::Value;

/// Enrich a response with persona graph data and lifecycle effects
///
/// This function:
/// 1. Extracts UnifiedState from request extensions
/// 2. Enriches the response with persona graph data if available
/// 3. Applies lifecycle effects based on the persona's lifecycle state
/// 4. Returns the enriched response
pub async fn enrich_response(
    request: &Request,
    mut response: Value,
    _workspace_id: &str,
    _endpoint_type: Option<&str>,
) -> Value {
    // Extract UnifiedState from request extensions
    let unified_state = request.extensions().get::<UnifiedState>();

    if let Some(_state) = unified_state {
        // Enrich with persona graph if available
        #[cfg(feature = "persona-graph")]
        if let Some(ref persona) = _state.active_persona {
            // Determine endpoint type from path if not provided
            let endpoint_type = _endpoint_type.unwrap_or_else(|| {
                let path = request.uri().path();
                if path.contains("/users/") || path.contains("/user/") {
                    "user"
                } else if path.contains("/orders/") || path.contains("/order/") {
                    "order"
                } else if path.contains("/billing")
                    || path.contains("/payment")
                    || path.contains("/subscription")
                {
                    "billing"
                } else if path.contains("/support") || path.contains("/tickets") {
                    "support"
                } else if path.contains("/fulfillment")
                    || path.contains("/shipment")
                    || path.contains("/delivery")
                {
                    "fulfillment"
                } else if path.contains("/loan")
                    || path.contains("/credit")
                    || path.contains("/application")
                {
                    "loan"
                } else {
                    "default"
                }
            });

            // Extract entity ID from path if available (e.g., /users/123 -> "123")
            let entity_id = extract_entity_id_from_path(request.uri().path());

            // Enrich response with persona graph data if graph is available
            if let Some(graph) = _state.persona_graph() {
                enrich_with_persona_graph(
                    graph,
                    &persona.id,
                    &mut response,
                    request.uri().path(),
                    entity_id.as_deref(),
                );
            }

            // Apply lifecycle effects if persona has a lifecycle
            if let Some(ref lifecycle) = persona.lifecycle {
                apply_lifecycle_effects(&mut response, lifecycle, endpoint_type);
            }
        }
    }

    response
}

/// Extract entity ID from path (e.g., /users/123 -> Some("123"))
#[cfg(feature = "persona-graph")]
fn extract_entity_id_from_path(path: &str) -> Option<String> {
    // Simple extraction: find the last segment that looks like an ID
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() >= 2 {
        // Check if last segment looks like an ID (not a query param)
        let last = segments.last()?;
        if !last.contains('?') && !last.contains('&') {
            return Some(last.to_string());
        }
    }
    None
}

/// Enrich response with persona graph data
#[cfg(feature = "persona-graph")]
fn enrich_with_persona_graph(
    graph: &mockforge_data::PersonaGraph,
    persona_id: &str,
    response: &mut Value,
    path: &str,
    entity_id: Option<&str>,
) {
    use mockforge_data::PersonaGraph;

    // Determine entity type from path
    let path_lower = path.to_lowercase();
    let entity_type = if path_lower.contains("/users/") || path_lower.contains("/user/") {
        "user"
    } else if path_lower.contains("/orders/") || path.contains("/order/") {
        "order"
    } else {
        return; // Unknown entity type
    };

    // Find related entities via persona graph
    if let Some(id) = entity_id {
        let entity_persona_id = format!("{}:{}", entity_type, id);

        // Find related orders for users
        if entity_type == "user" {
            let related_orders =
                graph.find_related_by_entity_type(&entity_persona_id, "order", Some("has_orders"));

            if let Some(obj) = response.as_object_mut() {
                if !related_orders.is_empty() {
                    let order_ids: Vec<String> = related_orders
                        .iter()
                        .filter_map(|pid| pid.strip_prefix("order:").map(|s| s.to_string()))
                        .collect();

                    obj.insert(
                        "order_ids".to_string(),
                        Value::Array(
                            order_ids.iter().map(|id| Value::String(id.clone())).collect(),
                        ),
                    );
                    obj.insert("order_count".to_string(), Value::Number(order_ids.len().into()));
                }
            }
        }

        // Find related payments for orders
        if entity_type == "order" {
            let related_payments = graph.find_related_by_entity_type(
                &entity_persona_id,
                "payment",
                Some("has_payment"),
            );

            if let Some(obj) = response.as_object_mut() {
                if !related_payments.is_empty() {
                    let payment_ids: Vec<String> = related_payments
                        .iter()
                        .filter_map(|pid| pid.strip_prefix("payment:").map(|s| s.to_string()))
                        .collect();

                    obj.insert(
                        "payment_ids".to_string(),
                        Value::Array(
                            payment_ids.iter().map(|id| Value::String(id.clone())).collect(),
                        ),
                    );
                }
            }
        }
    }
}

/// Extract workspace ID from request
///
/// Priority: X-MockForge-Workspace header > query param > default
pub fn extract_workspace_id(request: &Request) -> String {
    request
        .headers()
        .get("X-MockForge-Workspace")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            request.uri().query().and_then(|q| {
                q.split('&').find_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    if parts.next() == Some("workspace") {
                        parts.next().and_then(|v| {
                            urlencoding::decode(v).ok().map(|decoded| decoded.to_string())
                        })
                    } else {
                        None
                    }
                })
            })
        })
        .unwrap_or_else(|| "default".to_string())
}
