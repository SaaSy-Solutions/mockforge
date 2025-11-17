//! Persona Graph & Relationship Management
//!
//! This module provides graph-based relationship management for personas,
//! enabling coherent persona switching across related entities (user → orders → payments → support tickets).

use crate::persona::PersonaProfile;
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
            .or_insert_with(Vec::new)
            .push(related_persona_id);
    }

    /// Get all related personas for a relationship type
    pub fn get_related(&self, relationship_type: &str) -> Vec<String> {
        self.relationships
            .get(relationship_type)
            .cloned()
            .unwrap_or_default()
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
        edges.entry(from.clone()).or_insert_with(Vec::new).push(edge.clone());

        // Add reverse edge
        let mut reverse_edges = self.reverse_edges.write().unwrap();
        reverse_edges.entry(to_clone.clone()).or_insert_with(Vec::new).push(edge);

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

        let subgraph_nodes: Vec<PersonaNode> = all_ids
            .iter()
            .filter_map(|id| nodes.get(id).cloned())
            .collect();

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
                *relationship_type_counts
                    .entry(edge.relationship_type.clone())
                    .or_insert(0) += 1;
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
        let relationship_type = match (from_entity_type, to_entity_type) {
            ("user", "order") | ("user", "orders") => "has_orders",
            ("user", "account") | ("user", "accounts") => "has_accounts",
            ("user", "webhook") | ("user", "webhooks") => "has_webhooks",
            ("user", "tcp_message") | ("user", "tcp_messages") => "has_tcp_messages",
            ("order", "payment") | ("order", "payments") => "has_payments",
            ("account", "order") | ("account", "orders") => "has_orders",
            ("account", "payment") | ("account", "payments") => "has_payments",
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
