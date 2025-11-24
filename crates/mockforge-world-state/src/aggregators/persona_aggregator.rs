//! Persona Aggregator - Collect persona state into world state
//!
//! This aggregator collects persona profiles, relationships, and graph
//! information from the persona subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use mockforge_data::persona_graph::PersonaGraph;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

/// Aggregator for persona state
pub struct PersonaAggregator {
    /// Persona graph (if available)
    persona_graph: Option<Arc<PersonaGraph>>,
}

impl PersonaAggregator {
    /// Create a new persona aggregator
    pub fn new(persona_graph: Option<Arc<PersonaGraph>>) -> Self {
        Self { persona_graph }
    }
}

#[async_trait]
impl StateAggregator for PersonaAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Aggregate persona graph relationships
        if let Some(ref graph) = self.persona_graph {
            // Get all nodes from the graph
            let graph_nodes = graph.get_all_nodes();

            for node in graph_nodes {
                // Create a state node for each persona node
                let mut state_node = StateNode::new(
                    node.persona_id.clone(),
                    format!("Persona: {}", node.persona_id),
                    NodeType::Persona,
                    StateLayer::Personas,
                );

                // Add persona metadata
                state_node
                    .set_property("entity_type".to_string(), serde_json::json!(node.entity_type));

                // Add relationships as properties
                for (rel_type, related_ids) in &node.relationships {
                    state_node.set_property(
                        format!("relationship_{}", rel_type),
                        serde_json::json!(related_ids),
                    );
                }

                nodes.push(state_node);

                // Create edges for relationships
                for (rel_type, related_ids) in &node.relationships {
                    for related_id in related_ids {
                        let edge = StateEdge::new(
                            node.persona_id.clone(),
                            related_id.clone(),
                            rel_type.clone(),
                        );
                        edges.push(edge);
                    }
                }
            }
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Personas
    }
}
