//! Persona Graph & Relationship Management
//!
//! This module provides graph-based relationship management for personas,
//! enabling coherent persona switching across related entities (user → orders → payments → support tickets).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};

/// Represents a node in the persona graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaNode {
    /// Persona ID
    pub persona_id: String,
    /// Entity type (e.g., "user", "order", "payment", "support_ticket")
    pub entity_type: String,
    /// Relationships from this persona to others
    /// Key: relationship type (e.g., "has_orders", "has_payments")
    /// Value: List of related persona IDs
    pub relationships: HashMap<String, Vec<String>>,
    /// Additional metadata for the node
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PersonaNode {
    /// Create a new persona node
    pub fn new(persona_id: String, entity_type: String) -> Self {
        Self {
            persona_id,
            entity_type,
            relationships: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a relationship to another persona
    pub fn add_relationship(&mut self, relationship_type: String, related_persona_id: String) {
        self.relationships
            .entry(relationship_type)
            .or_default()
            .push(related_persona_id);
    }

    /// Get all related personas for a relationship type
    pub fn get_related(&self, relationship_type: &str) -> Vec<String> {
        self.relationships.get(relationship_type).cloned().unwrap_or_default()
    }

    /// Get all relationship types for this node
    pub fn get_relationship_types(&self) -> Vec<String> {
        self.relationships.keys().cloned().collect()
    }
}

/// Edge in the persona graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source persona ID
    pub from: String,
    /// Target persona ID
    pub to: String,
    /// Relationship type
    pub relationship_type: String,
    /// Edge weight (for weighted traversals, default 1.0)
    #[serde(default = "default_edge_weight")]
    pub weight: f64,
}

fn default_edge_weight() -> f64 {
    1.0
}

/// Persona graph for managing entity relationships
///
/// Maintains a graph structure of personas and their relationships,
/// enabling coherent persona switching across related entities.
#[derive(Debug, Clone)]
pub struct PersonaGraph {
    /// Graph nodes indexed by persona ID
    nodes: Arc<RwLock<HashMap<String, PersonaNode>>>,
    /// Graph edges indexed by source persona ID
    edges: Arc<RwLock<HashMap<String, Vec<Edge>>>>,
    /// Reverse edges for efficient backward traversal
    reverse_edges: Arc<RwLock<HashMap<String, Vec<Edge>>>>,
}

impl PersonaGraph {
    /// Create a new empty persona graph
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            edges: Arc::new(RwLock::new(HashMap::new())),
            reverse_edges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a persona node to the graph
    pub fn add_node(&self, node: PersonaNode) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node.persona_id.clone(), node);
    }

    /// Get a node by persona ID
    pub fn get_node(&self, persona_id: &str) -> Option<PersonaNode> {
        let nodes = self.nodes.read().unwrap();
        nodes.get(persona_id).cloned()
    }

    /// Add an edge between two personas
    pub fn add_edge(&self, from: String, to: String, relationship_type: String) {
        let to_clone = to.clone();
        let edge = Edge {
            from: from.clone(),
            to: to_clone.clone(),
            relationship_type: relationship_type.clone(),
            weight: 1.0,
        };

        // Add forward edge
        let mut edges = self.edges.write().unwrap();
        edges.entry(from.clone()).or_default().push(edge.clone());

        // Add reverse edge
        let mut reverse_edges = self.reverse_edges.write().unwrap();
        reverse_edges.entry(to_clone.clone()).or_default().push(edge);

        // Update node relationships
        if let Some(node) = self.get_node(&from) {
            let mut updated_node = node;
            updated_node.add_relationship(relationship_type, to_clone);
            self.add_node(updated_node);
        }
    }

    /// Get all edges from a persona
    pub fn get_edges_from(&self, persona_id: &str) -> Vec<Edge> {
        let edges = self.edges.read().unwrap();
        edges.get(persona_id).cloned().unwrap_or_default()
    }

    /// Get all edges to a persona
    pub fn get_edges_to(&self, persona_id: &str) -> Vec<Edge> {
        let reverse_edges = self.reverse_edges.read().unwrap();
        reverse_edges.get(persona_id).cloned().unwrap_or_default()
    }

    /// Find all related personas using BFS traversal
    ///
    /// Traverses the graph starting from the given persona ID,
    /// following relationships of the specified types.
    ///
    /// # Arguments
    /// * `start_persona_id` - Starting persona ID
    /// * `relationship_types` - Optional filter for relationship types to follow
    /// * `max_depth` - Maximum traversal depth (None = unlimited)
    ///
    /// # Returns
    /// Vector of persona IDs reachable from the start persona
    pub fn find_related_bfs(
        &self,
        start_persona_id: &str,
        relationship_types: Option<&[String]>,
        max_depth: Option<usize>,
    ) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start_persona_id.to_string(), 0));
        visited.insert(start_persona_id.to_string());

        while let Some((current_id, depth)) = queue.pop_front() {
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
            }

            let edges = self.get_edges_from(&current_id);
            for edge in edges {
                // Filter by relationship type if specified
                if let Some(types) = relationship_types {
                    if !types.contains(&edge.relationship_type) {
                        continue;
                    }
                }

                if !visited.contains(&edge.to) {
                    visited.insert(edge.to.clone());
                    result.push(edge.to.clone());
                    queue.push_back((edge.to.clone(), depth + 1));
                }
            }
        }

        result
    }

    /// Find all related personas using DFS traversal
    ///
    /// Traverses the graph starting from the given persona ID,
    /// following relationships of the specified types.
    ///
    /// # Arguments
    /// * `start_persona_id` - Starting persona ID
    /// * `relationship_types` - Optional filter for relationship types to follow
    /// * `max_depth` - Maximum traversal depth (None = unlimited)
    ///
    /// # Returns
    /// Vector of persona IDs reachable from the start persona
    pub fn find_related_dfs(
        &self,
        start_persona_id: &str,
        relationship_types: Option<&[String]>,
        max_depth: Option<usize>,
    ) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();

        self.dfs_recursive(
            start_persona_id,
            relationship_types,
            max_depth,
            0,
            &mut visited,
            &mut result,
        );

        result
    }

    /// Recursive helper for DFS traversal
    fn dfs_recursive(
        &self,
        current_id: &str,
        relationship_types: Option<&[String]>,
        max_depth: Option<usize>,
        current_depth: usize,
        visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        if visited.contains(current_id) {
            return;
        }

        if let Some(max) = max_depth {
            if current_depth >= max {
                return;
            }
        }

        visited.insert(current_id.to_string());
        if current_depth > 0 {
            // Don't include the start node in results
            result.push(current_id.to_string());
        }

        let edges = self.get_edges_from(current_id);
        for edge in edges {
            // Filter by relationship type if specified
            if let Some(types) = relationship_types {
                if !types.contains(&edge.relationship_type) {
                    continue;
                }
            }

            self.dfs_recursive(
                &edge.to,
                relationship_types,
                max_depth,
                current_depth + 1,
                visited,
                result,
            );
        }
    }

    /// Get the entire subgraph starting from a persona
    ///
    /// Returns all nodes and edges reachable from the start persona.
    pub fn get_subgraph(&self, start_persona_id: &str) -> (Vec<PersonaNode>, Vec<Edge>) {
        let related_ids = self.find_related_bfs(start_persona_id, None, None);
        let mut all_ids = vec![start_persona_id.to_string()];
        all_ids.extend(related_ids);

        let nodes = self.nodes.read().unwrap();
        let edges = self.edges.read().unwrap();

        let subgraph_nodes: Vec<PersonaNode> =
            all_ids.iter().filter_map(|id| nodes.get(id).cloned()).collect();

        let subgraph_edges: Vec<Edge> = all_ids
            .iter()
            .flat_map(|id| edges.get(id).cloned().unwrap_or_default())
            .filter(|edge| all_ids.contains(&edge.to))
            .collect();

        (subgraph_nodes, subgraph_edges)
    }

    /// Get all nodes in the graph
    pub fn get_all_nodes(&self) -> Vec<PersonaNode> {
        let nodes = self.nodes.read().unwrap();
        nodes.values().cloned().collect()
    }

    /// Remove a node and all its edges
    pub fn remove_node(&self, persona_id: &str) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.remove(persona_id);

        // Remove forward edges
        let mut edges = self.edges.write().unwrap();
        edges.remove(persona_id);

        // Remove reverse edges
        let mut reverse_edges = self.reverse_edges.write().unwrap();
        reverse_edges.remove(persona_id);

        // Remove edges pointing to this node
        for edges_list in edges.values_mut() {
            edges_list.retain(|e| e.to != persona_id);
        }
        for edges_list in reverse_edges.values_mut() {
            edges_list.retain(|e| e.from != persona_id);
        }
    }

    /// Clear the entire graph
    pub fn clear(&self) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.clear();

        let mut edges = self.edges.write().unwrap();
        edges.clear();

        let mut reverse_edges = self.reverse_edges.write().unwrap();
        reverse_edges.clear();
    }

    /// Get graph statistics
    pub fn get_stats(&self) -> GraphStats {
        let nodes = self.nodes.read().unwrap();
        let edges = self.edges.read().unwrap();

        let mut relationship_type_counts = HashMap::new();
        for edges_list in edges.values() {
            for edge in edges_list {
                *relationship_type_counts.entry(edge.relationship_type.clone()).or_insert(0) += 1;
            }
        }

        GraphStats {
            node_count: nodes.len(),
            edge_count: edges.values().map(|e| e.len()).sum(),
            relationship_types: relationship_type_counts,
        }
    }

    /// Link personas across entity types automatically
    ///
    /// Creates relationships between personas based on common entity type patterns:
    /// - user → has_orders → order
    /// - user → has_accounts → account
    /// - order → has_payments → payment
    /// - user → has_webhooks → webhook
    /// - user → has_tcp_messages → tcp_message
    ///
    /// # Arguments
    /// * `from_persona_id` - Source persona ID
    /// * `from_entity_type` - Source entity type (e.g., "user", "order")
    /// * `to_persona_id` - Target persona ID
    /// * `to_entity_type` - Target entity type (e.g., "order", "payment")
    pub fn link_entity_types(
        &self,
        from_persona_id: &str,
        from_entity_type: &str,
        to_persona_id: &str,
        to_entity_type: &str,
    ) {
        // Determine relationship type based on entity types
        let relationship_type: String = match (from_entity_type, to_entity_type) {
            ("user", "order") | ("user", "orders") => "has_orders".to_string(),
            ("user", "account") | ("user", "accounts") => "has_accounts".to_string(),
            ("user", "webhook") | ("user", "webhooks") => "has_webhooks".to_string(),
            ("user", "tcp_message") | ("user", "tcp_messages") => "has_tcp_messages".to_string(),
            ("order", "payment") | ("order", "payments") => "has_payments".to_string(),
            ("account", "order") | ("account", "orders") => "has_orders".to_string(),
            ("account", "payment") | ("account", "payments") => "has_payments".to_string(),
            _ => {
                // Generic relationship: from_entity_type -> to_entity_type
                format!("has_{}", to_entity_type.to_lowercase().trim_end_matches('s'))
            }
        };

        // Ensure both nodes exist
        if self.get_node(from_persona_id).is_none() {
            let node = PersonaNode::new(from_persona_id.to_string(), from_entity_type.to_string());
            self.add_node(node);
        }

        if self.get_node(to_persona_id).is_none() {
            let node = PersonaNode::new(to_persona_id.to_string(), to_entity_type.to_string());
            self.add_node(node);
        }

        // Add the edge
        self.add_edge(
            from_persona_id.to_string(),
            to_persona_id.to_string(),
            relationship_type.to_string(),
        );
    }

    /// Find all related personas of a specific entity type
    ///
    /// Traverses the graph to find all personas of the specified entity type
    /// that are related to the starting persona.
    ///
    /// # Arguments
    /// * `start_persona_id` - Starting persona ID
    /// * `target_entity_type` - Entity type to find (e.g., "order", "payment")
    /// * `relationship_type` - Optional relationship type filter (e.g., "has_orders")
    ///
    /// # Returns
    /// Vector of persona IDs matching the criteria
    pub fn find_related_by_entity_type(
        &self,
        start_persona_id: &str,
        target_entity_type: &str,
        relationship_type: Option<&str>,
    ) -> Vec<String> {
        let related_ids = if let Some(rel_type) = relationship_type {
            let rel_types = vec![rel_type.to_string()];
            self.find_related_bfs(start_persona_id, Some(&rel_types), Some(2))
        } else {
            self.find_related_bfs(start_persona_id, None, Some(2))
        };

        // Filter by entity type
        related_ids
            .into_iter()
            .filter_map(|persona_id| {
                if let Some(node) = self.get_node(&persona_id) {
                    if node.entity_type.to_lowercase() == target_entity_type.to_lowercase() {
                        Some(persona_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get or create a persona node and link it to related entities
    ///
    /// This is a convenience method that creates a persona node if it doesn't exist
    /// and automatically establishes relationships based on entity type patterns.
    ///
    /// # Arguments
    /// * `persona_id` - Persona ID
    /// * `entity_type` - Entity type (e.g., "user", "order", "payment")
    /// * `related_entity_id` - Optional related entity ID to link to
    /// * `related_entity_type` - Optional related entity type
    pub fn get_or_create_node_with_links(
        &self,
        persona_id: &str,
        entity_type: &str,
        related_entity_id: Option<&str>,
        related_entity_type: Option<&str>,
    ) -> PersonaNode {
        // Get or create the node
        let node = if let Some(existing) = self.get_node(persona_id) {
            existing
        } else {
            let new_node = PersonaNode::new(persona_id.to_string(), entity_type.to_string());
            self.add_node(new_node.clone());
            new_node
        };

        // Link to related entity if provided
        if let (Some(related_id), Some(related_type)) = (related_entity_id, related_entity_type) {
            self.link_entity_types(persona_id, entity_type, related_id, related_type);
        }

        node
    }
}

impl Default for PersonaGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    /// Number of nodes in the graph
    pub node_count: usize,
    /// Number of edges in the graph
    pub edge_count: usize,
    /// Count of edges by relationship type
    pub relationship_types: HashMap<String, usize>,
}

/// Graph visualization data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphVisualization {
    /// Nodes in the graph
    pub nodes: Vec<VisualizationNode>,
    /// Edges in the graph
    pub edges: Vec<VisualizationEdge>,
}

/// Node for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationNode {
    /// Persona ID
    pub id: String,
    /// Entity type
    pub entity_type: String,
    /// Display label
    pub label: String,
    /// Node position (for layout algorithms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<(f64, f64)>,
}

/// Edge for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationEdge {
    /// Source persona ID
    pub from: String,
    /// Target persona ID
    pub to: String,
    /// Relationship type
    pub relationship_type: String,
    /// Display label
    pub label: String,
}

impl PersonaGraph {
    /// Generate visualization data for the graph
    pub fn to_visualization(&self) -> GraphVisualization {
        let nodes = self.get_all_nodes();
        let edges = self.edges.read().unwrap();

        let vis_nodes: Vec<VisualizationNode> = nodes
            .iter()
            .map(|node| VisualizationNode {
                id: node.persona_id.clone(),
                entity_type: node.entity_type.clone(),
                label: format!("{} ({})", node.persona_id, node.entity_type),
                position: None,
            })
            .collect();

        let vis_edges: Vec<VisualizationEdge> = edges
            .values()
            .flatten()
            .map(|edge| VisualizationEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                relationship_type: edge.relationship_type.clone(),
                label: edge.relationship_type.clone(),
            })
            .collect();

        GraphVisualization {
            nodes: vis_nodes,
            edges: vis_edges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // PersonaNode tests
    // =========================================================================

    #[test]
    fn test_persona_node_new() {
        let node = PersonaNode::new("user-123".to_string(), "user".to_string());
        assert_eq!(node.persona_id, "user-123");
        assert_eq!(node.entity_type, "user");
        assert!(node.relationships.is_empty());
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_persona_node_add_relationship() {
        let mut node = PersonaNode::new("user-123".to_string(), "user".to_string());
        node.add_relationship("has_orders".to_string(), "order-1".to_string());
        node.add_relationship("has_orders".to_string(), "order-2".to_string());

        let related = node.get_related("has_orders");
        assert_eq!(related.len(), 2);
        assert!(related.contains(&"order-1".to_string()));
        assert!(related.contains(&"order-2".to_string()));
    }

    #[test]
    fn test_persona_node_get_related_empty() {
        let node = PersonaNode::new("user-123".to_string(), "user".to_string());
        let related = node.get_related("has_orders");
        assert!(related.is_empty());
    }

    #[test]
    fn test_persona_node_get_relationship_types() {
        let mut node = PersonaNode::new("user-123".to_string(), "user".to_string());
        node.add_relationship("has_orders".to_string(), "order-1".to_string());
        node.add_relationship("has_payments".to_string(), "payment-1".to_string());

        let types = node.get_relationship_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"has_orders".to_string()));
        assert!(types.contains(&"has_payments".to_string()));
    }

    #[test]
    fn test_persona_node_clone() {
        let mut node = PersonaNode::new("user-123".to_string(), "user".to_string());
        node.add_relationship("has_orders".to_string(), "order-1".to_string());

        let cloned = node.clone();
        assert_eq!(cloned.persona_id, node.persona_id);
        assert_eq!(cloned.entity_type, node.entity_type);
        assert_eq!(cloned.relationships, node.relationships);
    }

    #[test]
    fn test_persona_node_debug() {
        let node = PersonaNode::new("user-123".to_string(), "user".to_string());
        let debug_str = format!("{:?}", node);
        assert!(debug_str.contains("user-123"));
        assert!(debug_str.contains("user"));
    }

    #[test]
    fn test_persona_node_serialize_deserialize() {
        let mut node = PersonaNode::new("user-123".to_string(), "user".to_string());
        node.add_relationship("has_orders".to_string(), "order-1".to_string());

        let json = serde_json::to_string(&node).unwrap();
        let deserialized: PersonaNode = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.persona_id, "user-123");
        assert_eq!(deserialized.entity_type, "user");
    }

    // =========================================================================
    // Edge tests
    // =========================================================================

    #[test]
    fn test_edge_creation() {
        let edge = Edge {
            from: "user-123".to_string(),
            to: "order-456".to_string(),
            relationship_type: "has_orders".to_string(),
            weight: 1.0,
        };
        assert_eq!(edge.from, "user-123");
        assert_eq!(edge.to, "order-456");
        assert_eq!(edge.relationship_type, "has_orders");
        assert!((edge.weight - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_edge_clone() {
        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: "rel".to_string(),
            weight: 2.5,
        };
        let cloned = edge.clone();
        assert_eq!(cloned.from, edge.from);
        assert!((cloned.weight - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_edge_debug() {
        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: "rel".to_string(),
            weight: 1.0,
        };
        let debug_str = format!("{:?}", edge);
        assert!(debug_str.contains("from"));
        assert!(debug_str.contains("to"));
    }

    #[test]
    fn test_edge_serialize_default_weight() {
        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: "rel".to_string(),
            weight: 1.0,
        };
        let json = serde_json::to_string(&edge).unwrap();
        let deserialized: Edge = serde_json::from_str(&json).unwrap();
        assert!((deserialized.weight - 1.0).abs() < f64::EPSILON);
    }

    // =========================================================================
    // PersonaGraph tests
    // =========================================================================

    #[test]
    fn test_persona_graph_new() {
        let graph = PersonaGraph::new();
        let stats = graph.get_stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    #[test]
    fn test_persona_graph_default() {
        let graph = PersonaGraph::default();
        assert_eq!(graph.get_stats().node_count, 0);
    }

    #[test]
    fn test_persona_graph_add_node() {
        let graph = PersonaGraph::new();
        let node = PersonaNode::new("user-123".to_string(), "user".to_string());
        graph.add_node(node);

        let stats = graph.get_stats();
        assert_eq!(stats.node_count, 1);
    }

    #[test]
    fn test_persona_graph_get_node() {
        let graph = PersonaGraph::new();
        let node = PersonaNode::new("user-123".to_string(), "user".to_string());
        graph.add_node(node);

        let retrieved = graph.get_node("user-123");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().persona_id, "user-123");
    }

    #[test]
    fn test_persona_graph_get_node_not_found() {
        let graph = PersonaGraph::new();
        assert!(graph.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_persona_graph_add_edge() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        let stats = graph.get_stats();
        assert_eq!(stats.edge_count, 1);
    }

    #[test]
    fn test_persona_graph_get_edges_from() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("order-2".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("user-1".to_string(), "order-2".to_string(), "has_orders".to_string());

        let edges = graph.get_edges_from("user-1");
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_persona_graph_get_edges_to() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("user-2".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("user-2".to_string(), "order-1".to_string(), "has_orders".to_string());

        let edges = graph.get_edges_to("order-1");
        assert_eq!(edges.len(), 2);
    }

    #[test]
    fn test_persona_graph_find_related_bfs() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("payment-1".to_string(), "payment".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("order-1".to_string(), "payment-1".to_string(), "has_payments".to_string());

        let related = graph.find_related_bfs("user-1", None, None);
        assert_eq!(related.len(), 2);
        assert!(related.contains(&"order-1".to_string()));
        assert!(related.contains(&"payment-1".to_string()));
    }

    #[test]
    fn test_persona_graph_find_related_bfs_with_depth_limit() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("payment-1".to_string(), "payment".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("order-1".to_string(), "payment-1".to_string(), "has_payments".to_string());

        let related = graph.find_related_bfs("user-1", None, Some(1));
        assert_eq!(related.len(), 1);
        assert!(related.contains(&"order-1".to_string()));
    }

    #[test]
    fn test_persona_graph_find_related_bfs_with_type_filter() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("account-1".to_string(), "account".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("user-1".to_string(), "account-1".to_string(), "has_accounts".to_string());

        let filter = vec!["has_orders".to_string()];
        let related = graph.find_related_bfs("user-1", Some(&filter), None);
        assert_eq!(related.len(), 1);
        assert!(related.contains(&"order-1".to_string()));
    }

    #[test]
    fn test_persona_graph_find_related_dfs() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("payment-1".to_string(), "payment".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("order-1".to_string(), "payment-1".to_string(), "has_payments".to_string());

        let related = graph.find_related_dfs("user-1", None, None);
        assert_eq!(related.len(), 2);
    }

    #[test]
    fn test_persona_graph_find_related_dfs_with_depth_limit() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("a".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("b".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("c".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("d".to_string(), "node".to_string()));

        graph.add_edge("a".to_string(), "b".to_string(), "linked".to_string());
        graph.add_edge("b".to_string(), "c".to_string(), "linked".to_string());
        graph.add_edge("c".to_string(), "d".to_string(), "linked".to_string());

        // DFS implementation: max_depth=2 means we can go 0->1->2, but depth 2 is the cutoff
        // So we get nodes at depth 1 only (b), not c at depth 2
        let related = graph.find_related_dfs("a", None, Some(2));
        assert_eq!(related.len(), 1); // only b, depth 2 is the cutoff
        assert!(related.contains(&"b".to_string()));
    }

    #[test]
    fn test_persona_graph_get_subgraph() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("isolated".to_string(), "node".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        let (nodes, edges) = graph.get_subgraph("user-1");
        assert_eq!(nodes.len(), 2); // user-1 and order-1
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_persona_graph_get_all_nodes() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("a".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("b".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("c".to_string(), "node".to_string()));

        let nodes = graph.get_all_nodes();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn test_persona_graph_remove_node() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        graph.remove_node("order-1");

        assert!(graph.get_node("order-1").is_none());
        assert_eq!(graph.get_edges_from("user-1").len(), 0);
    }

    #[test]
    fn test_persona_graph_clear() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        graph.clear();

        let stats = graph.get_stats();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    #[test]
    fn test_persona_graph_get_stats() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("order-2".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("user-1".to_string(), "order-2".to_string(), "has_orders".to_string());

        let stats = graph.get_stats();
        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 2);
        assert_eq!(*stats.relationship_types.get("has_orders").unwrap(), 2);
    }

    // =========================================================================
    // Link entity types tests
    // =========================================================================

    #[test]
    fn test_persona_graph_link_entity_types_user_order() {
        let graph = PersonaGraph::new();
        graph.link_entity_types("user-1", "user", "order-1", "order");

        let node = graph.get_node("user-1").unwrap();
        assert_eq!(node.entity_type, "user");

        let related = node.get_related("has_orders");
        assert!(related.contains(&"order-1".to_string()));
    }

    #[test]
    fn test_persona_graph_link_entity_types_order_payment() {
        let graph = PersonaGraph::new();
        graph.link_entity_types("order-1", "order", "payment-1", "payment");

        let related = graph.get_node("order-1").unwrap().get_related("has_payments");
        assert!(related.contains(&"payment-1".to_string()));
    }

    #[test]
    fn test_persona_graph_link_entity_types_generic() {
        let graph = PersonaGraph::new();
        graph.link_entity_types("foo-1", "foo", "bar-1", "bars");

        let node = graph.get_node("foo-1").unwrap();
        // Generic relationship: has_bar (from "bars" -> "bar")
        let related = node.get_related("has_bar");
        assert!(related.contains(&"bar-1".to_string()));
    }

    #[test]
    fn test_persona_graph_link_entity_types_creates_nodes() {
        let graph = PersonaGraph::new();
        graph.link_entity_types("new-user", "user", "new-order", "order");

        assert!(graph.get_node("new-user").is_some());
        assert!(graph.get_node("new-order").is_some());
    }

    // =========================================================================
    // Find related by entity type tests
    // =========================================================================

    #[test]
    fn test_persona_graph_find_related_by_entity_type() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("order-2".to_string(), "order".to_string()));
        graph.add_node(PersonaNode::new("payment-1".to_string(), "payment".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());
        graph.add_edge("user-1".to_string(), "order-2".to_string(), "has_orders".to_string());
        graph.add_edge("user-1".to_string(), "payment-1".to_string(), "has_payments".to_string());

        let orders = graph.find_related_by_entity_type("user-1", "order", None);
        assert_eq!(orders.len(), 2);
    }

    #[test]
    fn test_persona_graph_find_related_by_entity_type_with_filter() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));

        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        let orders = graph.find_related_by_entity_type("user-1", "order", Some("has_orders"));
        assert_eq!(orders.len(), 1);
    }

    // =========================================================================
    // Get or create node with links tests
    // =========================================================================

    #[test]
    fn test_persona_graph_get_or_create_node_new() {
        let graph = PersonaGraph::new();
        let node = graph.get_or_create_node_with_links("user-new", "user", None, None);
        assert_eq!(node.persona_id, "user-new");
        assert!(graph.get_node("user-new").is_some());
    }

    #[test]
    fn test_persona_graph_get_or_create_node_existing() {
        let graph = PersonaGraph::new();
        let node1 = PersonaNode::new("user-existing".to_string(), "user".to_string());
        graph.add_node(node1);

        let node2 = graph.get_or_create_node_with_links("user-existing", "user", None, None);
        assert_eq!(node2.persona_id, "user-existing");
    }

    #[test]
    fn test_persona_graph_get_or_create_node_with_link() {
        let graph = PersonaGraph::new();
        let _node = graph.get_or_create_node_with_links(
            "user-link",
            "user",
            Some("order-link"),
            Some("order"),
        );

        assert!(graph.get_node("user-link").is_some());
        assert!(graph.get_node("order-link").is_some());
        assert_eq!(graph.get_edges_from("user-link").len(), 1);
    }

    // =========================================================================
    // GraphStats tests
    // =========================================================================

    #[test]
    fn test_graph_stats_clone() {
        let stats = GraphStats {
            node_count: 5,
            edge_count: 10,
            relationship_types: {
                let mut map = HashMap::new();
                map.insert("has_orders".to_string(), 5);
                map
            },
        };
        let cloned = stats.clone();
        assert_eq!(cloned.node_count, 5);
        assert_eq!(cloned.edge_count, 10);
    }

    #[test]
    fn test_graph_stats_debug() {
        let stats = GraphStats {
            node_count: 3,
            edge_count: 2,
            relationship_types: HashMap::new(),
        };
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("node_count"));
        assert!(debug_str.contains("edge_count"));
    }

    #[test]
    fn test_graph_stats_serialize() {
        let stats = GraphStats {
            node_count: 1,
            edge_count: 2,
            relationship_types: HashMap::new(),
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("node_count"));
    }

    // =========================================================================
    // Visualization tests
    // =========================================================================

    #[test]
    fn test_visualization_node_creation() {
        let node = VisualizationNode {
            id: "user-1".to_string(),
            entity_type: "user".to_string(),
            label: "User 1".to_string(),
            position: Some((0.0, 0.0)),
        };
        assert_eq!(node.id, "user-1");
    }

    #[test]
    fn test_visualization_edge_creation() {
        let edge = VisualizationEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: "linked".to_string(),
            label: "Linked".to_string(),
        };
        assert_eq!(edge.from, "a");
    }

    #[test]
    fn test_persona_graph_to_visualization() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));
        graph.add_node(PersonaNode::new("order-1".to_string(), "order".to_string()));
        graph.add_edge("user-1".to_string(), "order-1".to_string(), "has_orders".to_string());

        let viz = graph.to_visualization();
        assert_eq!(viz.nodes.len(), 2);
        assert_eq!(viz.edges.len(), 1);
    }

    #[test]
    fn test_visualization_serialize() {
        let viz = GraphVisualization {
            nodes: vec![VisualizationNode {
                id: "test".to_string(),
                entity_type: "node".to_string(),
                label: "Test".to_string(),
                position: None,
            }],
            edges: vec![],
        };
        let json = serde_json::to_string(&viz).unwrap();
        assert!(json.contains("test"));
    }

    // =========================================================================
    // Cycle detection tests
    // =========================================================================

    #[test]
    fn test_persona_graph_handles_cycles() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("a".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("b".to_string(), "node".to_string()));
        graph.add_node(PersonaNode::new("c".to_string(), "node".to_string()));

        // Create a cycle: a -> b -> c -> a
        graph.add_edge("a".to_string(), "b".to_string(), "linked".to_string());
        graph.add_edge("b".to_string(), "c".to_string(), "linked".to_string());
        graph.add_edge("c".to_string(), "a".to_string(), "linked".to_string());

        // BFS should not loop infinitely
        let related = graph.find_related_bfs("a", None, None);
        assert_eq!(related.len(), 2); // b and c (not a again)

        // DFS should not loop infinitely
        let related_dfs = graph.find_related_dfs("a", None, None);
        assert_eq!(related_dfs.len(), 2);
    }

    // =========================================================================
    // Clone tests
    // =========================================================================

    #[test]
    fn test_persona_graph_clone() {
        let graph = PersonaGraph::new();
        graph.add_node(PersonaNode::new("user-1".to_string(), "user".to_string()));

        let cloned = graph.clone();
        // Both graphs share the same underlying data via Arc
        assert!(cloned.get_node("user-1").is_some());
    }
}
