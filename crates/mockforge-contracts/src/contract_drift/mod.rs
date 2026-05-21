//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - forecasting: API change forecasting based on historical drift patterns
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.

pub mod forecasting;

pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
