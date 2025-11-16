//! Webhook testing utilities and endpoints
//!
//! This module provides endpoints for testing webhook notifications
//! and utilities for validating webhook payloads.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;

/// State for webhook testing
#[derive(Clone)]
pub struct WebhookTestState {
    /// Received webhooks (for testing)
    pub received_webhooks: Arc<tokio::sync::RwLock<Vec<ReceivedWebhook>>>,
}

/// Received webhook entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedWebhook {
    /// Timestamp when webhook was received
    pub received_at: String,
    /// Webhook URL
    pub url: String,
    /// Event type
    pub event: String,
    /// Payload
    pub payload: serde_json::Value,
    /// Headers
    pub headers: HashMap<String, String>,
}

impl Default for WebhookTestState {
    fn default() -> Self {
        Self {
            received_webhooks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

/// Request to test a webhook
#[derive(Debug, Deserialize, Serialize)]
pub struct TestWebhookRequest {
    /// Webhook URL
    pub url: String,
    /// Event type
    pub event: String,
    /// Payload to send
    pub payload: serde_json::Value,
    /// Optional headers
    pub headers: Option<HashMap<String, String>>,
}

/// Response for webhook test
#[derive(Debug, Serialize)]
pub struct TestWebhookResponse {
    /// Success status
    pub success: bool,
    /// Status code from webhook endpoint
    pub status_code: Option<u16>,
    /// Response body
    pub response_body: Option<String>,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Test a webhook by sending a request
///
/// POST /api/v1/webhooks/test
pub async fn test_webhook(
    State(_state): State<WebhookTestState>,
    Json(request): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, StatusCode> {
    let client = reqwest::Client::new();

    let mut req = client.post(&request.url).json(&request.payload);

    // Add headers if provided
    if let Some(headers) = &request.headers {
        for (key, value) in headers {
            req = req.header(key, value);
        }
    }

    match req.send().await {
        Ok(response) => {
            let status_code = response.status().as_u16();
            let response_body = response.text().await.ok();

            Ok(Json(TestWebhookResponse {
                success: status_code < 400,
                status_code: Some(status_code),
                response_body,
                error: None,
            }))
        }
        Err(e) => {
            Ok(Json(TestWebhookResponse {
                success: false,
                status_code: None,
                response_body: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Receive a webhook (for testing webhook delivery)
///
/// POST /api/v1/webhooks/receive
pub async fn receive_webhook(
    State(state): State<WebhookTestState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract event type from headers or payload
    let event = headers
        .get("x-webhook-event")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| payload.get("event").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    // Extract URL from headers
    let url = headers
        .get("x-webhook-url")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Convert headers to HashMap
    let mut header_map = HashMap::new();
    for (key, value) in headers.iter() {
        let key_str = key.as_str().to_string();
        if let Ok(value_str) = value.to_str() {
            header_map.insert(key_str, value_str.to_string());
        }
    }

    // Store received webhook
    let received = ReceivedWebhook {
        received_at: chrono::Utc::now().to_rfc3339(),
        url,
        event,
        payload,
        headers: header_map,
    };

    state.received_webhooks.write().await.push(received.clone());

    Ok(Json(serde_json::json!({
        "status": "received",
        "event": received.event,
        "received_at": received.received_at,
    })))
}

/// Get received webhooks (for testing)
///
/// GET /api/v1/webhooks/received
pub async fn get_received_webhooks(
    State(state): State<WebhookTestState>,
) -> Result<Json<Vec<ReceivedWebhook>>, StatusCode> {
    let webhooks = state.received_webhooks.read().await.clone();
    Ok(Json(webhooks))
}

/// Clear received webhooks (for testing)
///
/// DELETE /api/v1/webhooks/received
pub async fn clear_received_webhooks(
    State(state): State<WebhookTestState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.received_webhooks.write().await.clear();
    Ok(Json(serde_json::json!({"status": "cleared"})))
}

/// Create webhook test router
pub fn webhook_test_router(state: WebhookTestState) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/api/v1/webhooks/test", post(test_webhook))
        .route("/api/v1/webhooks/receive", post(receive_webhook))
        .route("/api/v1/webhooks/received", get(get_received_webhooks))
        .route("/api/v1/webhooks/received", delete(clear_received_webhooks))
        .with_state(state)
}
