//! Foundation types for MockForge
//!
//! This crate sits at the bottom of the MockForge dependency graph. It has **no
//! dependencies on other mockforge-* crates**, so both `mockforge-core` and the
//! various extracted crates (`mockforge-proxy`, `mockforge-import`, etc.) can depend
//! on it without creating circular dependencies.
//!
//! Currently exports:
//! - `Error`, `Result` — canonical error/result types used throughout MockForge
//! - `EncryptionError`, `EncryptionResult` — encryption-specific error types

pub mod ai_response;
pub mod ai_studio_types;
pub mod clock;
pub mod contract_diff_types;
pub mod contract_drift_types;
pub mod encryption_error;
pub mod error;
pub mod incidents_types;
pub mod intelligent_behavior;
pub mod latency;
pub mod multi_tenant_types;
pub mod protocol;
pub mod protocol_contract_types;
pub mod response_generation_trace;
pub mod response_selection;
pub mod schema_diff;
pub mod state_machine;
pub mod threat_modeling_types;
pub mod workspace_promotion;

pub use encryption_error::{EncryptionError, EncryptionResult};
pub use error::{Error, Result};
