//! Scenario and workspace management for tests

use crate::error::{Error, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::{debug, info};

/// Scenario manager for switching test scenarios
pub struct ScenarioManager {
    client: Client,
    base_url: String,
}

impl ScenarioManager {
    /// Create a new scenario manager
    ///
    /// # Arguments
    ///
    /// * `host` - Server host (e.g., "localhost")
    /// * `port` - Server port
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: format!("http://{}:{}", host, port),
        }
    }

    /// Switch to a different scenario/workspace
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario to switch to
    pub async fn switch_scenario(&self, scenario_name: &str) -> Result<()> {
        info!("Switching to scenario: {}", scenario_name);

        let url = format!("{}/__mockforge/workspace/switch", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "workspace": scenario_name
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ScenarioError(format!(
                "Failed to switch scenario: HTTP {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        debug!("Successfully switched to scenario: {}", scenario_name);
        Ok(())
    }

    /// Load a workspace configuration from a file
    ///
    /// # Arguments
    ///
    /// * `workspace_file` - Path to the workspace configuration file (JSON or YAML)
    pub async fn load_workspace<P: AsRef<std::path::Path>>(&self, workspace_file: P) -> Result<()> {
        let path = workspace_file.as_ref();
        info!("Loading workspace from: {}", path.display());

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::WorkspaceError(format!("Failed to read workspace file: {}", e)))?;

        let workspace: Value = if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };

        let url = format!("{}/__mockforge/workspace/load", self.base_url);

        let response = self.client.post(&url).json(&workspace).send().await?;

        if !response.status().is_success() {
            return Err(Error::WorkspaceError(format!(
                "Failed to load workspace: HTTP {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        debug!("Successfully loaded workspace from: {}", path.display());
        Ok(())
    }

    /// Update mock configuration dynamically
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The endpoint path to configure (e.g., "/users")
    /// * `config` - The mock configuration as JSON
    pub async fn update_mock(&self, endpoint: &str, config: Value) -> Result<()> {
        info!("Updating mock for endpoint: {}", endpoint);

        let url = format!("{}/__mockforge/config{}", self.base_url, endpoint);

        let response = self.client.post(&url).json(&config).send().await?;

        if !response.status().is_success() {
            return Err(Error::ScenarioError(format!(
                "Failed to update mock: HTTP {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        debug!("Successfully updated mock for: {}", endpoint);
        Ok(())
    }

    /// List available fixtures
    pub async fn list_fixtures(&self) -> Result<Vec<String>> {
        debug!("Listing available fixtures");

        let url = format!("{}/__mockforge/fixtures", self.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::ScenarioError(format!(
                "Failed to list fixtures: HTTP {}",
                response.status()
            )));
        }

        let fixtures: Vec<String> = response.json().await?;
        debug!("Found {} fixtures", fixtures.len());

        Ok(fixtures)
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> Result<Value> {
        debug!("Fetching server statistics");

        let url = format!("{}/__mockforge/stats", self.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::InvalidResponse(format!(
                "Failed to get stats: HTTP {}",
                response.status()
            )));
        }

        let stats: Value = response.json().await?;
        Ok(stats)
    }

    /// Reset all mocks to their initial state
    pub async fn reset(&self) -> Result<()> {
        info!("Resetting all mocks");

        let url = format!("{}/__mockforge/reset", self.base_url);

        let response = self.client.post(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::ScenarioError(format!(
                "Failed to reset mocks: HTTP {}",
                response.status()
            )));
        }

        debug!("Successfully reset all mocks");
        Ok(())
    }
}

/// Builder for creating scenario configurations
pub struct ScenarioBuilder {
    name: String,
    mocks: Vec<Value>,
}

impl ScenarioBuilder {
    /// Create a new scenario builder
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            mocks: Vec::new(),
        }
    }

    /// Add a mock endpoint
    pub fn mock(mut self, endpoint: &str, response: Value) -> Self {
        self.mocks.push(serde_json::json!({
            "endpoint": endpoint,
            "response": response
        }));
        self
    }

    /// Build the scenario configuration
    pub fn build(self) -> Value {
        serde_json::json!({
            "name": self.name,
            "mocks": self.mocks
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ScenarioBuilder tests
    #[test]
    fn test_scenario_builder() {
        let scenario = ScenarioBuilder::new("test-scenario")
            .mock(
                "/users",
                serde_json::json!({
                    "users": [
                        {"id": 1, "name": "Alice"},
                        {"id": 2, "name": "Bob"}
                    ]
                }),
            )
            .mock(
                "/posts",
                serde_json::json!({
                    "posts": []
                }),
            )
            .build();

        assert_eq!(scenario["name"], "test-scenario");
        assert_eq!(scenario["mocks"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_scenario_builder_new() {
        let builder = ScenarioBuilder::new("my-scenario");
        let scenario = builder.build();
        assert_eq!(scenario["name"], "my-scenario");
        assert!(scenario["mocks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_scenario_builder_with_string_name() {
        let name = String::from("string-scenario");
        let scenario = ScenarioBuilder::new(name).build();
        assert_eq!(scenario["name"], "string-scenario");
    }

    #[test]
    fn test_scenario_builder_single_mock() {
        let scenario = ScenarioBuilder::new("single-mock")
            .mock("/api/health", serde_json::json!({"status": "ok"}))
            .build();

        let mocks = scenario["mocks"].as_array().unwrap();
        assert_eq!(mocks.len(), 1);
        assert_eq!(mocks[0]["endpoint"], "/api/health");
    }

    #[test]
    fn test_scenario_builder_multiple_mocks() {
        let scenario = ScenarioBuilder::new("multi-mock")
            .mock("/api/v1/users", serde_json::json!([]))
            .mock("/api/v1/posts", serde_json::json!([]))
            .mock("/api/v1/comments", serde_json::json!([]))
            .build();

        let mocks = scenario["mocks"].as_array().unwrap();
        assert_eq!(mocks.len(), 3);
    }

    #[test]
    fn test_scenario_builder_complex_response() {
        let response = serde_json::json!({
            "data": {
                "user": {
                    "id": 123,
                    "name": "John Doe",
                    "roles": ["admin", "user"],
                    "metadata": {
                        "created_at": "2025-01-01T00:00:00Z"
                    }
                }
            },
            "pagination": {
                "total": 100,
                "page": 1,
                "per_page": 10
            }
        });

        let scenario =
            ScenarioBuilder::new("complex").mock("/api/profile", response.clone()).build();

        let mocks = scenario["mocks"].as_array().unwrap();
        assert_eq!(mocks[0]["response"]["data"]["user"]["id"], 123);
    }

    #[test]
    fn test_scenario_builder_null_response() {
        let scenario = ScenarioBuilder::new("null-response")
            .mock("/api/empty", serde_json::json!(null))
            .build();

        let mocks = scenario["mocks"].as_array().unwrap();
        assert!(mocks[0]["response"].is_null());
    }

    #[test]
    fn test_scenario_builder_array_response() {
        let scenario = ScenarioBuilder::new("array-response")
            .mock("/api/items", serde_json::json!([1, 2, 3, 4, 5]))
            .build();

        let mocks = scenario["mocks"].as_array().unwrap();
        let response = mocks[0]["response"].as_array().unwrap();
        assert_eq!(response.len(), 5);
    }

    // ScenarioManager tests
    #[test]
    fn test_scenario_manager_creation() {
        let manager = ScenarioManager::new("localhost", 3000);
        assert_eq!(manager.base_url, "http://localhost:3000");
    }

    #[test]
    fn test_scenario_manager_different_host() {
        let manager = ScenarioManager::new("192.168.1.100", 8080);
        assert_eq!(manager.base_url, "http://192.168.1.100:8080");
    }

    #[test]
    fn test_scenario_manager_hostname() {
        let manager = ScenarioManager::new("api.example.com", 443);
        assert_eq!(manager.base_url, "http://api.example.com:443");
    }

    #[test]
    fn test_scenario_manager_port_zero() {
        let manager = ScenarioManager::new("localhost", 0);
        assert_eq!(manager.base_url, "http://localhost:0");
    }

    #[test]
    fn test_scenario_manager_high_port() {
        let manager = ScenarioManager::new("localhost", 65535);
        assert_eq!(manager.base_url, "http://localhost:65535");
    }
}
