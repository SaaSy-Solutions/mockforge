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
