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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_node(id: &str, node_type: NodeType, layer: StateLayer) -> StateNode {
        StateNode {
            id: id.to_string(),
            label: format!("Test {}", id),
            node_type,
            layer,
            state: None,
            properties: std::collections::HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_query_new() {
        let query = WorldStateQuery::new();
        assert!(query.node_types.is_none());
        assert!(query.layers.is_none());
        assert!(query.node_ids.is_none());
        assert!(query.relationship_types.is_none());
        assert!(query.include_edges);
        assert!(query.max_depth.is_none());
    }

    #[test]
    fn test_query_default() {
        let query = WorldStateQuery::default();
        assert!(query.include_edges);
    }

    #[test]
    fn test_query_with_node_types() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);
        types.insert(NodeType::Entity);

        let query = WorldStateQuery::new().with_node_types(types.clone());
        assert_eq!(query.node_types, Some(types));
    }

    #[test]
    fn test_query_with_layers() {
        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);
        layers.insert(StateLayer::Lifecycle);

        let query = WorldStateQuery::new().with_layers(layers.clone());
        assert_eq!(query.layers, Some(layers));
    }

    #[test]
    fn test_query_with_node_ids() {
        let mut ids = HashSet::new();
        ids.insert("node1".to_string());
        ids.insert("node2".to_string());

        let query = WorldStateQuery::new().with_node_ids(ids.clone());
        assert_eq!(query.node_ids, Some(ids));
    }

    #[test]
    fn test_query_with_relationship_types() {
        let mut types = HashSet::new();
        types.insert("owns".to_string());
        types.insert("references".to_string());

        let query = WorldStateQuery::new().with_relationship_types(types.clone());
        assert_eq!(query.relationship_types, Some(types));
    }

    #[test]
    fn test_query_include_edges() {
        let query = WorldStateQuery::new().include_edges(false);
        assert!(!query.include_edges);
    }

    #[test]
    fn test_query_with_max_depth() {
        let query = WorldStateQuery::new().with_max_depth(3);
        assert_eq!(query.max_depth, Some(3));
    }

    #[test]
    fn test_query_builder_chaining() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);

        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let query = WorldStateQuery::new()
            .with_node_types(types)
            .with_layers(layers)
            .include_edges(false)
            .with_max_depth(5);

        assert!(query.node_types.is_some());
        assert!(query.layers.is_some());
        assert!(!query.include_edges);
        assert_eq!(query.max_depth, Some(5));
    }

    #[test]
    fn test_matches_node_no_filters() {
        let query = WorldStateQuery::new();
        let node = create_test_node("node1", NodeType::Persona, StateLayer::Personas);
        assert!(query.matches_node(&node));
    }

    #[test]
    fn test_matches_node_by_type() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(types);

        let persona_node = create_test_node("node1", NodeType::Persona, StateLayer::Personas);
        let entity_node = create_test_node("node2", NodeType::Entity, StateLayer::Lifecycle);

        assert!(query.matches_node(&persona_node));
        assert!(!query.matches_node(&entity_node));
    }

    #[test]
    fn test_matches_node_by_layer() {
        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let query = WorldStateQuery::new().with_layers(layers);

        let persona_node = create_test_node("node1", NodeType::Persona, StateLayer::Personas);
        let lifecycle_node = create_test_node("node2", NodeType::Entity, StateLayer::Lifecycle);

        assert!(query.matches_node(&persona_node));
        assert!(!query.matches_node(&lifecycle_node));
    }

    #[test]
    fn test_matches_node_by_id() {
        let mut ids = HashSet::new();
        ids.insert("node1".to_string());
        ids.insert("node2".to_string());

        let query = WorldStateQuery::new().with_node_ids(ids);

        let matching_node = create_test_node("node1", NodeType::Persona, StateLayer::Personas);
        let non_matching_node = create_test_node("node3", NodeType::Persona, StateLayer::Personas);

        assert!(query.matches_node(&matching_node));
        assert!(!query.matches_node(&non_matching_node));
    }

    #[test]
    fn test_matches_node_multiple_filters() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);

        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let query = WorldStateQuery::new().with_node_types(types).with_layers(layers);

        // Matches both filters
        let matching = create_test_node("node1", NodeType::Persona, StateLayer::Personas);
        assert!(query.matches_node(&matching));

        // Matches type but not layer
        let wrong_layer = create_test_node("node2", NodeType::Persona, StateLayer::Lifecycle);
        assert!(!query.matches_node(&wrong_layer));

        // Matches layer but not type
        let wrong_type = create_test_node("node3", NodeType::Entity, StateLayer::Personas);
        assert!(!query.matches_node(&wrong_type));
    }

    #[test]
    fn test_matches_edge_no_filters() {
        let query = WorldStateQuery::new();
        let edge = StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string());
        assert!(query.matches_edge(&edge));
    }

    #[test]
    fn test_matches_edge_by_relationship_type() {
        let mut types = HashSet::new();
        types.insert("owns".to_string());

        let query = WorldStateQuery::new().with_relationship_types(types);

        let matching_edge = StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string());
        let non_matching_edge =
            StateEdge::new("a".to_string(), "b".to_string(), "references".to_string());

        assert!(query.matches_edge(&matching_edge));
        assert!(!query.matches_edge(&non_matching_edge));
    }

    #[test]
    fn test_query_serialize() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(types);
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("\"persona\""));
    }

    #[test]
    fn test_query_deserialize() {
        let json = r#"{
            "node_types": ["persona"],
            "layers": ["personas"],
            "include_edges": false,
            "max_depth": 3
        }"#;
        let query: WorldStateQuery = serde_json::from_str(json).unwrap();
        assert!(query.node_types.is_some());
        assert!(query.layers.is_some());
        assert!(!query.include_edges);
        assert_eq!(query.max_depth, Some(3));
    }

    #[test]
    fn test_query_clone() {
        let mut types = HashSet::new();
        types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(types).with_max_depth(5);

        let cloned = query.clone();
        assert_eq!(query.node_types, cloned.node_types);
        assert_eq!(query.max_depth, cloned.max_depth);
    }
}
