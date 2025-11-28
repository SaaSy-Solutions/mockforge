//! Graph visualization handlers
//!
//! These handlers provide graph data for visualizing mock environments,
//! endpoints, their relationships, and state transitions.

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Json,
    },
};
use futures_util::stream::{self, Stream};
use mockforge_core::graph::GraphBuilder;
use mockforge_core::request_chaining::ChainDefinition;
use serde_json::Value;
use std::convert::Infallible;
use std::time::Duration;

use super::AdminState;
use crate::models::ApiResponse;

/// Get graph data for visualization
///
/// This endpoint aggregates data from multiple sources:
/// - Endpoints from UI Builder API
/// - Request chains
/// - State machines (if available)
/// - Workspaces/services
pub async fn get_graph(State(state): State<AdminState>) -> impl IntoResponse {
    let mut builder = GraphBuilder::new();

    // Fetch chains from the HTTP server
    if let Some(http_addr) = state.http_server_addr {
        match fetch_chains_from_server(http_addr).await {
            Ok(chains) => {
                builder.from_chains(&chains);
            }
            Err(e) => {
                tracing::warn!("Failed to fetch chains for graph: {}", e);
                // Continue without chains - graph will still work
            }
        }
    }

    // Fetch endpoints from UI Builder API if available
    if let Some(http_addr) = state.http_server_addr {
        match fetch_endpoints_from_ui_builder(http_addr).await {
            Ok(endpoints) => {
                // Convert UI Builder endpoints to graph format
                for endpoint in endpoints {
                    let protocol_str = match endpoint.protocol {
                        mockforge_http::ui_builder::Protocol::Http => "http",
                        mockforge_http::ui_builder::Protocol::Grpc => "grpc",
                        mockforge_http::ui_builder::Protocol::Websocket => "websocket",
                        mockforge_http::ui_builder::Protocol::Graphql => "graphql",
                        mockforge_http::ui_builder::Protocol::Mqtt => "mqtt",
                        mockforge_http::ui_builder::Protocol::Smtp => "smtp",
                        mockforge_http::ui_builder::Protocol::Kafka => "kafka",
                        mockforge_http::ui_builder::Protocol::Amqp => "amqp",
                        mockforge_http::ui_builder::Protocol::Ftp => "ftp",
                    };

                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("enabled".to_string(), Value::Bool(endpoint.enabled));
                    if let Some(desc) = endpoint.description {
                        metadata.insert("description".to_string(), Value::String(desc));
                    }

                    // Extract method and path from HTTP config if available
                    if let mockforge_http::ui_builder::EndpointProtocolConfig::Http(http_config) =
                        &endpoint.config
                    {
                        metadata.insert(
                            "method".to_string(),
                            Value::String(http_config.method.clone()),
                        );
                        metadata
                            .insert("path".to_string(), Value::String(http_config.path.clone()));
                    }

                    let protocol = match protocol_str {
                        "http" => mockforge_core::graph::Protocol::Http,
                        "grpc" => mockforge_core::graph::Protocol::Grpc,
                        "websocket" => mockforge_core::graph::Protocol::Websocket,
                        "graphql" => mockforge_core::graph::Protocol::Graphql,
                        "mqtt" => mockforge_core::graph::Protocol::Mqtt,
                        "smtp" => mockforge_core::graph::Protocol::Smtp,
                        "kafka" => mockforge_core::graph::Protocol::Kafka,
                        "amqp" => mockforge_core::graph::Protocol::Amqp,
                        "ftp" => mockforge_core::graph::Protocol::Ftp,
                        _ => mockforge_core::graph::Protocol::Http,
                    };

                    builder.add_endpoint(endpoint.id, endpoint.name, protocol, metadata);
                }
            }
            Err(e) => {
                tracing::debug!("UI Builder endpoints not available: {}", e);
                // Continue without endpoints - graph will still work with chains
            }
        }
    }

    // Build the graph
    let graph_data = builder.build();

    Json(ApiResponse::success(graph_data))
}

/// Fetch endpoints from UI Builder API
async fn fetch_endpoints_from_ui_builder(
    http_addr: std::net::SocketAddr,
) -> Result<Vec<mockforge_http::ui_builder::EndpointConfig>, String> {
    let url = format!("http://{}/__mockforge/ui-builder/endpoints", http_addr);
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch endpoints: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: Value =
        response.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract endpoints from response
    // Assuming it returns: { "endpoints": [...] } or { "data": { "endpoints": [...] } }
    let endpoints_array = json
        .get("endpoints")
        .or_else(|| json.get("data").and_then(|d| d.get("endpoints")))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Invalid response format: endpoints array not found".to_string())?;

    let mut endpoints = Vec::new();
    for endpoint_value in endpoints_array {
        if let Ok(endpoint) = serde_json::from_value::<mockforge_http::ui_builder::EndpointConfig>(
            endpoint_value.clone(),
        ) {
            endpoints.push(endpoint);
        }
    }

    Ok(endpoints)
}

/// SSE endpoint for real-time graph updates
pub async fn graph_sse(
    State(state): State<AdminState>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    tracing::info!("SSE endpoint /graph/sse accessed - starting real-time graph updates");

    // Clone state for use in the stream
    let http_addr = state.http_server_addr;

    let stream = stream::unfold((), move |_| {
        let http_addr = http_addr;
        async move {
            tokio::time::sleep(Duration::from_secs(5)).await; // Update every 5 seconds

            // Build graph data (same logic as get_graph)
            let mut builder = GraphBuilder::new();

            // Fetch chains
            if let Some(addr) = http_addr {
                if let Ok(chains) = fetch_chains_from_server(addr).await {
                    builder.from_chains(&chains);
                }

                // Fetch endpoints from UI Builder
                if let Ok(endpoints) = fetch_endpoints_from_ui_builder(addr).await {
                    for endpoint in endpoints {
                        let protocol_str = match endpoint.protocol {
                            mockforge_http::ui_builder::Protocol::Http => "http",
                            mockforge_http::ui_builder::Protocol::Grpc => "grpc",
                            mockforge_http::ui_builder::Protocol::Websocket => "websocket",
                            mockforge_http::ui_builder::Protocol::Graphql => "graphql",
                            mockforge_http::ui_builder::Protocol::Mqtt => "mqtt",
                            mockforge_http::ui_builder::Protocol::Smtp => "smtp",
                            mockforge_http::ui_builder::Protocol::Kafka => "kafka",
                            mockforge_http::ui_builder::Protocol::Amqp => "amqp",
                            mockforge_http::ui_builder::Protocol::Ftp => "ftp",
                        };

                        let mut metadata = std::collections::HashMap::new();
                        metadata.insert("enabled".to_string(), Value::Bool(endpoint.enabled));
                        if let Some(desc) = endpoint.description {
                            metadata.insert("description".to_string(), Value::String(desc));
                        }

                        if let mockforge_http::ui_builder::EndpointProtocolConfig::Http(
                            http_config,
                        ) = &endpoint.config
                        {
                            metadata.insert(
                                "method".to_string(),
                                Value::String(http_config.method.clone()),
                            );
                            metadata.insert(
                                "path".to_string(),
                                Value::String(http_config.path.clone()),
                            );
                        }

                        let protocol = match protocol_str {
                            "http" => mockforge_core::graph::Protocol::Http,
                            "grpc" => mockforge_core::graph::Protocol::Grpc,
                            "websocket" => mockforge_core::graph::Protocol::Websocket,
                            "graphql" => mockforge_core::graph::Protocol::Graphql,
                            "mqtt" => mockforge_core::graph::Protocol::Mqtt,
                            "smtp" => mockforge_core::graph::Protocol::Smtp,
                            "kafka" => mockforge_core::graph::Protocol::Kafka,
                            "amqp" => mockforge_core::graph::Protocol::Amqp,
                            "ftp" => mockforge_core::graph::Protocol::Ftp,
                            _ => mockforge_core::graph::Protocol::Http,
                        };

                        builder.add_endpoint(endpoint.id, endpoint.name, protocol, metadata);
                    }
                }
            }

            let graph_data = builder.build();
            let json_data = serde_json::to_string(&graph_data).unwrap_or_default();

            Some((Ok(Event::default().data(json_data)), ()))
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

/// Fetch chains from the HTTP server
async fn fetch_chains_from_server(
    http_addr: std::net::SocketAddr,
) -> Result<Vec<ChainDefinition>, String> {
    let url = format!("http://{}/__mockforge/chains", http_addr);
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch chains: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: Value =
        response.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract chains from the response
    // The response format depends on the chain API implementation
    // Assuming it returns: { "chains": [...] } or { "data": { "chains": [...] } }
    let chains_array = json
        .get("chains")
        .or_else(|| json.get("data").and_then(|d| d.get("chains")))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Invalid response format: chains array not found".to_string())?;

    let mut chains = Vec::new();
    for chain_value in chains_array {
        // Try to get the full chain definition
        // First try to get by ID and fetch full details
        if let Some(chain_id) = chain_value.get("id").and_then(|v| v.as_str()) {
            match fetch_chain_details(http_addr, chain_id).await {
                Ok(Some(chain)) => chains.push(chain),
                Ok(None) => {
                    // Chain not found, skip
                    tracing::warn!("Chain {} not found, skipping", chain_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch chain {}: {}", chain_id, e);
                    // Try to parse from summary if available
                    if let Ok(chain) =
                        serde_json::from_value::<ChainDefinition>(chain_value.clone())
                    {
                        chains.push(chain);
                    }
                }
            }
        }
    }

    Ok(chains)
}

/// Fetch full chain details by ID
async fn fetch_chain_details(
    http_addr: std::net::SocketAddr,
    chain_id: &str,
) -> Result<Option<ChainDefinition>, String> {
    let url = format!("http://{}/__mockforge/chains/{}", http_addr, chain_id);
    let client = reqwest::Client::new();

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch chain details: {}", e))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: Value =
        response.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract chain from response
    // Assuming it returns: { "chain": {...} } or { "data": {...} } or just the chain object
    let chain_value = json.get("chain").or_else(|| json.get("data")).unwrap_or(&json);

    serde_json::from_value::<ChainDefinition>(chain_value.clone())
        .map(Some)
        .map_err(|e| format!("Failed to deserialize chain: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn test_graph_builder_creation() {
        let builder = GraphBuilder::new();
        let graph = builder.build();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.clusters.len(), 0);
    }
}
