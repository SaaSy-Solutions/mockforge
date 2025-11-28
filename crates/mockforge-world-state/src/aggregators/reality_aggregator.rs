//! Reality Aggregator - Collect reality state into world state
//!
//! This aggregator collects reality levels, continuum ratios, and chaos
//! rules from the reality subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use mockforge_core::reality::{RealityEngine, RealityLevel};
use std::sync::Arc;

/// Aggregator for reality state
pub struct RealityAggregator {
    /// Reality engine for accessing reality state
    reality_engine: Option<Arc<RealityEngine>>,
    /// Reality continuum ratio (if available)
    continuum_ratio: Option<f64>,
}

impl RealityAggregator {
    /// Create a new reality aggregator
    pub fn new(reality_engine: Option<Arc<RealityEngine>>, continuum_ratio: Option<f64>) -> Self {
        Self {
            reality_engine,
            continuum_ratio,
        }
    }
}

#[async_trait]
impl StateAggregator for RealityAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Create a node for the reality level
        if let Some(ref engine) = self.reality_engine {
            let level = engine.get_level().await;
            let config = engine.get_config().await;

            let mut reality_node = StateNode::new(
                "reality:main".to_string(),
                "Reality Configuration".to_string(),
                NodeType::System,
                StateLayer::Reality,
            );

            reality_node.set_state(format!("{:?}", level));
            reality_node
                .set_property("level".to_string(), serde_json::json!(format!("{:?}", level)));
            reality_node.set_property("level_value".to_string(), serde_json::json!(level as u8));

            // Add chaos configuration
            let chaos_config = engine.get_chaos_config().await;
            reality_node
                .set_property("chaos_enabled".to_string(), serde_json::json!(chaos_config.enabled));

            // Add latency profile
            let latency_profile = engine.get_latency_profile().await;
            reality_node.set_property(
                "latency_base_ms".to_string(),
                serde_json::json!(latency_profile.base_ms),
            );

            nodes.push(reality_node);
        }

        // Create a node for reality continuum
        if let Some(ratio) = self.continuum_ratio {
            let mut continuum_node = StateNode::new(
                "reality:continuum".to_string(),
                "Reality Continuum".to_string(),
                NodeType::System,
                StateLayer::Reality,
            );

            continuum_node.set_property("blend_ratio".to_string(), serde_json::json!(ratio));
            continuum_node.set_property(
                "mock_percentage".to_string(),
                serde_json::json!((1.0 - ratio) * 100.0),
            );
            continuum_node
                .set_property("real_percentage".to_string(), serde_json::json!(ratio * 100.0));

            nodes.push(continuum_node);
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Reality
    }
}
