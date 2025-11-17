//! Cross-protocol consistency engine
//!
//! This module provides a unified state model that ensures all protocols
//! (HTTP, GraphQL, gRPC, WebSocket, TCP, webhooks) reflect the same underlying
//! state for a given scenario/persona. This creates a coherent world where
//! the frontend feels like it's talking to one unified backend.

pub mod adapters;
pub mod engine;
pub mod persona_graph_response;
pub mod state_model_registry;
pub mod types;

pub use engine::ConsistencyEngine;
pub use persona_graph_response::{
    enrich_order_response, enrich_response_via_graph, enrich_user_response,
    get_user_orders_via_graph,
};
pub use state_model_registry::StateModelRegistry;
pub use types::{
    EntityState, PersonaProfile, ProtocolState, SessionInfo, StateChangeEvent, UnifiedState,
};
