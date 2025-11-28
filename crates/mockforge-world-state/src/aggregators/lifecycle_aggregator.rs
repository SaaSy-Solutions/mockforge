//! Lifecycle Aggregator - Collect lifecycle state into world state
//!
//! This aggregator collects lifecycle states, transitions, and time-based
//! changes from the lifecycle subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use mockforge_data::persona_lifecycle::PersonaLifecycle;
use std::collections::HashMap;
use std::sync::{Arc, RwLock as StdRwLock};

/// Aggregator for lifecycle state
pub struct LifecycleAggregator {
    /// Map of entity IDs to their lifecycle states
    /// This would typically come from a lifecycle manager
    lifecycles: Option<Arc<StdRwLock<HashMap<String, PersonaLifecycle>>>>,
}

impl LifecycleAggregator {
    /// Create a new lifecycle aggregator
    pub fn new(lifecycles: Option<Arc<StdRwLock<HashMap<String, PersonaLifecycle>>>>) -> Self {
        Self { lifecycles }
    }
}

#[async_trait]
impl StateAggregator for LifecycleAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        if let Some(ref lifecycles) = self.lifecycles {
            let lifecycles = lifecycles.read().unwrap();

            for (entity_id, lifecycle) in lifecycles.iter() {
                // Create a node for the lifecycle state
                let mut node = StateNode::new(
                    format!("lifecycle:{}", entity_id),
                    format!("Lifecycle: {}", entity_id),
                    NodeType::Entity,
                    StateLayer::Lifecycle,
                );

                // Set current state
                node.set_state(format!("{:?}", lifecycle.current_state));

                // Add lifecycle metadata
                node.set_property(
                    "current_state".to_string(),
                    serde_json::json!(format!("{:?}", lifecycle.current_state)),
                );
                node.set_property("entity_id".to_string(), serde_json::json!(entity_id));

                // Add transition information
                node.set_property(
                    "entered_at".to_string(),
                    serde_json::json!(lifecycle.state_entered_at.to_rfc3339()),
                );

                nodes.push(node);

                // Create edges for state transitions (if we have transition history)
                // This would require lifecycle to track transition history
            }
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Lifecycle
    }
}
