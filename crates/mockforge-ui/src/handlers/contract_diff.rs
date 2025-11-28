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
