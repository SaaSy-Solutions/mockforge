//! Conformance testing API client
//!
//! Provides programmatic access to `MockForge`'s conformance testing API for
//! starting, monitoring, and retrieving OpenAPI conformance test results.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use
)]

use crate::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Conformance testing API client
pub struct ConformanceClient {
    base_url: String,
    client: Client,
}

/// Request body for starting a conformance run
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConformanceRunRequest {
    /// Target URL to test against
    pub target_url: String,
    /// Inline OpenAPI spec JSON/YAML (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    /// Categories to test (optional filter)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    /// Custom request headers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_headers: Option<Vec<(String, String)>>,
    /// API key for security tests
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Basic auth credentials (user:pass)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<String>,
    /// Skip TLS verification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_tls_verify: Option<bool>,
    /// API base path prefix
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
    /// Test all operations (not just representative samples)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_operations: Option<bool>,
    /// Inline YAML custom checks
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_checks_yaml: Option<String>,
}

/// Conformance run status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Run is queued
    Pending,
    /// Run is in progress
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed
    Failed,
}

/// A conformance test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceRun {
    /// Unique run ID
    pub id: Uuid,
    /// Current status
    pub status: RunStatus,
    /// Configuration used
    pub config: ConformanceRunRequest,
    /// Report (available when completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<serde_json::Value>,
    /// Error message (available when failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Number of checks completed so far
    pub checks_done: usize,
    /// Total number of checks
    pub total_checks: usize,
}

/// Summary of a conformance run (returned by list endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceRunSummary {
    /// Unique run ID
    pub id: Uuid,
    /// Current status
    pub status: RunStatus,
    /// Number of checks completed
    pub checks_done: usize,
    /// Total number of checks
    pub total_checks: usize,
    /// Target URL being tested
    pub target_url: String,
}

impl ConformanceClient {
    /// Create a new conformance client
    ///
    /// The base URL should be the admin API root (e.g., `http://localhost:9080`).
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut url = base_url.into();
        while url.ends_with('/') {
            url.pop();
        }
        Self {
            base_url: url,
            client: Client::new(),
        }
    }

    /// Start a new conformance test run
    ///
    /// Returns the UUID of the newly created run.
    pub async fn run(&self, config: ConformanceRunRequest) -> Result<Uuid> {
        let url = format!("{}/api/conformance/run", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&config)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to start conformance run: {e}")))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to start conformance run: HTTP {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {e}")))?;

        body["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| Error::General("Response missing 'id' field".to_string()))
    }

    /// Get the status and results of a conformance run
    pub async fn get_status(&self, id: Uuid) -> Result<ConformanceRun> {
        let url = format!("{}/api/conformance/run/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to get conformance run: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::General(format!("Conformance run not found: {id}")));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to get conformance run: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {e}")))
    }

    /// Get the report for a completed conformance run
    ///
    /// Returns the report JSON if the run is completed, or an error if not yet done.
    pub async fn get_report(&self, id: Uuid) -> Result<serde_json::Value> {
        let run = self.get_status(id).await?;
        match run.status {
            RunStatus::Completed => run
                .report
                .ok_or_else(|| Error::General("Run completed but no report available".to_string())),
            RunStatus::Failed => Err(Error::General(format!(
                "Conformance run failed: {}",
                run.error.unwrap_or_else(|| "unknown error".to_string())
            ))),
            _ => Err(Error::General(format!(
                "Conformance run not yet completed (status: {:?})",
                run.status
            ))),
        }
    }

    /// List all conformance runs
    pub async fn list_runs(&self) -> Result<Vec<ConformanceRunSummary>> {
        let url = format!("{}/api/conformance/runs", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to list conformance runs: {e}")))?;

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to list conformance runs: HTTP {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::General(format!("Failed to parse response: {e}")))
    }

    /// Delete a completed conformance run
    pub async fn delete_run(&self, id: Uuid) -> Result<()> {
        let url = format!("{}/api/conformance/run/{}", self.base_url, id);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| Error::General(format!("Failed to delete conformance run: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::General(format!("Conformance run not found: {id}")));
        }

        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::General("Cannot delete a running conformance test".to_string()));
        }

        if !response.status().is_success() {
            return Err(Error::General(format!(
                "Failed to delete conformance run: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Wait for a conformance run to complete, polling at the given interval
    ///
    /// Returns the report JSON once the run finishes, or an error if it fails.
    pub async fn wait_for_completion(
        &self,
        id: Uuid,
        poll_interval: Duration,
    ) -> Result<serde_json::Value> {
        loop {
            let run = self.get_status(id).await?;
            match run.status {
                RunStatus::Completed => {
                    return run.report.ok_or_else(|| {
                        Error::General("Run completed but no report available".to_string())
                    });
                }
                RunStatus::Failed => {
                    return Err(Error::General(format!(
                        "Conformance run failed: {}",
                        run.error.unwrap_or_else(|| "unknown error".to_string())
                    )));
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conformance_client_new() {
        let client = ConformanceClient::new("http://localhost:9080");
        assert_eq!(client.base_url, "http://localhost:9080");
    }

    #[test]
    fn test_conformance_client_strips_trailing_slash() {
        let client = ConformanceClient::new("http://localhost:9080/");
        assert_eq!(client.base_url, "http://localhost:9080");
    }

    #[test]
    fn test_conformance_run_request_default() {
        let req = ConformanceRunRequest::default();
        assert!(req.target_url.is_empty());
        assert!(req.spec.is_none());
        assert!(req.categories.is_none());
    }

    #[test]
    fn test_run_status_serialization() {
        let status = RunStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn test_run_status_deserialization() {
        let status: RunStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(status, RunStatus::Running);
    }
}
