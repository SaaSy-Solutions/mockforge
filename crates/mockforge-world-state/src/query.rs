//! World State Query - Flexible querying of world state
//!
//! This module provides a query builder for filtering and searching
//! world state snapshots.

use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Query for filtering world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateQuery {
    /// Filter by node types
    #[serde(default)]
    pub node_types: Option<HashSet<NodeType>>,
    /// Filter by layers
    #[serde(default)]
    pub layers: Option<HashSet<StateLayer>>,
    /// Filter by node IDs
    #[serde(default)]
    pub node_ids: Option<HashSet<String>>,
    /// Filter by relationship types
    #[serde(default)]
    pub relationship_types: Option<HashSet<String>>,
    /// Include edges in results
    #[serde(default = "default_true")]
    pub include_edges: bool,
    /// Maximum depth for traversal (for graph queries)
    #[serde(default)]
    pub max_depth: Option<usize>,
}

fn default_true() -> bool {
    true
}

impl Default for WorldStateQuery {
    fn default() -> Self {
        Self {
            node_types: None,
            layers: None,
            node_ids: None,
            relationship_types: None,
            include_edges: true,
            max_depth: None,
        }
    }
}

impl WorldStateQuery {
    /// Create a new empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by node types
    pub fn with_node_types(mut self, types: HashSet<NodeType>) -> Self {
        self.node_types = Some(types);
        self
    }

    /// Filter by layers
    pub fn with_layers(mut self, layers: HashSet<StateLayer>) -> Self {
        self.layers = Some(layers);
        self
    }

    /// Filter by node IDs
    pub fn with_node_ids(mut self, ids: HashSet<String>) -> Self {
        self.node_ids = Some(ids);
        self
    }

    /// Filter by relationship types
    pub fn with_relationship_types(mut self, types: HashSet<String>) -> Self {
        self.relationship_types = Some(types);
        self
    }

    /// Set whether to include edges
    pub fn include_edges(mut self, include: bool) -> Self {
        self.include_edges = include;
        self
    }

    /// Set maximum traversal depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Check if a node matches this query
    pub fn matches_node(&self, node: &StateNode) -> bool {
        // Check node type filter
        if let Some(ref types) = self.node_types {
            if !types.contains(&node.node_type) {
                return false;
            }
        }

        // Check layer filter
        if let Some(ref layers) = self.layers {
            if !layers.contains(&node.layer) {
                return false;
            }
        }

        // Check node ID filter
        if let Some(ref ids) = self.node_ids {
            if !ids.contains(&node.id) {
                return false;
            }
        }

        true
    }

    /// Check if an edge matches this query
    pub fn matches_edge(&self, edge: &StateEdge) -> bool {
        // Check relationship type filter
        if let Some(ref types) = self.relationship_types {
            if !types.contains(&edge.relationship_type) {
                return false;
            }
        }

        true
    }
}
