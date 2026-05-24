// Uses deprecated `mockforge_core::intelligent_behavior::openapi_generator`
// wrappers pending the eventual intelligent_behavior migration.
#![allow(deprecated)]

//! OpenAPI inference from recorded traffic
//! (`POST /__mockforge/mockai/generate-openapi`).
//!
//! Split out of the original `management/ai_gen.rs` under #656. This
//! handler reads recorded HTTP exchanges from `mockforge-recorder` and
//! synthesizes an OpenAPI spec via `mockforge_core::intelligent_behavior`.
//!
//! Stays in `mockforge-http` rather than moving to `mockforge-intelligence`:
//! `mockforge-recorder` already depends on `mockforge-intelligence`, so
//! the orchestration layer (this file) can't live in intelligence
//! without re-introducing a cycle — same constraint as
//! `handlers::behavioral_cloning`.

use serde::Deserialize;

// Axum + ManagementState imports are only consumed by the feature-gated
// handler below; gate the imports to match so non-`behavioral-cloning`
// builds don't trip the unused-imports lint.
#[cfg(feature = "behavioral-cloning")]
use super::ManagementState;
#[cfg(feature = "behavioral-cloning")]
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};

/// Request for OpenAPI generation from recorded traffic
#[derive(Debug, Deserialize)]
pub struct GenerateOpenApiFromTrafficRequest {
    /// Path to recorder database (optional, defaults to ./recordings.db)
    #[serde(default)]
    pub database_path: Option<String>,
    /// Start time for filtering (ISO 8601 format, e.g., 2025-01-01T00:00:00Z)
    #[serde(default)]
    pub since: Option<String>,
    /// End time for filtering (ISO 8601 format)
    #[serde(default)]
    pub until: Option<String>,
    /// Path pattern filter (supports wildcards, e.g., /api/*)
    #[serde(default)]
    pub path_pattern: Option<String>,
    /// Minimum confidence score for including paths (0.0 to 1.0)
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
}

fn default_min_confidence() -> f64 {
    0.7
}

/// Generate OpenAPI specification from recorded traffic
#[cfg(feature = "behavioral-cloning")]
pub(crate) async fn generate_openapi_from_traffic(
    State(_state): State<ManagementState>,
    Json(request): Json<GenerateOpenApiFromTrafficRequest>,
) -> impl IntoResponse {
    use chrono::{DateTime, Utc};
    use mockforge_core::intelligent_behavior::{
        openapi_generator::{OpenApiGenerationConfig, OpenApiSpecGenerator},
        IntelligentBehaviorConfig,
    };
    use mockforge_recorder::{
        database::RecorderDatabase,
        openapi_export::{QueryFilters, RecordingsToOpenApi},
    };
    use std::path::PathBuf;

    // Determine database path
    let db_path = if let Some(ref path) = request.database_path {
        PathBuf::from(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("recordings.db")
    };

    // Open database
    let db = match RecorderDatabase::new(&db_path).await {
        Ok(db) => db,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Database error",
                    "message": format!("Failed to open recorder database: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Parse time filters
    let since_dt = if let Some(ref since_str) = request.since {
        match DateTime::parse_from_rfc3339(since_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --since format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let until_dt = if let Some(ref until_str) = request.until {
        match DateTime::parse_from_rfc3339(until_str) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid date format",
                        "message": format!("Invalid --until format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    // Build query filters
    let query_filters = QueryFilters {
        since: since_dt,
        until: until_dt,
        path_pattern: request.path_pattern.clone(),
        min_status_code: None,
        max_requests: Some(1000),
    };

    // Query HTTP exchanges
    // Note: We need to convert from mockforge-recorder's HttpExchange to mockforge-core's HttpExchange
    // to avoid version mismatch issues. The converter returns the version from mockforge-recorder's
    // dependency, so we need to manually convert to the local version.
    let exchanges_from_recorder =
        match RecordingsToOpenApi::query_http_exchanges(&db, Some(query_filters)).await {
            Ok(exchanges) => exchanges,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Query error",
                        "message": format!("Failed to query HTTP exchanges: {}", e)
                    })),
                )
                    .into_response();
            }
        };

    if exchanges_from_recorder.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "No exchanges found",
                "message": "No HTTP exchanges found matching the specified filters"
            })),
        )
            .into_response();
    }

    // Convert to local HttpExchange type to avoid version mismatch
    use mockforge_core::intelligent_behavior::openapi_generator::HttpExchange as LocalHttpExchange;
    let exchanges: Vec<LocalHttpExchange> = exchanges_from_recorder
        .into_iter()
        .map(|e| LocalHttpExchange {
            method: e.method,
            path: e.path,
            query_params: e.query_params,
            headers: e.headers,
            body: e.body,
            body_encoding: e.body_encoding,
            status_code: e.status_code,
            response_headers: e.response_headers,
            response_body: e.response_body,
            response_body_encoding: e.response_body_encoding,
            timestamp: e.timestamp,
        })
        .collect();

    // Create OpenAPI generator config
    let behavior_config = IntelligentBehaviorConfig::default();
    let gen_config = OpenApiGenerationConfig {
        min_confidence: request.min_confidence,
        behavior_model: Some(behavior_config.behavior_model),
    };

    // Generate OpenAPI spec
    let generator = OpenApiSpecGenerator::new(gen_config);
    let result = match generator.generate_from_exchanges(exchanges).await {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Generation error",
                    "message": format!("Failed to generate OpenAPI spec: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Prepare response
    let spec_json = if let Some(ref raw) = result.spec.raw_document {
        raw.clone()
    } else {
        match serde_json::to_value(&result.spec.spec) {
            Ok(json) => json,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "Serialization error",
                        "message": format!("Failed to serialize OpenAPI spec: {}", e)
                    })),
                )
                    .into_response();
            }
        }
    };

    // Build response with metadata
    let response = serde_json::json!({
        "spec": spec_json,
        "metadata": {
            "requests_analyzed": result.metadata.requests_analyzed,
            "paths_inferred": result.metadata.paths_inferred,
            "path_confidence": result.metadata.path_confidence,
            "generated_at": result.metadata.generated_at.to_rfc3339(),
            "duration_ms": result.metadata.duration_ms,
        }
    });

    Json(response).into_response()
}
