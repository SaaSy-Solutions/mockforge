//! World State Engine - Central coordinator for unified world state
//!
//! This module provides the core engine that aggregates state from all
//! subsystems and maintains unified state snapshots.

use crate::aggregators::StateAggregator;
use crate::model::{StateLayer, WorldStateSnapshot};
use crate::query::WorldStateQuery;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// World State Engine
///
/// Central coordinator that aggregates state from all MockForge subsystems
/// and provides a unified view of the entire system state.
pub struct WorldStateEngine {
    /// Registered aggregators for each layer
    aggregators: HashMap<StateLayer, Arc<dyn StateAggregator>>,
    /// Historical snapshots (limited to recent ones)
    snapshots: Arc<RwLock<Vec<WorldStateSnapshot>>>,
    /// Maximum number of snapshots to keep
    max_snapshots: usize,
}

impl WorldStateEngine {
    /// Create a new world state engine
    pub fn new() -> Self {
        Self {
            aggregators: HashMap::new(),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots: 100,
        }
    }

    /// Register an aggregator for a layer
    pub fn register_aggregator(&mut self, aggregator: Arc<dyn StateAggregator>) {
        let layer = aggregator.layer();
        self.aggregators.insert(layer, aggregator);
        info!("Registered aggregator for layer: {:?}", layer);
    }

    /// Create a snapshot of the current world state
    pub async fn create_snapshot(&self) -> Result<WorldStateSnapshot> {
        debug!("Creating world state snapshot");

        let mut snapshot = WorldStateSnapshot::new();
        let mut all_nodes = Vec::new();
        let mut all_edges = Vec::new();

        // Aggregate state from all registered aggregators
        for (layer, aggregator) in &self.aggregators {
            match aggregator.aggregate().await {
                Ok((nodes, edges)) => {
                    debug!(
                        "Aggregated {} nodes and {} edges from layer: {:?}",
                        nodes.len(),
                        edges.len(),
                        layer
                    );
                    all_nodes.extend(nodes);
                    all_edges.extend(edges);
                    snapshot.layers.insert(*layer, true);
                }
                Err(e) => {
                    warn!("Failed to aggregate state from layer {:?}: {}", layer, e);
                    snapshot.layers.insert(*layer, false);
                }
            }
        }

        snapshot.nodes = all_nodes;
        snapshot.edges = all_edges;

        // Store snapshot
        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        // Limit snapshot history
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        info!(
            "Created world state snapshot with {} nodes and {} edges",
            snapshot.nodes.len(),
            snapshot.edges.len()
        );

        Ok(snapshot)
    }

    /// Get the current world state snapshot
    pub async fn get_current_snapshot(&self) -> Result<WorldStateSnapshot> {
        self.create_snapshot().await
    }

    /// Get a snapshot by ID
    pub async fn get_snapshot(&self, snapshot_id: &str) -> Option<WorldStateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.iter().find(|s| s.id == snapshot_id).cloned()
    }

    /// Get all available snapshots
    pub async fn get_all_snapshots(&self) -> Vec<WorldStateSnapshot> {
        let snapshots = self.snapshots.read().await;
        snapshots.clone()
    }

    /// Query the current world state with filters
    pub async fn query(&self, query: &WorldStateQuery) -> Result<WorldStateSnapshot> {
        let snapshot = self.create_snapshot().await?;

        // Filter nodes
        let filtered_nodes: Vec<_> =
            snapshot.nodes.iter().filter(|node| query.matches_node(node)).cloned().collect();

        // Filter edges
        let filtered_edges: Vec<_> = if query.include_edges {
            snapshot.edges.iter().filter(|edge| query.matches_edge(edge)).cloned().collect()
        } else {
            Vec::new()
        };

        // Create filtered snapshot
        let mut filtered_snapshot = snapshot;
        filtered_snapshot.nodes = filtered_nodes;
        filtered_snapshot.edges = filtered_edges;

        Ok(filtered_snapshot)
    }

    /// Get available layers
    pub fn get_layers(&self) -> Vec<StateLayer> {
        self.aggregators.keys().copied().collect()
    }

    /// Set maximum number of snapshots to keep
    pub fn set_max_snapshots(&mut self, max: usize) {
        self.max_snapshots = max;
    }
}

impl Default for WorldStateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeType, StateNode};
    use std::collections::HashSet;

    // Mock aggregator for testing
    struct MockAggregator {
        layer: StateLayer,
        nodes: Vec<StateNode>,
        edges: Vec<crate::model::StateEdge>,
        should_fail: bool,
    }

    impl MockAggregator {
        fn new(layer: StateLayer) -> Self {
            Self {
                layer,
                nodes: Vec::new(),
                edges: Vec::new(),
                should_fail: false,
            }
        }

        fn with_nodes(mut self, nodes: Vec<StateNode>) -> Self {
            self.nodes = nodes;
            self
        }

        fn with_edges(mut self, edges: Vec<crate::model::StateEdge>) -> Self {
            self.edges = edges;
            self
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait::async_trait]
    impl StateAggregator for MockAggregator {
        async fn aggregate(&self) -> Result<(Vec<StateNode>, Vec<crate::model::StateEdge>)> {
            if self.should_fail {
                anyhow::bail!("Mock aggregator failure");
            }
            Ok((self.nodes.clone(), self.edges.clone()))
        }

        fn layer(&self) -> StateLayer {
            self.layer
        }
    }

    #[test]
    fn test_world_state_engine_new() {
        let engine = WorldStateEngine::new();
        assert_eq!(engine.aggregators.len(), 0);
        assert_eq!(engine.max_snapshots, 100);
    }

    #[test]
    fn test_world_state_engine_default() {
        let engine = WorldStateEngine::default();
        assert_eq!(engine.aggregators.len(), 0);
        assert_eq!(engine.max_snapshots, 100);
    }

    #[test]
    fn test_register_aggregator() {
        let mut engine = WorldStateEngine::new();
        let aggregator = Arc::new(MockAggregator::new(StateLayer::Personas));

        engine.register_aggregator(aggregator);
        assert_eq!(engine.aggregators.len(), 1);
        assert!(engine.aggregators.contains_key(&StateLayer::Personas));
    }

    #[test]
    fn test_register_multiple_aggregators() {
        let mut engine = WorldStateEngine::new();

        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Personas)));
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Lifecycle)));
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Reality)));

        assert_eq!(engine.aggregators.len(), 3);
        assert!(engine.aggregators.contains_key(&StateLayer::Personas));
        assert!(engine.aggregators.contains_key(&StateLayer::Lifecycle));
        assert!(engine.aggregators.contains_key(&StateLayer::Reality));
    }

    #[test]
    fn test_register_aggregator_replacement() {
        let mut engine = WorldStateEngine::new();

        // Register first aggregator for Personas layer
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Personas)));
        assert_eq!(engine.aggregators.len(), 1);

        // Register second aggregator for same layer - should replace
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Personas)));
        assert_eq!(engine.aggregators.len(), 1);
    }

    #[tokio::test]
    async fn test_create_snapshot_empty() {
        let engine = WorldStateEngine::new();
        let snapshot = engine.create_snapshot().await.unwrap();

        assert!(snapshot.nodes.is_empty());
        assert!(snapshot.edges.is_empty());
        assert!(snapshot.layers.is_empty());
    }

    #[tokio::test]
    async fn test_create_snapshot_with_aggregator() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test-node".to_string(),
            "Test Node".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        let aggregator =
            Arc::new(MockAggregator::new(StateLayer::Personas).with_nodes(vec![node.clone()]));

        engine.register_aggregator(aggregator);

        let snapshot = engine.create_snapshot().await.unwrap();

        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].id, "test-node");
        assert_eq!(snapshot.edges.len(), 0);
        assert!(snapshot.layers.contains_key(&StateLayer::Personas));
        assert_eq!(snapshot.layers[&StateLayer::Personas], true);
    }

    #[tokio::test]
    async fn test_create_snapshot_with_edges() {
        let mut engine = WorldStateEngine::new();

        let node1 = StateNode::new(
            "node1".to_string(),
            "Node 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let node2 = StateNode::new(
            "node2".to_string(),
            "Node 2".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let edge = crate::model::StateEdge::new(
            "node1".to_string(),
            "node2".to_string(),
            "relates_to".to_string(),
        );

        let aggregator = Arc::new(
            MockAggregator::new(StateLayer::Personas)
                .with_nodes(vec![node1, node2])
                .with_edges(vec![edge]),
        );

        engine.register_aggregator(aggregator);

        let snapshot = engine.create_snapshot().await.unwrap();

        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.edges[0].from, "node1");
        assert_eq!(snapshot.edges[0].to, "node2");
    }

    #[tokio::test]
    async fn test_create_snapshot_multiple_aggregators() {
        let mut engine = WorldStateEngine::new();

        let persona_node = StateNode::new(
            "persona1".to_string(),
            "Persona 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let lifecycle_node = StateNode::new(
            "lifecycle1".to_string(),
            "Lifecycle 1".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![persona_node]),
        ));
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_nodes(vec![lifecycle_node]),
        ));

        let snapshot = engine.create_snapshot().await.unwrap();

        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.layers.len(), 2);
        assert!(snapshot.layers.contains_key(&StateLayer::Personas));
        assert!(snapshot.layers.contains_key(&StateLayer::Lifecycle));
    }

    #[tokio::test]
    async fn test_create_snapshot_aggregator_failure() {
        let mut engine = WorldStateEngine::new();

        let success_node = StateNode::new(
            "success".to_string(),
            "Success Node".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        // One aggregator succeeds
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![success_node]),
        ));

        // One aggregator fails
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_failure(),
        ));

        let snapshot = engine.create_snapshot().await.unwrap();

        // Should have successful nodes but mark failed layer
        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.layers[&StateLayer::Personas], true);
        assert_eq!(snapshot.layers[&StateLayer::Lifecycle], false);
    }

    #[tokio::test]
    async fn test_get_current_snapshot() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        let snapshot = engine.get_current_snapshot().await.unwrap();

        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].id, "test");
    }

    #[tokio::test]
    async fn test_snapshot_storage() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        // Create first snapshot
        let snapshot1 = engine.create_snapshot().await.unwrap();
        let snapshot1_id = snapshot1.id.clone();

        // Verify snapshot is stored
        let snapshots = engine.snapshots.read().await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, snapshot1_id);
    }

    #[tokio::test]
    async fn test_get_snapshot() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        // Create snapshot
        let snapshot = engine.create_snapshot().await.unwrap();
        let snapshot_id = snapshot.id.clone();

        // Retrieve by ID
        let retrieved = engine.get_snapshot(&snapshot_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, snapshot_id);

        // Try to retrieve non-existent snapshot
        let not_found = engine.get_snapshot("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_all_snapshots() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        // Create multiple snapshots
        engine.create_snapshot().await.unwrap();
        engine.create_snapshot().await.unwrap();
        engine.create_snapshot().await.unwrap();

        let all_snapshots = engine.get_all_snapshots().await;
        assert_eq!(all_snapshots.len(), 3);
    }

    #[tokio::test]
    async fn test_max_snapshots_limit() {
        let mut engine = WorldStateEngine::new();
        engine.set_max_snapshots(2);

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        // Create 3 snapshots (limit is 2)
        engine.create_snapshot().await.unwrap();
        engine.create_snapshot().await.unwrap();
        engine.create_snapshot().await.unwrap();

        let snapshots = engine.get_all_snapshots().await;
        assert_eq!(snapshots.len(), 2);
    }

    #[tokio::test]
    async fn test_query_no_filters() {
        let mut engine = WorldStateEngine::new();

        let node1 = StateNode::new(
            "node1".to_string(),
            "Node 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let node2 = StateNode::new(
            "node2".to_string(),
            "Node 2".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![node1]),
        ));
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_nodes(vec![node2]),
        ));

        let query = WorldStateQuery::new();
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_query_filter_by_node_type() {
        let mut engine = WorldStateEngine::new();

        let persona_node = StateNode::new(
            "persona".to_string(),
            "Persona".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let entity_node = StateNode::new(
            "entity".to_string(),
            "Entity".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![persona_node]),
        ));
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_nodes(vec![entity_node]),
        ));

        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(node_types);
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].node_type, NodeType::Persona);
    }

    #[tokio::test]
    async fn test_query_filter_by_layer() {
        let mut engine = WorldStateEngine::new();

        let node1 = StateNode::new(
            "node1".to_string(),
            "Node 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let node2 = StateNode::new(
            "node2".to_string(),
            "Node 2".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![node1]),
        ));
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_nodes(vec![node2]),
        ));

        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let query = WorldStateQuery::new().with_layers(layers);
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].layer, StateLayer::Personas);
    }

    #[tokio::test]
    async fn test_query_include_edges_false() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let edge = crate::model::StateEdge::new(
            "node".to_string(),
            "node".to_string(),
            "self".to_string(),
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas)
                .with_nodes(vec![node])
                .with_edges(vec![edge]),
        ));

        let query = WorldStateQuery::new().include_edges(false);
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.edges.len(), 0);
    }

    #[tokio::test]
    async fn test_query_filter_edges() {
        let mut engine = WorldStateEngine::new();

        let edge1 =
            crate::model::StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string());
        let edge2 = crate::model::StateEdge::new(
            "b".to_string(),
            "c".to_string(),
            "references".to_string(),
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_edges(vec![edge1, edge2]),
        ));

        let mut relationship_types = HashSet::new();
        relationship_types.insert("owns".to_string());

        let query = WorldStateQuery::new().with_relationship_types(relationship_types);
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.edges[0].relationship_type, "owns");
    }

    #[test]
    fn test_get_layers_empty() {
        let engine = WorldStateEngine::new();
        let layers = engine.get_layers();
        assert!(layers.is_empty());
    }

    #[test]
    fn test_get_layers() {
        let mut engine = WorldStateEngine::new();

        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Personas)));
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Lifecycle)));
        engine.register_aggregator(Arc::new(MockAggregator::new(StateLayer::Reality)));

        let layers = engine.get_layers();
        assert_eq!(layers.len(), 3);
        assert!(layers.contains(&StateLayer::Personas));
        assert!(layers.contains(&StateLayer::Lifecycle));
        assert!(layers.contains(&StateLayer::Reality));
    }

    #[test]
    fn test_set_max_snapshots() {
        let mut engine = WorldStateEngine::new();
        assert_eq!(engine.max_snapshots, 100);

        engine.set_max_snapshots(50);
        assert_eq!(engine.max_snapshots, 50);

        engine.set_max_snapshots(200);
        assert_eq!(engine.max_snapshots, 200);
    }

    #[tokio::test]
    async fn test_snapshot_pruning() {
        let mut engine = WorldStateEngine::new();
        engine.set_max_snapshots(3);

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        // Create 5 snapshots
        let snapshot1 = engine.create_snapshot().await.unwrap();
        let snapshot2 = engine.create_snapshot().await.unwrap();
        let snapshot3 = engine.create_snapshot().await.unwrap();
        let snapshot4 = engine.create_snapshot().await.unwrap();
        let snapshot5 = engine.create_snapshot().await.unwrap();

        let all_snapshots = engine.get_all_snapshots().await;
        assert_eq!(all_snapshots.len(), 3);

        // First two snapshots should be pruned
        assert!(engine.get_snapshot(&snapshot1.id).await.is_none());
        assert!(engine.get_snapshot(&snapshot2.id).await.is_none());

        // Last three should exist
        assert!(engine.get_snapshot(&snapshot3.id).await.is_some());
        assert!(engine.get_snapshot(&snapshot4.id).await.is_some());
        assert!(engine.get_snapshot(&snapshot5.id).await.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_snapshot_access() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::System).with_nodes(vec![node]),
        ));

        let engine = Arc::new(engine);

        // Create snapshot in background
        let engine_clone = Arc::clone(&engine);
        let handle1 = tokio::spawn(async move { engine_clone.create_snapshot().await });

        // Read snapshots concurrently
        let engine_clone = Arc::clone(&engine);
        let handle2 = tokio::spawn(async move { engine_clone.get_all_snapshots().await });

        // Both should complete successfully
        let snapshot = handle1.await.unwrap();
        let snapshots = handle2.await.unwrap();

        assert!(snapshot.is_ok());
        assert!(!snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_query_with_multiple_filters() {
        let mut engine = WorldStateEngine::new();

        let node1 = StateNode::new(
            "node1".to_string(),
            "Node 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        let node2 = StateNode::new(
            "node2".to_string(),
            "Node 2".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let node3 = StateNode::new(
            "node3".to_string(),
            "Node 3".to_string(),
            NodeType::Persona,
            StateLayer::Lifecycle,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![node1]),
        ));
        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Lifecycle).with_nodes(vec![node2, node3]),
        ));

        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);

        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let query = WorldStateQuery::new().with_node_types(node_types).with_layers(layers);

        let result = engine.query(&query).await.unwrap();

        // Should only match node1 (Persona type AND Personas layer)
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].id, "node1");
    }

    #[tokio::test]
    async fn test_empty_query_result() {
        let mut engine = WorldStateEngine::new();

        let node = StateNode::new(
            "node".to_string(),
            "Node".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        engine.register_aggregator(Arc::new(
            MockAggregator::new(StateLayer::Personas).with_nodes(vec![node]),
        ));

        // Query for non-existent node type
        let mut node_types = HashSet::new();
        node_types.insert(NodeType::System);

        let query = WorldStateQuery::new().with_node_types(node_types);
        let result = engine.query(&query).await.unwrap();

        assert_eq!(result.nodes.len(), 0);
        assert_eq!(result.edges.len(), 0);
    }

    #[tokio::test]
    async fn test_aggregator_metadata() {
        struct MetadataAggregator {
            layer: StateLayer,
        }

        #[async_trait::async_trait]
        impl StateAggregator for MetadataAggregator {
            async fn aggregate(&self) -> Result<(Vec<StateNode>, Vec<crate::model::StateEdge>)> {
                Ok((Vec::new(), Vec::new()))
            }

            fn layer(&self) -> StateLayer {
                self.layer
            }

            fn metadata(&self) -> HashMap<String, serde_json::Value> {
                let mut map = HashMap::new();
                map.insert("version".to_string(), serde_json::json!("1.0"));
                map.insert("enabled".to_string(), serde_json::json!(true));
                map
            }
        }

        let aggregator = MetadataAggregator {
            layer: StateLayer::Personas,
        };

        let metadata = aggregator.metadata();
        assert_eq!(metadata.len(), 2);
        assert_eq!(metadata.get("version"), Some(&serde_json::json!("1.0")));
        assert_eq!(metadata.get("enabled"), Some(&serde_json::json!(true)));
    }
}
