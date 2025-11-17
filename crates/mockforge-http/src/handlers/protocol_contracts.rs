//! Protocol contract management handlers
//!
//! This module provides HTTP handlers for managing protocol contracts (gRPC, WebSocket, MQTT, Kafka).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::contract_drift::protocol_contracts::{
    compare_contracts, ProtocolContractRegistry,
};
use mockforge_core::contract_drift::{
    GrpcContract, KafkaContract, KafkaTopicSchema, MqttContract, MqttTopicSchema,
    SchemaFormat, TopicSchema, WebSocketContract, WebSocketMessageType,
};
use mockforge_core::protocol_abstraction::Protocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Base64 encoding/decoding
use base64::{engine::general_purpose, Engine as _};

/// State for protocol contract handlers
#[derive(Clone)]
pub struct ProtocolContractState {
    /// Protocol contract registry
    pub registry: Arc<RwLock<ProtocolContractRegistry>>,
}

/// Request to create a gRPC contract
#[derive(Debug, Deserialize)]
pub struct CreateGrpcContractRequest {
    /// Contract ID
    pub contract_id: String,
    /// Contract version
    pub version: String,
    /// Protobuf descriptor set (base64 encoded)
    pub descriptor_set: String,
}

/// Request to create a WebSocket contract
#[derive(Debug, Deserialize)]
pub struct CreateWebSocketContractRequest {
    /// Contract ID
    pub contract_id: String,
    /// Contract version
    pub version: String,
    /// Message types
    pub message_types: Vec<WebSocketMessageTypeRequest>,
}

/// Request for a WebSocket message type
#[derive(Debug, Deserialize)]
pub struct WebSocketMessageTypeRequest {
    /// Message type identifier
    pub message_type: String,
    /// Optional topic or channel name
    pub topic: Option<String>,
    /// JSON schema for this message type
    pub schema: serde_json::Value,
    /// Direction: "inbound", "outbound", or "bidirectional"
    pub direction: String,
    /// Description of this message type
    pub description: Option<String>,
    /// Example message payload
    pub example: Option<serde_json::Value>,
}

/// Request to create an MQTT contract
#[derive(Debug, Deserialize)]
pub struct CreateMqttContractRequest {
    /// Contract ID
    pub contract_id: String,
    /// Contract version
    pub version: String,
    /// Topic schemas
    pub topics: Vec<MqttTopicSchemaRequest>,
}

/// Request for an MQTT topic schema
#[derive(Debug, Deserialize)]
pub struct MqttTopicSchemaRequest {
    /// Topic name
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

/// Request to create a Kafka contract
#[derive(Debug, Deserialize)]
pub struct CreateKafkaContractRequest {
    /// Contract ID
    pub contract_id: String,
    /// Contract version
    pub version: String,
    /// Topic schemas
    pub topics: Vec<KafkaTopicSchemaRequest>,
}

/// Request for a Kafka topic schema
#[derive(Debug, Deserialize)]
pub struct KafkaTopicSchemaRequest {
    /// Topic name
    pub topic: String,
    /// Key schema (optional)
    pub key_schema: Option<TopicSchemaRequest>,
    /// Value schema (required)
    pub value_schema: TopicSchemaRequest,
    /// Number of partitions
    pub partitions: Option<u32>,
    /// Replication factor
    pub replication_factor: Option<u16>,
    /// Description of this topic
    pub description: Option<String>,
    /// Evolution rules for schema changes
    pub evolution_rules: Option<EvolutionRulesRequest>,
}

/// Request for a topic schema (key or value)
#[derive(Debug, Deserialize)]
pub struct TopicSchemaRequest {
    /// Schema format: "json", "avro", or "protobuf"
    pub format: String,
    /// Schema definition
    pub schema: serde_json::Value,
    /// Schema registry ID (if using schema registry)
    pub schema_id: Option<String>,
    /// Schema version
    pub version: Option<String>,
}

/// Request for evolution rules
#[derive(Debug, Deserialize)]
pub struct EvolutionRulesRequest {
    /// Allow backward compatible changes
    pub allow_backward_compatible: bool,
    /// Allow forward compatible changes
    pub allow_forward_compatible: bool,
    /// Require explicit version bump for breaking changes
    pub require_version_bump: bool,
}

/// Response for protocol contract operations
#[derive(Debug, Serialize)]
pub struct ProtocolContractResponse {
    /// Contract ID
    pub contract_id: String,
    /// Contract version
    pub version: String,
    /// Protocol type
    pub protocol: String,
    /// Contract JSON representation
    pub contract: serde_json::Value,
}

/// Response for listing contracts
#[derive(Debug, Serialize)]
pub struct ListContractsResponse {
    /// List of contracts
    pub contracts: Vec<ProtocolContractResponse>,
    /// Total count
    pub total: usize,
}

/// Request to compare contracts
#[derive(Debug, Deserialize)]
pub struct CompareContractsRequest {
    /// Old contract ID
    pub old_contract_id: String,
    /// New contract ID
    pub new_contract_id: String,
}

/// Request to validate a message
#[derive(Debug, Deserialize)]
pub struct ValidateMessageRequest {
    /// Operation ID (endpoint, method, topic, etc.)
    pub operation_id: String,
    /// Message payload (base64 encoded or JSON)
    pub payload: serde_json::Value,
    /// Content type
    pub content_type: Option<String>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// List all protocol contracts
pub async fn list_contracts(
    State(state): State<ProtocolContractState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListContractsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.registry.read().await;

    let protocol_filter = params.get("protocol").and_then(|p| match p.as_str() {
        "grpc" => Some(Protocol::Grpc),
        "websocket" => Some(Protocol::WebSocket),
        "mqtt" => Some(Protocol::Mqtt),
        "kafka" => Some(Protocol::Kafka),
        _ => None,
    });

    let contracts: Vec<ProtocolContractResponse> = if let Some(protocol) = protocol_filter {
        registry
            .list_by_protocol(protocol)
            .iter()
            .map(|contract| {
                let contract_json = contract.to_json().unwrap_or_else(|_| serde_json::json!({}));
                ProtocolContractResponse {
                    contract_id: contract.contract_id().to_string(),
                    version: contract.version().to_string(),
                    protocol: format!("{:?}", contract.protocol()).to_lowercase(),
                    contract: contract_json,
                }
            })
            .collect()
    } else {
        registry
            .list()
            .iter()
            .map(|contract| {
                let contract_json = contract.to_json().unwrap_or_else(|_| serde_json::json!({}));
                ProtocolContractResponse {
                    contract_id: contract.contract_id().to_string(),
                    version: contract.version().to_string(),
                    protocol: format!("{:?}", contract.protocol()).to_lowercase(),
                    contract: contract_json,
                }
            })
            .collect()
    };

    Ok(Json(ListContractsResponse {
        total: contracts.len(),
        contracts,
    }))
}

/// Get a specific contract
pub async fn get_contract(
    State(state): State<ProtocolContractState>,
    Path(contract_id): Path<String>,
) -> Result<Json<ProtocolContractResponse>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.registry.read().await;

    let contract = registry
        .get(&contract_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Contract not found",
                    "contract_id": contract_id
                })),
            )
        })?;

    let contract_json = contract.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to serialize contract",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(ProtocolContractResponse {
        contract_id: contract.contract_id().to_string(),
        version: contract.version().to_string(),
        protocol: format!("{:?}", contract.protocol()).to_lowercase(),
        contract: contract_json,
    }))
}

/// Create a gRPC contract
pub async fn create_grpc_contract(
    State(state): State<ProtocolContractState>,
    Json(request): Json<CreateGrpcContractRequest>,
) -> Result<Json<ProtocolContractResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Decode base64 descriptor set
    let descriptor_bytes = general_purpose::STANDARD.decode(&request.descriptor_set).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid base64 descriptor set",
                "message": e.to_string()
            })),
        )
    })?;

    // Create descriptor pool from bytes
    // Note: GrpcContract::from_descriptor_set handles the descriptor pool creation
    let contract = GrpcContract::from_descriptor_set(
        request.contract_id.clone(),
        request.version.clone(),
        &descriptor_bytes,
    )
    .map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Failed to create gRPC contract",
                "message": e.to_string()
            })),
        )
    })?;

    // Register contract
    let mut registry = state.registry.write().await;
    registry.register(Box::new(contract));

    let contract = registry.get(&request.contract_id).unwrap();
    let contract_json = contract.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to serialize contract",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(ProtocolContractResponse {
        contract_id: request.contract_id,
        version: request.version,
        protocol: "grpc".to_string(),
        contract: contract_json,
    }))
}

/// Create a WebSocket contract
pub async fn create_websocket_contract(
    State(state): State<ProtocolContractState>,
    Json(request): Json<CreateWebSocketContractRequest>,
) -> Result<Json<ProtocolContractResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut contract = WebSocketContract::new(request.contract_id.clone(), request.version.clone());

    // Add message types
    for msg_type_req in request.message_types {
        let direction = match msg_type_req.direction.as_str() {
            "inbound" => mockforge_core::contract_drift::MessageDirection::Inbound,
            "outbound" => mockforge_core::contract_drift::MessageDirection::Outbound,
            "bidirectional" => mockforge_core::contract_drift::MessageDirection::Bidirectional,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid direction",
                        "message": "Direction must be 'inbound', 'outbound', or 'bidirectional'"
                    })),
                ));
            }
        };

        let message_type = WebSocketMessageType {
            message_type: msg_type_req.message_type,
            topic: msg_type_req.topic,
            schema: msg_type_req.schema,
            direction,
            description: msg_type_req.description,
            example: msg_type_req.example,
        };

        contract.add_message_type(message_type).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to add message type",
                    "message": e.to_string()
                })),
            )
        })?;
    }

    // Register contract
    let mut registry = state.registry.write().await;
    registry.register(Box::new(contract));

    let contract = registry.get(&request.contract_id).unwrap();
    let contract_json = contract.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to serialize contract",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(ProtocolContractResponse {
        contract_id: request.contract_id,
        version: request.version,
        protocol: "websocket".to_string(),
        contract: contract_json,
    }))
}

/// Create an MQTT contract
pub async fn create_mqtt_contract(
    State(state): State<ProtocolContractState>,
    Json(request): Json<CreateMqttContractRequest>,
) -> Result<Json<ProtocolContractResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut contract = MqttContract::new(request.contract_id.clone(), request.version.clone());

    // Add topics
    for topic_req in request.topics {
        let topic_schema = MqttTopicSchema {
            topic: topic_req.topic,
            qos: topic_req.qos,
            schema: topic_req.schema,
            retained: topic_req.retained,
            description: topic_req.description,
            example: topic_req.example,
        };

        contract.add_topic(topic_schema).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to add topic",
                    "message": e.to_string()
                })),
            )
        })?;
    }

    // Register contract
    let mut registry = state.registry.write().await;
    registry.register(Box::new(contract));

    let contract = registry.get(&request.contract_id).unwrap();
    let contract_json = contract.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to serialize contract",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(ProtocolContractResponse {
        contract_id: request.contract_id,
        version: request.version,
        protocol: "mqtt".to_string(),
        contract: contract_json,
    }))
}

/// Create a Kafka contract
pub async fn create_kafka_contract(
    State(state): State<ProtocolContractState>,
    Json(request): Json<CreateKafkaContractRequest>,
) -> Result<Json<ProtocolContractResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut contract = KafkaContract::new(request.contract_id.clone(), request.version.clone());

    // Add topics
    for topic_req in request.topics {
        let format = match topic_req.value_schema.format.as_str() {
            "json" => SchemaFormat::Json,
            "avro" => SchemaFormat::Avro,
            "protobuf" => SchemaFormat::Protobuf,
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid schema format",
                        "message": "Format must be 'json', 'avro', or 'protobuf'"
                    })),
                ));
            }
        };

        let value_schema = TopicSchema {
            format,
            schema: topic_req.value_schema.schema,
            schema_id: topic_req.value_schema.schema_id,
            version: topic_req.value_schema.version,
        };

        let key_schema = topic_req.key_schema.map(|ks_req| {
            let format = match ks_req.format.as_str() {
                "json" => SchemaFormat::Json,
                "avro" => SchemaFormat::Avro,
                "protobuf" => SchemaFormat::Protobuf,
                _ => SchemaFormat::Json, // Default to JSON
            };

            TopicSchema {
                format,
                schema: ks_req.schema,
                schema_id: ks_req.schema_id,
                version: ks_req.version,
            }
        });

        let evolution_rules = topic_req.evolution_rules.map(|er_req| {
            mockforge_core::contract_drift::EvolutionRules {
                allow_backward_compatible: er_req.allow_backward_compatible,
                allow_forward_compatible: er_req.allow_forward_compatible,
                require_version_bump: er_req.require_version_bump,
            }
        });

        let topic_schema = KafkaTopicSchema {
            topic: topic_req.topic,
            key_schema,
            value_schema,
            partitions: topic_req.partitions,
            replication_factor: topic_req.replication_factor,
            description: topic_req.description,
            evolution_rules,
        };

        contract.add_topic(topic_schema).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to add topic",
                    "message": e.to_string()
                })),
            )
        })?;
    }

    // Register contract
    let mut registry = state.registry.write().await;
    registry.register(Box::new(contract));

    let contract = registry.get(&request.contract_id).unwrap();
    let contract_json = contract.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to serialize contract",
                "message": e.to_string()
            })),
        )
    })?;

    Ok(Json(ProtocolContractResponse {
        contract_id: request.contract_id,
        version: request.version,
        protocol: "kafka".to_string(),
        contract: contract_json,
    }))
}

/// Delete a contract
pub async fn delete_contract(
    State(state): State<ProtocolContractState>,
    Path(contract_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut registry = state.registry.write().await;

    registry.remove(&contract_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Contract not found",
                "contract_id": contract_id
            })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "message": "Contract deleted",
        "contract_id": contract_id
    })))
}

/// Compare two contracts
pub async fn compare_contracts_handler(
    State(state): State<ProtocolContractState>,
    Json(request): Json<CompareContractsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.registry.read().await;

    let old_contract = registry.get(&request.old_contract_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Old contract not found",
                "contract_id": request.old_contract_id
            })),
        )
    })?;

    let new_contract = registry.get(&request.new_contract_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "New contract not found",
                "contract_id": request.new_contract_id
            })),
        )
    })?;

    let diff_result = compare_contracts(old_contract, new_contract)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to compare contracts",
                    "message": e.to_string()
                })),
            )
        })?;

    Ok(Json(serde_json::json!({
        "matches": diff_result.matches,
        "confidence": diff_result.confidence,
        "mismatches": diff_result.mismatches,
        "recommendations": diff_result.recommendations,
        "corrections": diff_result.corrections,
    })))
}

/// Validate a message against a contract
pub async fn validate_message(
    State(state): State<ProtocolContractState>,
    Path(contract_id): Path<String>,
    Json(request): Json<ValidateMessageRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.registry.read().await;

    let contract = registry.get(&contract_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Contract not found",
                "contract_id": contract_id
            })),
        )
    })?;

    // Convert payload to bytes
    let payload_bytes = match request.payload {
        serde_json::Value::String(s) => {
            // Try base64 decode first, then fall back to UTF-8
            general_purpose::STANDARD.decode(&s).unwrap_or_else(|_| s.into_bytes())
        }
        _ => serde_json::to_vec(&request.payload).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Failed to serialize payload",
                    "message": e.to_string()
                })),
            )
        })?,
    };

    let contract_request = mockforge_core::contract_drift::protocol_contracts::ContractRequest {
        protocol: contract.protocol(),
        operation_id: request.operation_id.clone(),
        payload: payload_bytes,
        content_type: request.content_type,
        metadata: request.metadata.unwrap_or_default(),
    };

    let validation_result = contract
        .validate(&request.operation_id, &contract_request)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Validation failed",
                    "message": e.to_string()
                })),
            )
        })?;

    Ok(Json(serde_json::json!({
        "valid": validation_result.valid,
        "errors": validation_result.errors,
        "warnings": validation_result.warnings,
    })))
}

/// Get contract router
pub fn protocol_contracts_router(state: ProtocolContractState) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/", get(list_contracts))
        .route("/{contract_id}", get(get_contract))
        .route("/{contract_id}", delete(delete_contract))
        .route("/grpc", post(create_grpc_contract))
        .route("/websocket", post(create_websocket_contract))
        .route("/mqtt", post(create_mqtt_contract))
        .route("/kafka", post(create_kafka_contract))
        .route("/compare", post(compare_contracts_handler))
        .route("/{contract_id}/validate", post(validate_message))
        .with_state(state)
}
