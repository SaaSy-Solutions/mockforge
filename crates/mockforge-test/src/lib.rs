//! # MockForge Test Utilities
//!
//! Test utilities for MockForge to simplify integration with test frameworks like Playwright and Vitest.
//!
//! ## Features
//!
//! - **Easy Server Spawning**: Start and stop MockForge servers programmatically
//! - **Health Checks**: Wait for server readiness with configurable timeouts
//! - **Scenario Management**: Switch scenarios/workspaces per-test
//! - **Process Management**: Automatic cleanup of spawned processes
//! - **Profile Support**: Run with different MockForge profiles
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mockforge_test::{MockForgeServer, ServerConfig};
//!
//! #[tokio::test]
//! async fn test_with_mockforge() {
//!     // Start MockForge server
//!     let server = MockForgeServer::builder()
//!         .http_port(3000)
//!         .build()
//!         .await
//!         .expect("Failed to start server");
//!
//!     // Server is ready - run your tests
//!     let response = reqwest::get("http://localhost:3000/health")
//!         .await
//!         .expect("Failed to get health");
//!
//!     assert!(response.status().is_success());
//!
//!     // Server automatically stops when dropped
//! }
//! ```
//!
//! ## Scenario Switching
//!
//! ```rust,no_run
//! use mockforge_test::MockForgeServer;
//!
//! #[tokio::test]
//! async fn test_scenario_switching() {
//!     let server = MockForgeServer::builder()
//!         .http_port(3000)
//!         .build()
//!         .await
//!         .expect("Failed to start server");
//!
//!     // Switch to a different scenario
//!     server.scenario("user-auth-success")
//!         .await
//!         .expect("Failed to switch scenario");
//!
//!     // Test with the new scenario
//!     // ...
//! }
//! ```

pub mod config;
pub mod error;
pub mod health;
pub mod process;
pub mod scenario;
pub mod server;

pub use config::{ServerConfig, ServerConfigBuilder};
pub use error::{Error, Result};
pub use health::{HealthCheck, HealthStatus};
pub use scenario::ScenarioManager;
pub use server::MockForgeServer;

/// Re-export commonly used types
pub use reqwest;
pub use tokio;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = ServerConfig::builder().http_port(3000).admin_port(3001).build();

        assert_eq!(config.http_port, 3000);
        assert_eq!(config.admin_port, Some(3001));
    }
}
