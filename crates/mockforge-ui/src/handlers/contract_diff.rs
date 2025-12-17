//! Contract diff API handlers
//!
//! This module provides API endpoints for:
//! - Manual request upload
//! - Programmatic request submission
//! - Retrieving captured requests
//! - Triggering contract diff analysis

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mockforge_core::{
    ai_contract_diff::{CapturedRequest, ContractDiffAnalyzer, ContractDiffConfig},
    openapi::OpenApiSpec,
    request_capture::{get_global_capture_manager, CaptureQuery},
    Error,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

/// Helper to convert Error to HTTP response
fn error_response(error: Error) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "success": false,
            "error": error.to_string()
        })),
    )
        .into_response()
}

/// Upload a request manually for contract diff analysis
pub async fn upload_request(Json(payload): Json<UploadRequestPayload>) -> impl IntoResponse {
    let request = CapturedRequest::new(&payload.method, &payload.path, "manual_upload")
        .with_headers(payload.headers.unwrap_or_default())
        .with_query_params(payload.query_params.unwrap_or_default());

    let request = if let Some(body) = payload.body {
        request.with_body(body)
    } else {
        request
    };

    let request = if let Some(status_code) = payload.status_code {
        request.with_response(status_code, payload.response_body)
    } else {
        request
    };

    // Capture the request
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    match capture_manager.capture(request).await {
        Ok(capture_id) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "capture_id": capture_id,
                "message": "Request captured successfully"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// Submit a request programmatically via API
pub async fn submit_request(Json(payload): Json<SubmitRequestPayload>) -> impl IntoResponse {
    let request = CapturedRequest::new(&payload.method, &payload.path, "api_endpoint")
        .with_headers(payload.headers.unwrap_or_default())
        .with_query_params(payload.query_params.unwrap_or_default());

    let request = if let Some(body) = payload.body {
        request.with_body(body)
    } else {
        request
    };

    let request = if let Some(status_code) = payload.status_code {
        request.with_response(status_code, payload.response_body)
    } else {
        request
    };

    // Capture the request
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    match capture_manager.capture(request).await {
        Ok(capture_id) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "capture_id": capture_id,
                "message": "Request submitted successfully"
            })),
        )
            .into_response(),
        Err(e) => error_response(e),
    }
}

/// Get captured requests with optional filters
pub async fn get_captured_requests(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    let query = CaptureQuery {
        source: params.get("source").cloned(),
        method: params.get("method").cloned(),
        path_pattern: params.get("path_pattern").cloned(),
        analyzed: params.get("analyzed").and_then(|s| s.parse().ok()),
        limit: params.get("limit").and_then(|s| s.parse().ok()),
        offset: params.get("offset").and_then(|s| s.parse().ok()),
        ..Default::default()
    };

    let captures = capture_manager.query_captures(query).await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "count": captures.len(),
            "captures": captures.iter().map(|(req, meta)| json!({
                "id": meta.id,
                "method": req.method,
                "path": req.path,
                "source": meta.source,
                "captured_at": meta.captured_at,
                "analyzed": meta.analyzed,
                "query_params": req.query_params,
                "headers": req.headers,
            })).collect::<Vec<_>>()
        })),
    )
        .into_response()
}

/// Get a specific captured request by ID
pub async fn get_captured_request(Path(capture_id): Path<String>) -> impl IntoResponse {
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    let (request, metadata) = match capture_manager.get_capture(&capture_id).await {
        Some(result) => result,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Capture not found: {}", capture_id)
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "capture": {
                "id": metadata.id,
                "method": request.method,
                "path": request.path,
                "source": metadata.source,
                "captured_at": metadata.captured_at,
                "analyzed": metadata.analyzed,
                "contract_id": metadata.contract_id,
                "analysis_result_id": metadata.analysis_result_id,
                "query_params": request.query_params,
                "headers": request.headers,
                "body": request.body,
                "status_code": request.status_code,
                "response_body": request.response_body,
                "user_agent": request.user_agent,
                "metadata": request.metadata,
            }
        })),
    )
        .into_response()
}

/// Analyze a captured request against a contract specification
pub async fn analyze_captured_request(
    Path(capture_id): Path<String>,
    Json(payload): Json<AnalyzeRequestPayload>,
) -> impl IntoResponse {
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    // Get the captured request
    let (request, _metadata) = match capture_manager.get_capture(&capture_id).await {
        Some(result) => result,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Capture not found: {}", capture_id)
                })),
            )
                .into_response();
        }
    };

    // Load the contract specification
    let spec = match if let Some(spec_path) = &payload.spec_path {
        OpenApiSpec::from_file(spec_path).await
    } else if let Some(spec_content) = &payload.spec_content {
        // Try to detect format from content
        let format = if spec_content.trim_start().starts_with('{') {
            None // JSON
        } else {
            Some("yaml") // YAML
        };
        OpenApiSpec::from_string(spec_content, format)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Either spec_path or spec_content must be provided"
            })),
        )
            .into_response();
    } {
        Ok(spec) => spec,
        Err(e) => return error_response(e),
    };

    // Create contract diff analyzer
    let config = payload.config.unwrap_or_else(ContractDiffConfig::default);
    let analyzer = match ContractDiffAnalyzer::new(config) {
        Ok(analyzer) => analyzer,
        Err(e) => return error_response(e),
    };

    // Analyze the request
    let result = match analyzer.analyze(&request, &spec).await {
        Ok(result) => result,
        Err(e) => return error_response(e),
    };

    // Mark as analyzed
    let analysis_result_id = uuid::Uuid::new_v4().to_string();
    let contract_id = payload.contract_id.unwrap_or_else(|| "default".to_string());
    if let Err(e) = capture_manager
        .mark_analyzed(&capture_id, &contract_id, &analysis_result_id)
        .await
    {
        return error_response(e);
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "analysis_result_id": analysis_result_id,
            "result": result
        })),
    )
        .into_response()
}

/// Get capture statistics
pub async fn get_capture_statistics() -> impl IntoResponse {
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    let stats = capture_manager.get_statistics().await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "statistics": stats
        })),
    )
        .into_response()
}

/// Generate patch file for correction proposals
pub async fn generate_patch_file(
    Path(capture_id): Path<String>,
    Json(payload): Json<GeneratePatchPayload>,
) -> impl IntoResponse {
    let capture_manager = match get_global_capture_manager() {
        Some(manager) => manager,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Capture manager not initialized"
                })),
            )
                .into_response();
        }
    };

    // Get the captured request
    let (request, _metadata) = match capture_manager.get_capture(&capture_id).await {
        Some(result) => result,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Capture not found: {}", capture_id)
                })),
            )
                .into_response();
        }
    };

    // Load the contract specification
    let spec = match if let Some(spec_path) = &payload.spec_path {
        OpenApiSpec::from_file(spec_path).await
    } else if let Some(spec_content) = &payload.spec_content {
        let format = if spec_content.trim_start().starts_with('{') {
            None // JSON
        } else {
            Some("yaml") // YAML
        };
        OpenApiSpec::from_string(spec_content, format)
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Either spec_path or spec_content must be provided"
            })),
        )
            .into_response();
    } {
        Ok(spec) => spec,
        Err(e) => return error_response(e),
    };

    // Create contract diff analyzer
    let config = payload.config.unwrap_or_else(ContractDiffConfig::default);
    let analyzer = match ContractDiffAnalyzer::new(config) {
        Ok(analyzer) => analyzer,
        Err(e) => return error_response(e),
    };

    // Analyze the request
    let result = match analyzer.analyze(&request, &spec).await {
        Ok(result) => result,
        Err(e) => return error_response(e),
    };

    // Generate patch file
    if result.corrections.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "No corrections available to generate patch"
            })),
        )
            .into_response();
    }

    let spec_version = if spec.spec.info.version.is_empty() {
        "1.0.0".to_string()
    } else {
        spec.spec.info.version.clone()
    };
    let patch_file = analyzer.generate_patch_file(&result.corrections, &spec_version);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "patch_file": patch_file,
            "corrections_count": result.corrections.len()
        })),
    )
        .into_response()
}

/// Request payload for patch generation
#[derive(Debug, Deserialize)]
pub struct GeneratePatchPayload {
    /// Path to contract specification file
    pub spec_path: Option<String>,

    /// Contract specification content (OpenAPI YAML/JSON)
    pub spec_content: Option<String>,

    /// Contract diff configuration
    pub config: Option<ContractDiffConfig>,
}

/// Request payload for manual upload
#[derive(Debug, Deserialize)]
pub struct UploadRequestPayload {
    pub method: String,
    pub path: String,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub status_code: Option<u16>,
    pub response_body: Option<serde_json::Value>,
}

/// Request payload for programmatic submission
#[derive(Debug, Deserialize)]
pub struct SubmitRequestPayload {
    pub method: String,
    pub path: String,
    pub headers: Option<HashMap<String, String>>,
    pub query_params: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub status_code: Option<u16>,
    pub response_body: Option<serde_json::Value>,
}

/// Request payload for analysis
#[derive(Debug, Deserialize)]
pub struct AnalyzeRequestPayload {
    /// Path to contract specification file
    pub spec_path: Option<String>,

    /// Contract specification content (OpenAPI YAML/JSON)
    pub spec_content: Option<String>,

    /// Contract ID for tracking
    pub contract_id: Option<String>,

    /// Contract diff configuration
    pub config: Option<ContractDiffConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== UploadRequestPayload Tests ====================

    #[test]
    fn test_upload_request_payload_minimal() {
        let payload = UploadRequestPayload {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            headers: None,
            query_params: None,
            body: None,
            status_code: None,
            response_body: None,
        };

        assert_eq!(payload.method, "GET");
        assert_eq!(payload.path, "/api/users");
        assert!(payload.headers.is_none());
    }

    #[test]
    fn test_upload_request_payload_full() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let payload = UploadRequestPayload {
            method: "POST".to_string(),
            path: "/api/orders".to_string(),
            headers: Some(headers),
            query_params: Some(query_params),
            body: Some(serde_json::json!({"item": "book"})),
            status_code: Some(201),
            response_body: Some(serde_json::json!({"id": 123})),
        };

        assert_eq!(payload.method, "POST");
        assert_eq!(payload.status_code, Some(201));
        assert!(payload.body.is_some());
    }

    #[test]
    fn test_upload_request_payload_deserialization() {
        let json = r#"{
            "method": "DELETE",
            "path": "/api/items/123",
            "status_code": 204
        }"#;

        let payload: UploadRequestPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.method, "DELETE");
        assert_eq!(payload.path, "/api/items/123");
        assert_eq!(payload.status_code, Some(204));
    }

    #[test]
    fn test_upload_request_payload_debug() {
        let payload = UploadRequestPayload {
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: None,
            query_params: None,
            body: None,
            status_code: None,
            response_body: None,
        };

        let debug = format!("{:?}", payload);
        assert!(debug.contains("GET"));
        assert!(debug.contains("/test"));
    }

    // ==================== SubmitRequestPayload Tests ====================

    #[test]
    fn test_submit_request_payload_minimal() {
        let payload = SubmitRequestPayload {
            method: "PUT".to_string(),
            path: "/api/update".to_string(),
            headers: None,
            query_params: None,
            body: None,
            status_code: None,
            response_body: None,
        };

        assert_eq!(payload.method, "PUT");
        assert_eq!(payload.path, "/api/update");
    }

    #[test]
    fn test_submit_request_payload_with_body() {
        let payload = SubmitRequestPayload {
            method: "POST".to_string(),
            path: "/api/data".to_string(),
            headers: None,
            query_params: None,
            body: Some(serde_json::json!({"key": "value"})),
            status_code: Some(200),
            response_body: Some(serde_json::json!({"success": true})),
        };

        assert!(payload.body.is_some());
        assert!(payload.response_body.is_some());
    }

    #[test]
    fn test_submit_request_payload_deserialization() {
        let json = r#"{
            "method": "PATCH",
            "path": "/api/partial",
            "body": {"field": "updated"}
        }"#;

        let payload: SubmitRequestPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.method, "PATCH");
        assert!(payload.body.is_some());
    }

    // ==================== AnalyzeRequestPayload Tests ====================

    #[test]
    fn test_analyze_request_payload_with_spec_path() {
        let payload = AnalyzeRequestPayload {
            spec_path: Some("/path/to/spec.yaml".to_string()),
            spec_content: None,
            contract_id: Some("contract-123".to_string()),
            config: None,
        };

        assert!(payload.spec_path.is_some());
        assert!(payload.spec_content.is_none());
        assert_eq!(payload.contract_id, Some("contract-123".to_string()));
    }

    #[test]
    fn test_analyze_request_payload_with_spec_content() {
        let spec_content = r#"
            openapi: "3.0.0"
            info:
              title: Test API
              version: "1.0.0"
        "#;

        let payload = AnalyzeRequestPayload {
            spec_path: None,
            spec_content: Some(spec_content.to_string()),
            contract_id: None,
            config: None,
        };

        assert!(payload.spec_path.is_none());
        assert!(payload.spec_content.is_some());
    }

    #[test]
    fn test_analyze_request_payload_deserialization() {
        let json = r#"{
            "spec_path": "/specs/api.yaml",
            "contract_id": "my-contract"
        }"#;

        let payload: AnalyzeRequestPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.spec_path, Some("/specs/api.yaml".to_string()));
        assert_eq!(payload.contract_id, Some("my-contract".to_string()));
    }

    #[test]
    fn test_analyze_request_payload_empty() {
        let json = r#"{}"#;

        let payload: AnalyzeRequestPayload = serde_json::from_str(json).unwrap();
        assert!(payload.spec_path.is_none());
        assert!(payload.spec_content.is_none());
        assert!(payload.contract_id.is_none());
        assert!(payload.config.is_none());
    }

    // ==================== GeneratePatchPayload Tests ====================

    #[test]
    fn test_generate_patch_payload_with_spec_path() {
        let payload = GeneratePatchPayload {
            spec_path: Some("/path/to/spec.json".to_string()),
            spec_content: None,
            config: None,
        };

        assert!(payload.spec_path.is_some());
        assert!(payload.spec_content.is_none());
    }

    #[test]
    fn test_generate_patch_payload_with_spec_content() {
        let payload = GeneratePatchPayload {
            spec_path: None,
            spec_content: Some("{}".to_string()),
            config: None,
        };

        assert!(payload.spec_path.is_none());
        assert!(payload.spec_content.is_some());
    }

    #[test]
    fn test_generate_patch_payload_deserialization() {
        let json = r#"{
            "spec_path": "/api/openapi.yaml"
        }"#;

        let payload: GeneratePatchPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.spec_path, Some("/api/openapi.yaml".to_string()));
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_error_response_creation() {
        let error = Error::validation("test error");
        let response = error_response(error);
        // Response is created successfully
        let _ = response;
    }

    // ==================== HTTP Method Coverage ====================

    #[test]
    fn test_all_http_methods() {
        let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

        for method in methods {
            let payload = UploadRequestPayload {
                method: method.to_string(),
                path: "/test".to_string(),
                headers: None,
                query_params: None,
                body: None,
                status_code: None,
                response_body: None,
            };

            assert_eq!(payload.method, method);
        }
    }

    #[test]
    fn test_various_status_codes() {
        let status_codes = vec![200, 201, 204, 400, 401, 403, 404, 500, 502, 503];

        for code in status_codes {
            let payload = UploadRequestPayload {
                method: "GET".to_string(),
                path: "/test".to_string(),
                headers: None,
                query_params: None,
                body: None,
                status_code: Some(code),
                response_body: None,
            };

            assert_eq!(payload.status_code, Some(code));
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_payload_with_empty_path() {
        let payload = UploadRequestPayload {
            method: "GET".to_string(),
            path: "".to_string(),
            headers: None,
            query_params: None,
            body: None,
            status_code: None,
            response_body: None,
        };

        assert!(payload.path.is_empty());
    }

    #[test]
    fn test_payload_with_complex_body() {
        let body = serde_json::json!({
            "user": {
                "name": "John",
                "roles": ["admin", "user"],
                "settings": {
                    "theme": "dark",
                    "notifications": true
                }
            },
            "items": [1, 2, 3, 4, 5]
        });

        let payload = UploadRequestPayload {
            method: "POST".to_string(),
            path: "/api/complex".to_string(),
            headers: None,
            query_params: None,
            body: Some(body),
            status_code: None,
            response_body: None,
        };

        assert!(payload.body.is_some());
        let body_val = payload.body.unwrap();
        assert!(body_val.get("user").is_some());
        assert!(body_val.get("items").is_some());
    }

    #[test]
    fn test_payload_with_many_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("X-Request-ID".to_string(), "uuid-123".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let payload = UploadRequestPayload {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            headers: Some(headers.clone()),
            query_params: None,
            body: None,
            status_code: None,
            response_body: None,
        };

        assert!(payload.headers.is_some());
        assert_eq!(payload.headers.unwrap().len(), 5);
    }

    #[test]
    fn test_payload_with_many_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "50".to_string());
        query_params.insert("sort".to_string(), "created_at".to_string());
        query_params.insert("order".to_string(), "desc".to_string());
        query_params.insert("filter".to_string(), "active".to_string());

        let payload = UploadRequestPayload {
            method: "GET".to_string(),
            path: "/api/list".to_string(),
            headers: None,
            query_params: Some(query_params.clone()),
            body: None,
            status_code: None,
            response_body: None,
        };

        assert!(payload.query_params.is_some());
        assert_eq!(payload.query_params.unwrap().len(), 5);
    }
}
