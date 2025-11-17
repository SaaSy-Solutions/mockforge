//! Consumer contracts handlers
//!
//! This module provides HTTP handlers for managing consumer contracts and usage tracking.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use mockforge_core::consumer_contracts::{
    Consumer, ConsumerBreakingChangeDetector, ConsumerIdentifier, ConsumerRegistry, ConsumerType,
    ConsumerUsage, ConsumerViolation, UsageRecorder,
};

/// State for consumer contracts handlers
#[derive(Clone)]
pub struct ConsumerContractsState {
    /// Consumer registry
    pub registry: Arc<ConsumerRegistry>,
    /// Usage recorder
    pub usage_recorder: Arc<UsageRecorder>,
    /// Breaking change detector
    pub detector: Arc<ConsumerBreakingChangeDetector>,
}

/// Request to register a consumer
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterConsumerRequest {
    /// Consumer name
    pub name: String,
    /// Consumer type
    pub consumer_type: String,
    /// Identifier value
    pub identifier: String,
    /// Workspace ID (optional)
    pub workspace_id: Option<String>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Response for consumer registration
#[derive(Debug, Serialize)]
pub struct ConsumerResponse {
    /// Consumer ID
    pub id: String,
    /// Consumer name
    pub name: String,
    /// Consumer type
    pub consumer_type: String,
    /// Identifier
    pub identifier: String,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Created at
    pub created_at: i64,
}

/// Request to query consumers
#[derive(Debug, Deserialize)]
pub struct ListConsumersRequest {
    /// Filter by workspace ID
    pub workspace_id: Option<String>,
    /// Filter by consumer type
    pub consumer_type: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Response for listing consumers
#[derive(Debug, Serialize)]
pub struct ListConsumersResponse {
    /// List of consumers
    pub consumers: Vec<ConsumerResponse>,
    /// Total count
    pub total: usize,
}

/// Response for consumer usage
#[derive(Debug, Serialize)]
pub struct ConsumerUsageResponse {
    /// Consumer ID
    pub consumer_id: String,
    /// Usage data
    pub usage: Vec<ConsumerUsage>,
}

/// Response for consumer violations
#[derive(Debug, Serialize)]
pub struct ConsumerViolationsResponse {
    /// Consumer ID
    pub consumer_id: String,
    /// Violations
    pub violations: Vec<ConsumerViolation>,
}

/// Register a consumer
///
/// POST /api/v1/consumers
pub async fn register_consumer(
    State(state): State<ConsumerContractsState>,
    Json(request): Json<RegisterConsumerRequest>,
) -> Result<Json<ConsumerResponse>, StatusCode> {
    let consumer_type = match request.consumer_type.as_str() {
        "workspace" => ConsumerType::Workspace,
        "custom" => ConsumerType::Custom,
        "api_key" => ConsumerType::ApiKey,
        "auth_token" => ConsumerType::AuthToken,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let identifier = match consumer_type {
        ConsumerType::Workspace => ConsumerIdentifier::workspace(request.identifier),
        ConsumerType::Custom => ConsumerIdentifier::custom(request.identifier),
        ConsumerType::ApiKey => ConsumerIdentifier::api_key(request.identifier),
        ConsumerType::AuthToken => ConsumerIdentifier::auth_token(request.identifier),
    };

    let consumer = state
        .registry
        .get_or_create(identifier, request.name.clone(), request.workspace_id.clone())
        .await;

    Ok(Json(ConsumerResponse {
        id: consumer.id,
        name: consumer.name,
        consumer_type: format!("{:?}", consumer.identifier.consumer_type),
        identifier: consumer.identifier.value,
        workspace_id: consumer.workspace_id,
        created_at: consumer.created_at,
    }))
}

/// List consumers
///
/// GET /api/v1/consumers
pub async fn list_consumers(
    State(state): State<ConsumerContractsState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListConsumersResponse>, StatusCode> {
    let mut consumers = state.registry.list_all().await;

    // Apply filters
    if let Some(workspace_id) = params.get("workspace_id") {
        consumers.retain(|c| c.workspace_id.as_ref().map(|w| w == workspace_id).unwrap_or(false));
    }

    if let Some(consumer_type_str) = params.get("consumer_type") {
        let consumer_type = match consumer_type_str.as_str() {
            "workspace" => ConsumerType::Workspace,
            "custom" => ConsumerType::Custom,
            "api_key" => ConsumerType::ApiKey,
            "auth_token" => ConsumerType::AuthToken,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        consumers.retain(|c| c.identifier.consumer_type == consumer_type);
    }

    let total = consumers.len();

    // Apply pagination
    let offset = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);
    let limit = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);

    consumers = consumers.into_iter().skip(offset).take(limit).collect();

    let consumer_responses: Vec<ConsumerResponse> = consumers
        .into_iter()
        .map(|c| ConsumerResponse {
            id: c.id,
            name: c.name,
            consumer_type: format!("{:?}", c.identifier.consumer_type),
            identifier: c.identifier.value,
            workspace_id: c.workspace_id,
            created_at: c.created_at,
        })
        .collect();

    Ok(Json(ListConsumersResponse {
        consumers: consumer_responses,
        total,
    }))
}

/// Get a specific consumer
///
/// GET /api/v1/consumers/{id}
pub async fn get_consumer(
    State(state): State<ConsumerContractsState>,
    Path(id): Path<String>,
) -> Result<Json<ConsumerResponse>, StatusCode> {
    let consumer = state.registry.get_by_id(&id).await.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ConsumerResponse {
        id: consumer.id,
        name: consumer.name,
        consumer_type: format!("{:?}", consumer.identifier.consumer_type),
        identifier: consumer.identifier.value,
        workspace_id: consumer.workspace_id,
        created_at: consumer.created_at,
    }))
}

/// Get consumer usage
///
/// GET /api/v1/consumers/{id}/usage
pub async fn get_consumer_usage(
    State(state): State<ConsumerContractsState>,
    Path(id): Path<String>,
) -> Result<Json<ConsumerUsageResponse>, StatusCode> {
    // Verify consumer exists
    state.registry.get_by_id(&id).await.ok_or(StatusCode::NOT_FOUND)?;

    let usage = state.usage_recorder.get_usage(&id).await;

    Ok(Json(ConsumerUsageResponse {
        consumer_id: id,
        usage,
    }))
}

/// Get consumer violations
///
/// GET /api/v1/consumers/{id}/violations
pub async fn get_consumer_violations(
    State(_state): State<ConsumerContractsState>,
    Path(_id): Path<String>,
) -> Result<Json<ConsumerViolationsResponse>, StatusCode> {
    // In a full implementation, this would query violations from storage
    // For now, return empty list
    Ok(Json(ConsumerViolationsResponse {
        consumer_id: _id,
        violations: vec![],
    }))
}

/// Create consumer contracts router
pub fn consumer_contracts_router(state: ConsumerContractsState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/api/v1/consumers", post(register_consumer))
        .route("/api/v1/consumers", get(list_consumers))
        .route("/api/v1/consumers/{id}", get(get_consumer))
        .route("/api/v1/consumers/{id}/usage", get(get_consumer_usage))
        .route("/api/v1/consumers/{id}/violations", get(get_consumer_violations))
        .with_state(state)
}
