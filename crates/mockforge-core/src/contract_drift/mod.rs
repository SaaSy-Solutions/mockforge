//! Contract drift detection and budget management
//!
//! This module provides functionality for tracking contract drift, managing drift budgets,
//! and detecting breaking changes according to configurable rules.

pub mod breaking_change_detector;
pub mod budget_engine;
pub mod field_tracking;
pub mod types;

pub use breaking_change_detector::BreakingChangeDetector;
pub use budget_engine::DriftBudgetEngine;
pub use field_tracking::{FieldCountRecord, FieldCountTracker};
pub use types::{
    BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType, DriftBudget,
    DriftBudgetConfig, DriftMetrics, DriftResult,
};
