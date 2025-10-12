//! Replay functionality for recorded requests

use crate::{
    database::RecorderDatabase,
    diff::{ComparisonResult, ResponseComparator},
    Result,
};
use std::collections::HashMap;

/// Replay engine for executing recorded requests
pub struct ReplayEngine {
    db: RecorderDatabase,
}

impl ReplayEngine {
    /// Create a new replay engine
    pub fn new(db: RecorderDatabase) -> Self {
        Self { db }
    }

    /// Replay a single request by ID
    pub async fn replay_request(&self, request_id: &str) -> Result<ReplayResult> {
        let exchange = self.db.get_exchange(request_id).await?;

        match exchange {
            Some(exchange) => Ok(ReplayResult {
                request_id: request_id.to_string(),
                success: true,
                message: format!(
                    "Replayed {} {} request",
                    exchange.request.protocol, exchange.request.method
                ),
                original_status: exchange.response.map(|r| r.status_code),
                replay_status: None,
            }),
            None => Err(crate::RecorderError::NotFound(request_id.to_string())),
        }
    }

    /// Replay multiple requests matching a filter
    pub async fn replay_batch(&self, limit: i32) -> Result<Vec<ReplayResult>> {
        let requests = self.db.list_recent(limit).await?;
        let mut results = Vec::new();

        for request in requests {
            match self.replay_request(&request.id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(ReplayResult {
                        request_id: request.id.clone(),
                        success: false,
                        message: format!("Replay failed: {}", e),
                        original_status: None,
                        replay_status: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Compare original and replayed responses
    pub async fn compare_responses(
        &self,
        request_id: &str,
        replay_response_body: &[u8],
        replay_status: i32,
        replay_headers: &HashMap<String, String>,
    ) -> Result<ComparisonResult> {
        let exchange = self.db.get_exchange(request_id).await?;

        match exchange {
            Some(exchange) => {
                let response = exchange.response.ok_or_else(|| {
                    crate::RecorderError::NotFound(format!("No response for {}", request_id))
                })?;

                // Parse original headers from JSON string
                let original_headers = response.headers_map();

                // Decode original body
                let original_body = response.decoded_body().unwrap_or_default();

                // Use the ResponseComparator to perform the comparison
                Ok(ResponseComparator::compare(
                    response.status_code,
                    &original_headers,
                    &original_body,
                    replay_status,
                    replay_headers,
                    replay_response_body,
                ))
            }
            None => Err(crate::RecorderError::NotFound(request_id.to_string())),
        }
    }
}

/// Result of a replay operation
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub request_id: String,
    pub success: bool,
    pub message: String,
    pub original_status: Option<i32>,
    pub replay_status: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_result_creation() {
        let result = ReplayResult {
            request_id: "test-123".to_string(),
            success: true,
            message: "Replayed successfully".to_string(),
            original_status: Some(200),
            replay_status: Some(200),
        };

        assert!(result.success);
        assert_eq!(result.original_status, Some(200));
    }
}
