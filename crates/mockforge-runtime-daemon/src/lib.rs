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

pub use auto_generator::AutoGenerator;
pub use config::RuntimeDaemonConfig;
pub use detector::NotFoundDetector;

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

    /// Opt-in: spawn a [`mockforge_scenarios::FederationScenarioPoller`]
    /// using the standard `MOCKFORGE_FEDERATION_POLL_*` env vars.
    ///
    /// Returns `Ok(None)` when the env vars aren't configured (the 99%
    /// zero-config case — the daemon stays pure 404-generation). Returns
    /// `Ok(Some(handle))` when a poller spawns; drop the handle to stop it.
    ///
    /// The default [`mockforge_scenarios::LoggingApplicator`] only logs
    /// observed overrides. A real embedder wanting to push them to the
    /// daemon's auto-generator (e.g. bumping chaos level based on a
    /// scenario) should call [`mockforge_scenarios::spawn_federation_scenario_poller`]
    /// directly with a custom `ScenarioApplicator` impl.
    ///
    /// # Errors
    ///
    /// Returns an error string when env vars are malformed (bad UUID,
    /// non-numeric interval). Missing vars are not an error.
    pub fn spawn_federation_scenario_poller(
        &self,
    ) -> Result<Option<tokio::task::JoinHandle<()>>, String> {
        mockforge_scenarios::spawn_federation_scenario_poller(
            mockforge_scenarios::LoggingApplicator,
        )
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

    #[tokio::test]
    async fn spawn_federation_scenario_poller_returns_none_when_env_missing() {
        // Clearing env would race with parallel tests, so rely on the fact
        // that these vars are never set under `cargo test` in CI. If this
        // test flakes, gate it behind a `serial_test` annotation.
        let daemon = RuntimeDaemon::new(RuntimeDaemonConfig::default());
        let handle = daemon.spawn_federation_scenario_poller().unwrap();
        assert!(handle.is_none(), "expected None when MOCKFORGE_FEDERATION_POLL_URL unset");
    }
}
