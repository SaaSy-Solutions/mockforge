//! State Aggregators - Collect state from various MockForge subsystems
//!
//! Each aggregator is responsible for collecting and transforming state
//! from a specific subsystem into the unified world state model.

pub mod behavior_aggregator;
pub mod lifecycle_aggregator;
pub mod persona_aggregator;
pub mod protocol_aggregator;
pub mod reality_aggregator;
pub mod recorded_aggregator;
pub mod schema_aggregator;
pub mod time_aggregator;

use crate::model::{StateEdge, StateNode};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for aggregating state from a subsystem
#[async_trait]
pub trait StateAggregator: Send + Sync {
    /// Aggregate state from the subsystem into nodes and edges
    ///
    /// Returns a tuple of (nodes, edges) that represent the current
    /// state of this subsystem.
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)>;

    /// Get the layer this aggregator belongs to
    fn layer(&self) -> crate::model::StateLayer;

    /// Get metadata about this aggregator
    fn metadata(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}
