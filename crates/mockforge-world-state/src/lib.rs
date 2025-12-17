//! Pillars: [Reality][DevX]
//!
//! World State Engine - Unified visualization of all MockForge state systems
//!
//! This crate provides a unified "world state" that aggregates and visualizes
//! all state systems in MockForge, including personas, lifecycle, reality,
//! time, multi-protocol state, behavior trees, generative schemas, recorded
//! data, and AI modifiers. Think of it as a "miniature game engine for your backend."
//!
//! # Features
//!
//! - **Unified State Aggregation**: Collects state from all MockForge subsystems
//! - **Graph Visualization**: Represents state as nodes and edges for visualization
//! - **Real-time Updates**: Streams state changes in real-time
//! - **Time Travel**: View state at any point in time
//! - **Query Interface**: Flexible querying of state with filters
//! - **Export Capabilities**: Export state in various formats (JSON, GraphML, DOT)

pub mod aggregators;
pub mod engine;
pub mod model;
pub mod query;

pub use engine::WorldStateEngine;
pub use model::{StateEdge, StateLayer, StateNode, WorldStateSnapshot};
pub use query::WorldStateQuery;

#[cfg(test)]
mod tests {
    use super::*;
    use model::{NodeType, StateEdge, StateLayer, StateNode, WorldStateSnapshot};
    use query::WorldStateQuery;
    use std::collections::HashSet;

    #[test]
    fn test_module_exports() {
        // Verify all main types are accessible through the crate root
        let _snapshot: WorldStateSnapshot;
        let _node: StateNode;
        let _edge: StateEdge;
        let _layer: StateLayer;
        let _engine: WorldStateEngine;
        let _query: WorldStateQuery;
    }

    #[test]
    fn test_create_world_state_snapshot() {
        let snapshot = WorldStateSnapshot::new();
        assert!(!snapshot.id.is_empty());
        assert!(snapshot.nodes.is_empty());
        assert!(snapshot.edges.is_empty());
    }

    #[test]
    fn test_create_state_node() {
        let node = StateNode::new(
            "test-id".to_string(),
            "Test Label".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        assert_eq!(node.id, "test-id");
        assert_eq!(node.label, "Test Label");
        assert_eq!(node.node_type, NodeType::Persona);
        assert_eq!(node.layer, StateLayer::Personas);
    }

    #[test]
    fn test_create_state_edge() {
        let edge = StateEdge::new(
            "from-node".to_string(),
            "to-node".to_string(),
            "relationship".to_string(),
        );

        assert_eq!(edge.from, "from-node");
        assert_eq!(edge.to, "to-node");
        assert_eq!(edge.relationship_type, "relationship");
    }

    #[test]
    fn test_state_layer_all() {
        let layers = StateLayer::all();
        assert!(!layers.is_empty());
        assert!(layers.contains(&StateLayer::Personas));
        assert!(layers.contains(&StateLayer::Lifecycle));
    }

    #[test]
    fn test_world_state_query_builder() {
        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(node_types.clone());

        assert_eq!(query.node_types, Some(node_types));
    }

    #[test]
    fn test_world_state_engine_creation() {
        let engine = WorldStateEngine::new();
        let layers = engine.get_layers();
        assert!(layers.is_empty());
    }

    #[test]
    fn test_integration_snapshot_with_nodes_and_edges() {
        let mut snapshot = WorldStateSnapshot::new();

        // Create nodes
        let node1 = StateNode::new(
            "user-1".to_string(),
            "User 1".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        let node2 = StateNode::new(
            "order-1".to_string(),
            "Order 1".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        // Create edge
        let edge = StateEdge::new("user-1".to_string(), "order-1".to_string(), "owns".to_string());

        snapshot.nodes.push(node1);
        snapshot.nodes.push(node2);
        snapshot.edges.push(edge);

        // Query the snapshot
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);

        let user_node = snapshot.get_node("user-1");
        assert!(user_node.is_some());
        assert_eq!(user_node.unwrap().label, "User 1");

        let user_edges = snapshot.edges_for_node("user-1");
        assert_eq!(user_edges.len(), 1);
        assert_eq!(user_edges[0].to, "order-1");
    }

    #[test]
    fn test_integration_query_workflow() {
        let mut snapshot = WorldStateSnapshot::new();

        // Add diverse nodes
        let persona = StateNode::new(
            "persona-1".to_string(),
            "John Doe".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        let entity = StateNode::new(
            "entity-1".to_string(),
            "Payment".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        let session = StateNode::new(
            "session-1".to_string(),
            "Session 123".to_string(),
            NodeType::Session,
            StateLayer::Protocols,
        );

        snapshot.nodes.push(persona.clone());
        snapshot.nodes.push(entity.clone());
        snapshot.nodes.push(session.clone());

        // Test layer filtering
        let persona_layer_nodes = snapshot.nodes_in_layer(&StateLayer::Personas);
        assert_eq!(persona_layer_nodes.len(), 1);
        assert_eq!(persona_layer_nodes[0].id, "persona-1");

        let lifecycle_layer_nodes = snapshot.nodes_in_layer(&StateLayer::Lifecycle);
        assert_eq!(lifecycle_layer_nodes.len(), 1);
        assert_eq!(lifecycle_layer_nodes[0].id, "entity-1");
    }

    #[test]
    fn test_integration_state_node_properties() {
        let mut node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );

        // Set multiple properties
        node.set_property("status".to_string(), serde_json::json!("active"));
        node.set_property("count".to_string(), serde_json::json!(42));
        node.set_property("metadata".to_string(), serde_json::json!({"key": "value"}));

        // Verify properties
        assert_eq!(node.get_property("status"), Some(&serde_json::json!("active")));
        assert_eq!(node.get_property("count"), Some(&serde_json::json!(42)));
        assert!(node.get_property("metadata").is_some());
        assert!(node.get_property("nonexistent").is_none());

        // Set state
        node.set_state("running".to_string());
        assert_eq!(node.state, Some("running".to_string()));
    }

    #[test]
    fn test_integration_state_edge_properties() {
        let mut edge = StateEdge::new("a".to_string(), "b".to_string(), "connects".to_string());

        edge.set_property("weight".to_string(), serde_json::json!(0.75));
        edge.set_property("bidirectional".to_string(), serde_json::json!(true));

        assert_eq!(edge.properties.get("weight"), Some(&serde_json::json!(0.75)));
        assert_eq!(edge.properties.get("bidirectional"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_integration_snapshot_metadata() {
        let mut snapshot = WorldStateSnapshot::new();

        snapshot.metadata.insert("version".to_string(), serde_json::json!("1.0.0"));
        snapshot.metadata.insert("environment".to_string(), serde_json::json!("test"));

        assert_eq!(snapshot.metadata.get("version"), Some(&serde_json::json!("1.0.0")));
        assert_eq!(snapshot.metadata.get("environment"), Some(&serde_json::json!("test")));
    }

    #[test]
    fn test_integration_layer_management() {
        let mut snapshot = WorldStateSnapshot::new();

        // Mark layers as active/inactive
        snapshot.layers.insert(StateLayer::Personas, true);
        snapshot.layers.insert(StateLayer::Lifecycle, true);
        snapshot.layers.insert(StateLayer::Reality, false);

        assert_eq!(snapshot.layers.get(&StateLayer::Personas), Some(&true));
        assert_eq!(snapshot.layers.get(&StateLayer::Lifecycle), Some(&true));
        assert_eq!(snapshot.layers.get(&StateLayer::Reality), Some(&false));
        assert_eq!(snapshot.layers.get(&StateLayer::Time), None);
    }

    #[test]
    fn test_all_node_types_coverage() {
        // Ensure all node types can be created and used
        let types = vec![
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

        for (i, node_type) in types.iter().enumerate() {
            let node = StateNode::new(
                format!("node-{}", i),
                format!("Node {}", i),
                *node_type,
                StateLayer::System,
            );
            assert_eq!(node.node_type, *node_type);
        }
    }

    #[test]
    fn test_all_state_layers_coverage() {
        // Ensure all state layers can be used
        let all_layers = StateLayer::all();

        for layer in &all_layers {
            let name = layer.name();
            assert!(!name.is_empty());

            let node =
                StateNode::new("test".to_string(), "Test".to_string(), NodeType::System, *layer);
            assert_eq!(node.layer, *layer);
        }

        // Verify count
        assert_eq!(all_layers.len(), 10);
    }

    #[test]
    fn test_serialization_roundtrip_snapshot() {
        let mut snapshot = WorldStateSnapshot::new();

        let node = StateNode::new(
            "node-1".to_string(),
            "Node 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        snapshot.nodes.push(node);

        let edge =
            StateEdge::new("node-1".to_string(), "node-2".to_string(), "relates".to_string());
        snapshot.edges.push(edge);

        // Serialize
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(!json.is_empty());

        // Deserialize
        let deserialized: WorldStateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot.id, deserialized.id);
        assert_eq!(snapshot.nodes.len(), deserialized.nodes.len());
        assert_eq!(snapshot.edges.len(), deserialized.edges.len());
    }

    #[test]
    fn test_serialization_roundtrip_query() {
        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);
        node_types.insert(NodeType::Entity);

        let query = WorldStateQuery::new()
            .with_node_types(node_types)
            .include_edges(false)
            .with_max_depth(5);

        // Serialize
        let json = serde_json::to_string(&query).unwrap();

        // Deserialize
        let deserialized: WorldStateQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(query.include_edges, deserialized.include_edges);
        assert_eq!(query.max_depth, deserialized.max_depth);
    }

    #[test]
    fn test_complex_graph_structure() {
        let mut snapshot = WorldStateSnapshot::new();

        // Create a graph: A -> B -> C
        //                 A -> D
        let node_a = StateNode::new(
            "a".to_string(),
            "Node A".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let node_b = StateNode::new(
            "b".to_string(),
            "Node B".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let node_c = StateNode::new(
            "c".to_string(),
            "Node C".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );
        let node_d = StateNode::new(
            "d".to_string(),
            "Node D".to_string(),
            NodeType::Entity,
            StateLayer::Lifecycle,
        );

        snapshot.nodes.push(node_a);
        snapshot.nodes.push(node_b);
        snapshot.nodes.push(node_c);
        snapshot.nodes.push(node_d);

        snapshot
            .edges
            .push(StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string()));
        snapshot
            .edges
            .push(StateEdge::new("b".to_string(), "c".to_string(), "owns".to_string()));
        snapshot.edges.push(StateEdge::new(
            "a".to_string(),
            "d".to_string(),
            "references".to_string(),
        ));

        // Verify graph structure
        let edges_from_a = snapshot.edges_for_node("a");
        assert_eq!(edges_from_a.len(), 2);

        let edges_from_b = snapshot.edges_for_node("b");
        assert_eq!(edges_from_b.len(), 2); // One incoming, one outgoing

        let edges_from_c = snapshot.edges_for_node("c");
        assert_eq!(edges_from_c.len(), 1); // Only incoming

        let edges_from_d = snapshot.edges_for_node("d");
        assert_eq!(edges_from_d.len(), 1); // Only incoming
    }

    #[tokio::test]
    async fn test_full_integration_workflow() {
        use aggregators::StateAggregator;
        use std::sync::Arc;

        // Mock aggregator
        struct TestAggregator;

        #[async_trait::async_trait]
        impl StateAggregator for TestAggregator {
            async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
                let node1 = StateNode::new(
                    "test-1".to_string(),
                    "Test 1".to_string(),
                    NodeType::Persona,
                    StateLayer::Personas,
                );
                let node2 = StateNode::new(
                    "test-2".to_string(),
                    "Test 2".to_string(),
                    NodeType::Entity,
                    StateLayer::Lifecycle,
                );
                let edge =
                    StateEdge::new("test-1".to_string(), "test-2".to_string(), "owns".to_string());

                Ok((vec![node1, node2], vec![edge]))
            }

            fn layer(&self) -> StateLayer {
                StateLayer::Personas
            }
        }

        // Create engine and register aggregator
        let mut engine = WorldStateEngine::new();
        engine.register_aggregator(Arc::new(TestAggregator));

        // Create snapshot
        let snapshot = engine.create_snapshot().await.unwrap();
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);

        // Query by node type
        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);

        let query = WorldStateQuery::new().with_node_types(node_types);
        let filtered = engine.query(&query).await.unwrap();

        assert_eq!(filtered.nodes.len(), 1);
        assert_eq!(filtered.nodes[0].id, "test-1");

        // Get all snapshots
        let all_snapshots = engine.get_all_snapshots().await;
        assert_eq!(all_snapshots.len(), 2); // One from create_snapshot, one from query
    }

    #[test]
    fn test_node_equality() {
        let node1 = StateNode::new(
            "same-id".to_string(),
            "Label 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        let node2 = StateNode::new(
            "same-id".to_string(),
            "Label 1".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );

        // Nodes with same data should be equal (if timestamps are close enough)
        // Note: This test is time-sensitive due to timestamps
        assert_eq!(node1.id, node2.id);
        assert_eq!(node1.label, node2.label);
        assert_eq!(node1.node_type, node2.node_type);
        assert_eq!(node1.layer, node2.layer);
    }

    #[test]
    fn test_edge_equality() {
        let edge1 = StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string());

        let edge2 = StateEdge::new("a".to_string(), "b".to_string(), "owns".to_string());

        assert_eq!(edge1.from, edge2.from);
        assert_eq!(edge1.to, edge2.to);
        assert_eq!(edge1.relationship_type, edge2.relationship_type);
    }

    #[test]
    fn test_snapshot_cloning() {
        let mut snapshot = WorldStateSnapshot::new();
        let node = StateNode::new(
            "test".to_string(),
            "Test".to_string(),
            NodeType::Entity,
            StateLayer::System,
        );
        snapshot.nodes.push(node);

        let cloned = snapshot.clone();
        assert_eq!(snapshot.id, cloned.id);
        assert_eq!(snapshot.nodes.len(), cloned.nodes.len());
        assert_eq!(snapshot.nodes[0].id, cloned.nodes[0].id);
    }

    #[test]
    fn test_query_matches_complex_scenarios() {
        let mut node_types = HashSet::new();
        node_types.insert(NodeType::Persona);
        node_types.insert(NodeType::Entity);

        let mut layers = HashSet::new();
        layers.insert(StateLayer::Personas);

        let mut ids = HashSet::new();
        ids.insert("specific-id".to_string());

        // Build complex query
        let query = WorldStateQuery::new()
            .with_node_types(node_types.clone())
            .with_layers(layers.clone())
            .with_node_ids(ids.clone());

        // Test matching node
        let matching = StateNode::new(
            "specific-id".to_string(),
            "Match".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        assert!(query.matches_node(&matching));

        // Test non-matching nodes
        let wrong_id = StateNode::new(
            "other-id".to_string(),
            "Wrong".to_string(),
            NodeType::Persona,
            StateLayer::Personas,
        );
        assert!(!query.matches_node(&wrong_id));

        let wrong_type = StateNode::new(
            "specific-id".to_string(),
            "Wrong".to_string(),
            NodeType::System,
            StateLayer::Personas,
        );
        assert!(!query.matches_node(&wrong_type));

        let wrong_layer = StateNode::new(
            "specific-id".to_string(),
            "Wrong".to_string(),
            NodeType::Persona,
            StateLayer::Lifecycle,
        );
        assert!(!query.matches_node(&wrong_layer));
    }
}
