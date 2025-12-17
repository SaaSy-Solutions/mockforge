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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AnalyzeFailureRequest Tests ====================

    #[test]
    fn test_analyze_failure_request_minimal() {
        let request = AnalyzeFailureRequest {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            status_code: None,
            response_headers: HashMap::new(),
            response_body: None,
            duration_ms: None,
            error_message: None,
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/users");
        assert!(request.headers.is_empty());
    }

    #[test]
    fn test_analyze_failure_request_full() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let request = AnalyzeFailureRequest {
            method: "POST".to_string(),
            path: "/api/orders".to_string(),
            headers,
            query_params,
            body: Some(serde_json::json!({"item": "book", "quantity": 1})),
            status_code: Some(500),
            response_headers: HashMap::new(),
            response_body: Some(serde_json::json!({"error": "Internal Server Error"})),
            duration_ms: Some(1500),
            error_message: Some("Database connection failed".to_string()),
        };

        assert_eq!(request.method, "POST");
        assert_eq!(request.status_code, Some(500));
        assert_eq!(request.duration_ms, Some(1500));
        assert!(request.error_message.is_some());
    }

    #[test]
    fn test_analyze_failure_request_serialization() {
        let request = AnalyzeFailureRequest {
            method: "DELETE".to_string(),
            path: "/api/items/123".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            status_code: Some(404),
            response_headers: HashMap::new(),
            response_body: None,
            duration_ms: Some(50),
            error_message: Some("Not found".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("DELETE"));
        assert!(json.contains("/api/items/123"));
        assert!(json.contains("404"));
    }

    #[test]
    fn test_analyze_failure_request_deserialization() {
        let json = r#"{
            "method": "PUT",
            "path": "/api/profile",
            "headers": {"Content-Type": "application/json"},
            "query_params": {},
            "body": {"name": "Test"},
            "status_code": 400,
            "response_headers": {},
            "response_body": {"error": "Validation failed"},
            "duration_ms": 100,
            "error_message": "Invalid input"
        }"#;

        let request: AnalyzeFailureRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "PUT");
        assert_eq!(request.path, "/api/profile");
        assert_eq!(request.status_code, Some(400));
        assert_eq!(request.error_message, Some("Invalid input".to_string()));
    }

    #[test]
    fn test_analyze_failure_request_clone() {
        let request = AnalyzeFailureRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            status_code: Some(200),
            response_headers: HashMap::new(),
            response_body: None,
            duration_ms: Some(10),
            error_message: None,
        };

        let cloned = request.clone();
        assert_eq!(cloned.method, request.method);
        assert_eq!(cloned.path, request.path);
    }

    // ==================== AnalyzeFailureResponse Tests ====================

    fn create_test_failure_context() -> mockforge_core::FailureContext {
        mockforge_core::FailureContext {
            request: mockforge_core::failure_analysis::RequestDetails {
                method: "GET".to_string(),
                path: "/test".to_string(),
                headers: HashMap::new(),
                query_params: HashMap::new(),
                body: None,
            },
            response: None,
            chaos_configs: vec![],
            consistency_rules: vec![],
            contract_validation: None,
            behavioral_rules: vec![],
            hook_results: vec![],
            error_message: None,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_analyze_failure_response_creation() {
        let response = AnalyzeFailureResponse {
            request_id: "test-uuid-123".to_string(),
            context: create_test_failure_context(),
            narrative: None,
            error: None,
        };

        assert_eq!(response.request_id, "test-uuid-123");
        assert!(response.narrative.is_none());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_analyze_failure_response_with_error() {
        let response = AnalyzeFailureResponse {
            request_id: "test-123".to_string(),
            context: create_test_failure_context(),
            narrative: None,
            error: Some("Analysis failed: timeout".to_string()),
        };

        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "Analysis failed: timeout");
    }

    #[test]
    fn test_analyze_failure_response_serialization() {
        let response = AnalyzeFailureResponse {
            request_id: "uuid-456".to_string(),
            context: create_test_failure_context(),
            narrative: None,
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("uuid-456"));
        assert!(json.contains("request_id"));
    }

    #[test]
    fn test_analyze_failure_response_clone() {
        let response = AnalyzeFailureResponse {
            request_id: "clone-test".to_string(),
            context: create_test_failure_context(),
            narrative: None,
            error: Some("Test error".to_string()),
        };

        let cloned = response.clone();
        assert_eq!(cloned.request_id, response.request_id);
        assert_eq!(cloned.error, response.error);
    }

    // ==================== FailureSummary Tests ====================

    #[test]
    fn test_failure_summary_creation() {
        let summary = FailureSummary {
            request_id: "summary-123".to_string(),
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            status_code: Some(500),
            error_message: Some("Internal error".to_string()),
            timestamp: chrono::Utc::now(),
            has_narrative: true,
        };

        assert_eq!(summary.request_id, "summary-123");
        assert_eq!(summary.method, "GET");
        assert!(summary.has_narrative);
    }

    #[test]
    fn test_failure_summary_no_status_code() {
        let summary = FailureSummary {
            request_id: "no-status".to_string(),
            method: "POST".to_string(),
            path: "/api/action".to_string(),
            status_code: None,
            error_message: Some("Connection timeout".to_string()),
            timestamp: chrono::Utc::now(),
            has_narrative: false,
        };

        assert!(summary.status_code.is_none());
        assert!(!summary.has_narrative);
    }

    #[test]
    fn test_failure_summary_serialization() {
        let summary = FailureSummary {
            request_id: "serialize-test".to_string(),
            method: "DELETE".to_string(),
            path: "/api/item/1".to_string(),
            status_code: Some(403),
            error_message: Some("Forbidden".to_string()),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            has_narrative: true,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("serialize-test"));
        assert!(json.contains("DELETE"));
        assert!(json.contains("403"));
        assert!(json.contains("Forbidden"));
    }

    #[test]
    fn test_failure_summary_deserialization() {
        let json = r#"{
            "request_id": "deser-test",
            "method": "PUT",
            "path": "/api/update",
            "status_code": 422,
            "error_message": "Unprocessable Entity",
            "timestamp": "2024-01-15T12:00:00Z",
            "has_narrative": false
        }"#;

        let summary: FailureSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.request_id, "deser-test");
        assert_eq!(summary.method, "PUT");
        assert_eq!(summary.status_code, Some(422));
        assert!(!summary.has_narrative);
    }

    #[test]
    fn test_failure_summary_clone() {
        let summary = FailureSummary {
            request_id: "clone-test".to_string(),
            method: "PATCH".to_string(),
            path: "/api/partial".to_string(),
            status_code: Some(200),
            error_message: None,
            timestamp: chrono::Utc::now(),
            has_narrative: true,
        };

        let cloned = summary.clone();
        assert_eq!(cloned.request_id, summary.request_id);
        assert_eq!(cloned.method, summary.method);
        assert_eq!(cloned.has_narrative, summary.has_narrative);
    }

    #[test]
    fn test_failure_summary_debug() {
        let summary = FailureSummary {
            request_id: "debug-test".to_string(),
            method: "GET".to_string(),
            path: "/debug".to_string(),
            status_code: Some(200),
            error_message: None,
            timestamp: chrono::Utc::now(),
            has_narrative: false,
        };

        let debug = format!("{:?}", summary);
        assert!(debug.contains("debug-test"));
        assert!(debug.contains("GET"));
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_analyze_failure_request_with_complex_body() {
        let body = serde_json::json!({
            "user": {
                "name": "John",
                "roles": ["admin", "user"],
                "metadata": {
                    "created": "2024-01-01"
                }
            },
            "items": [1, 2, 3]
        });

        let request = AnalyzeFailureRequest {
            method: "POST".to_string(),
            path: "/api/complex".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: Some(body.clone()),
            status_code: Some(201),
            response_headers: HashMap::new(),
            response_body: None,
            duration_ms: Some(500),
            error_message: None,
        };

        assert!(request.body.is_some());
        let body_value = request.body.unwrap();
        assert!(body_value.get("user").is_some());
    }

    #[test]
    fn test_failure_summary_various_http_methods() {
        let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];

        for method in methods {
            let summary = FailureSummary {
                request_id: format!("method-{}", method),
                method: method.to_string(),
                path: "/test".to_string(),
                status_code: Some(200),
                error_message: None,
                timestamp: chrono::Utc::now(),
                has_narrative: false,
            };

            assert_eq!(summary.method, method);
        }
    }

    #[test]
    fn test_failure_summary_various_status_codes() {
        let status_codes = vec![200, 201, 400, 401, 403, 404, 500, 502, 503];

        for code in status_codes {
            let summary = FailureSummary {
                request_id: format!("status-{}", code),
                method: "GET".to_string(),
                path: "/test".to_string(),
                status_code: Some(code),
                error_message: None,
                timestamp: chrono::Utc::now(),
                has_narrative: false,
            };

            assert_eq!(summary.status_code, Some(code));
        }
    }
}
