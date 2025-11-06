//! Graph builder for aggregating data from multiple sources
//!
//! This module builds a complete graph representation by aggregating data from:
//! - UI Builder endpoints
//! - Request chains
//! - State machines
//! - Workspaces/services

use crate::graph::{
    ClusterType, EdgeType, GraphCluster, GraphData, GraphEdge, GraphNode, NodeType, Protocol,
};
use crate::intelligent_behavior::rules::{StateMachine, StateTransition};
use crate::request_chaining::ChainDefinition;
use serde_json::Value;
use std::collections::HashMap;

/// Convert UI Builder Protocol to Graph Protocol
pub fn convert_protocol(protocol: &str) -> Protocol {
    match protocol.to_lowercase().as_str() {
        "http" => Protocol::Http,
        "grpc" => Protocol::Grpc,
        "websocket" => Protocol::Websocket,
        "graphql" => Protocol::Graphql,
        "mqtt" => Protocol::Mqtt,
        "smtp" => Protocol::Smtp,
        "kafka" => Protocol::Kafka,
        "amqp" => Protocol::Amqp,
        "ftp" => Protocol::Ftp,
        _ => Protocol::Http, // Default to HTTP
    }
}

/// Builder for constructing graph data from various sources
pub struct GraphBuilder {
    /// Accumulated graph data
    graph: GraphData,

    /// Map of endpoint IDs to node IDs for quick lookup
    endpoint_to_node: HashMap<String, String>,

    /// Map of chain IDs to cluster IDs
    chain_to_cluster: HashMap<String, String>,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: GraphData::new(),
            endpoint_to_node: HashMap::new(),
            chain_to_cluster: HashMap::new(),
        }
    }

    /// Build the complete graph from all available data sources
    pub fn build(mut self) -> GraphData {
        self.graph
    }

    /// Add an endpoint node from UI Builder endpoint configuration
    pub fn add_endpoint(
        &mut self,
        endpoint_id: String,
        name: String,
        protocol: Protocol,
        metadata: HashMap<String, Value>,
    ) {
        let node_id = format!("endpoint:{}", endpoint_id);
        self.endpoint_to_node.insert(endpoint_id.clone(), node_id.clone());

        let node = GraphNode {
            id: node_id,
            label: name,
            node_type: NodeType::Endpoint,
            protocol: Some(protocol),
            current_state: None,
            metadata,
        };

        self.graph.add_node(node);
    }

    /// Add a chain and create edges for dependencies
    pub fn add_chain(&mut self, chain: &ChainDefinition) {
        // Create a cluster for this chain
        let cluster_id = format!("chain:{}", chain.id);
        let cluster = GraphCluster {
            id: cluster_id.clone(),
            label: chain.name.clone(),
            cluster_type: ClusterType::Chain,
            node_ids: Vec::new(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("chain_id".to_string(), Value::String(chain.id.clone()));
                meta.insert(
                    "description".to_string(),
                    Value::String(chain.description.clone().unwrap_or_default()),
                );
                meta
            },
        };
        self.graph.add_cluster(cluster);
        self.chain_to_cluster.insert(chain.id.clone(), cluster_id.clone());

        // Process each link in the chain
        for link in &chain.links {
            let link_node_id = format!("chain_link:{}:{}", chain.id, link.request.id);

            // Create a node for the chain link if it references an endpoint
            // Otherwise, it's just a step in the chain
            let link_node = GraphNode {
                id: link_node_id.clone(),
                label: format!("{} ({})", link.request.id, link.request.method),
                node_type: NodeType::Endpoint,
                protocol: Some(Protocol::Http), // Chains are typically HTTP
                current_state: None,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("chain_id".to_string(), Value::String(chain.id.clone()));
                    meta.insert("request_id".to_string(), Value::String(link.request.id.clone()));
                    meta.insert("method".to_string(), Value::String(link.request.method.clone()));
                    meta.insert("url".to_string(), Value::String(link.request.url.clone()));
                    meta
                },
            };
            self.graph.add_node(link_node);

            // Add to cluster
            if let Some(cluster) = self.graph.clusters.iter_mut().find(|c| c.id == cluster_id) {
                cluster.node_ids.push(link_node_id.clone());
            }

            // Create dependency edges
            for dep_id in &link.request.depends_on {
                let dep_node_id = format!("chain_link:{}:{}", chain.id, dep_id);
                let edge = GraphEdge {
                    from: dep_node_id,
                    to: link_node_id.clone(),
                    edge_type: EdgeType::Dependency,
                    label: Some("depends on".to_string()),
                    metadata: HashMap::new(),
                };
                self.graph.add_edge(edge);
            }
        }
    }

    /// Add a state transition edge
    pub fn add_state_transition(
        &mut self,
        from_node_id: String,
        to_node_id: String,
        transition_label: Option<String>,
    ) {
        let edge = GraphEdge {
            from: from_node_id,
            to: to_node_id,
            edge_type: EdgeType::StateTransition,
            label: transition_label,
            metadata: HashMap::new(),
        };
        self.graph.add_edge(edge);
    }

    /// Add a service call edge (cross-service communication)
    pub fn add_service_call(
        &mut self,
        from_node_id: String,
        to_node_id: String,
        call_label: Option<String>,
    ) {
        let edge = GraphEdge {
            from: from_node_id,
            to: to_node_id,
            edge_type: EdgeType::ServiceCall,
            label: call_label,
            metadata: HashMap::new(),
        };
        self.graph.add_edge(edge);
    }

    /// Add a workspace cluster
    pub fn add_workspace(
        &mut self,
        workspace_id: String,
        workspace_name: String,
        endpoint_ids: Vec<String>,
    ) {
        let cluster_id = format!("workspace:{}", workspace_id);
        let cluster = GraphCluster {
            id: cluster_id,
            label: workspace_name,
            cluster_type: ClusterType::Workspace,
            node_ids: endpoint_ids,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("workspace_id".to_string(), Value::String(workspace_id));
                meta
            },
        };
        self.graph.add_cluster(cluster);
    }

    /// Add a service cluster (micro-mock)
    pub fn add_service(
        &mut self,
        service_id: String,
        service_name: String,
        endpoint_ids: Vec<String>,
    ) {
        let cluster_id = format!("service:{}", service_id);
        let cluster = GraphCluster {
            id: cluster_id,
            label: service_name,
            cluster_type: ClusterType::Service,
            node_ids: endpoint_ids,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("service_id".to_string(), Value::String(service_id));
                meta
            },
        };
        self.graph.add_cluster(cluster);
    }

    /// Update the current state of a node
    pub fn update_node_state(&mut self, node_id: &str, state: String) {
        if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
            node.current_state = Some(state);
        }
    }

    /// Get the graph data (consumes the builder)
    pub fn into_graph(self) -> GraphData {
        self.graph
    }

    /// Build graph from UI Builder endpoints
    pub fn from_endpoints(
        &mut self,
        endpoints: &[(String, String, String, String)], // (id, name, protocol, description)
    ) {
        for (id, name, protocol_str, description) in endpoints {
            let protocol = convert_protocol(protocol_str);
            let mut metadata = HashMap::new();
            if !description.is_empty() {
                metadata.insert("description".to_string(), Value::String(description.clone()));
            }

            self.add_endpoint(id.clone(), name.clone(), protocol, metadata);
        }
    }

    /// Build graph from chain definitions
    pub fn from_chains(&mut self, chains: &[ChainDefinition]) {
        for chain in chains {
            self.add_chain(chain);
        }
    }

    /// Build graph from state machines
    pub fn from_state_machines(&mut self, state_machines: &[StateMachine]) {
        for state_machine in state_machines {
            // Create nodes for each state
            for state in &state_machine.states {
                let node_id = format!("state:{}:{}", state_machine.resource_type, state);
                let node = GraphNode {
                    id: node_id.clone(),
                    label: format!("{} ({})", state, state_machine.resource_type),
                    node_type: NodeType::Endpoint,
                    protocol: None,
                    current_state: Some(state.clone()),
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert(
                            "resource_type".to_string(),
                            Value::String(state_machine.resource_type.clone()),
                        );
                        meta.insert(
                            "is_initial".to_string(),
                            Value::Bool(*state == state_machine.initial_state),
                        );
                        meta
                    },
                };
                self.graph.add_node(node);
            }

            // Create edges for transitions
            for transition in &state_machine.transitions {
                let from_node_id =
                    format!("state:{}:{}", state_machine.resource_type, transition.from_state);
                let to_node_id =
                    format!("state:{}:{}", state_machine.resource_type, transition.to_state);

                self.add_state_transition(
                    from_node_id,
                    to_node_id,
                    Some(format!("{} → {}", transition.from_state, transition.to_state)),
                );
            }
        }
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
