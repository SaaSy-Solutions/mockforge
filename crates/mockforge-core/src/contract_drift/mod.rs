//! Pillars: [Contracts]
//!
//! Contract drift detection and budget management
//!
//! This module provides functionality for tracking contract drift, managing drift budgets,
//! and detecting breaking changes according to configurable rules.
//!
//! Only `budget_engine` remains here. All other contract-drift modules have moved to
//! `mockforge-contracts::contract_drift`. Forwarding re-exports below keep existing
//! `crate::contract_drift::*` paths in core resolving without touching callers.

pub mod budget_engine;
pub mod consumer_mapping;
pub mod fitness;
/// `threat_modeling` lives in `mockforge_intelligence::threat_modeling`
/// (Issue #562 phase 3). Re-exported here so existing
/// `crate::contract_drift::threat_modeling::*` paths keep resolving.
pub use mockforge_intelligence::threat_modeling;

pub use budget_engine::DriftBudgetEngine;
pub use consumer_mapping::{
    AppType, ConsumerImpact, ConsumerImpactAnalyzer, ConsumerMapping, ConsumerMappingRegistry,
    ConsumingApp, SDKMethod,
};
pub use fitness::{
    FitnessEvaluator, FitnessFunction, FitnessFunctionRegistry, FitnessFunctionType, FitnessScope,
    FitnessTestResult,
};
pub use threat_modeling::{
    AggregationLevel, DosAnalyzer, ErrorAnalyzer, PiiDetector, RemediationGenerator,
    RemediationSuggestion, SchemaAnalyzer, ThreatAnalyzer, ThreatAssessment, ThreatCategory,
    ThreatFinding, ThreatLevel, ThreatModelingConfig,
};

// Forwarding re-exports: types, breaking_change_detector, field_tracking, and the
// protocol_contracts cluster have moved to mockforge-contracts. Re-export from there
// so existing `crate::contract_drift::{DriftBudgetConfig, ProtocolContract, …}` paths
// in core keep resolving.
pub use mockforge_contracts::contract_drift::breaking_change_detector::BreakingChangeDetector;
pub use mockforge_contracts::contract_drift::field_tracking::{
    FieldCountRecord, FieldCountTracker,
};
pub use mockforge_contracts::contract_drift::grpc_contract::{diff_grpc_contracts, GrpcContract};
pub use mockforge_contracts::contract_drift::mqtt_kafka_contracts::{
    diff_kafka_contracts, diff_mqtt_contracts, EvolutionRules, KafkaContract, KafkaTopicSchema,
    MqttContract, MqttTopicSchema, SchemaFormat, TopicSchema,
};
pub use mockforge_contracts::contract_drift::protocol_contracts::{
    classify_change, compare_contracts, extract_breaking_changes, generate_grpc_drift_report,
    ChangeClassification, ContractError, ContractMetadata, ContractOperation, ContractRequest,
    OperationType, ProtocolContract, ProtocolContractRegistry, ValidationError, ValidationResult,
};
pub use mockforge_contracts::contract_drift::types::{
    drift_result_from_diff, BreakingChangeRule, BreakingChangeRuleConfig, BreakingChangeRuleType,
    DriftBudget, DriftBudgetConfig, DriftMetrics, DriftResult,
};
pub use mockforge_contracts::contract_drift::websocket_contract::{
    diff_websocket_contracts, MessageDirection, WebSocketContract, WebSocketMessageType,
};
