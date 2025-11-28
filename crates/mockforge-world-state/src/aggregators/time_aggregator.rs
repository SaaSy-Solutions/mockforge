//! Time Aggregator - Collect time/temporal state into world state
//!
//! This aggregator collects virtual clock state, scheduled events, and
//! time scale information from the time travel subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use mockforge_core::time_travel::VirtualClock;
use std::sync::Arc;

/// Aggregator for time/temporal state
pub struct TimeAggregator {
    /// Virtual clock (if available)
    virtual_clock: Option<Arc<VirtualClock>>,
}

impl TimeAggregator {
    /// Create a new time aggregator
    pub fn new(virtual_clock: Option<Arc<VirtualClock>>) -> Self {
        Self { virtual_clock }
    }
}

#[async_trait]
impl StateAggregator for TimeAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let edges = Vec::new();

        if let Some(ref clock) = self.virtual_clock {
            let mut time_node = StateNode::new(
                "time:virtual_clock".to_string(),
                "Virtual Clock".to_string(),
                NodeType::System,
                StateLayer::Time,
            );

            let now = clock.now();
            let enabled = clock.is_enabled();
            let scale = clock.get_scale();

            time_node.set_property("enabled".to_string(), serde_json::json!(enabled));
            time_node.set_property("current_time".to_string(), serde_json::json!(now.to_rfc3339()));
            time_node.set_property("scale_factor".to_string(), serde_json::json!(scale));

            if enabled {
                time_node.set_state("enabled".to_string());
            } else {
                time_node.set_state("disabled".to_string());
            }

            nodes.push(time_node);
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Time
    }
}
