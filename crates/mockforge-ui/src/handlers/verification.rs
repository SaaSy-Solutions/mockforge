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
    Json(request): Json<VerifyRequest>,
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
    Json(request): Json<CountRequest>,
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
    Json(request): Json<SequenceRequest>,
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
    Json(pattern): Json<VerificationRequest>,
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
    Json(request): Json<AtLeastRequest>,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== VerifyRequest Tests ====================

    #[test]
    fn test_verify_request_creation() {
        let pattern = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            ..Default::default()
        };

        let request = VerifyRequest {
            pattern,
            expected: VerificationCount::Exactly(1),
        };

        assert!(request.pattern.method.is_some());
        assert_eq!(request.pattern.method.unwrap(), "GET");
    }

    #[test]
    fn test_verify_request_structure() {
        let request = VerifyRequest {
            pattern: VerificationRequest {
                method: Some("POST".to_string()),
                path: Some("/api/orders".to_string()),
                ..Default::default()
            },
            expected: VerificationCount::AtLeast(2),
        };

        // Verify structure without serialization (avoids tagged enum issues)
        assert_eq!(request.pattern.method, Some("POST".to_string()));
        assert_eq!(request.pattern.path, Some("/api/orders".to_string()));
    }

    #[test]
    fn test_verify_request_pattern_fields() {
        let pattern = VerificationRequest {
            method: Some("DELETE".to_string()),
            path: Some("/api/items/123".to_string()),
            ..Default::default()
        };

        let request = VerifyRequest {
            pattern,
            expected: VerificationCount::Exactly(1),
        };

        assert_eq!(request.pattern.method, Some("DELETE".to_string()));
        assert_eq!(request.pattern.path, Some("/api/items/123".to_string()));
    }

    #[test]
    fn test_verify_request_clone() {
        let request = VerifyRequest {
            pattern: VerificationRequest {
                method: Some("PUT".to_string()),
                path: None,
                ..Default::default()
            },
            expected: VerificationCount::Never,
        };

        let cloned = request.clone();
        assert_eq!(cloned.pattern.method, request.pattern.method);
    }

    // ==================== CountRequest Tests ====================

    #[test]
    fn test_count_request_creation() {
        let request = CountRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/health".to_string()),
                ..Default::default()
            },
        };

        assert_eq!(request.pattern.method, Some("GET".to_string()));
    }

    #[test]
    fn test_count_request_serialization() {
        let request = CountRequest {
            pattern: VerificationRequest {
                method: Some("POST".to_string()),
                path: Some("/api/data".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("POST"));
        assert!(json.contains("/api/data"));
    }

    #[test]
    fn test_count_request_clone() {
        let request = CountRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                ..Default::default()
            },
        };

        let cloned = request.clone();
        assert_eq!(cloned.pattern.method, request.pattern.method);
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
        let json = r#"{"count": 25}"#;
        let response: CountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.count, 25);
    }

    #[test]
    fn test_count_response_clone() {
        let response = CountResponse { count: 10 };
        let cloned = response.clone();
        assert_eq!(cloned.count, response.count);
    }

    #[test]
    fn test_count_response_zero() {
        let response = CountResponse { count: 0 };
        assert_eq!(response.count, 0);
    }

    // ==================== SequenceRequest Tests ====================

    #[test]
    fn test_sequence_request_creation() {
        let request = SequenceRequest {
            patterns: vec![
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
            ],
        };

        assert_eq!(request.patterns.len(), 2);
    }

    #[test]
    fn test_sequence_request_empty() {
        let request = SequenceRequest { patterns: vec![] };
        assert!(request.patterns.is_empty());
    }

    #[test]
    fn test_sequence_request_serialization() {
        let request = SequenceRequest {
            patterns: vec![VerificationRequest {
                method: Some("GET".to_string()),
                ..Default::default()
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("GET"));
    }

    #[test]
    fn test_sequence_request_clone() {
        let request = SequenceRequest {
            patterns: vec![VerificationRequest {
                method: Some("POST".to_string()),
                ..Default::default()
            }],
        };

        let cloned = request.clone();
        assert_eq!(cloned.patterns.len(), request.patterns.len());
    }

    // ==================== AtLeastRequest Tests ====================

    #[test]
    fn test_at_least_request_creation() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("GET".to_string()),
                path: Some("/api/users".to_string()),
                ..Default::default()
            },
            min: 5,
        };

        assert_eq!(request.min, 5);
    }

    #[test]
    fn test_at_least_request_serialization() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("POST".to_string()),
                ..Default::default()
            },
            min: 10,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("10"));
    }

    #[test]
    fn test_at_least_request_pattern() {
        let request = AtLeastRequest {
            pattern: VerificationRequest {
                method: Some("DELETE".to_string()),
                ..Default::default()
            },
            min: 3,
        };

        assert_eq!(request.min, 3);
        assert_eq!(request.pattern.method, Some("DELETE".to_string()));
    }

    #[test]
    fn test_at_least_request_clone() {
        let request = AtLeastRequest {
            pattern: VerificationRequest::default(),
            min: 1,
        };

        let cloned = request.clone();
        assert_eq!(cloned.min, request.min);
    }

    #[test]
    fn test_at_least_request_zero_min() {
        let request = AtLeastRequest {
            pattern: VerificationRequest::default(),
            min: 0,
        };

        assert_eq!(request.min, 0);
    }

    // ==================== Debug Tests ====================

    #[test]
    fn test_verify_request_debug() {
        let request = VerifyRequest {
            pattern: VerificationRequest::default(),
            expected: VerificationCount::Exactly(1),
        };

        let debug = format!("{:?}", request);
        assert!(debug.contains("VerifyRequest"));
    }

    #[test]
    fn test_count_request_debug() {
        let request = CountRequest {
            pattern: VerificationRequest::default(),
        };

        let debug = format!("{:?}", request);
        assert!(debug.contains("CountRequest"));
    }

    #[test]
    fn test_count_response_debug() {
        let response = CountResponse { count: 5 };
        let debug = format!("{:?}", response);
        assert!(debug.contains("5"));
    }

    #[test]
    fn test_sequence_request_debug() {
        let request = SequenceRequest { patterns: vec![] };
        let debug = format!("{:?}", request);
        assert!(debug.contains("SequenceRequest"));
    }

    #[test]
    fn test_at_least_request_debug() {
        let request = AtLeastRequest {
            pattern: VerificationRequest::default(),
            min: 2,
        };

        let debug = format!("{:?}", request);
        assert!(debug.contains("AtLeastRequest"));
    }
}
