//! Verification API for MockForge SDK
//!
//! Provides methods to verify that specific requests were made (or not made)
//! during test execution.

use crate::Error;
use mockforge_core::{
    request_logger::get_global_logger,
    verification::{
        verify_at_least, verify_never, verify_requests, verify_sequence, VerificationCount,
        VerificationRequest, VerificationResult,
    },
};

/// Extension trait for verification methods on MockServer
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
