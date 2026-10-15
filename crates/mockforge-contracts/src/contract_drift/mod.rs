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
//! Budget evaluation consumes typed caller-provided fitness and consumer sources.
//! The Core compatibility layer retains its concrete registry/configuration APIs.

pub mod breaking_change_detector;
pub mod budget_engine;
pub mod budget_providers;
pub mod field_tracking;
pub mod forecasting;
pub mod grpc_contract;
pub mod mqtt_kafka_contracts;
pub mod protocol_contracts;
pub mod types;
pub mod websocket_contract;

pub use breaking_change_detector::BreakingChangeDetector;
pub use budget_engine::DriftBudgetEngine;
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
