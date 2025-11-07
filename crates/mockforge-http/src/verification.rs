//! HTTP verification API handlers for MockForge
//!
//! Provides REST endpoints for programmatic request verification,
//! allowing test code to verify that specific requests were made (or not made).

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use mockforge_core::{
    request_logger::get_global_logger,
    verification::{
        verify_at_least, verify_never, verify_requests, verify_sequence, VerificationCount,
        VerificationRequest, VerificationResult,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Request body for verification endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    /// Pattern to match requests
    pub pattern: VerificationRequest,
    /// Expected count assertion
    pub expected: VerificationCount,
}

/// Request body for count endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRequest {
    /// Pattern to match requests
    pub pattern: VerificationRequest,
}

/// Response for count endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    /// Number of matching requests
    pub count: usize,
}

/// Request body for sequence verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceRequest {
    /// Patterns to match in sequence
    pub patterns: Vec<VerificationRequest>,
}

/// Shared state for verification API (currently empty, but kept for future extensibility)
#[derive(Clone)]
pub struct VerificationState;

impl VerificationState {
    /// Create a new verification state
    pub fn new() -> Self {
        Self
    }
}

/// Create the verification API router
pub fn verification_router() -> Router {
    Router::new()
        .route("/api/verification/verify", post(handle_verify))
        .route("/api/verification/count", post(handle_count))
        .route("/api/verification/sequence", post(handle_sequence))
        .route("/api/verification/never", post(handle_never))
        .route("/api/verification/at-least", post(handle_at_least))
}

/// Verify requests against a pattern and count assertion
async fn handle_verify(Json(request): Json<VerifyRequest>) -> impl IntoResponse {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(VerificationResult::failure(
                    0,
                    request.expected.clone(),
                    Vec::new(),
                    "Request logger not initialized".to_string(),
                )),
            )
                .into_response();
        }
    };

    let result = verify_requests(logger, &request.pattern, request.expected).await;

    let status = if result.matched {
        StatusCode::OK
    } else {
        StatusCode::EXPECTATION_FAILED
    };

    (status, Json(result)).into_response()
}

/// Get count of matching requests
async fn handle_count(Json(request): Json<CountRequest>) -> impl IntoResponse {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(CountResponse { count: 0 }))
                .into_response();
        }
    };

    let count = logger.count_matching_requests(&request.pattern).await;

    (StatusCode::OK, Json(CountResponse { count })).into_response()
}

/// Verify that requests occurred in a specific sequence
async fn handle_sequence(Json(request): Json<SequenceRequest>) -> impl IntoResponse {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(VerificationResult::failure(
                    0,
                    VerificationCount::Exactly(request.patterns.len()),
                    Vec::new(),
                    "Request logger not initialized".to_string(),
                )),
            )
                .into_response();
        }
    };

    let result = verify_sequence(logger, &request.patterns).await;

    let status = if result.matched {
        StatusCode::OK
    } else {
        StatusCode::EXPECTATION_FAILED
    };

    (status, Json(result)).into_response()
}

/// Verify that a request was never made
async fn handle_never(Json(request): Json<VerificationRequest>) -> impl IntoResponse {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(VerificationResult::failure(
                    0,
                    VerificationCount::Never,
                    Vec::new(),
                    "Request logger not initialized".to_string(),
                )),
            )
                .into_response();
        }
    };

    let result = verify_never(logger, &request).await;

    let status = if result.matched {
        StatusCode::OK
    } else {
        StatusCode::EXPECTATION_FAILED
    };

    (status, Json(result)).into_response()
}

/// Verify that a request was made at least N times
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtLeastRequest {
    /// Pattern to match requests
    pub pattern: VerificationRequest,
    /// Minimum count
    pub min: usize,
}

async fn handle_at_least(Json(request): Json<AtLeastRequest>) -> impl IntoResponse {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(VerificationResult::failure(
                    0,
                    VerificationCount::AtLeast(request.min),
                    Vec::new(),
                    "Request logger not initialized".to_string(),
                )),
            )
                .into_response();
        }
    };

    let result = verify_at_least(logger, &request.pattern, request.min).await;

    let status = if result.matched {
        StatusCode::OK
    } else {
        StatusCode::EXPECTATION_FAILED
    };

    (status, Json(result)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_verification_router_creation() {
        let router = verification_router();
        // Router should be created without panicking
        assert!(true);
    }
}
