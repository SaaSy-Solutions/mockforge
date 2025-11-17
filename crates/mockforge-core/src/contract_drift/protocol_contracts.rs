//! Protocol-agnostic contract abstractions for multi-protocol drift detection
//!
//! This module provides a unified interface for contract definitions across different
//! protocols (HTTP/OpenAPI, gRPC, WebSocket, MQTT, Kafka), enabling consistent drift
//! detection and analysis regardless of the transport layer.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity};
use crate::protocol_abstraction::Protocol;
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

/// Registry for managing protocol contracts
pub struct ProtocolContractRegistry {
    contracts: HashMap<String, Box<dyn ProtocolContract>>,
}

impl ProtocolContractRegistry {
    /// Create a new contract registry
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    /// Register a contract
    pub fn register(&mut self, contract: Box<dyn ProtocolContract>) {
        let id = contract.contract_id().to_string();
        self.contracts.insert(id, contract);
    }

    /// Get a contract by ID
    pub fn get(&self, contract_id: &str) -> Option<&dyn ProtocolContract> {
        self.contracts.get(contract_id).map(|c| c.as_ref())
    }

    /// List all contracts
    pub fn list(&self) -> Vec<&dyn ProtocolContract> {
        self.contracts.values().map(|c| c.as_ref()).collect()
    }

    /// List contracts by protocol
    pub fn list_by_protocol(&self, protocol: Protocol) -> Vec<&dyn ProtocolContract> {
        self.contracts
            .values()
            .filter(|c| c.protocol() == protocol)
            .map(|c| c.as_ref())
            .collect()
    }

    /// Remove a contract
    pub fn remove(&mut self, contract_id: &str) -> Option<Box<dyn ProtocolContract>> {
        self.contracts.remove(contract_id)
    }
}

impl Default for ProtocolContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to compare two contracts and generate drift analysis
pub async fn compare_contracts(
    old_contract: &dyn ProtocolContract,
    new_contract: &dyn ProtocolContract,
) -> Result<ContractDiffResult, ContractError> {
    // Ensure protocols match
    if old_contract.protocol() != new_contract.protocol() {
        return Err(ContractError::Other(format!(
            "Cannot compare contracts of different protocols: {:?} vs {:?}",
            old_contract.protocol(),
            new_contract.protocol()
        )));
    }

    // Use the contract's diff method
    old_contract.diff(new_contract).await
}

/// Helper function to extract breaking changes from a diff result
///
/// Breaking changes are defined as mismatches with Critical or High severity
/// that indicate structural incompatibilities (missing required fields, type mismatches, etc.)
pub fn extract_breaking_changes(diff: &ContractDiffResult) -> Vec<&Mismatch> {
    diff.mismatches
        .iter()
        .filter(|m| {
            matches!(m.severity, MismatchSeverity::Critical | MismatchSeverity::High)
                && matches!(
                    m.mismatch_type,
                    crate::ai_contract_diff::MismatchType::MissingRequiredField
                        | crate::ai_contract_diff::MismatchType::TypeMismatch
                        | crate::ai_contract_diff::MismatchType::EndpointNotFound
                        | crate::ai_contract_diff::MismatchType::MethodNotAllowed
                        | crate::ai_contract_diff::MismatchType::SchemaMismatch
                )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_serialization() {
        let op_type = OperationType::HttpEndpoint {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
        };
        let json = serde_json::to_string(&op_type).unwrap();
        assert!(json.contains("http_endpoint"));
        assert!(json.contains("GET"));
        assert!(json.contains("/api/users"));
    }

    #[test]
    fn test_contract_registry() {
        // This test would require a mock implementation of ProtocolContract
        // For now, just test the registry structure
        let mut registry = ProtocolContractRegistry::new();
        assert_eq!(registry.list().len(), 0);
    }
}
