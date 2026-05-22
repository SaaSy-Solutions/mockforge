//! Pillars: [Contracts]
//!
//! Contract drift detection — independent subsystems
//!
//! This module contains the independently extractable parts of contract drift:
//! - breaking_change_detector: Three-way classification of contract diffs (breaking / potentially-breaking / non-breaking)
//! - field_tracking: Field-count tracking + history for drift-budget calculations
//! - forecasting: API change forecasting based on historical drift patterns
//! - grpc_contract: gRPC contract implementation (prost_reflect)
//! - mqtt_kafka_contracts: MQTT and Kafka contract implementations (jsonschema)
//! - protocol_contracts: Protocol-agnostic contract trait + registry
//! - types: Shared drift-related types (DriftBudget, DriftMetrics, etc.) + drift_result_from_diff helper
//! - websocket_contract: WebSocket contract implementation (jsonschema)
//!
//! NOTE: Only `budget_engine` remains in `mockforge-core::contract_drift`,
//! because it depends on `mockforge-openapi::OpenApiSpec` and on
//! in-core sibling types (`consumer_mapping::ConsumerImpactAnalyzer`,
//! `fitness::FitnessFunctionRegistry`). Moving it would require contracts
//! to depend on `mockforge-openapi`, expanding the contracts surface area.
//! Tracked for future re-audit at issue #604's closing comment.
//!
//! Update from earlier NOTE: the previously-listed blockers
//! (`ai_contract_diff::Mismatch`, `ContractDiffResult`, etc.) are
//! actually in `mockforge-foundation::contract_diff_types` already —
//! they were promoted in an earlier migration (Phase 6 / A5). Modules
//! that used those types as their only "core-only" dep moved to
//! contracts in #604.
//!
//! `consumer_mapping` and `fitness` previously lived here as duplicates of the
//! core copies; they were deleted in #602 since the contracts copies had no
//! callers. The 3 shared `FitnessFunction*` types live in
//! `mockforge_foundation::contract_drift_types`.

pub mod breaking_change_detector;
pub mod field_tracking;
pub mod forecasting;
pub mod grpc_contract;
pub mod mqtt_kafka_contracts;
pub mod protocol_contracts;
pub mod types;
pub mod websocket_contract;

pub use breaking_change_detector::BreakingChangeDetector;
pub use field_tracking::{FieldCountRecord, FieldCountTracker};
pub use forecasting::{
    ChangeForecast, ForecastAggregationLevel, ForecastPattern, ForecastStatistics, Forecaster,
    ForecastingConfig, PatternAnalysis, PatternAnalyzer, PatternSignature, PatternType,
    SeasonalPattern, StatisticalModel,
};
pub use grpc_contract::{diff_grpc_contracts, GrpcContract};
pub use mqtt_kafka_contracts::{
    diff_kafka_contracts, diff_mqtt_contracts, EvolutionRules, KafkaContract, KafkaTopicSchema,
    MqttContract, MqttTopicSchema, SchemaFormat, TopicSchema,
};
pub use protocol_contracts::{
    classify_change, compare_contracts, extract_breaking_changes, generate_grpc_drift_report,
    ChangeClassification, ContractError, ContractMetadata, ContractOperation, ContractRequest,
    OperationType, ProtocolContract, ProtocolContractRegistry, ValidationError, ValidationResult,
};
pub use types::{
    drift_result_from_diff, BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType,
    DriftBudget, DriftBudgetConfig, DriftMetrics, DriftResult,
};
pub use websocket_contract::{
    diff_websocket_contracts, MessageDirection, WebSocketContract, WebSocketMessageType,
};
