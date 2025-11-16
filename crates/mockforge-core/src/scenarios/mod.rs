//! Scenario-First SDKs
//!
//! This module provides high-level scenario execution APIs that chain multiple
//! endpoint calls together, enabling developers to work with business-level
//! scenarios (e.g., "CheckoutSuccess") instead of individual API calls.

pub mod types;
pub mod registry;
pub mod executor;

pub use types::{ScenarioDefinition, ScenarioStep, ScenarioParameter, ScenarioResult, StepResult};
pub use registry::ScenarioRegistry;
pub use executor::ScenarioExecutor;
