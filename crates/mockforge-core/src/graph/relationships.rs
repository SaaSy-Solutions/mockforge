//! Relationship discovery for graph visualization
//!
//! This module analyzes various data sources to discover relationships
//! between endpoints, services, and chains.

use crate::graph::{EdgeType, GraphEdge};
use crate::request_chaining::{ChainDefinition, ChainLink};
use std::collections::HashMap;

/// Discover relationships from chain definitions
pub fn discover_chain_relationships(chains: &[ChainDefinition]) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    for chain in chains {
        for link in &chain.links {
            let link_node_id = format!("chain_link:{}:{}", chain.id, link.request.id);

            // Create dependency edges
            for dep_id in &link.request.depends_on {
                let dep_node_id = format!("chain_link:{}:{}", chain.id, dep_id);
                edges.push(GraphEdge {
                    from: dep_node_id,
                    to: link_node_id.clone(),
                    edge_type: EdgeType::Dependency,
                    label: Some("depends on".to_string()),
                    metadata: HashMap::new(),
                });
            }

            // Try to discover cross-service calls from URLs
            if let Some(service_call) = discover_service_call_from_url(&link.request.url) {
                edges.push(service_call);
            }
        }
    }

    edges
}

/// Discover service call relationships from URL patterns
fn discover_service_call_from_url(url: &str) -> Option<GraphEdge> {
    // Simple heuristic: if URL contains a different service identifier
    // This is a placeholder - in production, you'd want more sophisticated
    // URL parsing and service discovery
    if url.contains("://") {
        // Could parse URL and identify service boundaries
        // For now, return None as this requires more context
        None
    } else {
        None
    }
}

/// Discover state transition relationships from state machines
pub fn discover_state_transitions(
    state_machines: &[crate::intelligent_behavior::rules::StateMachine],
) -> Vec<GraphEdge> {
    use serde_json;
    let mut edges = Vec::new();

    for state_machine in state_machines {
        for transition in &state_machine.transitions {
            // Create edges for state transitions
            // Note: This requires mapping states to endpoint nodes
            // For now, we'll create placeholder edges that need to be
            // connected to actual nodes by the builder
            let from_node_id =
                format!("state:{}:{}", state_machine.resource_type, transition.from_state);
            let to_node_id =
                format!("state:{}:{}", state_machine.resource_type, transition.to_state);

            edges.push(GraphEdge {
                from: from_node_id,
                to: to_node_id,
                edge_type: EdgeType::StateTransition,
                label: Some(format!("{} → {}", transition.from_state, transition.to_state)),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert(
                        "resource_type".to_string(),
                        serde_json::Value::String(state_machine.resource_type.clone()),
                    );
                    meta.insert(
                        "probability".to_string(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(transition.probability)
                                .unwrap_or_else(|| serde_json::Number::from(0)),
                        ),
                    );
                    meta
                },
            });
        }
    }

    edges
}

/// Extract endpoint IDs from chain links
pub fn extract_endpoint_ids_from_chain(chain: &ChainDefinition) -> Vec<String> {
    chain.links.iter().map(|link| link.request.id.clone()).collect()
}

/// Group endpoints by service based on URL patterns
pub fn group_endpoints_by_service(
    endpoints: &[(String, String)], // (endpoint_id, url)
) -> HashMap<String, Vec<String>> {
    let mut service_groups: HashMap<String, Vec<String>> = HashMap::new();

    for (endpoint_id, url) in endpoints {
        // Simple heuristic: extract service name from URL
        // In production, this would be more sophisticated
        let service_name = extract_service_name_from_url(url);
        service_groups
            .entry(service_name)
            .or_insert_with(Vec::new)
            .push(endpoint_id.clone());
    }

    service_groups
}

/// Extract service name from URL
fn extract_service_name_from_url(url: &str) -> String {
    // Simple heuristic: use domain or first path segment
    if let Some(domain) = url.split("://").nth(1) {
        if let Some(host) = domain.split('/').next() {
            return host.split('.').next().unwrap_or("default").to_string();
        }
    }

    // Fallback: use first path segment
    if let Some(first_segment) = url.split('/').nth(1) {
        return first_segment.to_string();
    }

    "default".to_string()
}
