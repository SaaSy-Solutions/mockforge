//! Scenario type definitions
//!
//! Re-exports from `mockforge_foundation::scenario_types`. The canonical
//! definitions live in foundation so that crates which cannot depend on
//! `mockforge-core` (e.g. `mockforge-intelligence`) can still consume
//! `ScenarioDefinition` / `ScenarioStep`. See foundation's `scenario_types`
//! for the actual types — this module exists for backwards-compatible access
//! via the `mockforge_core::scenarios::types::*` path.

pub use mockforge_foundation::scenario_types::{
    ScenarioDefinition, ScenarioParameter, ScenarioResult, ScenarioStep, StepResult,
};
