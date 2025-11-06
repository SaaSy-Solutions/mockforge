//! Graph visualization data structures for MockForge
//!
//! This module provides data structures for representing mock environments
//! as a dependency graph, showing endpoints, their relationships, state transitions,
//! and micro-mock connections.

pub mod builder;
pub mod relationships;

pub use builder::GraphBuilder;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete graph data structure containing all nodes, edges, and clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphData {
    /// All nodes in the graph (endpoints, services, workspaces)
    pub nodes: Vec<GraphNode>,

    /// All edges in the graph (dependencies, state transitions, service calls)
    pub edges: Vec<GraphEdge>,

    /// Clusters for grouping related nodes (micro-mocks, workspaces)
    pub clusters: Vec<GraphCluster>,
}

impl GraphData {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Add a cluster to the graph
    pub fn add_cluster(&mut self, cluster: GraphCluster) {
        self.clusters.push(cluster);
    }

    /// Find a node by ID
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find all edges connected to a node
    pub fn edges_for_node(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == node_id || e.to == node_id).collect()
    }
}

impl Default for GraphData {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the graph representing an endpoint, service, or workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Unique identifier for the node
    pub id: String,

    /// Human-readable label for the node
    pub label: String,

    /// Type of node (endpoint, service, workspace)
    pub node_type: NodeType,

    /// Protocol used by this node (if applicable)
    pub protocol: Option<Protocol>,

    /// Current state of the node (if state machine is active)
    pub current_state: Option<String>,

    /// Additional metadata about the node
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of node in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    /// An individual endpoint (HTTP, gRPC, WebSocket, etc.)
    Endpoint,

    /// A service grouping multiple endpoints
    Service,

    /// A workspace grouping multiple services/endpoints
    Workspace,
}

/// Protocol type for endpoints
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// HTTP/REST protocol
    Http,

    /// gRPC protocol
    Grpc,

    /// WebSocket protocol
    Websocket,

    /// GraphQL protocol
    Graphql,

    /// MQTT protocol
    Mqtt,

    /// SMTP protocol
    Smtp,

    /// Kafka protocol
    Kafka,

    /// AMQP protocol
    Amqp,

    /// FTP protocol
    Ftp,
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
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
}

/// An edge in the graph representing a relationship between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// Source node ID
    pub from: String,

    /// Target node ID
    pub to: String,

    /// Type of relationship
    pub edge_type: EdgeType,

    /// Optional label for the edge
    pub label: Option<String>,

    /// Additional metadata about the edge
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of edge/relationship in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeType {
    /// Dependency relationship (e.g., chain dependency)
    Dependency,

    /// State transition relationship
    StateTransition,

    /// Service call relationship (cross-service communication)
    ServiceCall,

    /// Data flow relationship
    DataFlow,

    /// Workspace/service grouping relationship
    Contains,
}

/// A cluster for grouping related nodes (e.g., micro-mocks, workspaces)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphCluster {
    /// Unique identifier for the cluster
    pub id: String,

    /// Human-readable label for the cluster
    pub label: String,

    /// Type of cluster (workspace, service, etc.)
    pub cluster_type: ClusterType,

    /// IDs of nodes that belong to this cluster
    pub node_ids: Vec<String>,

    /// Additional metadata about the cluster
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of cluster in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClusterType {
    /// Workspace cluster
    Workspace,

    /// Service cluster (micro-mock)
    Service,

    /// Chain cluster (grouping chain-related endpoints)
    Chain,
}
