//! Verification API handlers for Admin UI
//!
//! Provides endpoints for request verification that can be used by the Admin UI
//! to verify that specific requests were made (or not made).

use axum::extract::State;
use axum::response::Json;
use mockforge_core::{
    request_logger::get_global_logger,
    verification::{
        verify_at_least, verify_never, verify_requests, verify_sequence, VerificationCount,
        VerificationRequest, VerificationResult,
    },
};
use serde::{Deserialize, Serialize};

use crate::handlers::AdminState;
use crate::models::ApiResponse;

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

/// Request body for at-least verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtLeastRequest {
    /// Pattern to match requests
    pub pattern: VerificationRequest,
    /// Minimum count
    pub min: usize,
}

/// Verify requests against a pattern and count assertion
pub async fn verify(
    State(_state): State<AdminState>,
    axum::extract::Json(request): axum::extract::Json<VerifyRequest>,
) -> Json<ApiResponse<VerificationResult>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    let result = verify_requests(logger, &request.pattern, request.expected).await;

    if result.matched {
        Json(ApiResponse::success(result))
    } else {
        Json(ApiResponse::error(result.error_message.unwrap_or_else(|| {
            format!(
                "Verification failed: expected {:?}, but found {} matching requests",
                result.expected, result.count
            )
        })))
    }
}

/// Get count of matching requests
pub async fn count(
    State(_state): State<AdminState>,
    axum::extract::Json(request): axum::extract::Json<CountRequest>,
) -> Json<ApiResponse<CountResponse>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    let count = logger.count_matching_requests(&request.pattern).await;

    Json(ApiResponse::success(CountResponse { count }))
}

/// Verify that requests occurred in a specific sequence
pub async fn verify_sequence_handler(
    State(_state): State<AdminState>,
    axum::extract::Json(request): axum::extract::Json<SequenceRequest>,
) -> Json<ApiResponse<VerificationResult>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    let result = verify_sequence(logger, &request.patterns).await;

    if result.matched {
        Json(ApiResponse::success(result))
    } else {
        Json(ApiResponse::error(
            result
                .error_message
                .unwrap_or_else(|| "Sequence verification failed".to_string()),
        ))
    }
}

/// Verify that a request was never made
pub async fn verify_never_handler(
    State(_state): State<AdminState>,
    axum::extract::Json(pattern): axum::extract::Json<VerificationRequest>,
) -> Json<ApiResponse<VerificationResult>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    let result = verify_never(logger, &pattern).await;

    if result.matched {
        Json(ApiResponse::success(result))
    } else {
        Json(ApiResponse::error(
            result.error_message.unwrap_or_else(|| {
                format!(
                    "Verification failed: expected request to never occur, but found {} matching requests",
                    result.count
                )
            }),
        ))
    }
}

/// Verify that a request was made at least N times
pub async fn verify_at_least_handler(
    State(_state): State<AdminState>,
    axum::extract::Json(request): axum::extract::Json<AtLeastRequest>,
) -> Json<ApiResponse<VerificationResult>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    let result = verify_at_least(logger, &request.pattern, request.min).await;

    if result.matched {
        Json(ApiResponse::success(result))
    } else {
        Json(ApiResponse::error(result.error_message.unwrap_or_else(|| {
            format!(
                "Verification failed: expected at least {} requests, but found {}",
                request.min, result.count
            )
        })))
    }
}
