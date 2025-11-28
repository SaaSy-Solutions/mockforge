//! Failure analysis and root-cause narrative generation
//!
//! This module provides AI-powered analysis of request failures, generating
//! human-readable narratives that explain why failures occurred and what
//! rules, personas, or contracts caused them.

pub mod context_collector;
pub mod narrative_generator;
pub mod types;

pub use context_collector::FailureContextCollector;
pub use narrative_generator::FailureNarrativeGenerator;
pub use types::*;
