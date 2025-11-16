//! Cross-protocol consistency engine
//!
//! This module provides a unified state model that ensures all protocols
//! (HTTP, GraphQL, gRPC, WebSocket, TCP, webhooks) reflect the same underlying
//! state for a given scenario/persona. This creates a coherent world where
//! the frontend feels like it's talking to one unified backend.

pub mod adapters;
pub mod engine;
pub mod types;

pub use engine::ConsistencyEngine;
pub use types::{
    EntityState, PersonaProfile, ProtocolState, SessionInfo, StateChangeEvent, UnifiedState,
};

