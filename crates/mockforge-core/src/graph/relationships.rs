//! Relationship discovery for graph visualization
//!
//! This module analyzes various data sources to discover relationships
//! between endpoints, services, and chains.

use crate::graph::{EdgeType, GraphEdge};
use crate::request_chaining::ChainDefinition;
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

/// Discover service call relationships from URL patterns.
///
/// Extracts service identity from the URL's host component and creates
/// a graph edge representing a cross-service call. Relative URLs (no host)
/// are treated as calls within the same service and return None.
fn discover_service_call_from_url(url: &str) -> Option<GraphEdge> {
    // Only process absolute URLs that reference another service
    let host =
        if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
            // Extract host (everything before the first '/' or end of string)
            let host_part = rest.split('/').next().unwrap_or(rest);
            // Strip port if present
            host_part.split(':').next().unwrap_or(host_part)
        } else {
            // Relative URL — same service, no cross-service edge
            return None;
        };

    if host.is_empty() || host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" {
        return None;
    }

    // Extract the service name from the host (first subdomain or the domain itself)
    let service_name = host.split('.').next().unwrap_or(host);

    // Extract the path for the edge label
    let path = url
        .find("://")
        .and_then(|i| url[i + 3..].find('/'))
        .map(|i| &url[url.find("://").unwrap() + 3 + i..])
        .unwrap_or("/");

    Some(GraphEdge {
        from: "caller".to_string(),
        to: format!("service:{service_name}"),
        edge_type: EdgeType::ServiceCall,
        label: Some(format!("calls {path}")),
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("target_host".to_string(), serde_json::Value::String(host.to_string()));
            meta.insert("url".to_string(), serde_json::Value::String(url.to_string()));
            meta
        },
    })
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
        service_groups.entry(service_name).or_default().push(endpoint_id.clone());
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
