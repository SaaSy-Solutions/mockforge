//! World State Model - Core data structures for representing unified state
//!
//! This module defines the core data structures that represent the unified
//! world state of MockForge, including nodes, edges, layers, and snapshots.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Complete world state snapshot at a point in time
///
/// This represents the entire state of the MockForge world at a specific
/// moment, including all personas, entities, relationships, and system state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    /// Unique identifier for this snapshot
    pub id: String,
    /// Timestamp when this snapshot was created
    pub timestamp: DateTime<Utc>,
    /// All state nodes in this snapshot
    pub nodes: Vec<StateNode>,
    /// All state edges (relationships) in this snapshot
    pub edges: Vec<StateEdge>,
    /// State layers and their visibility
    pub layers: HashMap<StateLayer, bool>,
    /// Additional metadata about this snapshot
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl WorldStateSnapshot {
    /// Create a new empty snapshot
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            nodes: Vec::new(),
            edges: Vec::new(),
            layers: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get all nodes in a specific layer
    pub fn nodes_in_layer(&self, layer: &StateLayer) -> Vec<&StateNode> {
        self.nodes.iter().filter(|node| node.layer == *layer).collect()
    }

    /// Get all edges connected to a node
    pub fn edges_for_node(&self, node_id: &str) -> Vec<&StateEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node_id || edge.to == node_id)
            .collect()
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&StateNode> {
        self.nodes.iter().find(|node| node.id == node_id)
    }
}

impl Default for WorldStateSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the world state graph
///
/// Represents any stateful entity in MockForge, such as a persona, entity,
/// session, or system component.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateNode {
    /// Unique identifier for this node
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Type of node (persona, entity, session, etc.)
    pub node_type: NodeType,
    /// Layer this node belongs to
    pub layer: StateLayer,
    /// Current state value (if applicable)
    pub state: Option<String>,
    /// Additional properties/metadata
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    /// Timestamp when this node was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when this node was last updated
    pub updated_at: DateTime<Utc>,
}

impl StateNode {
    /// Create a new state node
    pub fn new(id: String, label: String, node_type: NodeType, layer: StateLayer) -> Self {
        let now = Utc::now();
        Self {
            id,
            label,
            node_type,
            layer,
            state: None,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a property on this node
    pub fn set_property(&mut self, key: String, value: Value) {
        self.properties.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Get a property from this node
    pub fn get_property(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    /// Set the current state of this node
    pub fn set_state(&mut self, state: String) {
        self.state = Some(state);
        self.updated_at = Utc::now();
    }
}

/// Type of node in the world state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// A persona profile
    Persona,
    /// An entity (user, order, payment, etc.)
    Entity,
    /// A session or connection
    Session,
    /// A protocol handler
    Protocol,
    /// A behavior rule or tree
    Behavior,
    /// A schema definition
    Schema,
    /// Recorded data or fixture
    Recorded,
    /// AI modifier or configuration
    AiModifier,
    /// System component
    System,
}

/// An edge (relationship) between two nodes
///
/// Represents relationships between state entities, such as persona
/// relationships, entity ownership, or protocol connections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Type of relationship
    pub relationship_type: String,
    /// Additional edge properties
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    /// Timestamp when this edge was created
    pub created_at: DateTime<Utc>,
}

impl StateEdge {
    /// Create a new state edge
    pub fn new(from: String, to: String, relationship_type: String) -> Self {
        Self {
            from,
            to,
            relationship_type,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set a property on this edge
    pub fn set_property(&mut self, key: String, value: Value) {
        self.properties.insert(key, value);
    }
}

/// State layer grouping related state
///
/// Layers organize state into logical groups for visualization and filtering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum StateLayer {
    /// Personas and persona relationships
    Personas,
    /// Lifecycle states and transitions
    Lifecycle,
    /// Reality levels and continuum
    Reality,
    /// Time and temporal state
    Time,
    /// Multi-protocol state
    Protocols,
    /// Behavior trees and rules
    Behavior,
    /// Generative schemas
    Schemas,
    /// Recorded data and fixtures
    Recorded,
    /// AI modifiers and configurations
    AiModifiers,
    /// System-level state
    System,
}

impl StateLayer {
    /// Get all available layers
    pub fn all() -> Vec<Self> {
        vec![
            Self::Personas,
            Self::Lifecycle,
            Self::Reality,
            Self::Time,
            Self::Protocols,
            Self::Behavior,
            Self::Schemas,
            Self::Recorded,
            Self::AiModifiers,
            Self::System,
        ]
    }

    /// Get human-readable name for this layer
    pub fn name(&self) -> &'static str {
        match self {
            Self::Personas => "Personas",
            Self::Lifecycle => "Lifecycle",
            Self::Reality => "Reality",
            Self::Time => "Time",
            Self::Protocols => "Protocols",
            Self::Behavior => "Behavior",
            Self::Schemas => "Schemas",
            Self::Recorded => "Recorded",
            Self::AiModifiers => "AI Modifiers",
            Self::System => "System",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_state_snapshot_new() {
        let snapshot = WorldStateSnapshot::new();
        assert!(!snapshot.id.is_empty());
        assert!(snapshot.nodes.is_empty());
        assert!(snapshot.edges.is_empty());
        assert!(snapshot.layers.is_empty());
        assert!(snapshot.metadata.is_empty());
    }

    #[test]
    fn test_world_state_snapshot_default() {
        let snapshot = WorldStateSnapshot::default();
        assert!(!snapshot.id.is_empty());
    }

    #[test]
    fn test_world_state_snapshot_nodes_in_layer() {
        let mut snapshot = WorldStateSnapshot::new();

        let node1 = StateNode::new(
            "node1".to_string(),
            "Persona 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let node2 = StateNode::new(
            "node2".to_string(),
            "Entity 1".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let node3 = StateNode::new(
            "node3".to_string(),
            "Persona 2".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        snapshot.nodes.push(node1);
        snapshot.nodes.push(node2);
        snapshot.nodes.push(node3);

        let persona_nodes = snapshot.nodes_in_layer(&StateLayer::Personas);
        assert_eq!(persona_nodes.len(), 2);

        let lifecycle_nodes = snapshot.nodes_in_layer(&StateLayer::Lifecycle);
        assert_eq!(lifecycle_nodes.len(), 1);

        let reality_nodes = snapshot.nodes_in_layer(&StateLayer::Reality);
        assert!(reality_nodes.is_empty());
    }

    #[test]
    fn test_world_state_snapshot_edges_for_node() {
        let mut snapshot = WorldStateSnapshot::new();

        let edge1 = StateEdge::new("node1".to_string(), "node2".to_string(), "owns".to_string());
        let edge2 =
            StateEdge::new("node2".to_string(), "node3".to_string(), "references".to_string());
        let edge3 =
            StateEdge::new("node1".to_string(), "node3".to_string(), "relates_to".to_string());

        snapshot.edges.push(edge1);
        snapshot.edges.push(edge2);
        snapshot.edges.push(edge3);

        let node1_edges = snapshot.edges_for_node("node1");
        assert_eq!(node1_edges.len(), 2);

        let node2_edges = snapshot.edges_for_node("node2");
        assert_eq!(node2_edges.len(), 2);

        let node4_edges = snapshot.edges_for_node("node4");
        assert!(node4_edges.is_empty());
    }

    #[test]
    fn test_world_state_snapshot_get_node() {
        let mut snapshot = WorldStateSnapshot::new();

        let node = StateNode::new(
            "test-node".to_string(),
            "Test Node".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        snapshot.nodes.push(node);

        let found = snapshot.get_node("test-node");
        assert!(found.is_some());
        assert_eq!(found.unwrap().label, "Test Node");

        let not_found = snapshot.get_node("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_state_node_new() {
        let node = StateNode::new(
            "node-123".to_string(),
            "My Node".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        assert_eq!(node.id, "node-123");
        assert_eq!(node.label, "My Node");
        assert_eq!(node.node_type, NodeType::Persona);
        assert_eq!(node.layer, StateLayer::Personas);
        assert!(node.state.is_none());
        assert!(node.properties.is_empty());
    }

    #[test]
    fn test_state_node_set_property() {
        let mut node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        let original_updated_at = node.updated_at;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1));

        node.set_property("key".to_string(), serde_json::json!("value"));

        assert_eq!(node.get_property("key"), Some(&serde_json::json!("value")));
        assert!(node.updated_at >= original_updated_at);
    }

    #[test]
    fn test_state_node_get_property() {
        let mut node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        assert!(node.get_property("nonexistent").is_none());

        node.set_property("exists".to_string(), serde_json::json!(42));
        assert_eq!(node.get_property("exists"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_state_node_set_state() {
        let mut node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        assert!(node.state.is_none());

        node.set_state("active".to_string());
        assert_eq!(node.state, Some("active".to_string()));
    }

    #[test]
    fn test_node_type_serialize() {
        let node_type = NodeType::Persona;
        let json = serde_json::to_string(&node_type).unwrap();
        assert_eq!(json, "\"persona\"");

        let node_type = NodeType::AiModifier;
        let json = serde_json::to_string(&node_type).unwrap();
        assert_eq!(json, "\"ai_modifier\"");
    }

    #[test]
    fn test_node_type_deserialize() {
        let node_type: NodeType = serde_json::from_str("\"persona\"").unwrap();
        assert_eq!(node_type, NodeType::Persona);

        let node_type: NodeType = serde_json::from_str("\"entity\"").unwrap();
        assert_eq!(node_type, NodeType::Entity);

        let node_type: NodeType = serde_json::from_str("\"behavior\"").unwrap();
        assert_eq!(node_type, NodeType::Behavior);
    }

    #[test]
    fn test_node_type_all_variants() {
        // Test all variants can be serialized and deserialized
        let variants = vec![
            NodeType::Persona,
            NodeType::Entity,
            NodeType::Session,
            NodeType::Protocol,
            NodeType::Behavior,
            NodeType::Schema,
            NodeType::Recorded,
            NodeType::AiModifier,
            NodeType::System,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: NodeType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, deserialized);
        }
    }

    #[test]
    fn test_state_edge_new() {
        let edge = StateEdge::new("node1".to_string(), "node2".to_string(), "owns".to_string());

        assert_eq!(edge.from, "node1");
        assert_eq!(edge.to, "node2");
        assert_eq!(edge.relationship_type, "owns");
        assert!(edge.properties.is_empty());
    }

    #[test]
    fn test_state_edge_set_property() {
        let mut edge = StateEdge::new("a".to_string(), "b".to_string(), "relates".to_string());

        edge.set_property("weight".to_string(), serde_json::json!(1.5));
        assert_eq!(edge.properties.get("weight"), Some(&serde_json::json!(1.5)));
    }

    #[test]
    fn test_state_layer_all() {
        let layers = StateLayer::all();
        assert_eq!(layers.len(), 10);
        assert!(layers.contains(&StateLayer::Personas));
        assert!(layers.contains(&StateLayer::Lifecycle));
        assert!(layers.contains(&StateLayer::Reality));
        assert!(layers.contains(&StateLayer::Time));
        assert!(layers.contains(&StateLayer::Protocols));
        assert!(layers.contains(&StateLayer::Behavior));
        assert!(layers.contains(&StateLayer::Schemas));
        assert!(layers.contains(&StateLayer::Recorded));
        assert!(layers.contains(&StateLayer::AiModifiers));
        assert!(layers.contains(&StateLayer::System));
    }

    #[test]
    fn test_state_layer_name() {
        assert_eq!(StateLayer::Personas.name(), "Personas");
        assert_eq!(StateLayer::Lifecycle.name(), "Lifecycle");
        assert_eq!(StateLayer::Reality.name(), "Reality");
        assert_eq!(StateLayer::Time.name(), "Time");
        assert_eq!(StateLayer::Protocols.name(), "Protocols");
        assert_eq!(StateLayer::Behavior.name(), "Behavior");
        assert_eq!(StateLayer::Schemas.name(), "Schemas");
        assert_eq!(StateLayer::Recorded.name(), "Recorded");
        assert_eq!(StateLayer::AiModifiers.name(), "AI Modifiers");
        assert_eq!(StateLayer::System.name(), "System");
    }

    #[test]
    fn test_state_layer_serialize() {
        let layer = StateLayer::Personas;
        let json = serde_json::to_string(&layer).unwrap();
        assert_eq!(json, "\"personas\"");

        let layer = StateLayer::AiModifiers;
        let json = serde_json::to_string(&layer).unwrap();
        assert_eq!(json, "\"ai_modifiers\"");
    }

    #[test]
    fn test_state_layer_deserialize() {
        let layer: StateLayer = serde_json::from_str("\"personas\"").unwrap();
        assert_eq!(layer, StateLayer::Personas);

        let layer: StateLayer = serde_json::from_str("\"ai_modifiers\"").unwrap();
        assert_eq!(layer, StateLayer::AiModifiers);
    }

    #[test]
    fn test_state_layer_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(StateLayer::Personas);
        set.insert(StateLayer::Lifecycle);
        set.insert(StateLayer::Personas); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_world_state_snapshot_serialize() {
        let snapshot = WorldStateSnapshot::new();
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
    }

    #[test]
    fn test_state_node_clone() {
        let node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let cloned = node.clone();
        assert_eq!(node.id, cloned.id);
        assert_eq!(node.label, cloned.label);
    }

    #[test]
    fn test_state_edge_clone() {
        let edge = StateEdge::new("a".to_string(), "b".to_string(), "relates".to_string());
        let cloned = edge.clone();
        assert_eq!(edge.from, cloned.from);
        assert_eq!(edge.to, cloned.to);
    }

    #[test]
    fn test_state_layer_ordering() {
        // Test that layers can be ordered (for sorting)
        let mut layers = vec![
            StateLayer::System,
            StateLayer::Personas,
            StateLayer::Lifecycle,
        ];
        layers.sort();
        assert_eq!(layers[0], StateLayer::Personas);
    }
}
