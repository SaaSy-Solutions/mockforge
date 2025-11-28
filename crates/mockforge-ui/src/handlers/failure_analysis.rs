//! Failure analysis API handlers
//!
//! Provides endpoints for retrieving failure narratives and analyzing request failures.

use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::Json as ResponseJson,
};
use mockforge_core::failure_analysis::{FailureContextCollector, FailureNarrativeGenerator};
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::ApiResponse;

/// In-memory storage for failure narratives
/// In a production system, this would be persisted to a database
type FailureStorage = Arc<RwLock<HashMap<String, StoredFailure>>>;

/// Stored failure with narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredFailure {
    /// Request ID
    request_id: String,
    /// Failure context
    context: mockforge_core::FailureContext,
    /// Generated narrative
    narrative: Option<mockforge_core::FailureNarrative>,
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// Global failure storage (in-memory)
static FAILURE_STORAGE: once_cell::sync::Lazy<FailureStorage> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Request to analyze a failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFailureRequest {
    /// Request method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Request body
    pub body: Option<Value>,
    /// Response status code (if available)
    pub status_code: Option<u16>,
    /// Response headers
    #[serde(default)]
    pub response_headers: HashMap<String, String>,
    /// Response body
    pub response_body: Option<Value>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Error message
    pub error_message: Option<String>,
}

/// Response from failure analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFailureResponse {
    /// Request ID (generated)
    pub request_id: String,
    /// Failure context
    pub context: mockforge_core::FailureContext,
    /// Generated narrative
    pub narrative: Option<mockforge_core::FailureNarrative>,
    /// Error message (if analysis failed)
    pub error: Option<String>,
}

/// Analyze a failure and generate a narrative
///
/// POST /api/v2/failures/analyze
pub async fn analyze_failure(
    Json(request): Json<AnalyzeFailureRequest>,
) -> Result<ResponseJson<ApiResponse<AnalyzeFailureResponse>>, StatusCode> {
    // Generate a request ID
    let request_id = Uuid::new_v4().to_string();

    // Create context collector
    let collector = FailureContextCollector::new();

    // Collect failure context
    let context = collector
        .collect_context_with_details(
            &request.method,
            &request.path,
            request.headers,
            request.query_params,
            request.body,
            request.status_code,
            request.response_headers,
            request.response_body,
            request.duration_ms,
            request.error_message,
            vec![], // chaos_configs - would be populated from actual system
            vec![], // consistency_rules - would be populated from actual system
            None,   // contract_validation - would be populated from actual system
            vec![], // behavioral_rules - would be populated from actual system
            vec![], // hook_results - would be populated from actual system
        )
        .map_err(|e| {
            tracing::error!("Failed to collect failure context: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Generate narrative
    let config = IntelligentBehaviorConfig::default();
    let generator = FailureNarrativeGenerator::new(config);
    let narrative = match generator.generate_narrative(&context).await {
        Ok(narrative) => Some(narrative),
        Err(e) => {
            tracing::warn!("Failed to generate narrative: {}", e);
            None
        }
    };

    // Store failure
    let stored = StoredFailure {
        request_id: request_id.clone(),
        context: context.clone(),
        narrative: narrative.clone(),
        timestamp: chrono::Utc::now(),
    };

    {
        let mut storage = FAILURE_STORAGE.write().await;
        storage.insert(request_id.clone(), stored);
    }

    let response = AnalyzeFailureResponse {
        request_id,
        context,
        narrative,
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Get failure analysis by request ID
///
/// GET /api/v2/failures/{request_id}
pub async fn get_failure_analysis(
    Path(request_id): Path<String>,
) -> Result<ResponseJson<ApiResponse<AnalyzeFailureResponse>>, StatusCode> {
    let storage = FAILURE_STORAGE.read().await;
    let stored = storage.get(&request_id).ok_or(StatusCode::NOT_FOUND)?;

    let response = AnalyzeFailureResponse {
        request_id: stored.request_id.clone(),
        context: stored.context.clone(),
        narrative: stored.narrative.clone(),
        error: None,
    };

    Ok(ResponseJson(ApiResponse::success(response)))
}

/// List recent failures
///
/// GET /api/v2/failures/recent
pub async fn list_recent_failures(
) -> Result<ResponseJson<ApiResponse<Vec<FailureSummary>>>, StatusCode> {
    let storage = FAILURE_STORAGE.read().await;

    // Get all failures, sorted by timestamp (most recent first)
    let mut failures: Vec<_> = storage
        .values()
        .map(|f| FailureSummary {
            request_id: f.request_id.clone(),
            method: f.context.request.method.clone(),
            path: f.context.request.path.clone(),
            status_code: f.context.response.as_ref().map(|r| r.status_code),
            error_message: f.context.error_message.clone(),
            timestamp: f.timestamp,
            has_narrative: f.narrative.is_some(),
        })
        .collect();

    failures.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    failures.truncate(50); // Limit to 50 most recent

    Ok(ResponseJson(ApiResponse::success(failures)))
}

/// Failure summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSummary {
    /// Request ID
    pub request_id: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Status code (if available)
    pub status_code: Option<u16>,
    /// Error message
    pub error_message: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether a narrative was generated
    pub has_narrative: bool,
}
