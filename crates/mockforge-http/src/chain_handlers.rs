//! Chain management HTTP handlers for MockForge
//!
//! This module provides REST endpoints for managing and executing request chains
//! through the HTTP API.

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use mockforge_core::chain_execution::ChainExecutionEngine;
use mockforge_core::request_chaining::RequestChainRegistry;

/// Shared state for chain management
#[derive(Clone)]
pub struct ChainState {
    /// Request chain registry for storing and retrieving chains
    registry: Arc<RequestChainRegistry>,
    /// Chain execution engine for running request chains
    engine: Arc<ChainExecutionEngine>,
}

/// Create the chain state with registry and engine
///
/// # Arguments
/// * `registry` - Request chain registry for chain storage
/// * `engine` - Chain execution engine for running chains
pub fn create_chain_state(
    registry: Arc<RequestChainRegistry>,
    engine: Arc<ChainExecutionEngine>,
) -> ChainState {
    ChainState { registry, engine }
}

/// Request body for executing a request chain
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionRequest {
    /// Optional variables to pass to the chain execution
    pub variables: Option<serde_json::Value>,
}

/// Response from chain execution
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionResponse {
    /// ID of the executed chain
    pub chain_id: String,
    /// Execution status ("successful", "partial_success", "failed")
    pub status: String,
    /// Total execution duration in milliseconds
    pub total_duration_ms: u64,
    /// Results of individual requests in the chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_results: Option<serde_json::Value>,
    /// Error message if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Response listing all available chains
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainListResponse {
    /// List of chain summaries
    pub chains: Vec<ChainSummary>,
    /// Total number of chains
    pub total: usize,
}

/// Summary information for a request chain
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainSummary {
    /// Unique chain identifier
    pub id: String,
    /// Human-readable chain name
    pub name: String,
    /// Optional chain description
    pub description: Option<String>,
    /// Tags associated with this chain
    pub tags: Vec<String>,
    /// Whether this chain is enabled
    pub enabled: bool,
    /// Number of links (requests) in this chain
    pub link_count: usize,
}

/// Request body for creating a new chain
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainCreateRequest {
    /// YAML definition of the chain
    pub definition: String,
}

/// Response from chain creation
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainCreateResponse {
    /// ID of the created chain
    pub id: String,
    /// Success message
    pub message: String,
}

/// Response from chain validation
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainValidationResponse {
    /// Whether the chain is valid
    pub valid: bool,
    /// Validation error messages if any
    pub errors: Vec<String>,
    /// Validation warnings if any
    pub warnings: Vec<String>,
}

/// Response containing chain execution history
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionHistoryResponse {
    /// ID of the chain
    pub chain_id: String,
    /// List of execution records
    pub executions: Vec<ChainExecutionRecord>,
    /// Total number of executions
    pub total: usize,
}

/// Record of a single chain execution
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionRecord {
    /// ISO 8601 timestamp when execution started
    pub executed_at: String,
    /// Execution status ("successful", "partial_success", "failed")
    pub status: String,
    /// Total execution duration in milliseconds
    pub total_duration_ms: u64,
    /// Number of requests in the chain
    pub request_count: usize,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

/// GET /chains - List all available request chains
pub async fn list_chains(State(state): State<ChainState>) -> impl IntoResponse {
    let chain_ids = state.registry.list_chains().await;
    let mut chains = Vec::new();

    for id in chain_ids {
        if let Some(chain) = state.registry.get_chain(&id).await {
            chains.push(ChainSummary {
                id: chain.id.clone(),
                name: chain.name.clone(),
                description: chain.description.clone(),
                tags: chain.tags.clone(),
                enabled: chain.config.enabled,
                link_count: chain.links.len(),
            });
        }
    }

    let total = chains.len();
    Json(ChainListResponse { chains, total })
}

/// GET /chains/:id - Get details for a specific chain
pub async fn get_chain(Path(chain_id): Path<String>, State(state): State<ChainState>) -> Response {
    match state.registry.get_chain(&chain_id).await {
        Some(chain) => Json(chain).into_response(),
        None => (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response(),
    }
}

/// POST /chains - Create a new request chain from YAML definition
pub async fn create_chain(
    State(state): State<ChainState>,
    Json(request): Json<ChainCreateRequest>,
) -> Response {
    match state.registry.register_from_yaml(&request.definition).await {
        Ok(id) => Json(ChainCreateResponse {
            id: id.clone(),
            message: format!("Chain '{}' created successfully", id),
        })
        .into_response(),
        Err(e) => {
            (StatusCode::BAD_REQUEST, format!("Failed to create chain: {}", e)).into_response()
        }
    }
}

/// PUT /chains/:id - Update an existing chain with new definition
pub async fn update_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
    Json(request): Json<ChainCreateRequest>,
) -> Response {
    // Remove the old chain first
    if state.registry.remove_chain(&chain_id).await.is_err() {
        return (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response();
    }

    // Create the new chain
    match state.registry.register_from_yaml(&request.definition).await {
        Ok(new_id) => {
            if new_id != chain_id {
                return (StatusCode::BAD_REQUEST, "Chain ID mismatch in update".to_string())
                    .into_response();
            }
            Json(serde_json::json!({
                "id": new_id,
                "message": "Chain updated successfully"
            }))
            .into_response()
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, format!("Failed to update chain: {}", e)).into_response()
        }
    }
}

/// DELETE /chains/:id - Delete a request chain
pub async fn delete_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
) -> Response {
    match state.registry.remove_chain(&chain_id).await {
        Ok(_) => Json(serde_json::json!({
            "id": chain_id,
            "message": "Chain deleted successfully"
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete chain: {}", e))
            .into_response(),
    }
}

/// POST /chains/:id/execute - Execute a request chain with optional variables
pub async fn execute_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
    Json(request): Json<ChainExecutionRequest>,
) -> Response {
    match state.engine.execute_chain(&chain_id, request.variables).await {
        Ok(result) => Json(ChainExecutionResponse {
            chain_id: result.chain_id,
            status: match result.status {
                mockforge_core::chain_execution::ChainExecutionStatus::Successful => {
                    "successful".to_string()
                }
                mockforge_core::chain_execution::ChainExecutionStatus::PartialSuccess => {
                    "partial_success".to_string()
                }
                mockforge_core::chain_execution::ChainExecutionStatus::Failed => {
                    "failed".to_string()
                }
            },
            total_duration_ms: result.total_duration_ms,
            request_results: Some(serde_json::to_value(result.request_results).unwrap_or_default()),
            error_message: result.error_message,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to execute chain: {}", e))
            .into_response(),
    }
}

/// POST /chains/:id/validate - Validate chain definition for correctness
pub async fn validate_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
) -> Response {
    match state.registry.get_chain(&chain_id).await {
        Some(chain) => {
            match state.registry.validate_chain(&chain).await {
                Ok(()) => Json(ChainValidationResponse {
                    valid: true,
                    errors: vec![],
                    warnings: vec![], // Could add warnings for potential issues
                })
                .into_response(),
                Err(e) => Json(ChainValidationResponse {
                    valid: false,
                    errors: vec![e.to_string()],
                    warnings: vec![],
                })
                .into_response(),
            }
        }
        None => (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response(),
    }
}

/// GET /chains/:id/history - Get execution history for a chain
pub async fn get_chain_history(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
) -> Response {
    // Check if chain exists
    if state.registry.get_chain(&chain_id).await.is_none() {
        return (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response();
    }

    let history = state.engine.get_chain_history(&chain_id).await;

    let executions: Vec<ChainExecutionRecord> = history
        .into_iter()
        .map(|record| ChainExecutionRecord {
            executed_at: record.executed_at,
            status: match record.result.status {
                mockforge_core::chain_execution::ChainExecutionStatus::Successful => {
                    "successful".to_string()
                }
                mockforge_core::chain_execution::ChainExecutionStatus::PartialSuccess => {
                    "partial_success".to_string()
                }
                mockforge_core::chain_execution::ChainExecutionStatus::Failed => {
                    "failed".to_string()
                }
            },
            total_duration_ms: record.result.total_duration_ms,
            request_count: record.result.request_results.len(),
            error_message: record.result.error_message,
        })
        .collect();

    let total = executions.len();

    Json(ChainExecutionHistoryResponse {
        chain_id,
        executions,
        total,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::chain_execution::ChainExecutionEngine;
    use mockforge_core::request_chaining::{ChainConfig, RequestChainRegistry};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_chain_state_creation() {
        let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
        let engine = Arc::new(ChainExecutionEngine::new(registry.clone(), ChainConfig::default()));
        let _state = create_chain_state(registry, engine);

        // Just verify creation works
    }
}
