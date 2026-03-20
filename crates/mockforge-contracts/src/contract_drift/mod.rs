//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - consumer_mapping: Endpoint to SDK method to consuming app relationships
//! - fitness: Fitness function types for validating contract changes
//! - forecasting: API change forecasting based on historical drift patterns
//!
//! NOTE: The following remain in `mockforge-core::contract_drift` due to dependencies:
//! - budget_engine (depends on `OpenApiSpec`)
//! - breaking_change_detector (depends on `ai_contract_diff::Mismatch`)
//! - field_tracking, types (depend on `ai_contract_diff` types)
//! - threat_modeling (depends on `OpenApiSpec` and LLM)
//! - protocol contracts (gRPC, WebSocket, MQTT/Kafka — depend on `ai_contract_diff` types)

pub mod consumer_mapping;
pub mod fitness;
pub mod forecasting;

pub use consumer_mapping::{
    AppType, ConsumerImpact, ConsumerImpactAnalyzer, ConsumerMapping, ConsumerMappingRegistry,
    ConsumingApp, SDKMethod,
};
pub use fitness::{FitnessFunction, FitnessFunctionType, FitnessScope, FitnessTestResult};
pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
