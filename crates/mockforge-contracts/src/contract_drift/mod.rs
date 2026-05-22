//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - breaking_change_detector: Three-way classification of contract diffs (breaking / potentially-breaking / non-breaking)
//! - field_tracking: Field-count tracking + history for drift-budget calculations
//! - forecasting: API change forecasting based on historical drift patterns
//! - types: Shared drift-related types (DriftBudget, DriftMetrics, etc.) + drift_result_from_diff helper
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.

pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;
pub mod types;

pub use breaking_change_detector::BreakingChangeDetector;
pub use field_tracking::{FieldCountRecord, FieldCountTracker};
pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
pub use types::{
    drift_result_from_diff, BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType,
    DriftBudget, DriftBudgetConfig, DriftMetrics, DriftResult,
};
