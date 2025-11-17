//! MQTT and Kafka contract implementations for protocol-agnostic contract drift detection
//!
//! This module provides `MqttContract` and `KafkaContract` structs that implement the
//! `ProtocolContract` trait for MQTT and Kafka protocols, enabling drift detection and
//! analysis for topic-based messaging systems.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchSeverity, MismatchType};
use crate::contract_drift::protocol_contracts::{
    ContractError, ContractOperation, ContractRequest, OperationType, ProtocolContract,
    ValidationError, ValidationResult,
};
use crate::protocol_abstraction::Protocol;
use jsonschema::{self, Draft, Validator as JSONSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// MQTT Contract
// ============================================================================

/// MQTT topic schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttTopicSchema {
    /// Topic name (supports wildcards + and #)
    pub topic: String,
    /// Quality of Service level (0, 1, or 2)
    pub qos: Option<u8>,
    /// JSON schema for messages on this topic
    pub schema: Value,
    /// Whether messages are retained
    pub retained: Option<bool>,
    /// Description of this topic
    pub description: Option<String>,
    /// Example message payload
    pub example: Option<Value>,
}

/// MQTT contract implementation
///
/// Defines topic schemas for MQTT messaging, enabling schema validation
/// and drift detection for IoT and pub/sub systems.
pub struct MqttContract {
    /// Unique identifier for this contract
    contract_id: String,
    /// Contract version
    version: String,
    /// Map of topic names to topic schemas
    topics: HashMap<String, MqttTopicSchema>,
    /// Compiled JSON schemas for validation (cached)
    #[allow(dead_code)]
    schema_cache: HashMap<String, JSONSchema>,
    /// Cached contract operations for quick lookup
    operations_cache: HashMap<String, ContractOperation>,
    /// Contract metadata
    metadata: HashMap<String, String>,
}

impl MqttContract {
    /// Create a new MQTT contract
    pub fn new(contract_id: String, version: String) -> Self {
        Self {
            contract_id,
            version,
            topics: HashMap::new(),
            schema_cache: HashMap::new(),
            operations_cache: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a topic schema to the contract
    pub fn add_topic(&mut self, topic_schema: MqttTopicSchema) -> Result<(), ContractError> {
        let topic_name = topic_schema.topic.clone();

        // Compile and cache the JSON schema for validation
        let schema = jsonschema::options()
            .with_draft(Draft::Draft7)
            .build(&topic_schema.schema)
            .map_err(|e| ContractError::SchemaValidation(format!("Invalid JSON schema: {}", e)))?;
        self.schema_cache.insert(topic_name.clone(), schema);

        // Add to topics
        self.topics.insert(topic_name.clone(), topic_schema.clone());

        // Cache the contract operation
        let operation = ContractOperation {
            id: topic_name.clone(),
            name: topic_name.clone(),
            operation_type: OperationType::MqttTopic {
                topic: topic_name.clone(),
                qos: topic_schema.qos,
            },
            input_schema: Some(topic_schema.schema.clone()),
            output_schema: Some(topic_schema.schema.clone()), // MQTT is one-way, but schema applies to both publish/subscribe
            metadata: {
                let mut meta = HashMap::new();
                if let Some(retained) = topic_schema.retained {
                    meta.insert("retained".to_string(), retained.to_string());
                }
                if let Some(ref desc) = topic_schema.description {
                    meta.insert("description".to_string(), desc.clone());
                }
                meta
            },
        };
        self.operations_cache.insert(topic_name, operation);

        Ok(())
    }

    /// Remove a topic from the contract
    pub fn remove_topic(&mut self, topic_name: &str) {
        if self.topics.remove(topic_name).is_some() {
            self.schema_cache.remove(topic_name);
            self.operations_cache.remove(topic_name);
        }
    }

    /// Compare two MQTT contracts and detect differences
    fn diff_contracts(&self, other: &MqttContract) -> Result<ContractDiffResult, ContractError> {
        let mut mismatches = Vec::new();

        // Collect all topic names
        let all_topics: std::collections::HashSet<String> =
            self.topics.keys().chain(other.topics.keys()).cloned().collect();

        // Check for removed topics (breaking change)
        for topic_name in &all_topics {
            if self.topics.contains_key(topic_name) && !other.topics.contains_key(topic_name) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: topic_name.clone(),
                    method: None,
                    expected: Some(format!("Topic {} should exist", topic_name)),
                    actual: Some("Topic removed".to_string()),
                    description: format!("Topic {} was removed", topic_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Check for added topics (non-breaking)
        for topic_name in &all_topics {
            if !self.topics.contains_key(topic_name) && other.topics.contains_key(topic_name) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: topic_name.clone(),
                    method: None,
                    expected: None,
                    actual: Some(format!("New topic {}", topic_name)),
                    description: format!("New topic {} was added", topic_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Compare topic schemas for topics that exist in both
        for topic_name in all_topics
            .intersection(&self.topics.keys().cloned().collect::<std::collections::HashSet<_>>())
        {
            if let (Some(old_topic), Some(new_topic)) =
                (self.topics.get(topic_name), other.topics.get(topic_name))
            {
                // Compare QoS changes
                if old_topic.qos != new_topic.qos {
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SchemaMismatch,
                        path: format!("{}.qos", topic_name),
                        method: None,
                        expected: old_topic.qos.map(|q| format!("QoS: {}", q)),
                        actual: new_topic.qos.map(|q| format!("QoS: {}", q)),
                        description: format!(
                            "QoS changed for topic {}: {:?} -> {:?}",
                            topic_name, old_topic.qos, new_topic.qos
                        ),
                        severity: MismatchSeverity::Medium,
                        confidence: 1.0,
                        context: HashMap::new(),
                    });
                }

                // Compare schemas
                let schema_mismatches =
                    Self::compare_json_schemas(&old_topic.schema, &new_topic.schema, topic_name);
                mismatches.extend(schema_mismatches);
            }
        }

        let matches = mismatches.is_empty();
        let confidence = if matches { 1.0 } else { 0.8 };

        Ok(ContractDiffResult {
            matches,
            confidence,
            mismatches,
            recommendations: Vec::new(),
            corrections: Vec::new(),
            metadata: crate::ai_contract_diff::DiffMetadata {
                analyzed_at: chrono::Utc::now(),
                request_source: "mqtt_contract_diff".to_string(),
                contract_version: Some(self.version.clone()),
                contract_format: "mqtt_schema".to_string(),
                endpoint_path: "".to_string(),
                http_method: "".to_string(),
                request_count: 1,
                llm_provider: None,
                llm_model: None,
            },
        })
    }

    /// Compare two JSON schemas and identify differences (shared with Kafka)
    fn compare_json_schemas(
        old_schema: &Value,
        new_schema: &Value,
        path_prefix: &str,
    ) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        // Check for required fields changes
        if let (Some(old_required), Some(new_required)) = (
            old_schema.get("required").and_then(|v| v.as_array()),
            new_schema.get("required").and_then(|v| v.as_array()),
        ) {
            let old_required_set: std::collections::HashSet<&str> =
                old_required.iter().filter_map(|v| v.as_str()).collect();
            let new_required_set: std::collections::HashSet<&str> =
                new_required.iter().filter_map(|v| v.as_str()).collect();

            // Check for newly required fields (breaking change)
            for new_req in new_required_set.difference(&old_required_set) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::MissingRequiredField,
                    path: format!("{}.{}", path_prefix, new_req),
                    method: None,
                    expected: Some(format!("Field {} should be optional", new_req)),
                    actual: Some(format!("Field {} is now required", new_req)),
                    description: format!("Field {} became required", new_req),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Check for property type changes
        if let (Some(old_props), Some(new_props)) = (
            old_schema.get("properties").and_then(|v| v.as_object()),
            new_schema.get("properties").and_then(|v| v.as_object()),
        ) {
            for (prop_name, new_prop_schema) in new_props {
                if let Some(old_prop_schema) = old_props.get(prop_name) {
                    if let (Some(old_type), Some(new_type)) = (
                        old_prop_schema.get("type").and_then(|v| v.as_str()),
                        new_prop_schema.get("type").and_then(|v| v.as_str()),
                    ) {
                        if old_type != new_type {
                            mismatches.push(Mismatch {
                                mismatch_type: MismatchType::TypeMismatch,
                                path: format!("{}.{}", path_prefix, prop_name),
                                method: None,
                                expected: Some(format!("Type: {}", old_type)),
                                actual: Some(format!("Type: {}", new_type)),
                                description: format!(
                                    "Property {} type changed from {} to {}",
                                    prop_name, old_type, new_type
                                ),
                                severity: MismatchSeverity::High,
                                confidence: 1.0,
                                context: HashMap::new(),
                            });
                        }
                    }
                }
            }

            // Check for removed properties
            for prop_name in old_props.keys() {
                if !new_props.contains_key(prop_name) {
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::UnexpectedField,
                        path: format!("{}.{}", path_prefix, prop_name),
                        method: None,
                        expected: Some(format!("Property {} should exist", prop_name)),
                        actual: Some("Property removed".to_string()),
                        description: format!("Property {} was removed", prop_name),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context: HashMap::new(),
                    });
                }
            }
        }

        mismatches
    }

    /// Validate a message against a topic schema
    fn validate_message_against_schema(
        &self,
        topic_name: &str,
        message: &Value,
    ) -> Result<ValidationResult, ContractError> {
        let schema = self
            .schema_cache
            .get(topic_name)
            .ok_or_else(|| ContractError::OperationNotFound(topic_name.to_string()))?;

        // Use iter_errors for validation
        let mut validation_errors = Vec::new();
        for error in schema.iter_errors(message) {
            validation_errors.push(ValidationError {
                message: error.to_string(),
                path: Some(error.instance_path.to_string()),
                code: Some("SCHEMA_VALIDATION_ERROR".to_string()),
            });
        }

        Ok(ValidationResult {
            valid: validation_errors.is_empty(),
            errors: validation_errors,
            warnings: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl ProtocolContract for MqttContract {
    fn protocol(&self) -> Protocol {
        Protocol::Mqtt
    }

    fn contract_id(&self) -> &str {
        &self.contract_id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn operations(&self) -> Vec<ContractOperation> {
        self.operations_cache.values().cloned().collect()
    }

    fn get_operation(&self, operation_id: &str) -> Option<&ContractOperation> {
        self.operations_cache.get(operation_id)
    }

    async fn diff(
        &self,
        other: &dyn ProtocolContract,
    ) -> Result<ContractDiffResult, ContractError> {
        if other.protocol() != Protocol::Mqtt {
            return Err(ContractError::UnsupportedProtocol(other.protocol()));
        }

        Err(ContractError::Other(
            "Direct comparison of MqttContract instances requires type information. \
             Use MqttContract::diff_contracts() for comparing two MqttContract instances."
                .to_string(),
        ))
    }

    async fn validate(
        &self,
        operation_id: &str,
        request: &ContractRequest,
    ) -> Result<ValidationResult, ContractError> {
        // Parse the message payload as JSON
        let message: Value = serde_json::from_slice(&request.payload)
            .map_err(|e| ContractError::SchemaValidation(format!("Invalid JSON: {}", e)))?;

        // Validate against the topic schema
        self.validate_message_against_schema(operation_id, &message)
    }

    fn get_schema(&self, operation_id: &str) -> Option<serde_json::Value> {
        self.topics.get(operation_id).map(|t| t.schema.clone())
    }

    fn to_json(&self) -> Result<serde_json::Value, ContractError> {
        let topics: Vec<serde_json::Value> = self
            .topics
            .values()
            .map(|topic| {
                serde_json::json!({
                    "topic": topic.topic,
                    "qos": topic.qos,
                    "schema": topic.schema,
                    "retained": topic.retained,
                    "description": topic.description,
                    "example": topic.example,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "contract_id": self.contract_id,
            "version": self.version,
            "protocol": "mqtt",
            "topics": topics,
            "metadata": self.metadata,
        }))
    }
}

/// Helper function to compare two MqttContract instances
pub fn diff_mqtt_contracts(
    old_contract: &MqttContract,
    new_contract: &MqttContract,
) -> Result<ContractDiffResult, ContractError> {
    old_contract.diff_contracts(new_contract)
}

// ============================================================================
// Kafka Contract
// ============================================================================

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
    pub schema: Value,
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

/// Kafka contract implementation
///
/// Defines topic schemas for Kafka messaging, enabling schema validation
/// and drift detection for event streaming systems.
pub struct KafkaContract {
    /// Unique identifier for this contract
    contract_id: String,
    /// Contract version
    version: String,
    /// Map of topic names to topic schemas
    topics: HashMap<String, KafkaTopicSchema>,
    /// Compiled JSON schemas for validation (cached)
    #[allow(dead_code)]
    schema_cache: HashMap<String, (Option<JSONSchema>, JSONSchema)>, // (key_schema, value_schema)
    /// Cached contract operations for quick lookup
    operations_cache: HashMap<String, ContractOperation>,
    /// Contract metadata
    metadata: HashMap<String, String>,
}

impl KafkaContract {
    /// Create a new Kafka contract
    pub fn new(contract_id: String, version: String) -> Self {
        Self {
            contract_id,
            version,
            topics: HashMap::new(),
            schema_cache: HashMap::new(),
            operations_cache: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a topic schema to the contract
    pub fn add_topic(&mut self, topic_schema: KafkaTopicSchema) -> Result<(), ContractError> {
        let topic_name = topic_schema.topic.clone();

        // Compile and cache JSON schemas for validation
        // Note: Avro and Protobuf schemas would need additional processing
        let value_schema = match topic_schema.value_schema.format {
            SchemaFormat::Json => jsonschema::options()
                .with_draft(Draft::Draft7)
                .build(&topic_schema.value_schema.schema)
                .map_err(|e| {
                    ContractError::SchemaValidation(format!("Invalid JSON schema: {}", e))
                })?,
            SchemaFormat::Avro | SchemaFormat::Protobuf => {
                // For now, we'll store the schema but not compile it
                // In a full implementation, we'd parse Avro/Protobuf schemas
                return Err(ContractError::Other(
                    "Avro and Protobuf schema validation not yet implemented".to_string(),
                ));
            }
        };

        let key_schema = if let Some(ref key_schema_def) = topic_schema.key_schema {
            match key_schema_def.format {
                SchemaFormat::Json => Some(
                    jsonschema::options()
                        .with_draft(Draft::Draft7)
                        .build(&key_schema_def.schema)
                        .map_err(|e| {
                        ContractError::SchemaValidation(format!("Invalid JSON schema: {}", e))
                    })?,
                ),
                SchemaFormat::Avro | SchemaFormat::Protobuf => {
                    return Err(ContractError::Other(
                        "Avro and Protobuf schema validation not yet implemented".to_string(),
                    ));
                }
            }
        } else {
            None
        };

        self.schema_cache.insert(topic_name.clone(), (key_schema, value_schema));

        // Add to topics
        self.topics.insert(topic_name.clone(), topic_schema.clone());

        // Cache the contract operation
        let operation = ContractOperation {
            id: topic_name.clone(),
            name: topic_name.clone(),
            operation_type: OperationType::KafkaTopic {
                topic: topic_name.clone(),
                key_schema: topic_schema.key_schema.as_ref().and_then(|s| s.schema_id.clone()),
                value_schema: topic_schema.value_schema.schema_id.clone(),
            },
            input_schema: Some(serde_json::json!({
                "key": topic_schema.key_schema.as_ref().map(|s| s.schema.clone()),
                "value": topic_schema.value_schema.schema.clone(),
            })),
            output_schema: Some(serde_json::json!({
                "key": topic_schema.key_schema.as_ref().map(|s| s.schema.clone()),
                "value": topic_schema.value_schema.schema.clone(),
            })),
            metadata: {
                let mut meta = HashMap::new();
                if let Some(partitions) = topic_schema.partitions {
                    meta.insert("partitions".to_string(), partitions.to_string());
                }
                if let Some(ref desc) = topic_schema.description {
                    meta.insert("description".to_string(), desc.clone());
                }
                meta
            },
        };
        self.operations_cache.insert(topic_name, operation);

        Ok(())
    }

    /// Remove a topic from the contract
    pub fn remove_topic(&mut self, topic_name: &str) {
        if self.topics.remove(topic_name).is_some() {
            self.schema_cache.remove(topic_name);
            self.operations_cache.remove(topic_name);
        }
    }

    /// Compare two Kafka contracts and detect differences
    fn diff_contracts(&self, other: &KafkaContract) -> Result<ContractDiffResult, ContractError> {
        let mut mismatches = Vec::new();

        // Collect all topic names
        let all_topics: std::collections::HashSet<String> =
            self.topics.keys().chain(other.topics.keys()).cloned().collect();

        // Check for removed topics (breaking change)
        for topic_name in &all_topics {
            if self.topics.contains_key(topic_name) && !other.topics.contains_key(topic_name) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: topic_name.clone(),
                    method: None,
                    expected: Some(format!("Topic {} should exist", topic_name)),
                    actual: Some("Topic removed".to_string()),
                    description: format!("Topic {} was removed", topic_name),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Check for added topics (non-breaking)
        for topic_name in &all_topics {
            if !self.topics.contains_key(topic_name) && other.topics.contains_key(topic_name) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: topic_name.clone(),
                    method: None,
                    expected: None,
                    actual: Some(format!("New topic {}", topic_name)),
                    description: format!("New topic {} was added", topic_name),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
            }
        }

        // Compare topic schemas for topics that exist in both
        for topic_name in all_topics
            .intersection(&self.topics.keys().cloned().collect::<std::collections::HashSet<_>>())
        {
            if let (Some(old_topic), Some(new_topic)) =
                (self.topics.get(topic_name), other.topics.get(topic_name))
            {
                // Compare key schema changes
                if old_topic.key_schema.is_some() != new_topic.key_schema.is_some() {
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SchemaMismatch,
                        path: format!("{}.key_schema", topic_name),
                        method: None,
                        expected: Some(if old_topic.key_schema.is_some() {
                            "Key schema should exist".to_string()
                        } else {
                            "Key schema should not exist".to_string()
                        }),
                        actual: Some(if new_topic.key_schema.is_some() {
                            "Key schema added".to_string()
                        } else {
                            "Key schema removed".to_string()
                        }),
                        description: format!(
                            "Key schema presence changed for topic {}",
                            topic_name
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context: HashMap::new(),
                    });
                } else if let (Some(old_key), Some(new_key)) =
                    (&old_topic.key_schema, &new_topic.key_schema)
                {
                    if old_key.schema != new_key.schema {
                        let key_mismatches = MqttContract::compare_json_schemas(
                            &old_key.schema,
                            &new_key.schema,
                            &format!("{}.key", topic_name),
                        );
                        mismatches.extend(key_mismatches);
                    }
                }

                // Compare value schema changes
                if old_topic.value_schema.schema != new_topic.value_schema.schema {
                    let value_mismatches = MqttContract::compare_json_schemas(
                        &old_topic.value_schema.schema,
                        &new_topic.value_schema.schema,
                        &format!("{}.value", topic_name),
                    );
                    mismatches.extend(value_mismatches);
                }

                // Check evolution rules compliance
                if let Some(ref evolution_rules) = new_topic.evolution_rules {
                    // Check if changes violate evolution rules
                    let has_breaking_changes = mismatches.iter().any(|m| {
                        matches!(m.severity, MismatchSeverity::Critical | MismatchSeverity::High)
                    });

                    if has_breaking_changes && !evolution_rules.allow_backward_compatible {
                        mismatches.push(Mismatch {
                            mismatch_type: MismatchType::SchemaMismatch,
                            path: format!("{}.evolution_rules", topic_name),
                            method: None,
                            expected: Some("Backward compatible changes only".to_string()),
                            actual: Some("Breaking changes detected".to_string()),
                            description: format!(
                                "Topic {} has breaking changes but evolution rules require backward compatibility",
                                topic_name
                            ),
                            severity: MismatchSeverity::High,
                            confidence: 1.0,
                            context: HashMap::new(),
                        });
                    }
                }
            }
        }

        let matches = mismatches.is_empty();
        let confidence = if matches { 1.0 } else { 0.8 };

        Ok(ContractDiffResult {
            matches,
            confidence,
            mismatches,
            recommendations: Vec::new(),
            corrections: Vec::new(),
            metadata: crate::ai_contract_diff::DiffMetadata {
                analyzed_at: chrono::Utc::now(),
                request_source: "kafka_contract_diff".to_string(),
                contract_version: Some(self.version.clone()),
                contract_format: "kafka_schema".to_string(),
                endpoint_path: "".to_string(),
                http_method: "".to_string(),
                request_count: 1,
                llm_provider: None,
                llm_model: None,
            },
        })
    }

    /// Validate a message against a topic schema
    fn validate_message_against_schema(
        &self,
        topic_name: &str,
        key: Option<&Value>,
        value: &Value,
    ) -> Result<ValidationResult, ContractError> {
        let (key_schema_opt, value_schema) = self
            .schema_cache
            .get(topic_name)
            .ok_or_else(|| ContractError::OperationNotFound(topic_name.to_string()))?;

        let mut validation_errors = Vec::new();

        // Validate key if present and schema exists
        if let (Some(key_value), Some(key_schema)) = (key, key_schema_opt) {
            for error in key_schema.iter_errors(key_value) {
                validation_errors.push(ValidationError {
                    message: format!("Key validation error: {}", error.to_string()),
                    path: Some(format!("{}.key{}", topic_name, error.instance_path)),
                    code: Some("KEY_SCHEMA_VALIDATION_ERROR".to_string()),
                });
            }
        }

        // Validate value
        for error in value_schema.iter_errors(value) {
            validation_errors.push(ValidationError {
                message: format!("Value validation error: {}", error.to_string()),
                path: Some(format!("{}.value{}", topic_name, error.instance_path)),
                code: Some("VALUE_SCHEMA_VALIDATION_ERROR".to_string()),
            });
        }

        Ok(ValidationResult {
            valid: validation_errors.is_empty(),
            errors: validation_errors,
            warnings: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl ProtocolContract for KafkaContract {
    fn protocol(&self) -> Protocol {
        Protocol::Kafka
    }

    fn contract_id(&self) -> &str {
        &self.contract_id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn operations(&self) -> Vec<ContractOperation> {
        self.operations_cache.values().cloned().collect()
    }

    fn get_operation(&self, operation_id: &str) -> Option<&ContractOperation> {
        self.operations_cache.get(operation_id)
    }

    async fn diff(
        &self,
        other: &dyn ProtocolContract,
    ) -> Result<ContractDiffResult, ContractError> {
        if other.protocol() != Protocol::Kafka {
            return Err(ContractError::UnsupportedProtocol(other.protocol()));
        }

        Err(ContractError::Other(
            "Direct comparison of KafkaContract instances requires type information. \
             Use KafkaContract::diff_contracts() for comparing two KafkaContract instances."
                .to_string(),
        ))
    }

    async fn validate(
        &self,
        operation_id: &str,
        request: &ContractRequest,
    ) -> Result<ValidationResult, ContractError> {
        // Parse the message payload
        // For Kafka, the payload might contain both key and value
        // For simplicity, we'll assume the payload is the value and key is in metadata
        let value: Value = serde_json::from_slice(&request.payload)
            .map_err(|e| ContractError::SchemaValidation(format!("Invalid JSON: {}", e)))?;

        let key = request.metadata.get("key").and_then(|k| serde_json::from_str::<Value>(k).ok());

        // Validate against the topic schema
        self.validate_message_against_schema(operation_id, key.as_ref(), &value)
    }

    fn get_schema(&self, operation_id: &str) -> Option<serde_json::Value> {
        self.topics.get(operation_id).map(|topic| {
            serde_json::json!({
                "key": topic.key_schema.as_ref().map(|s| s.schema.clone()),
                "value": topic.value_schema.schema.clone(),
            })
        })
    }

    fn to_json(&self) -> Result<serde_json::Value, ContractError> {
        let topics: Vec<serde_json::Value> = self
            .topics
            .values()
            .map(|topic| {
                serde_json::json!({
                    "topic": topic.topic,
                    "key_schema": topic.key_schema.as_ref().map(|s| {
                        serde_json::json!({
                            "format": topic.key_schema.as_ref().unwrap().format,
                            "schema": topic.key_schema.as_ref().unwrap().schema,
                            "schema_id": topic.key_schema.as_ref().unwrap().schema_id,
                            "version": topic.key_schema.as_ref().unwrap().version,
                        })
                    }),
                    "value_schema": {
                        "format": topic.value_schema.format,
                        "schema": topic.value_schema.schema,
                        "schema_id": topic.value_schema.schema_id,
                        "version": topic.value_schema.version,
                    },
                    "partitions": topic.partitions,
                    "replication_factor": topic.replication_factor,
                    "description": topic.description,
                    "evolution_rules": topic.evolution_rules,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "contract_id": self.contract_id,
            "version": self.version,
            "protocol": "kafka",
            "topics": topics,
            "metadata": self.metadata,
        }))
    }
}

/// Helper function to compare two KafkaContract instances
pub fn diff_kafka_contracts(
    old_contract: &KafkaContract,
    new_contract: &KafkaContract,
) -> Result<ContractDiffResult, ContractError> {
    old_contract.diff_contracts(new_contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_contract_creation() {
        let contract = MqttContract::new("test-contract".to_string(), "1.0.0".to_string());
        assert_eq!(contract.contract_id(), "test-contract");
        assert_eq!(contract.version(), "1.0.0");
    }

    #[test]
    fn test_kafka_contract_creation() {
        let contract = KafkaContract::new("test-contract".to_string(), "1.0.0".to_string());
        assert_eq!(contract.contract_id(), "test-contract");
        assert_eq!(contract.version(), "1.0.0");
    }
}
