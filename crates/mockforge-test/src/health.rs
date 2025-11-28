//! Health check utilities for MockForge servers

use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{interval, timeout};
use tracing::{debug, trace};

/// Health status response from MockForge server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Status of the server ("healthy" or "unhealthy: <reason>")
    pub status: String,

    /// Timestamp of the health check
    pub timestamp: String,

    /// Server uptime in seconds
    pub uptime_seconds: u64,

    /// Server version
    pub version: String,
}

impl HealthStatus {
    /// Check if the server is healthy
    pub fn is_healthy(&self) -> bool {
        self.status == "healthy"
    }
}

/// Health check client for MockForge servers
pub struct HealthCheck {
    client: Client,
    base_url: String,
}

impl HealthCheck {
    /// Create a new health check client
    ///
    /// # Arguments
    ///
    /// * `host` - Server host (e.g., "localhost")
    /// * `port` - Server port
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: format!("http://{}:{}", host, port),
        }
    }

    /// Perform a single health check
    pub async fn check(&self) -> Result<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        trace!("Checking health at: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::HealthCheckFailed(format!(
                "HTTP {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let status: HealthStatus = response.json().await?;
        debug!("Health check response: {:?}", status);

        Ok(status)
    }

    /// Wait for the server to become healthy
    ///
    /// # Arguments
    ///
    /// * `timeout_duration` - Maximum time to wait
    /// * `check_interval` - Interval between health checks
    pub async fn wait_until_healthy(
        &self,
        timeout_duration: Duration,
        check_interval: Duration,
    ) -> Result<HealthStatus> {
        debug!(
            "Waiting for server to become healthy (timeout: {:?}, interval: {:?})",
            timeout_duration, check_interval
        );

        let check_fut = async {
            let mut check_timer = interval(check_interval);

            loop {
                check_timer.tick().await;

                match self.check().await {
                    Ok(status) => {
                        if status.is_healthy() {
                            debug!("Server is healthy!");
                            return Ok(status);
                        }
                        trace!("Server not healthy yet: {}", status.status);
                    }
                    Err(e) => {
                        trace!("Health check failed: {}", e);
                    }
                }
            }
        };

        timeout(timeout_duration, check_fut)
            .await
            .map_err(|_| Error::HealthCheckTimeout(timeout_duration.as_secs()))?
    }

    /// Check if the server is ready (health endpoint returns 200)
    pub async fn is_ready(&self) -> bool {
        self.check().await.is_ok()
    }

    /// Get the base URL of the server
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_is_healthy() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 10,
            version: "0.1.0".to_string(),
        };

        assert!(status.is_healthy());
    }

    #[test]
    fn test_health_status_is_not_healthy() {
        let status = HealthStatus {
            status: "unhealthy: database connection failed".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 10,
            version: "0.1.0".to_string(),
        };

        assert!(!status.is_healthy());
    }

    #[test]
    fn test_health_check_creation() {
        let health = HealthCheck::new("localhost", 3000);
        assert_eq!(health.base_url(), "http://localhost:3000");
    }
}
