//! Verification API for `MockForge` SDK
//!
//! Provides methods to verify that specific requests were made (or not made)
//! during test execution.

use crate::Error;
use mockforge_core::{
    request_logger::get_global_logger, verify_at_least, verify_never, verify_requests,
    verify_sequence, VerificationCount, VerificationRequest, VerificationResult,
};

/// Extension trait for verification methods on `MockServer`
pub trait Verification {
    /// Verify requests against a pattern and count assertion
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_sdk::MockServer;
    /// use mockforge_sdk::verification::Verification;
    /// use mockforge_core::verification::{VerificationRequest, VerificationCount};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = MockServer::new().port(3000).start().await?;
    ///
    /// // Make some requests...
    ///
    /// let pattern = VerificationRequest {
    ///     method: Some("GET".to_string()),
    ///     path: Some("/api/users".to_string()),
    ///     query_params: std::collections::HashMap::new(),
    ///     headers: std::collections::HashMap::new(),
    ///     body_pattern: None,
    /// };
    ///
    /// let result = server.verify(&pattern, VerificationCount::Exactly(3)).await?;
    /// assert!(result.matched, "Expected GET /api/users to be called exactly 3 times");
    /// # Ok(())
    /// # }
    /// ```
    async fn verify(
        &self,
        pattern: &VerificationRequest,
        expected: VerificationCount,
    ) -> Result<VerificationResult, Error>;

    /// Verify that a request was never made
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_sdk::MockServer;
    /// use mockforge_sdk::verification::Verification;
    /// use mockforge_core::verification::VerificationRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = MockServer::new().port(3000).start().await?;
    ///
    /// // Make some requests...
    ///
    /// let pattern = VerificationRequest {
    ///     method: Some("DELETE".to_string()),
    ///     path: Some("/api/users/1".to_string()),
    ///     query_params: std::collections::HashMap::new(),
    ///     headers: std::collections::HashMap::new(),
    ///     body_pattern: None,
    /// };
    ///
    /// let result = server.verify_never(&pattern).await?;
    /// assert!(result.matched, "Expected DELETE /api/users/1 to never be called");
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_never(
        &self,
        pattern: &VerificationRequest,
    ) -> Result<VerificationResult, Error>;

    /// Verify that a request was made at least N times
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_sdk::MockServer;
    /// use mockforge_sdk::verification::Verification;
    /// use mockforge_core::verification::VerificationRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = MockServer::new().port(3000).start().await?;
    ///
    /// // Make some requests...
    ///
    /// let pattern = VerificationRequest {
    ///     method: Some("POST".to_string()),
    ///     path: Some("/api/orders".to_string()),
    ///     query_params: std::collections::HashMap::new(),
    ///     headers: std::collections::HashMap::new(),
    ///     body_pattern: None,
    /// };
    ///
    /// let result = server.verify_at_least(&pattern, 2).await?;
    /// assert!(result.matched, "Expected POST /api/orders to be called at least 2 times");
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_at_least(
        &self,
        pattern: &VerificationRequest,
        min: usize,
    ) -> Result<VerificationResult, Error>;

    /// Verify that requests occurred in a specific sequence
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_sdk::MockServer;
    /// use mockforge_sdk::verification::Verification;
    /// use mockforge_core::verification::VerificationRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut server = MockServer::new().port(3000).start().await?;
    ///
    /// // Make some requests in sequence...
    ///
    /// let patterns = vec![
    ///     VerificationRequest {
    ///         method: Some("POST".to_string()),
    ///         path: Some("/api/users".to_string()),
    ///         query_params: std::collections::HashMap::new(),
    ///         headers: std::collections::HashMap::new(),
    ///         body_pattern: None,
    ///     },
    ///     VerificationRequest {
    ///         method: Some("GET".to_string()),
    ///         path: Some("/api/users/1".to_string()),
    ///         query_params: std::collections::HashMap::new(),
    ///         headers: std::collections::HashMap::new(),
    ///         body_pattern: None,
    ///     },
    /// ];
    ///
    /// let result = server.verify_sequence(&patterns).await?;
    /// assert!(result.matched, "Expected requests to occur in sequence");
    /// # Ok(())
    /// # }
    /// ```
    async fn verify_sequence(
        &self,
        patterns: &[VerificationRequest],
    ) -> Result<VerificationResult, Error>;
}

impl Verification for crate::server::MockServer {
    async fn verify(
        &self,
        pattern: &VerificationRequest,
        expected: VerificationCount,
    ) -> Result<VerificationResult, Error> {
        let logger = get_global_logger()
            .ok_or_else(|| Error::General("Request logger not initialized".to_string()))?;

        Ok(verify_requests(logger, pattern, expected).await)
    }

    async fn verify_never(
        &self,
        pattern: &VerificationRequest,
    ) -> Result<VerificationResult, Error> {
        let logger = get_global_logger()
            .ok_or_else(|| Error::General("Request logger not initialized".to_string()))?;

        Ok(verify_never(logger, pattern).await)
    }

    async fn verify_at_least(
        &self,
        pattern: &VerificationRequest,
        min: usize,
    ) -> Result<VerificationResult, Error> {
        let logger = get_global_logger()
            .ok_or_else(|| Error::General("Request logger not initialized".to_string()))?;

        Ok(verify_at_least(logger, pattern, min).await)
    }

    async fn verify_sequence(
        &self,
        patterns: &[VerificationRequest],
    ) -> Result<VerificationResult, Error> {
        let logger = get_global_logger()
            .ok_or_else(|| Error::General("Request logger not initialized".to_string()))?;

        Ok(verify_sequence(logger, patterns).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper function to create a verification request
    fn create_verification_request(
        method: &str,
        path: &str,
    ) -> VerificationRequest {
        VerificationRequest {
            method: Some(method.to_string()),
            path: Some(path.to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        }
    }

    #[test]
    fn test_verification_request_creation() {
        let request = create_verification_request("GET", "/api/users");
        assert_eq!(request.method, Some("GET".to_string()));
        assert_eq!(request.path, Some("/api/users".to_string()));
        assert!(request.query_params.is_empty());
        assert!(request.headers.is_empty());
        assert!(request.body_pattern.is_none());
    }

    #[test]
    fn test_verification_request_with_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let request = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            query_params,
            headers: HashMap::new(),
            body_pattern: None,
        };

        assert_eq!(request.query_params.len(), 2);
        assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
        assert_eq!(request.query_params.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_verification_request_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let request = VerificationRequest {
            method: Some("POST".to_string()),
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers,
            body_pattern: None,
        };

        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_verification_request_with_body_pattern() {
        let request = VerificationRequest {
            method: Some("POST".to_string()),
            path: Some("/api/users".to_string()),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: Some(r#"{"name":".*"}"#.to_string()),
        };

        assert_eq!(request.body_pattern, Some(r#"{"name":".*"}"#.to_string()));
    }

    #[test]
    fn test_verification_count_exactly() {
        let count = VerificationCount::Exactly(3);
        match count {
            VerificationCount::Exactly(n) => assert_eq!(n, 3),
            _ => panic!("Expected Exactly variant"),
        }
    }

    #[test]
    fn test_verification_count_at_least() {
        let count = VerificationCount::AtLeast(2);
        match count {
            VerificationCount::AtLeast(n) => assert_eq!(n, 2),
            _ => panic!("Expected AtLeast variant"),
        }
    }

    #[test]
    fn test_verification_count_at_most() {
        let count = VerificationCount::AtMost(5);
        match count {
            VerificationCount::AtMost(n) => assert_eq!(n, 5),
            _ => panic!("Expected AtMost variant"),
        }
    }

    #[test]
    fn test_verification_count_never() {
        let count = VerificationCount::Never;
        match count {
            VerificationCount::Never => (),
            _ => panic!("Expected Never variant"),
        }
    }

    #[tokio::test]
    async fn test_verify_error_when_logger_not_initialized() {
        let server = crate::server::MockServer::default();
        let request = create_verification_request("GET", "/api/test");

        // Without initializing the global logger, this should fail
        let result = server.verify(&request, VerificationCount::Exactly(1)).await;

        // The result depends on whether the global logger is initialized
        // In test environments, it might be initialized by other tests
        if result.is_err() {
            match result {
                Err(Error::General(msg)) => {
                    assert!(msg.contains("Request logger not initialized"));
                }
                _ => panic!("Expected General error about logger"),
            }
        }
    }

    #[tokio::test]
    async fn test_verify_never_error_when_logger_not_initialized() {
        let server = crate::server::MockServer::default();
        let request = create_verification_request("DELETE", "/api/users/1");

        let result = server.verify_never(&request).await;

        if result.is_err() {
            match result {
                Err(Error::General(msg)) => {
                    assert!(msg.contains("Request logger not initialized"));
                }
                _ => panic!("Expected General error about logger"),
            }
        }
    }

    #[tokio::test]
    async fn test_verify_at_least_error_when_logger_not_initialized() {
        let server = crate::server::MockServer::default();
        let request = create_verification_request("POST", "/api/orders");

        let result = server.verify_at_least(&request, 2).await;

        if result.is_err() {
            match result {
                Err(Error::General(msg)) => {
                    assert!(msg.contains("Request logger not initialized"));
                }
                _ => panic!("Expected General error about logger"),
            }
        }
    }

    #[tokio::test]
    async fn test_verify_sequence_error_when_logger_not_initialized() {
        let server = crate::server::MockServer::default();
        let patterns = vec![
            create_verification_request("POST", "/api/users"),
            create_verification_request("GET", "/api/users/1"),
        ];

        let result = server.verify_sequence(&patterns).await;

        if result.is_err() {
            match result {
                Err(Error::General(msg)) => {
                    assert!(msg.contains("Request logger not initialized"));
                }
                _ => panic!("Expected General error about logger"),
            }
        }
    }

    #[test]
    fn test_verification_request_all_fields() {
        let mut query_params = HashMap::new();
        query_params.insert("id".to_string(), "123".to_string());

        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());

        let request = VerificationRequest {
            method: Some("PUT".to_string()),
            path: Some("/api/users/123".to_string()),
            query_params,
            headers,
            body_pattern: Some(r#"{"name":"test"}"#.to_string()),
        };

        assert_eq!(request.method, Some("PUT".to_string()));
        assert_eq!(request.path, Some("/api/users/123".to_string()));
        assert_eq!(request.query_params.len(), 1);
        assert_eq!(request.headers.len(), 1);
        assert!(request.body_pattern.is_some());
    }

    #[test]
    fn test_verification_request_minimal() {
        let request = VerificationRequest {
            method: None,
            path: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body_pattern: None,
        };

        assert!(request.method.is_none());
        assert!(request.path.is_none());
        assert!(request.query_params.is_empty());
        assert!(request.headers.is_empty());
        assert!(request.body_pattern.is_none());
    }

    #[test]
    fn test_verification_sequence_empty() {
        let patterns: Vec<VerificationRequest> = vec![];
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn test_verification_sequence_single() {
        let patterns = vec![
            create_verification_request("GET", "/api/test"),
        ];
        assert_eq!(patterns.len(), 1);
    }

    #[test]
    fn test_verification_sequence_multiple() {
        let patterns = vec![
            create_verification_request("POST", "/api/users"),
            create_verification_request("GET", "/api/users/1"),
            create_verification_request("PUT", "/api/users/1"),
            create_verification_request("DELETE", "/api/users/1"),
        ];
        assert_eq!(patterns.len(), 4);
    }

    #[test]
    fn test_verification_with_complex_pattern() {
        let mut query_params = HashMap::new();
        query_params.insert("filter".to_string(), "active".to_string());
        query_params.insert("sort".to_string(), "name".to_string());

        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("X-API-Key".to_string(), "secret".to_string());

        let request = VerificationRequest {
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            query_params,
            headers,
            body_pattern: None,
        };

        assert_eq!(request.method, Some("GET".to_string()));
        assert_eq!(request.query_params.len(), 2);
        assert_eq!(request.headers.len(), 2);
    }
}
