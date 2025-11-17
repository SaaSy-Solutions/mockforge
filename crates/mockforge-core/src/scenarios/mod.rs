//! Scenario-First SDKs
//!
//! This module provides high-level scenario execution APIs that chain multiple
//! endpoint calls together, enabling developers to work with business-level
//! scenarios (e.g., "CheckoutSuccess") instead of individual API calls.

pub mod executor;
pub mod registry;
pub mod types;

pub use executor::ScenarioExecutor;
pub use registry::ScenarioRegistry;
pub use types::{ScenarioDefinition, ScenarioParameter, ScenarioResult, ScenarioStep, StepResult};
