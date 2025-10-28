//! MockForge server management for tests

use crate::config::{ServerConfig, ServerConfigBuilder};
use crate::error::Result;
use crate::health::{HealthCheck, HealthStatus};
use crate::process::{find_available_port, ManagedProcess};
use crate::scenario::ScenarioManager;
use parking_lot::Mutex;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// A managed MockForge server instance for testing
pub struct MockForgeServer {
    process: Arc<Mutex<ManagedProcess>>,
    health: HealthCheck,
    scenario: ScenarioManager,
    http_port: u16,
}

impl MockForgeServer {
    /// Create a new builder for MockForgeServer
    pub fn builder() -> MockForgeServerBuilder {
        MockForgeServerBuilder::default()
    }

    /// Start a MockForge server with the given configuration
    pub async fn start(config: ServerConfig) -> Result<Self> {
        // Resolve port (auto-assign if 0)
        let mut resolved_config = config.clone();
        if resolved_config.http_port == 0 {
            resolved_config.http_port = find_available_port(30000)?;
            info!("Auto-assigned HTTP port: {}", resolved_config.http_port);
        }

        // Spawn the process
        let process = ManagedProcess::spawn(&resolved_config)?;
        let http_port = process.http_port();

        info!("MockForge server started on port {}", http_port);

        // Create health check client
        let health = HealthCheck::new("localhost", http_port);

        // Wait for server to become healthy
        debug!("Waiting for server to become healthy...");
        health
            .wait_until_healthy(resolved_config.health_timeout, resolved_config.health_interval)
            .await?;

        info!("MockForge server is healthy and ready");

        // Create scenario manager
        let scenario = ScenarioManager::new("localhost", http_port);

        Ok(Self {
            process: Arc::new(Mutex::new(process)),
            health,
            scenario,
            http_port,
        })
    }

    /// Get the HTTP port the server is running on
    pub fn http_port(&self) -> u16 {
        self.http_port
    }

    /// Get the base URL of the server
    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.http_port)
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.process.lock().pid()
    }

    /// Check if the server is still running
    pub fn is_running(&self) -> bool {
        self.process.lock().is_running()
    }

    /// Perform a health check
    pub async fn health_check(&self) -> Result<HealthStatus> {
        self.health.check().await
    }

    /// Check if the server is ready
    pub async fn is_ready(&self) -> bool {
        self.health.is_ready().await
    }

    /// Switch to a different scenario/workspace
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario to switch to
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use mockforge_test::MockForgeServer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = MockForgeServer::builder().build().await?;
    /// server.scenario("user-auth-success").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn scenario(&self, scenario_name: &str) -> Result<()> {
        self.scenario.switch_scenario(scenario_name).await
    }

    /// Load a workspace configuration from a file
    pub async fn load_workspace<P: AsRef<Path>>(&self, workspace_file: P) -> Result<()> {
        self.scenario.load_workspace(workspace_file).await
    }

    /// Update mock configuration for a specific endpoint
    pub async fn update_mock(&self, endpoint: &str, config: Value) -> Result<()> {
        self.scenario.update_mock(endpoint, config).await
    }

    /// List available fixtures
    pub async fn list_fixtures(&self) -> Result<Vec<String>> {
        self.scenario.list_fixtures().await
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> Result<Value> {
        self.scenario.get_stats().await
    }

    /// Reset all mocks to their initial state
    pub async fn reset(&self) -> Result<()> {
        self.scenario.reset().await
    }

    /// Stop the server
    pub fn stop(&self) -> Result<()> {
        info!("Stopping MockForge server (port: {})", self.http_port);
        self.process.lock().kill()
    }
}

impl Drop for MockForgeServer {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            eprintln!("Failed to stop MockForge server on drop: {}", e);
        }
    }
}

/// Builder for MockForgeServer
pub struct MockForgeServerBuilder {
    config_builder: ServerConfigBuilder,
}

impl Default for MockForgeServerBuilder {
    fn default() -> Self {
        Self {
            config_builder: ServerConfig::builder(),
        }
    }
}

impl MockForgeServerBuilder {
    /// Set HTTP port (0 for auto-assign)
    pub fn http_port(mut self, port: u16) -> Self {
        self.config_builder = self.config_builder.http_port(port);
        self
    }

    /// Set WebSocket port
    pub fn ws_port(mut self, port: u16) -> Self {
        self.config_builder = self.config_builder.ws_port(port);
        self
    }

    /// Set gRPC port
    pub fn grpc_port(mut self, port: u16) -> Self {
        self.config_builder = self.config_builder.grpc_port(port);
        self
    }

    /// Set admin UI port
    pub fn admin_port(mut self, port: u16) -> Self {
        self.config_builder = self.config_builder.admin_port(port);
        self
    }

    /// Set metrics port
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.config_builder = self.config_builder.metrics_port(port);
        self
    }

    /// Set OpenAPI specification file
    pub fn spec_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_builder = self.config_builder.spec_file(path);
        self
    }

    /// Set workspace directory
    pub fn workspace_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_builder = self.config_builder.workspace_dir(path);
        self
    }

    /// Set profile name
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.profile(profile);
        self
    }

    /// Enable admin UI
    pub fn enable_admin(mut self, enable: bool) -> Self {
        self.config_builder = self.config_builder.enable_admin(enable);
        self
    }

    /// Enable metrics endpoint
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config_builder = self.config_builder.enable_metrics(enable);
        self
    }

    /// Add extra CLI argument
    pub fn extra_arg(mut self, arg: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.extra_arg(arg);
        self
    }

    /// Set health check timeout
    pub fn health_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.config_builder = self.config_builder.health_timeout(timeout);
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_builder = self.config_builder.working_dir(path);
        self
    }

    /// Add environment variable
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.env_var(key, value);
        self
    }

    /// Set path to mockforge binary
    pub fn binary_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config_builder = self.config_builder.binary_path(path);
        self
    }

    /// Build and start the MockForge server
    pub async fn build(self) -> Result<MockForgeServer> {
        let config = self.config_builder.build();
        MockForgeServer::start(config).await
    }
}

// Helper function for use with test frameworks
/// Create a test server with default configuration
pub async fn with_mockforge<F, Fut>(test: F) -> Result<()>
where
    F: FnOnce(MockForgeServer) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let server = MockForgeServer::builder().build().await?;
    test(server).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let _builder =
            MockForgeServer::builder().http_port(3000).enable_admin(true).profile("test");

        // Builder should compile without errors
        assert!(true);
    }
}
