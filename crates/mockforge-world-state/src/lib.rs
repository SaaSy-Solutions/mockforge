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
