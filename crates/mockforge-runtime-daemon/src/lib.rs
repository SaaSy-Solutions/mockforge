//! # MockForge Runtime Daemon
//!
//! Zero-config mode runtime daemon that automatically creates mocks from 404 responses.
//!
//! This crate provides the "invisible mock server" experience - when a user hits an endpoint
//! that doesn't exist, the daemon automatically:
//! - Creates a mock endpoint
//! - Generates a type
//! - Generates a client stub
//! - Adds to OpenAPI schema
//! - Adds an example response
//! - Sets up a scenario
//!
//! This is "mock server in your shadow" — an AI-assisted backend-on-demand.

pub mod auto_generator;
pub mod config;
pub mod detector;

pub use config::RuntimeDaemonConfig;
pub use detector::NotFoundDetector;
pub use auto_generator::AutoGenerator;

/// Runtime daemon for auto-creating mocks from 404s
pub struct RuntimeDaemon {
    /// Configuration for the daemon
    config: RuntimeDaemonConfig,
}

impl RuntimeDaemon {
    /// Create a new runtime daemon with the given configuration
    pub fn new(config: RuntimeDaemonConfig) -> Self {
        Self { config }
    }

    /// Check if the daemon is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configuration
    pub fn config(&self) -> &RuntimeDaemonConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_creation() {
        let config = RuntimeDaemonConfig::default();
        let daemon = RuntimeDaemon::new(config);
        assert!(!daemon.is_enabled()); // Default should be disabled
    }
}

