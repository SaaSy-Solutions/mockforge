//! Protocol-agnostic contract types
//!
//! Extracted from `mockforge-core::contract_drift::protocol_contracts` so
//! consumers can reference the contract abstraction without depending on
//! deprecated core modules.
//!
//! The `ProtocolContract` trait and supporting data types live here. The
//! per-protocol implementations (HTTP/OpenAPI, gRPC, WebSocket, MQTT, Kafka)
//! remain in core because they depend on protocol-specific libraries and
//! `OpenApiSpec`.

use crate::contract_diff_types::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
use crate::protocol::Protocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol-agnostic contract definition
///
/// This trait allows different protocol implementations (HTTP, gRPC, WebSocket, etc.)
/// to provide a unified interface for contract drift detection.
#[async_trait::async_trait]
pub trait ProtocolContract: Send + Sync {
    /// Get the protocol type this contract represents
    fn protocol(&self) -> Protocol;

    /// Get a unique identifier for this contract
    fn contract_id(&self) -> &str;

    /// Get the contract version
    fn version(&self) -> &str;

    /// Get all operations/methods/topics defined in this contract
    fn operations(&self) -> Vec<ContractOperation>;

    /// Get a specific operation by identifier
    fn get_operation(&self, operation_id: &str) -> Option<&ContractOperation>;

    /// Compare this contract with another contract of the same protocol
    ///
    /// Returns a `ContractDiffResult` describing the differences
    async fn diff(&self, other: &dyn ProtocolContract)
        -> Result<ContractDiffResult, ContractError>;

    /// Validate a request/message against this contract
    ///
    /// Returns validation errors if the request doesn't match the contract
    async fn validate(
        &self,
        operation_id: &str,
        request: &ContractRequest,
    ) -> Result<ValidationResult, ContractError>;

    /// Get schema information for an operation
    ///
    /// Returns a JSON-serializable representation of the schema
    fn get_schema(&self, operation_id: &str) -> Option<serde_json::Value>;

    /// Serialize the contract to a JSON representation
    fn to_json(&self) -> Result<serde_json::Value, ContractError>;
}

/// An operation defined in a contract (method, endpoint, topic, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractOperation {
    /// Unique identifier for this operation
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Operation type (varies by protocol)
    pub operation_type: OperationType,
    /// Input schema (request/message schema)
    pub input_schema: Option<serde_json::Value>,
    /// Output schema (response/message schema)
    pub output_schema: Option<serde_json::Value>,
    /// Metadata (tags, descriptions, etc.)
    pub metadata: HashMap<String, String>,
}

/// Type of operation (protocol-specific)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// HTTP endpoint (GET, POST, etc.)
    HttpEndpoint {
        /// HTTP method (GET, POST, PUT, DELETE, etc.)
        method: String,
        /// Endpoint path (e.g., "/api/users")
        path: String,
    },
    /// gRPC method (service.method)
    GrpcMethod {
        /// gRPC service name
        service: String,
        /// gRPC method name
        method: String,
    },
    /// WebSocket message type
    WebSocketMessage {
        /// Message type identifier
        message_type: String,
        /// Optional topic or channel name
        topic: Option<String>,
    },
    /// MQTT topic
    MqttTopic {
        /// MQTT topic name
        topic: String,
        /// Quality of Service level (0, 1, or 2)
        qos: Option<u8>,
    },
    /// Kafka topic
    KafkaTopic {
        /// Kafka topic name
        topic: String,
        /// Optional key schema identifier (Avro schema ID, etc.)
        key_schema: Option<String>,
        /// Optional value schema identifier (Avro schema ID, etc.)
        value_schema: Option<String>,
    },
}

/// Protocol-agnostic request representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRequest {
    /// Protocol type
    pub protocol: Protocol,
    /// Operation identifier
    pub operation_id: String,
    /// Request payload (serialized)
    pub payload: Vec<u8>,
    /// Content type or encoding
    pub content_type: Option<String>,
    /// Additional metadata (headers, properties, etc.)
    pub metadata: HashMap<String, String>,
}

/// Validation result for contract validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the request is valid
    pub valid: bool,
    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Path to the field causing the error (JSONPath, field name, etc.)
    pub path: Option<String>,
    /// Error code or type
    pub code: Option<String>,
}

/// Contract-related errors
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    /// Contract was not found in the registry
    #[error("Contract not found: {0}")]
    NotFound(String),
    /// Contract format is invalid or cannot be parsed
    #[error("Invalid contract format: {0}")]
    InvalidFormat(String),
    /// Protocol is not supported for contract operations
    #[error("Unsupported protocol: {0:?}")]
    UnsupportedProtocol(Protocol),
    /// Operation was not found in the contract
    #[error("Operation not found: {0}")]
    OperationNotFound(String),
    /// Schema validation failed
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),
    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// I/O error (file reading, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Other error occurred
    #[error("Other error: {0}")]
    Other(String),
}

/// Contract metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    /// Contract name
    pub name: String,
    /// Contract version
    pub version: String,
    /// Protocol type
    pub protocol: Protocol,
    /// Description
    pub description: Option<String>,
    /// Tags or categories
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: Option<i64>,
    /// Last update timestamp
    pub updated_at: Option<i64>,
}

/// Classification result for a change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeClassification {
    /// Whether this change is additive (new methods, fields, services)
    pub is_additive: bool,
    /// Whether this change is breaking (removed methods, type changes, etc.)
    pub is_breaking: bool,
    /// Category of change (e.g., "method_added", "method_removed", "type_changed")
    pub change_category: Option<String>,
}

/// Extract change classification from a mismatch
///
/// Uses the context field to determine if a change is additive, breaking, or both
pub fn classify_change(mismatch: &Mismatch) -> ChangeClassification {
    // Check if classification is already in context (from gRPC diff)
    let is_additive =
        mismatch.context.get("is_additive").and_then(|v| v.as_bool()).unwrap_or(false);

    let is_breaking = mismatch.context.get("is_breaking").and_then(|v| v.as_bool()).unwrap_or({
        // Fallback: infer from severity and type
        matches!(mismatch.severity, MismatchSeverity::Critical | MismatchSeverity::High)
            && matches!(
                mismatch.mismatch_type,
                MismatchType::MissingRequiredField
                    | MismatchType::TypeMismatch
                    | MismatchType::EndpointNotFound
                    | MismatchType::MethodNotAllowed
                    | MismatchType::SchemaMismatch
            )
    });

    let change_category = mismatch
        .context
        .get("change_category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ChangeClassification {
        is_additive,
        is_breaking,
        change_category,
    }
}

/// Helper function to extract breaking changes from a diff result
///
/// Breaking changes are defined as mismatches with Critical or High severity
/// that indicate structural incompatibilities (missing required fields, type
/// mismatches, etc.)
pub fn extract_breaking_changes(diff: &ContractDiffResult) -> Vec<&Mismatch> {
    diff.mismatches
        .iter()
        .filter(|m| {
            matches!(m.severity, MismatchSeverity::Critical | MismatchSeverity::High)
                && matches!(
                    m.mismatch_type,
                    MismatchType::MissingRequiredField
                        | MismatchType::TypeMismatch
                        | MismatchType::EndpointNotFound
                        | MismatchType::MethodNotAllowed
                        | MismatchType::SchemaMismatch
                )
        })
        .collect()
}

// ============================================================================
// WebSocket, MQTT, Kafka topic/message schema types (A21) — pure data;
// per-protocol contract impls (WebSocketContract, MqttContract, KafkaContract)
// stay in core because they hold compiled JSON schema caches and perform
// validation.
// ============================================================================

/// WebSocket message type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageType {
    /// Unique identifier for this message type
    pub message_type: String,
    /// Optional topic or channel name
    pub topic: Option<String>,
    /// JSON schema for this message type
    pub schema: serde_json::Value,
    /// Direction: "inbound" (client to server), "outbound" (server to client), or "bidirectional"
    pub direction: MessageDirection,
    /// Description of this message type
    pub description: Option<String>,
    /// Example message payload
    pub example: Option<serde_json::Value>,
}

/// Message direction for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    /// Message sent from client to server
    Inbound,
    /// Message sent from server to client
    Outbound,
    /// Message can be sent in either direction
    Bidirectional,
}

/// MQTT topic schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttTopicSchema {
    /// Topic name (supports wildcards + and #)
    pub topic: String,
    /// Quality of Service level (0, 1, or 2)
    pub qos: Option<u8>,
    /// JSON schema for messages on this topic
    pub schema: serde_json::Value,
    /// Whether messages are retained
    pub retained: Option<bool>,
    /// Description of this topic
    pub description: Option<String>,
    /// Example message payload
    pub example: Option<serde_json::Value>,
}

/// Schema format for Kafka messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaFormat {
    /// JSON schema format
    Json,
    /// Avro schema format
    Avro,
    /// Protobuf schema format
    Protobuf,
}

/// Kafka topic schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopicSchema {
    /// Topic name
    pub topic: String,
    /// Key schema (optional - for keyed messages)
    pub key_schema: Option<TopicSchema>,
    /// Value schema (required - message payload)
    pub value_schema: TopicSchema,
    /// Number of partitions
    pub partitions: Option<u32>,
    /// Replication factor
    pub replication_factor: Option<u16>,
    /// Description of this topic
    pub description: Option<String>,
    /// Evolution rules for schema changes
    pub evolution_rules: Option<EvolutionRules>,
}

/// Schema definition for a topic (key or value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSchema {
    /// Schema format (JSON, Avro, Protobuf)
    pub format: SchemaFormat,
    /// Schema definition (JSON schema, Avro schema JSON, or proto descriptor)
    pub schema: serde_json::Value,
    /// Schema registry ID (if using schema registry)
    pub schema_id: Option<String>,
    /// Schema version
    pub version: Option<String>,
}

/// Evolution rules for schema changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRules {
    /// Allow backward compatible changes (add optional fields, etc.)
    pub allow_backward_compatible: bool,
    /// Allow forward compatible changes (remove optional fields, etc.)
    pub allow_forward_compatible: bool,
    /// Require explicit version bump for breaking changes
    pub require_version_bump: bool,
}

impl Default for EvolutionRules {
    fn default() -> Self {
        Self {
            allow_backward_compatible: true,
            allow_forward_compatible: false,
            require_version_bump: true,
        }
    }
}
