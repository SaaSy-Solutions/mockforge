//! Protocol Aggregator - Collect multi-protocol state into world state
//!
//! This aggregator collects protocol-specific state, sessions, and entity
//! state from the multi-protocol consistency subsystem.

use crate::aggregators::StateAggregator;
use crate::model::{NodeType, StateEdge, StateLayer, StateNode};
use async_trait::async_trait;
use mockforge_core::consistency::types::UnifiedState;
use std::sync::{Arc, RwLock as StdRwLock};

/// Aggregator for multi-protocol state
pub struct ProtocolAggregator {
    /// Unified state (if available)
    unified_state: Option<Arc<StdRwLock<UnifiedState>>>,
}

impl ProtocolAggregator {
    /// Create a new protocol aggregator
    pub fn new(unified_state: Option<Arc<StdRwLock<UnifiedState>>>) -> Self {
        Self { unified_state }
    }
}

#[async_trait]
impl StateAggregator for ProtocolAggregator {
    async fn aggregate(&self) -> anyhow::Result<(Vec<StateNode>, Vec<StateEdge>)> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        if let Some(ref state) = self.unified_state {
            let state = state.read().unwrap();

            // Create a node for the workspace
            let mut workspace_node = StateNode::new(
                format!("workspace:{}", state.workspace_id),
                format!("Workspace: {}", state.workspace_id),
                NodeType::System,
                StateLayer::Protocols,
            );

            workspace_node
                .set_property("workspace_id".to_string(), serde_json::json!(state.workspace_id));
            workspace_node.set_property(
                "reality_level".to_string(),
                serde_json::json!(format!("{:?}", state.reality_level)),
            );
            workspace_node.set_property(
                "reality_continuum_ratio".to_string(),
                serde_json::json!(state.reality_continuum_ratio),
            );

            nodes.push(workspace_node);

            // Create nodes for each protocol
            for (protocol, protocol_state) in &state.protocol_states {
                let mut protocol_node = StateNode::new(
                    format!("protocol:{:?}", protocol),
                    format!("Protocol: {:?}", protocol),
                    NodeType::Protocol,
                    StateLayer::Protocols,
                );

                protocol_node.set_property(
                    "protocol".to_string(),
                    serde_json::json!(format!("{:?}", protocol)),
                );
                protocol_node.set_property(
                    "active_sessions".to_string(),
                    serde_json::json!(protocol_state.active_sessions.len()),
                );

                nodes.push(protocol_node);

                // Create edge from workspace to protocol
                let edge = StateEdge::new(
                    format!("workspace:{}", state.workspace_id),
                    format!("protocol:{:?}", protocol),
                    "has_protocol".to_string(),
                );
                edges.push(edge);

                // Create nodes for sessions
                for session in &protocol_state.active_sessions {
                    let mut session_node = StateNode::new(
                        format!("session:{}", session.session_id),
                        format!("Session: {}", session.session_id),
                        NodeType::Session,
                        StateLayer::Protocols,
                    );

                    session_node.set_property(
                        "session_id".to_string(),
                        serde_json::json!(session.session_id),
                    );
                    if let Some(ref persona_id) = session.persona_id {
                        session_node
                            .set_property("persona_id".to_string(), serde_json::json!(persona_id));
                    }

                    nodes.push(session_node);

                    // Create edge from protocol to session
                    let edge = StateEdge::new(
                        format!("protocol:{:?}", protocol),
                        format!("session:{}", session.session_id),
                        "has_session".to_string(),
                    );
                    edges.push(edge);
                }
            }

            // Create nodes for entities
            for (entity_key, entity_state) in &state.entity_state {
                let mut entity_node = StateNode::new(
                    format!("entity:{}", entity_key),
                    format!("Entity: {}", entity_key),
                    NodeType::Entity,
                    StateLayer::Protocols,
                );

                entity_node.set_property(
                    "entity_type".to_string(),
                    serde_json::json!(entity_state.entity_type),
                );
                entity_node.set_property(
                    "entity_id".to_string(),
                    serde_json::json!(entity_state.entity_id),
                );

                nodes.push(entity_node);

                // Create edge from workspace to entity
                let edge = StateEdge::new(
                    format!("workspace:{}", state.workspace_id),
                    format!("entity:{}", entity_key),
                    "has_entity".to_string(),
                );
                edges.push(edge);
            }
        }

        Ok((nodes, edges))
    }

    fn layer(&self) -> StateLayer {
        StateLayer::Protocols
    }
}
