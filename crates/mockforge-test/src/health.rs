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

    fn create_healthy_status() -> HealthStatus {
        HealthStatus {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 10,
            version: "0.1.0".to_string(),
        }
    }

    fn create_unhealthy_status(reason: &str) -> HealthStatus {
        HealthStatus {
            status: format!("unhealthy: {}", reason),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 10,
            version: "0.1.0".to_string(),
        }
    }

    // HealthStatus tests
    #[test]
    fn test_health_status_is_healthy() {
        let status = create_healthy_status();
        assert!(status.is_healthy());
    }

    #[test]
    fn test_health_status_is_not_healthy() {
        let status = create_unhealthy_status("database connection failed");
        assert!(!status.is_healthy());
    }

    #[test]
    fn test_health_status_empty_status_not_healthy() {
        let status = HealthStatus {
            status: "".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 0,
            version: "0.1.0".to_string(),
        };
        assert!(!status.is_healthy());
    }

    #[test]
    fn test_health_status_clone() {
        let status = create_healthy_status();
        let cloned = status.clone();
        assert_eq!(status.status, cloned.status);
        assert_eq!(status.uptime_seconds, cloned.uptime_seconds);
        assert_eq!(status.version, cloned.version);
    }

    #[test]
    fn test_health_status_debug() {
        let status = create_healthy_status();
        let debug = format!("{:?}", status);
        assert!(debug.contains("HealthStatus"));
        assert!(debug.contains("healthy"));
    }

    #[test]
    fn test_health_status_serialize() {
        let status = create_healthy_status();
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"uptime_seconds\":10"));
        assert!(json.contains("\"version\":\"0.1.0\""));
    }

    #[test]
    fn test_health_status_deserialize() {
        let json = r#"{
            "status": "healthy",
            "timestamp": "2025-01-01T12:00:00Z",
            "uptime_seconds": 3600,
            "version": "1.0.0"
        }"#;

        let status: HealthStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.status, "healthy");
        assert_eq!(status.uptime_seconds, 3600);
        assert_eq!(status.version, "1.0.0");
        assert!(status.is_healthy());
    }

    #[test]
    fn test_health_status_with_long_uptime() {
        let status = HealthStatus {
            status: "healthy".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            uptime_seconds: 86400 * 365, // 1 year
            version: "0.1.0".to_string(),
        };
        assert!(status.is_healthy());
        assert_eq!(status.uptime_seconds, 31_536_000);
    }

    // HealthCheck tests
    #[test]
    fn test_health_check_creation() {
        let health = HealthCheck::new("localhost", 3000);
        assert_eq!(health.base_url(), "http://localhost:3000");
    }

    #[test]
    fn test_health_check_creation_different_host() {
        let health = HealthCheck::new("192.168.1.100", 8080);
        assert_eq!(health.base_url(), "http://192.168.1.100:8080");
    }

    #[test]
    fn test_health_check_creation_with_hostname() {
        let health = HealthCheck::new("api.example.com", 443);
        assert_eq!(health.base_url(), "http://api.example.com:443");
    }

    #[test]
    fn test_health_check_base_url_method() {
        let health = HealthCheck::new("test-server", 9000);
        let url = health.base_url();
        assert_eq!(url, "http://test-server:9000");
    }
}
