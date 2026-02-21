//! Protocol Contracts API Handlers
//!
//! Handles CRUD operations for protocol contracts (gRPC, WebSocket, MQTT, Kafka).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Protocol types supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Grpc,
    Websocket,
    Mqtt,
    Kafka,
}

/// A protocol contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolContract {
    pub contract_id: String,
    pub version: String,
    pub protocol: ProtocolType,
    pub contract: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// List contracts response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListContractsResponse {
    pub contracts: Vec<ProtocolContract>,
    pub total: usize,
}

/// Create gRPC contract request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGrpcContractRequest {
    pub contract_id: String,
    pub version: String,
    pub descriptor_set: String, // base64 encoded
}

/// WebSocket message type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageType {
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    pub schema: serde_json::Value,
    pub direction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

/// Create WebSocket contract request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWebSocketContractRequest {
    pub contract_id: String,
    pub version: String,
    pub message_types: Vec<WebSocketMessageType>,
}

/// MQTT topic schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttTopicSchema {
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qos: Option<u8>,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retained: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

/// Create MQTT contract request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMqttContractRequest {
    pub contract_id: String,
    pub version: String,
    pub topics: Vec<MqttTopicSchema>,
}

/// Topic schema format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSchema {
    pub format: String, // json, avro, protobuf
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Evolution rules for Kafka schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRules {
    pub allow_backward_compatible: bool,
    pub allow_forward_compatible: bool,
    pub require_version_bump: bool,
}

/// Kafka topic schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopicSchema {
    pub topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_schema: Option<TopicSchema>,
    pub value_schema: TopicSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partitions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_factor: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evolution_rules: Option<EvolutionRules>,
}

/// Create Kafka contract request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKafkaContractRequest {
    pub contract_id: String,
    pub version: String,
    pub topics: Vec<KafkaTopicSchema>,
}

/// Compare contracts request
#[derive(Debug, Clone, Deserialize)]
pub struct CompareContractsRequest {
    pub old_contract_id: String,
    pub new_contract_id: String,
}

/// Contract change
#[derive(Debug, Clone, Serialize)]
pub struct ContractChange {
    pub operation_id: String,
    pub change_type: String,
    pub description: String,
}

/// Compare contracts response
#[derive(Debug, Clone, Serialize)]
pub struct CompareContractsResponse {
    pub breaking_changes: Vec<ContractChange>,
    pub non_breaking_changes: Vec<ContractChange>,
    pub summary: CompareSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompareSummary {
    pub total_operations: usize,
    pub breaking_count: usize,
    pub non_breaking_count: usize,
}

/// Validate message request
#[derive(Debug, Clone, Deserialize)]
pub struct ValidateMessageRequest {
    pub operation_id: String,
    pub message: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_format: Option<String>,
}

/// Validation error
#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

/// Validate message response
#[derive(Debug, Clone, Serialize)]
pub struct ValidateMessageResponse {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

/// Query parameters for listing contracts
#[derive(Debug, Clone, Deserialize)]
pub struct ListContractsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

/// State for protocol contracts
#[derive(Clone)]
pub struct ProtocolContractsState {
    contracts: Arc<RwLock<HashMap<String, ProtocolContract>>>,
}

impl Default for ProtocolContractsState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolContractsState {
    pub fn new() -> Self {
        Self {
            contracts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// List all contracts
pub async fn list_contracts(
    State(state): State<ProtocolContractsState>,
    Query(query): Query<ListContractsQuery>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let filtered: Vec<ProtocolContract> = contracts
        .values()
        .filter(|c| {
            if let Some(ref protocol) = query.protocol {
                match protocol.to_lowercase().as_str() {
                    "grpc" => c.protocol == ProtocolType::Grpc,
                    "websocket" => c.protocol == ProtocolType::Websocket,
                    "mqtt" => c.protocol == ProtocolType::Mqtt,
                    "kafka" => c.protocol == ProtocolType::Kafka,
                    _ => true,
                }
            } else {
                true
            }
        })
        .cloned()
        .collect();

    let total = filtered.len();

    Json(serde_json::json!({
        "data": ListContractsResponse {
            contracts: filtered,
            total,
        }
    }))
}

/// Get a specific contract
pub async fn get_contract(
    State(state): State<ProtocolContractsState>,
    Path(contract_id): Path<String>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    match contracts.get(&contract_id) {
        Some(contract) => Json(serde_json::json!({
            "data": contract
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Contract '{}' not found", contract_id)
            })),
        )
            .into_response(),
    }
}

/// Delete a contract
pub async fn delete_contract(
    State(state): State<ProtocolContractsState>,
    Path(contract_id): Path<String>,
) -> impl IntoResponse {
    let mut contracts = state.contracts.write().await;

    match contracts.remove(&contract_id) {
        Some(_) => Json(serde_json::json!({
            "message": format!("Contract '{}' deleted", contract_id)
        }))
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Contract '{}' not found", contract_id)
            })),
        )
            .into_response(),
    }
}

/// Create a gRPC contract
pub async fn create_grpc_contract(
    State(state): State<ProtocolContractsState>,
    Json(request): Json<CreateGrpcContractRequest>,
) -> impl IntoResponse {
    let mut contracts = state.contracts.write().await;

    let now = chrono::Utc::now().to_rfc3339();
    let contract = ProtocolContract {
        contract_id: request.contract_id.clone(),
        version: request.version,
        protocol: ProtocolType::Grpc,
        contract: serde_json::json!({
            "descriptor_set": request.descriptor_set
        }),
        created_at: Some(now.clone()),
        updated_at: Some(now),
    };

    contracts.insert(request.contract_id.clone(), contract.clone());

    (StatusCode::CREATED, Json(serde_json::json!({ "data": contract })))
}

/// Create a WebSocket contract
pub async fn create_websocket_contract(
    State(state): State<ProtocolContractsState>,
    Json(request): Json<CreateWebSocketContractRequest>,
) -> impl IntoResponse {
    let mut contracts = state.contracts.write().await;

    let now = chrono::Utc::now().to_rfc3339();
    let contract = ProtocolContract {
        contract_id: request.contract_id.clone(),
        version: request.version,
        protocol: ProtocolType::Websocket,
        contract: serde_json::json!({
            "message_types": request.message_types
        }),
        created_at: Some(now.clone()),
        updated_at: Some(now),
    };

    contracts.insert(request.contract_id.clone(), contract.clone());

    (StatusCode::CREATED, Json(serde_json::json!({ "data": contract })))
}

/// Create an MQTT contract
pub async fn create_mqtt_contract(
    State(state): State<ProtocolContractsState>,
    Json(request): Json<CreateMqttContractRequest>,
) -> impl IntoResponse {
    let mut contracts = state.contracts.write().await;

    let now = chrono::Utc::now().to_rfc3339();
    let contract = ProtocolContract {
        contract_id: request.contract_id.clone(),
        version: request.version,
        protocol: ProtocolType::Mqtt,
        contract: serde_json::json!({
            "topics": request.topics
        }),
        created_at: Some(now.clone()),
        updated_at: Some(now),
    };

    contracts.insert(request.contract_id.clone(), contract.clone());

    (StatusCode::CREATED, Json(serde_json::json!({ "data": contract })))
}

/// Create a Kafka contract
pub async fn create_kafka_contract(
    State(state): State<ProtocolContractsState>,
    Json(request): Json<CreateKafkaContractRequest>,
) -> impl IntoResponse {
    let mut contracts = state.contracts.write().await;

    let now = chrono::Utc::now().to_rfc3339();
    let contract = ProtocolContract {
        contract_id: request.contract_id.clone(),
        version: request.version,
        protocol: ProtocolType::Kafka,
        contract: serde_json::json!({
            "topics": request.topics
        }),
        created_at: Some(now.clone()),
        updated_at: Some(now),
    };

    contracts.insert(request.contract_id.clone(), contract.clone());

    (StatusCode::CREATED, Json(serde_json::json!({ "data": contract })))
}

/// Compare two contracts
pub async fn compare_contracts(
    State(state): State<ProtocolContractsState>,
    Json(request): Json<CompareContractsRequest>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let old_contract = match contracts.get(&request.old_contract_id) {
        Some(c) => c,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Contract '{}' not found", request.old_contract_id)
                })),
            )
                .into_response()
        }
    };

    let new_contract = match contracts.get(&request.new_contract_id) {
        Some(c) => c,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Contract '{}' not found", request.new_contract_id)
                })),
            )
                .into_response()
        }
    };

    // Simple comparison - in production this would do deep schema comparison
    let mut breaking_changes = Vec::new();
    let mut non_breaking_changes = Vec::new();

    if old_contract.protocol != new_contract.protocol {
        breaking_changes.push(ContractChange {
            operation_id: "protocol".to_string(),
            change_type: "protocol_change".to_string(),
            description: format!(
                "Protocol changed from {:?} to {:?}",
                old_contract.protocol, new_contract.protocol
            ),
        });
    }

    if old_contract.version != new_contract.version {
        non_breaking_changes.push(ContractChange {
            operation_id: "version".to_string(),
            change_type: "version_bump".to_string(),
            description: format!(
                "Version changed from {} to {}",
                old_contract.version, new_contract.version
            ),
        });
    }

    let response = CompareContractsResponse {
        summary: CompareSummary {
            total_operations: breaking_changes.len() + non_breaking_changes.len(),
            breaking_count: breaking_changes.len(),
            non_breaking_count: non_breaking_changes.len(),
        },
        breaking_changes,
        non_breaking_changes,
    };

    Json(serde_json::json!({ "data": response })).into_response()
}

/// Validate a message against a contract
pub async fn validate_message(
    State(state): State<ProtocolContractsState>,
    Path(contract_id): Path<String>,
    Json(request): Json<ValidateMessageRequest>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    match contracts.get(&contract_id) {
        Some(_contract) => {
            // Simple validation - in production this would validate against the actual schema
            let response = ValidateMessageResponse {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            };

            Json(serde_json::json!({ "data": response }))
        }
        None => Json(serde_json::json!({
            "error": format!("Contract '{}' not found", contract_id)
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_type_serialization() {
        assert_eq!(serde_json::to_string(&ProtocolType::Grpc).unwrap(), "\"grpc\"");
        assert_eq!(serde_json::to_string(&ProtocolType::Websocket).unwrap(), "\"websocket\"");
        assert_eq!(serde_json::to_string(&ProtocolType::Mqtt).unwrap(), "\"mqtt\"");
        assert_eq!(serde_json::to_string(&ProtocolType::Kafka).unwrap(), "\"kafka\"");
    }

    #[tokio::test]
    async fn test_protocol_contracts_state_new() {
        let state = ProtocolContractsState::new();
        // Should be empty initially
        let contracts = state.contracts.read().await;
        assert!(contracts.is_empty());
    }
}
