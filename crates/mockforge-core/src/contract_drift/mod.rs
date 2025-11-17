//! Contract drift detection and budget management
//!
//! This module provides functionality for tracking contract drift, managing drift budgets,
//! and detecting breaking changes according to configurable rules.

pub mod breaking_change_detector;
pub mod budget_engine;
pub mod consumer_mapping;
pub mod field_tracking;
pub mod fitness;
pub mod grpc_contract;
pub mod mqtt_kafka_contracts;
pub mod protocol_contracts;
pub mod types;
pub mod websocket_contract;

pub use breaking_change_detector::BreakingChangeDetector;
pub use budget_engine::DriftBudgetEngine;
pub use consumer_mapping::{
    AppType, ConsumerImpact, ConsumerImpactAnalyzer, ConsumerMapping, ConsumerMappingRegistry,
    ConsumingApp, SDKMethod,
};
pub use field_tracking::{FieldCountRecord, FieldCountTracker};
pub use fitness::{
    FitnessEvaluator, FitnessFunction, FitnessFunctionRegistry, FitnessFunctionType, FitnessScope,
    FitnessTestResult,
};
pub use grpc_contract::{diff_grpc_contracts, GrpcContract};
pub use mqtt_kafka_contracts::{
    diff_kafka_contracts, diff_mqtt_contracts, EvolutionRules, KafkaContract, KafkaTopicSchema,
    MqttContract, MqttTopicSchema, SchemaFormat, TopicSchema,
};
pub use protocol_contracts::{
    compare_contracts, extract_breaking_changes, ContractError, ContractMetadata,
    ContractOperation, ContractRequest, OperationType, ProtocolContract, ProtocolContractRegistry,
    ValidationError, ValidationResult,
};
pub use types::{
    BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType, DriftBudget,
    DriftBudgetConfig, DriftMetrics, DriftResult,
};
pub use websocket_contract::{
    diff_websocket_contracts, MessageDirection, WebSocketContract, WebSocketMessageType,
};
