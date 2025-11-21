//! WebSocket contract implementation for protocol-agnostic contract drift detection
//!
//! This module provides a `WebSocketContract` struct that implements the `ProtocolContract` trait
//! for WebSocket connections, enabling drift detection and analysis for WebSocket message schemas
//! and topics.

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
use std::collections::HashSet;

/// WebSocket message type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageType {
    /// Unique identifier for this message type
    pub message_type: String,
    /// Optional topic or channel name
    pub topic: Option<String>,
    /// JSON schema for this message type
    pub schema: Value,
    /// Direction: "inbound" (client to server), "outbound" (server to client), or "bidirectional"
    pub direction: MessageDirection,
    /// Description of this message type
    pub description: Option<String>,
    /// Example message payload
    pub example: Option<Value>,
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

/// WebSocket contract implementation
///
/// Defines message types and topics for a WebSocket connection, enabling
/// schema validation and drift detection.
pub struct WebSocketContract {
    /// Unique identifier for this contract
    contract_id: String,
    /// Contract version
    version: String,
    /// Map of message type identifiers to message type definitions
    message_types: HashMap<String, WebSocketMessageType>,
    /// Map of topics to message types that can be sent on that topic
    topics: HashMap<String, Vec<String>>,
    /// Compiled JSON schemas for validation (cached)
    #[allow(dead_code)]
    schema_cache: HashMap<String, JSONSchema>,
    /// Cached contract operations for quick lookup
    operations_cache: HashMap<String, ContractOperation>,
    /// Contract metadata
    metadata: HashMap<String, String>,
}

impl WebSocketContract {
    /// Create a new WebSocket contract
    pub fn new(contract_id: String, version: String) -> Self {
        Self {
            contract_id,
            version,
            message_types: HashMap::new(),
            topics: HashMap::new(),
            schema_cache: HashMap::new(),
            operations_cache: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a message type to the contract
    pub fn add_message_type(
        &mut self,
        message_type: WebSocketMessageType,
    ) -> Result<(), ContractError> {
        let message_type_id = message_type.message_type.clone();

        // Compile and cache the JSON schema for validation
        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .build(&message_type.schema)
            .map_err(|e| ContractError::SchemaValidation(format!("Invalid JSON schema: {}", e)))?;
        self.schema_cache.insert(message_type_id.clone(), schema);

        // Add to message types
        self.message_types.insert(message_type_id.clone(), message_type.clone());

        // Build operation ID (topic:message_type or just message_type)
        let operation_id = if let Some(ref topic) = message_type.topic {
            format!("{}:{}", topic, message_type_id)
        } else {
            message_type_id.clone()
        };

        // Cache the contract operation
        let operation = ContractOperation {
            id: operation_id.clone(),
            name: message_type.message_type.clone(),
            operation_type: OperationType::WebSocketMessage {
                message_type: message_type.message_type.clone(),
                topic: message_type.topic.clone(),
            },
            input_schema: Some(message_type.schema.clone()),
            output_schema: Some(message_type.schema.clone()), // WebSocket messages can be bidirectional
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("direction".to_string(), format!("{:?}", message_type.direction));
                if let Some(ref desc) = message_type.description {
                    meta.insert("description".to_string(), desc.clone());
                }
                meta
            },
        };
        self.operations_cache.insert(operation_id, operation);

        // Index by topic if topic is specified
        if let Some(topic) = &message_type.topic {
            self.topics.entry(topic.clone()).or_insert_with(Vec::new).push(message_type_id);
        }

        Ok(())
    }

    /// Remove a message type from the contract
    pub fn remove_message_type(&mut self, message_type_id: &str) {
        if let Some(message_type) = self.message_types.remove(message_type_id) {
            self.schema_cache.remove(message_type_id);

            // Remove from topic index
            if let Some(topic) = &message_type.topic {
                if let Some(message_types) = self.topics.get_mut(topic) {
                    message_types.retain(|id| id != message_type_id);
                    if message_types.is_empty() {
                        self.topics.remove(topic);
                    }
                }
            }

            // Store topic before moving message_type
            let topic = message_type.topic.clone();

            // Remove from operations cache
            let operation_id = if let Some(ref topic_name) = topic {
                format!("{}:{}", topic_name, message_type_id)
            } else {
                message_type_id.to_string()
            };
            self.operations_cache.remove(&operation_id);
        }
    }

    /// Get message types for a specific topic
    pub fn get_message_types_for_topic(&self, topic: &str) -> Vec<&WebSocketMessageType> {
        self.topics
            .get(topic)
            .map(|ids| ids.iter().filter_map(|id| self.message_types.get(id)).collect())
            .unwrap_or_default()
    }

    /// Compare two WebSocket contracts and detect differences
    fn diff_contracts(
        &self,
        other: &WebSocketContract,
    ) -> Result<ContractDiffResult, ContractError> {
        let mut mismatches = Vec::new();

        // Collect all message type IDs
        let all_message_types: std::collections::HashSet<String> =
            self.message_types.keys().chain(other.message_types.keys()).cloned().collect();

        // Check for removed message types (breaking change)
        for message_type_id in &all_message_types {
            if self.message_types.contains_key(message_type_id)
                && !other.message_types.contains_key(message_type_id)
            {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("message_type_removed"));
                context.insert("message_type".to_string(), serde_json::json!(message_type_id));
                
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: message_type_id.clone(),
                    method: None,
                    expected: Some(format!("Message type {} should exist", message_type_id)),
                    actual: Some("Message type removed".to_string()),
                    description: format!("Message type {} was removed", message_type_id),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for added message types (non-breaking, additive)
        for message_type_id in &all_message_types {
            if !self.message_types.contains_key(message_type_id)
                && other.message_types.contains_key(message_type_id)
            {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(true));
                context.insert("is_breaking".to_string(), serde_json::json!(false));
                context.insert("change_category".to_string(), serde_json::json!("message_type_added"));
                context.insert("message_type".to_string(), serde_json::json!(message_type_id));
                
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: message_type_id.clone(),
                    method: None,
                    expected: None,
                    actual: Some(format!("New message type {}", message_type_id)),
                    description: format!("New message type {} was added", message_type_id),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Compare message type schemas for types that exist in both
        for message_type_id in all_message_types.intersection(
            &self.message_types.keys().cloned().collect::<std::collections::HashSet<_>>(),
        ) {
            if let (Some(old_type), Some(new_type)) = (
                self.message_types.get(message_type_id),
                other.message_types.get(message_type_id),
            ) {
                let schema_mismatches =
                    Self::diff_message_type_schemas(message_type_id, old_type, new_type)?;
                mismatches.extend(schema_mismatches);

                // Check for topic changes (breaking change)
                if old_type.topic != new_type.topic {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert("change_category".to_string(), serde_json::json!("topic_changed"));
                    context.insert("message_type".to_string(), serde_json::json!(message_type_id));
                    context.insert("old_topic".to_string(), serde_json::json!(old_type.topic));
                    context.insert("new_topic".to_string(), serde_json::json!(new_type.topic));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SchemaMismatch,
                        path: format!("{}.topic", message_type_id),
                        method: None,
                        expected: old_type.topic.clone().map(|t| format!("Topic: {}", t)),
                        actual: new_type.topic.clone().map(|t| format!("Topic: {}", t)),
                        description: format!(
                            "Topic changed for message type {}: {:?} -> {:?}",
                            message_type_id, old_type.topic, new_type.topic
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }

                // Check for direction changes (breaking change)
                if old_type.direction != new_type.direction {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert("change_category".to_string(), serde_json::json!("direction_changed"));
                    context.insert("message_type".to_string(), serde_json::json!(message_type_id));
                    context.insert("old_direction".to_string(), serde_json::json!(format!("{:?}", old_type.direction)));
                    context.insert("new_direction".to_string(), serde_json::json!(format!("{:?}", new_type.direction)));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SchemaMismatch,
                        path: format!("{}.direction", message_type_id),
                        method: None,
                        expected: Some(format!("Direction: {:?}", old_type.direction)),
                        actual: Some(format!("Direction: {:?}", new_type.direction)),
                        description: format!(
                            "Direction changed for message type {}: {:?} -> {:?}",
                            message_type_id, old_type.direction, new_type.direction
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }
            }
        }

        // Compare topics
        let all_topics: std::collections::HashSet<String> =
            self.topics.keys().chain(other.topics.keys()).cloned().collect();

        for topic in &all_topics {
            let old_message_types = self.get_message_types_for_topic(topic);
            let new_message_types = other.get_message_types_for_topic(topic);

            let old_ids: std::collections::HashSet<String> =
                old_message_types.iter().map(|mt| mt.message_type.clone()).collect();
            let new_ids: std::collections::HashSet<String> =
                new_message_types.iter().map(|mt| mt.message_type.clone()).collect();

            // Check for removed message types from topic
            for removed_id in old_ids.difference(&new_ids) {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::SchemaMismatch,
                    path: format!("topic:{}.{}", topic, removed_id),
                    method: None,
                    expected: Some(format!(
                        "Message type {} should be available on topic {}",
                        removed_id, topic
                    )),
                    actual: Some("Message type removed from topic".to_string()),
                    description: format!(
                        "Message type {} was removed from topic {}",
                        removed_id, topic
                    ),
                    severity: MismatchSeverity::High,
                    confidence: 1.0,
                    context: HashMap::new(),
                });
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
                request_source: "websocket_contract_diff".to_string(),
                contract_version: Some(self.version.clone()),
                contract_format: "websocket_schema".to_string(),
                endpoint_path: "".to_string(),
                http_method: "".to_string(),
                request_count: 1,
                llm_provider: None,
                llm_model: None,
            },
        })
    }

    /// Compare message type schemas
    fn diff_message_type_schemas(
        message_type_id: &str,
        old_type: &WebSocketMessageType,
        new_type: &WebSocketMessageType,
    ) -> Result<Vec<Mismatch>, ContractError> {
        let mut mismatches = Vec::new();

        // Detect schema format
        let old_format = Self::detect_schema_format(&old_type.schema);
        let new_format = Self::detect_schema_format(&new_type.schema);

        // Check for schema format changes (breaking change)
        if old_format != new_format {
            let mut context = HashMap::new();
            context.insert("is_additive".to_string(), serde_json::json!(false));
            context.insert("is_breaking".to_string(), serde_json::json!(true));
            context.insert("change_category".to_string(), serde_json::json!("schema_format_changed"));
            context.insert("message_type".to_string(), serde_json::json!(message_type_id));
            context.insert("old_format".to_string(), serde_json::json!(old_format));
            context.insert("new_format".to_string(), serde_json::json!(new_format));
            
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::SchemaMismatch,
                path: format!("{}.schema_format", message_type_id),
                method: None,
                expected: Some(format!("Schema format: {}", old_format)),
                actual: Some(format!("Schema format: {}", new_format)),
                description: format!(
                    "Schema format changed from {} to {} for message type {}",
                    old_format, new_format, message_type_id
                ),
                severity: MismatchSeverity::High,
                confidence: 1.0,
                context,
            });
        }

        // Compare schemas based on format
        if old_type.schema != new_type.schema {
            match (old_format.as_str(), new_format.as_str()) {
                ("json_schema", "json_schema") => {
                    let schema_diff =
                        Self::compare_json_schemas(&old_type.schema, &new_type.schema, message_type_id);
                    mismatches.extend(schema_diff);
                }
                ("avro", "avro") => {
                    let schema_diff =
                        Self::compare_avro_schemas(&old_type.schema, &new_type.schema, message_type_id)?;
                    mismatches.extend(schema_diff);
                }
                ("json_shape", "json_shape") => {
                    let schema_diff =
                        Self::compare_json_shape_schemas(&old_type.schema, &new_type.schema, message_type_id);
                    mismatches.extend(schema_diff);
                }
                _ => {
                    // Different formats - already handled above
                }
            }
        }

        Ok(mismatches)
    }

    /// Detect the schema format (JSON Schema, Avro, or JSON-shape)
    fn detect_schema_format(schema: &Value) -> String {
        // Check for Avro schema indicators
        if schema.get("type").and_then(|v| v.as_str()) == Some("record")
            || schema.get("fields").is_some()
        {
            return "avro".to_string();
        }

        // Check for JSON Schema indicators
        if schema.get("$schema").is_some()
            || (schema.get("type").is_some() && schema.get("properties").is_some())
            || schema.get("required").is_some()
        {
            return "json_schema".to_string();
        }

        // Check for JSON-shape (simple object with type strings)
        if let Some(obj) = schema.as_object() {
            let all_strings = obj.values().all(|v| {
                v.as_str().is_some()
                    || (v.is_object() && v.get("type").and_then(|t| t.as_str()).is_some())
            });
            if all_strings && !obj.is_empty() {
                return "json_shape".to_string();
            }
        }

        // Default to JSON Schema if unclear
        "json_schema".to_string()
    }

    /// Compare Avro schemas and identify differences
    fn compare_avro_schemas(
        old_schema: &Value,
        new_schema: &Value,
        path_prefix: &str,
    ) -> Result<Vec<Mismatch>, ContractError> {
        let mut mismatches = Vec::new();

        // Extract fields from Avro schema
        let old_fields = old_schema
            .get("fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ContractError::SchemaValidation("Invalid Avro schema: missing fields".to_string()))?;
        let new_fields = new_schema
            .get("fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ContractError::SchemaValidation("Invalid Avro schema: missing fields".to_string()))?;

        // Build field maps by name
        let old_fields_map: HashMap<String, &Value> = old_fields
            .iter()
            .filter_map(|f| {
                f.get("name").and_then(|n| n.as_str()).map(|name| (name.to_string(), f))
            })
            .collect();
        let new_fields_map: HashMap<String, &Value> = new_fields
            .iter()
            .filter_map(|f| {
                f.get("name").and_then(|n| n.as_str()).map(|name| (name.to_string(), f))
            })
            .collect();

        // Check for removed fields (breaking change)
        for (field_name, old_field) in &old_fields_map {
            if !new_fields_map.contains_key(field_name) {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("field_removed"));
                context.insert("field_name".to_string(), serde_json::json!(field_name));
                context.insert("schema_format".to_string(), serde_json::json!("avro"));
                
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::EndpointNotFound,
                    path: format!("{}.{}", path_prefix, field_name),
                    method: None,
                    expected: Some(format!("Field {} should exist", field_name)),
                    actual: Some("Field removed".to_string()),
                    description: format!("Avro field {} was removed", field_name),
                    severity: MismatchSeverity::High,
                    confidence: 1.0,
                    context,
                });
            }
        }

        // Check for added fields
        for (field_name, new_field) in &new_fields_map {
            if !old_fields_map.contains_key(field_name) {
                // In Avro, fields without defaults are required
                let has_default = new_field.get("default").is_some();
                let is_required = !has_default;
                
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(!is_required));
                context.insert("is_breaking".to_string(), serde_json::json!(is_required));
                context.insert("change_category".to_string(), serde_json::json!(if is_required { "required_field_added" } else { "field_added" }));
                context.insert("field_name".to_string(), serde_json::json!(field_name));
                context.insert("schema_format".to_string(), serde_json::json!("avro"));
                context.insert("has_default".to_string(), serde_json::json!(has_default));
                
                mismatches.push(Mismatch {
                    mismatch_type: if is_required { MismatchType::MissingRequiredField } else { MismatchType::UnexpectedField },
                    path: format!("{}.{}", path_prefix, field_name),
                    method: None,
                    expected: None,
                    actual: Some(format!("New Avro field {} ({})", field_name, if is_required { "required" } else { "optional" })),
                    description: format!(
                        "New Avro field {} was added ({})",
                        field_name,
                        if is_required { "required - breaking" } else { "optional - additive" }
                    ),
                    severity: if is_required { MismatchSeverity::High } else { MismatchSeverity::Low },
                    confidence: 1.0,
                    context,
                });
            } else {
                // Check for type changes
                let old_field = old_fields_map[field_name];
                let old_type = old_field.get("type");
                let new_type = new_field.get("type");
                
                if old_type != new_type {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert("change_category".to_string(), serde_json::json!("field_type_changed"));
                    context.insert("field_name".to_string(), serde_json::json!(field_name));
                    context.insert("schema_format".to_string(), serde_json::json!("avro"));
                    context.insert("old_type".to_string(), serde_json::json!(old_type));
                    context.insert("new_type".to_string(), serde_json::json!(new_type));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::TypeMismatch,
                        path: format!("{}.{}", path_prefix, field_name),
                        method: None,
                        expected: Some(format!("Type: {:?}", old_type)),
                        actual: Some(format!("Type: {:?}", new_type)),
                        description: format!(
                            "Avro field {} type changed",
                            field_name
                        ),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }
            }
        }

        Ok(mismatches)
    }

    /// Compare JSON-shape schemas (simplified format)
    fn compare_json_shape_schemas(
        old_schema: &Value,
        new_schema: &Value,
        path_prefix: &str,
    ) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        if let (Some(old_obj), Some(new_obj)) = (old_schema.as_object(), new_schema.as_object()) {
            // Check for removed properties (breaking)
            for (prop_name, _) in old_obj {
                if !new_obj.contains_key(prop_name) {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert("change_category".to_string(), serde_json::json!("property_removed"));
                    context.insert("field_name".to_string(), serde_json::json!(prop_name));
                    context.insert("schema_format".to_string(), serde_json::json!("json_shape"));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::UnexpectedField,
                        path: format!("{}.{}", path_prefix, prop_name),
                        method: None,
                        expected: Some(format!("Property {} should exist", prop_name)),
                        actual: Some("Property removed".to_string()),
                        description: format!("Property {} was removed", prop_name),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }
            }

            // Check for added properties (additive)
            for (prop_name, _) in new_obj {
                if !old_obj.contains_key(prop_name) {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(true));
                    context.insert("is_breaking".to_string(), serde_json::json!(false));
                    context.insert("change_category".to_string(), serde_json::json!("property_added"));
                    context.insert("field_name".to_string(), serde_json::json!(prop_name));
                    context.insert("schema_format".to_string(), serde_json::json!("json_shape"));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::UnexpectedField,
                        path: format!("{}.{}", path_prefix, prop_name),
                        method: None,
                        expected: None,
                        actual: Some(format!("New property {}", prop_name)),
                        description: format!("New property {} was added", prop_name),
                        severity: MismatchSeverity::Low,
                        confidence: 1.0,
                        context,
                    });
                } else {
                    // Check for type changes
                    let old_type = old_obj[prop_name].as_str().or_else(|| {
                        old_obj[prop_name].get("type").and_then(|t| t.as_str())
                    });
                    let new_type = new_obj[prop_name].as_str().or_else(|| {
                        new_obj[prop_name].get("type").and_then(|t| t.as_str())
                    });
                    
                    if old_type != new_type {
                        let mut context = HashMap::new();
                        context.insert("is_additive".to_string(), serde_json::json!(false));
                        context.insert("is_breaking".to_string(), serde_json::json!(true));
                        context.insert("change_category".to_string(), serde_json::json!("property_type_changed"));
                        context.insert("field_name".to_string(), serde_json::json!(prop_name));
                        context.insert("schema_format".to_string(), serde_json::json!("json_shape"));
                        context.insert("old_type".to_string(), serde_json::json!(old_type));
                        context.insert("new_type".to_string(), serde_json::json!(new_type));
                        
                        mismatches.push(Mismatch {
                            mismatch_type: MismatchType::TypeMismatch,
                            path: format!("{}.{}", path_prefix, prop_name),
                            method: None,
                            expected: old_type.map(|t| format!("Type: {}", t)),
                            actual: new_type.map(|t| format!("Type: {}", t)),
                            description: format!(
                                "Property {} type changed",
                                prop_name
                            ),
                            severity: MismatchSeverity::High,
                            confidence: 1.0,
                            context,
                        });
                    }
                }
            }
        }

        mismatches
    }

    /// Compare two JSON schemas and identify differences
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
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(false));
                context.insert("is_breaking".to_string(), serde_json::json!(true));
                context.insert("change_category".to_string(), serde_json::json!("required_field_added"));
                context.insert("field_name".to_string(), serde_json::json!(new_req));
                
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::MissingRequiredField,
                    path: format!("{}.{}", path_prefix, new_req),
                    method: None,
                    expected: Some(format!("Field {} should be optional", new_req)),
                    actual: Some(format!("Field {} is now required", new_req)),
                    description: format!("Field {} became required", new_req),
                    severity: MismatchSeverity::Critical,
                    confidence: 1.0,
                    context,
                });
            }
            
            // Check for removed required fields (additive - field is now optional)
            for removed_req in old_required_set.difference(&new_required_set) {
                let mut context = HashMap::new();
                context.insert("is_additive".to_string(), serde_json::json!(true));
                context.insert("is_breaking".to_string(), serde_json::json!(false));
                context.insert("change_category".to_string(), serde_json::json!("required_field_removed"));
                context.insert("field_name".to_string(), serde_json::json!(removed_req));
                
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::UnexpectedField,
                    path: format!("{}.{}", path_prefix, removed_req),
                    method: None,
                    expected: Some(format!("Field {} was required", removed_req)),
                    actual: Some(format!("Field {} is now optional", removed_req)),
                    description: format!("Field {} is no longer required", removed_req),
                    severity: MismatchSeverity::Low,
                    confidence: 1.0,
                    context,
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
                            let mut context = HashMap::new();
                            context.insert("is_additive".to_string(), serde_json::json!(false));
                            context.insert("is_breaking".to_string(), serde_json::json!(true));
                            context.insert("change_category".to_string(), serde_json::json!("property_type_changed"));
                            context.insert("field_name".to_string(), serde_json::json!(prop_name));
                            context.insert("old_type".to_string(), serde_json::json!(old_type));
                            context.insert("new_type".to_string(), serde_json::json!(new_type));
                            
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
                                context,
                            });
                        }
                    }
                } else {
                    // New property added (additive change)
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(true));
                    context.insert("is_breaking".to_string(), serde_json::json!(false));
                    context.insert("change_category".to_string(), serde_json::json!("property_added"));
                    context.insert("field_name".to_string(), serde_json::json!(prop_name));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::UnexpectedField,
                        path: format!("{}.{}", path_prefix, prop_name),
                        method: None,
                        expected: None,
                        actual: Some(format!("New property {}", prop_name)),
                        description: format!("New property {} was added", prop_name),
                        severity: MismatchSeverity::Low,
                        confidence: 1.0,
                        context,
                    });
                }
            }

            // Check for removed properties (breaking change)
            for prop_name in old_props.keys() {
                if !new_props.contains_key(prop_name) {
                    let mut context = HashMap::new();
                    context.insert("is_additive".to_string(), serde_json::json!(false));
                    context.insert("is_breaking".to_string(), serde_json::json!(true));
                    context.insert("change_category".to_string(), serde_json::json!("property_removed"));
                    context.insert("field_name".to_string(), serde_json::json!(prop_name));
                    
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::UnexpectedField,
                        path: format!("{}.{}", path_prefix, prop_name),
                        method: None,
                        expected: Some(format!("Property {} should exist", prop_name)),
                        actual: Some("Property removed".to_string()),
                        description: format!("Property {} was removed", prop_name),
                        severity: MismatchSeverity::High,
                        confidence: 1.0,
                        context,
                    });
                }
            }
        }

        mismatches
    }

    /// Validate a message against a message type schema
    fn validate_message_against_schema(
        &self,
        message_type_id: &str,
        message: &Value,
    ) -> Result<ValidationResult, ContractError> {
        let schema = self
            .schema_cache
            .get(message_type_id)
            .ok_or_else(|| ContractError::OperationNotFound(message_type_id.to_string()))?;

        // Use iter_errors instead of validate which returns Result
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
impl ProtocolContract for WebSocketContract {
    fn protocol(&self) -> Protocol {
        Protocol::WebSocket
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
        // Try direct lookup first
        if let Some(operation) = self.operations_cache.get(operation_id) {
            return Some(operation);
        }

        // Try to find by message type only (if operation_id doesn't include topic)
        if !operation_id.contains(':') {
            // Search for operation with this message type
            for (op_id, operation) in &self.operations_cache {
                if let OperationType::WebSocketMessage { message_type, .. } =
                    &operation.operation_type
                {
                    if message_type == operation_id {
                        return Some(operation);
                    }
                }
            }
        }

        None
    }

    async fn diff(
        &self,
        other: &dyn ProtocolContract,
    ) -> Result<ContractDiffResult, ContractError> {
        // Ensure the other contract is also a WebSocket contract
        if other.protocol() != Protocol::WebSocket {
            return Err(ContractError::UnsupportedProtocol(other.protocol()));
        }

        // Similar limitation as GrpcContract - we need type information to compare
        Err(ContractError::Other(
            "Direct comparison of WebSocketContract instances requires type information. \
             Use WebSocketContract::diff_contracts() for comparing two WebSocketContract instances."
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

        // Extract message type from operation_id (could be "topic:message_type" or just "message_type")
        let message_type_id = if let Some((_, message_type)) = operation_id.split_once(':') {
            message_type
        } else {
            operation_id
        };

        // Validate against the schema
        self.validate_message_against_schema(message_type_id, &message)
    }

    fn get_schema(&self, operation_id: &str) -> Option<serde_json::Value> {
        // Extract message type from operation_id
        let message_type_id = if let Some((_, message_type)) = operation_id.split_once(':') {
            message_type
        } else {
            operation_id
        };

        self.message_types.get(message_type_id).map(|mt| mt.schema.clone())
    }

    fn to_json(&self) -> Result<serde_json::Value, ContractError> {
        let message_types: Vec<serde_json::Value> = self
            .message_types
            .values()
            .map(|mt| {
                serde_json::json!({
                    "message_type": mt.message_type,
                    "topic": mt.topic,
                    "schema": mt.schema,
                    "direction": mt.direction,
                    "description": mt.description,
                    "example": mt.example,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "contract_id": self.contract_id,
            "version": self.version,
            "protocol": "websocket",
            "message_types": message_types,
            "topics": self.topics.keys().collect::<Vec<_>>(),
            "metadata": self.metadata,
        }))
    }
}

/// Helper function to compare two WebSocketContract instances
pub fn diff_websocket_contracts(
    old_contract: &WebSocketContract,
    new_contract: &WebSocketContract,
) -> Result<ContractDiffResult, ContractError> {
    old_contract.diff_contracts(new_contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_contract_creation() {
        let contract = WebSocketContract::new("test-contract".to_string(), "1.0.0".to_string());
        assert_eq!(contract.contract_id(), "test-contract");
        assert_eq!(contract.version(), "1.0.0");
    }

    #[test]
    fn test_add_message_type() {
        let mut contract = WebSocketContract::new("test".to_string(), "1.0.0".to_string());
        let message_type = WebSocketMessageType {
            message_type: "chat_message".to_string(),
            topic: Some("chat".to_string()),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "user": {"type": "string"}
                },
                "required": ["text", "user"]
            }),
            direction: MessageDirection::Bidirectional,
            description: Some("Chat message".to_string()),
            example: None,
        };

        assert!(contract.add_message_type(message_type).is_ok());
        assert_eq!(contract.message_types.len(), 1);
    }
}
