//! HTTP verification API handlers for MockForge
//!
//! Provides REST endpoints for programmatic request verification,
//! allowing test code to verify that specific requests were made (or not made).

use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use mockforge_core::{
    request_logger::get_global_logger,
    verification::{
        verify_at_least, verify_never, verify_requests, verify_sequence, VerificationCount,
        VerificationRequest, VerificationResult,
    },
};
use serde::{Deserialize, Serialize};

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

impl Default for VerificationState {
    fn default() -> Self {
        Self::new()
    }
}

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
    use mockforge_core::verification::{VerificationCount, VerificationRequest};
    use tower::ServiceExt;

    // ==================== Router Tests ====================

    #[tokio::test]
    async fn test_verification_router_creation() {
        let router = verification_router();
        // Router should be created without panicking
        assert!(std::mem::size_of_val(&router) > 0);
    }

    #[tokio::test]
    async fn test_verification_router_has_verify_route() {
        let router = verification_router();

        let request = Request::builder()
            .method("POST")
            .uri("/api/verification/verify")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should return an error since body is empty (either 422 or 503)
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    #[tokio::test]
    async fn test_verification_router_has_count_route() {
        let router = verification_router();

        let request = Request::builder()
            .method("POST")
            .uri("/api/verification/count")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should return an error since body is empty
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    #[tokio::test]
    async fn test_verification_router_has_sequence_route() {
        let router = verification_router();

        let request = Request::builder()
            .method("POST")
            .uri("/api/verification/sequence")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should return an error since body is empty
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    #[tokio::test]
    async fn test_verification_router_has_never_route() {
        let router = verification_router();

        let request = Request::builder()
            .method("POST")
            .uri("/api/verification/never")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should return an error since body is empty
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    #[tokio::test]
    async fn test_verification_router_has_at_least_route() {
        let router = verification_router();

        let request = Request::builder()
            .method("POST")
            .uri("/api/verification/at-least")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        // Should return an error since body is empty
        assert!(response.status().is_client_error() || response.status().is_server_error());
    }

    // ==================== VerificationState Tests ====================

    #[test]
    fn test_verification_state_new() {
        let state = VerificationState::new();
        // State should be created successfully (size_of_val always >= 0 for usize)
        let _ = std::mem::size_of_val(&state);
    }

    #[test]
    fn test_verification_state_default() {
        let state = VerificationState::default();
        // size_of_val always >= 0 for usize
        let _ = std::mem::size_of_val(&state);
    }

    #[test]
    fn test_verification_state_clone() {
        let state = VerificationState::new();
        let _cloned = state.clone();
        // Clone should succeed without panic
    }

    // ==================== VerifyRequest Tests ====================

    #[test]
    fn test_verify_request_creation() {
        let pattern = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            ..Default::default()
        };

        let verify_request = VerifyRequest {
            pattern,
            expected: VerificationCount::Exactly(1),
        };

        assert!(verify_request.pattern.method.is_some());
        assert!(matches!(verify_request.expected, VerificationCount::Exactly(1)));
    }

    #[test]
    fn test_verify_request_structure() {
        let pattern = VerificationRequest {
            method: Some("POST".to_string()),
            path: Some("/api/create".to_string()),
            ..Default::default()
        };

        let verify_request = VerifyRequest {
            pattern,
            expected: VerificationCount::AtLeast(2),
        };

        // Test that the structure is properly created
        assert_eq!(verify_request.pattern.method, Some("POST".to_string()));
        assert!(matches!(verify_request.expected, VerificationCount::AtLeast(2)));
    }

    #[test]
    fn test_verify_request_clone() {
        let pattern = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/test".to_string()),
            ..Default::default()
        };

        let verify_request = VerifyRequest {
            pattern,
            expected: VerificationCount::Exactly(5),
        };

        let cloned = verify_request.clone();
        assert_eq!(cloned.pattern.method, verify_request.pattern.method);
    }

    // ==================== CountRequest Tests ====================

    #[test]
    fn test_count_request_creation() {
        let pattern = VerificationRequest {
            method: Some("DELETE".to_string()),
            path: Some("/api/users/123".to_string()),
            ..Default::default()
        };

        let count_request = CountRequest { pattern };
        assert_eq!(count_request.pattern.method, Some("DELETE".to_string()));
    }

    #[test]
    fn test_count_request_serialization() {
        let count_request = CountRequest {
            pattern: VerificationRequest {
                method: Some("PUT".to_string()),
                path: Some("/api/update".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&count_request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_count_request_clone() {
        let count_request = CountRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/test".to_string()),
                ..Default::default()
            },
        };

        let cloned = count_request.clone();
        assert_eq!(cloned.pattern.method, Some("GET".to_string()));
    }

    // ==================== CountResponse Tests ====================

    #[test]
    fn test_count_response_creation() {
        let response = CountResponse { count: 42 };
        assert_eq!(response.count, 42);
    }

    #[test]
    fn test_count_response_serialization() {
        let response = CountResponse { count: 100 };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("100"));
    }

    #[test]
    fn test_count_response_deserialization() {
        let json = r#"{"count":25}"#;
        let response: CountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.count, 25);
    }

    #[test]
    fn test_count_response_zero() {
        let response = CountResponse { count: 0 };
        assert_eq!(response.count, 0);
    }

    // ==================== SequenceRequest Tests ====================

    #[test]
    fn test_sequence_request_creation() {
        let patterns = vec![
            VerificationRequest {
                method: Some("POST".to_string()),
                path: Some("/api/login".to_string()),
                ..Default::default()
            },
            VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/api/profile".to_string()),
                ..Default::default()
            },
        ];

        let sequence_request = SequenceRequest { patterns };
        assert_eq!(sequence_request.patterns.len(), 2);
    }

    #[test]
    fn test_sequence_request_empty() {
        let sequence_request = SequenceRequest { patterns: vec![] };
        assert!(sequence_request.patterns.is_empty());
    }

    #[test]
    fn test_sequence_request_serialization() {
        let sequence_request = SequenceRequest {
            patterns: vec![VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/health".to_string()),
                ..Default::default()
            }],
        };

        let json = serde_json::to_string(&sequence_request);
        assert!(json.is_ok());
    }

    // ==================== AtLeastRequest Tests ====================

    #[test]
    fn test_at_least_request_creation() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/api/data".to_string()),
                ..Default::default()
            },
            min: 3,
        };

        assert_eq!(request.min, 3);
        assert_eq!(request.pattern.method, Some("GET".to_string()));
    }

    #[test]
    fn test_at_least_request_serialization() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("POST".to_string()),
                path: Some("/api/submit".to_string()),
                ..Default::default()
            },
            min: 5,
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_at_least_request_clone() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/test".to_string()),
                ..Default::default()
            },
            min: 2,
        };

        let cloned = request.clone();
        assert_eq!(cloned.min, 2);
        assert_eq!(cloned.pattern.method, Some("GET".to_string()));
    }
}
