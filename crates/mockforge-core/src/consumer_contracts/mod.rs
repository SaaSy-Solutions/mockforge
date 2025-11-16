//! Consumer-driven contracts
//!
//! This module provides functionality for tracking consumer usage and detecting
//! consumer-specific breaking changes.

pub mod detector;
pub mod registry;
pub mod types;
pub mod usage_recorder;

pub use detector::ConsumerBreakingChangeDetector;
pub use registry::ConsumerRegistry;
pub use types::{
    Consumer, ConsumerIdentifier, ConsumerType, ConsumerUsage, ConsumerViolation,
};
pub use usage_recorder::UsageRecorder;
