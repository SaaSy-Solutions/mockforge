//! Chain management HTTP handlers for MockForge
//!
//! This module provides REST endpoints for managing and executing request chains
//! through the HTTP API.

use axum::extract::{Path, State};
use axum::{Json, http::StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use mockforge_core::chain_execution::ChainExecutionEngine;
use mockforge_core::request_chaining::RequestChainRegistry;

/// Shared state for chain management
#[derive(Clone)]
pub struct ChainState {
    registry: Arc<RequestChainRegistry>,
    engine: Arc<ChainExecutionEngine>,
}

/// Create the chain state with registry and engine
pub fn create_chain_state(
    registry: Arc<RequestChainRegistry>,
    engine: Arc<ChainExecutionEngine>,
) -> ChainState {
    ChainState { registry, engine }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionRequest {
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionResponse {
    pub chain_id: String,
    pub status: String,
    pub total_duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainListResponse {
    pub chains: Vec<ChainSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub link_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainCreateRequest {
    pub definition: String, // YAML content
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainCreateResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionHistoryResponse {
    pub chain_id: String,
    pub executions: Vec<ChainExecutionRecord>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExecutionRecord {
    pub executed_at: String,
    pub status: String,
    pub total_duration_ms: u64,
    pub request_count: usize,
    pub error_message: Option<String>,
}

// GET /chains - List all chains
pub async fn list_chains(
    State(state): State<ChainState>,
) -> impl IntoResponse {
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
    Json(ChainListResponse {
        chains,
        total,
    })
}

// GET /chains/:id - Get a specific chain
pub async fn get_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
) -> Response {
    match state.registry.get_chain(&chain_id).await {
        Some(chain) => Json(chain).into_response(),
        None => (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response(),
    }
}

// POST /chains - Create a new chain
pub async fn create_chain(
    State(state): State<ChainState>,
    Json(request): Json<ChainCreateRequest>,
) -> Response {
    match state.registry.register_from_yaml(&request.definition).await {
        Ok(id) => Json(ChainCreateResponse {
            id: id.clone(),
            message: format!("Chain '{}' created successfully", id),
        }).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Failed to create chain: {}", e)).into_response(),
    }
}

// PUT /chains/:id - Update an existing chain
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
                return (StatusCode::BAD_REQUEST, "Chain ID mismatch in update".to_string()).into_response();
            }
            Json(serde_json::json!({
                "id": new_id,
                "message": "Chain updated successfully"
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, format!("Failed to update chain: {}", e)).into_response(),
    }
}

// DELETE /chains/:id - Delete a chain
pub async fn delete_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
) -> Response {
    match state.registry.remove_chain(&chain_id).await {
        Ok(_) => Json(serde_json::json!({
            "id": chain_id,
            "message": "Chain deleted successfully"
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete chain: {}", e)).into_response(),
    }
}

// POST /chains/:id/execute - Execute a chain
pub async fn execute_chain(
    Path(chain_id): Path<String>,
    State(state): State<ChainState>,
    Json(request): Json<ChainExecutionRequest>,
) -> Response {
    match state.engine.execute_chain(&chain_id, request.variables).await {
        Ok(result) => Json(ChainExecutionResponse {
            chain_id: result.chain_id,
            status: match result.status {
                mockforge_core::chain_execution::ChainExecutionStatus::Successful => "successful".to_string(),
                mockforge_core::chain_execution::ChainExecutionStatus::PartialSuccess => "partial_success".to_string(),
                mockforge_core::chain_execution::ChainExecutionStatus::Failed => "failed".to_string(),
            },
            total_duration_ms: result.total_duration_ms,
            request_results: Some(serde_json::to_value(result.request_results).unwrap_or_default()),
            error_message: result.error_message,
        }).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to execute chain: {}", e)).into_response(),
    }
}

// POST /chains/:id/validate - Validate a chain
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
                }).into_response(),
                Err(e) => Json(ChainValidationResponse {
                    valid: false,
                    errors: vec![e.to_string()],
                    warnings: vec![],
                }).into_response(),
            }
        }
        None => (StatusCode::NOT_FOUND, format!("Chain '{}' not found", chain_id)).into_response(),
    }
}

// GET /chains/:id/history - Get execution history
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
                mockforge_core::chain_execution::ChainExecutionStatus::Successful => "successful".to_string(),
                mockforge_core::chain_execution::ChainExecutionStatus::PartialSuccess => "partial_success".to_string(),
                mockforge_core::chain_execution::ChainExecutionStatus::Failed => "failed".to_string(),
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
    }).into_response()
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
        let state = create_chain_state(registry, engine);

        // Just verify creation works
        assert!(true);
    }
}
