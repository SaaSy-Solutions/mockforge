//! Chaos testing utilities with orchestration and randomness
//!
//! This module provides high-level chaos testing utilities that randomly inject
//! errors, delays, and other failures. It builds on the existing latency and
//! failure injection systems to provide easy-to-use chaos testing capabilities.

use crate::failure_injection::{FailureConfig, FailureInjector};
use crate::latency::{FaultConfig, LatencyInjector, LatencyProfile};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Chaos mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChaosConfig {
    /// Enable chaos mode
    pub enabled: bool,
    /// Error injection rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Delay injection rate (0.0 to 1.0)
    pub delay_rate: f64,
    /// Minimum delay in milliseconds
    pub min_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Status codes to randomly inject
    pub status_codes: Vec<u16>,
    /// Whether to inject random timeouts
    pub inject_timeouts: bool,
    /// Timeout duration in milliseconds
    pub timeout_ms: u64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            error_rate: 0.1, // 10% error rate
            delay_rate: 0.3, // 30% delay rate
            min_delay_ms: 100,
            max_delay_ms: 2000,
            status_codes: vec![500, 502, 503, 504],
            inject_timeouts: false,
            timeout_ms: 5000,
        }
    }
}

impl ChaosConfig {
    /// Create a new chaos config with custom error and delay rates
    pub fn new(error_rate: f64, delay_rate: f64) -> Self {
        Self {
            enabled: true,
            error_rate: error_rate.clamp(0.0, 1.0),
            delay_rate: delay_rate.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Configure the delay range
    pub fn with_delay_range(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.min_delay_ms = min_ms;
        self.max_delay_ms = max_ms;
        self
    }

    /// Configure status codes to inject
    pub fn with_status_codes(mut self, codes: Vec<u16>) -> Self {
        self.status_codes = codes;
        self
    }

    /// Enable timeout injection
    pub fn with_timeouts(mut self, timeout_ms: u64) -> Self {
        self.inject_timeouts = true;
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Chaos engine that orchestrates random failure injection
#[derive(Debug, Clone)]
pub struct ChaosEngine {
    config: Arc<RwLock<ChaosConfig>>,
    latency_injector: Arc<RwLock<LatencyInjector>>,
    failure_injector: Arc<RwLock<FailureInjector>>,
}

impl ChaosEngine {
    /// Create a new chaos engine
    pub fn new(config: ChaosConfig) -> Self {
        // Create latency injector from config
        let latency_profile = LatencyProfile::new(
            (config.min_delay_ms + config.max_delay_ms) / 2,
            (config.max_delay_ms - config.min_delay_ms) / 2,
        );

        let fault_config = FaultConfig {
            failure_rate: config.error_rate,
            status_codes: config.status_codes.clone(),
            error_responses: Default::default(),
        };

        let latency_injector = LatencyInjector::new(latency_profile, fault_config);

        // Create failure injector from config
        let failure_config = FailureConfig {
            global_error_rate: config.error_rate,
            default_status_codes: config.status_codes.clone(),
            tag_configs: Default::default(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
        };

        let failure_injector = FailureInjector::new(Some(failure_config), config.enabled);

        Self {
            config: Arc::new(RwLock::new(config)),
            latency_injector: Arc::new(RwLock::new(latency_injector)),
            failure_injector: Arc::new(RwLock::new(failure_injector)),
        }
    }

    /// Check if chaos mode is enabled
    pub async fn is_enabled(&self) -> bool {
        self.config.read().await.enabled
    }

    /// Enable or disable chaos mode
    pub async fn set_enabled(&self, enabled: bool) {
        let mut config = self.config.write().await;
        config.enabled = enabled;

        // Update injectors
        let mut latency = self.latency_injector.write().await;
        latency.set_enabled(enabled);

        let mut failure = self.failure_injector.write().await;
        failure.set_enabled(enabled);
    }

    /// Update chaos configuration
    pub async fn update_config(&self, new_config: ChaosConfig) {
        let mut config = self.config.write().await;
        *config = new_config.clone();

        // Update injectors with new config
        let latency_profile = LatencyProfile::new(
            (new_config.min_delay_ms + new_config.max_delay_ms) / 2,
            (new_config.max_delay_ms - new_config.min_delay_ms) / 2,
        );

        let fault_config = FaultConfig {
            failure_rate: new_config.error_rate,
            status_codes: new_config.status_codes.clone(),
            error_responses: Default::default(),
        };

        let mut latency = self.latency_injector.write().await;
        *latency = LatencyInjector::new(latency_profile, fault_config);
        latency.set_enabled(new_config.enabled);

        let failure_config = FailureConfig {
            global_error_rate: new_config.error_rate,
            default_status_codes: new_config.status_codes.clone(),
            tag_configs: Default::default(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
        };

        let mut failure = self.failure_injector.write().await;
        failure.update_config(Some(failure_config));
        failure.set_enabled(new_config.enabled);
    }

    /// Process a request with random chaos injection
    /// Returns Some((status_code, error_message)) if an error should be injected
    pub async fn process_request(&self, _tags: &[String]) -> ChaosResult {
        let config = self.config.read().await;

        if !config.enabled {
            return ChaosResult::Success;
        }

        let mut rng = rand::rng();

        // First, randomly decide if we should inject an error
        if rng.random_bool(config.error_rate) {
            let status_code = if config.status_codes.is_empty() {
                500
            } else {
                let index = rng.random_range(0..config.status_codes.len());
                config.status_codes[index]
            };

            return ChaosResult::Error {
                status_code,
                message: format!("Chaos-injected error (rate: {:.1}%)", config.error_rate * 100.0),
            };
        }

        // Then, randomly decide if we should inject a delay
        if rng.random_bool(config.delay_rate) {
            let delay_ms = rng.random_range(config.min_delay_ms..=config.max_delay_ms);
            return ChaosResult::Delay { delay_ms };
        }

        // Finally, check for timeout injection
        if config.inject_timeouts && rng.random_bool(0.05) {
            // 5% chance of timeout
            return ChaosResult::Timeout {
                timeout_ms: config.timeout_ms,
            };
        }

        ChaosResult::Success
    }

    /// Inject latency for a request (respects delay_rate)
    pub async fn inject_latency(&self, tags: &[String]) -> crate::Result<()> {
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(());
        }

        let mut rng = rand::rng();

        // Only inject latency based on delay_rate
        if rng.random_bool(config.delay_rate) {
            let latency = self.latency_injector.read().await;
            latency.inject_latency(tags).await?;
        }

        Ok(())
    }

    /// Check if an error should be injected (respects error_rate)
    pub async fn should_inject_error(&self, tags: &[String]) -> bool {
        let config = self.config.read().await;

        if !config.enabled {
            return false;
        }

        let failure = self.failure_injector.read().await;
        failure.should_inject_failure(tags)
    }

    /// Get a random error response
    pub async fn get_error_response(&self) -> Option<(u16, String)> {
        let config = self.config.read().await;

        if !config.enabled {
            return None;
        }

        let mut rng = rand::rng();
        let status_code = if config.status_codes.is_empty() {
            500
        } else {
            let index = rng.random_range(0..config.status_codes.len());
            config.status_codes[index]
        };

        Some((
            status_code,
            format!("Chaos-injected error (rate: {:.1}%)", config.error_rate * 100.0),
        ))
    }

    /// Get current chaos configuration
    pub async fn get_config(&self) -> ChaosConfig {
        self.config.read().await.clone()
    }

    /// Get chaos statistics
    pub async fn get_statistics(&self) -> ChaosStatistics {
        let config = self.config.read().await;
        ChaosStatistics {
            enabled: config.enabled,
            error_rate: config.error_rate,
            delay_rate: config.delay_rate,
            min_delay_ms: config.min_delay_ms,
            max_delay_ms: config.max_delay_ms,
            inject_timeouts: config.inject_timeouts,
        }
    }
}

impl Default for ChaosEngine {
    fn default() -> Self {
        Self::new(ChaosConfig::default())
    }
}

/// Result of chaos engineering evaluation, indicating what effect to apply
#[derive(Debug, Clone)]
pub enum ChaosResult {
    /// No chaos effect - request should proceed normally
    Success,
    /// Inject an error response
    Error {
        /// HTTP status code for the error
        status_code: u16,
        /// Error message to include
        message: String,
    },
    /// Inject a delay before processing
    Delay {
        /// Delay duration in milliseconds
        delay_ms: u64,
    },
    /// Inject a timeout
    Timeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },
}

/// Statistics for chaos engineering engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosStatistics {
    /// Whether chaos engineering is enabled
    pub enabled: bool,
    /// Error injection rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Delay injection rate (0.0 to 1.0)
    pub delay_rate: f64,
    /// Minimum delay in milliseconds
    pub min_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Whether to inject timeouts
    pub inject_timeouts: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_config_default() {
        let config = ChaosConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.error_rate, 0.1);
        assert_eq!(config.delay_rate, 0.3);
        assert!(!config.inject_timeouts);
    }

    #[test]
    fn test_chaos_config_new() {
        let config = ChaosConfig::new(0.2, 0.5);
        assert!(config.enabled);
        assert_eq!(config.error_rate, 0.2);
        assert_eq!(config.delay_rate, 0.5);
    }

    #[test]
    fn test_chaos_config_builder() {
        let config = ChaosConfig::new(0.1, 0.2)
            .with_delay_range(50, 500)
            .with_status_codes(vec![500, 503])
            .with_timeouts(3000);

        assert_eq!(config.min_delay_ms, 50);
        assert_eq!(config.max_delay_ms, 500);
        assert_eq!(config.status_codes, vec![500, 503]);
        assert!(config.inject_timeouts);
        assert_eq!(config.timeout_ms, 3000);
    }

    #[tokio::test]
    async fn test_chaos_engine_creation() {
        let config = ChaosConfig::new(0.5, 0.5);
        let engine = ChaosEngine::new(config);

        assert!(engine.is_enabled().await);
    }

    #[tokio::test]
    async fn test_chaos_engine_enable_disable() {
        let config = ChaosConfig::new(0.5, 0.5);
        let engine = ChaosEngine::new(config);

        assert!(engine.is_enabled().await);

        engine.set_enabled(false).await;
        assert!(!engine.is_enabled().await);

        engine.set_enabled(true).await;
        assert!(engine.is_enabled().await);
    }

    #[tokio::test]
    async fn test_chaos_engine_disabled_returns_success() {
        let mut config = ChaosConfig::new(1.0, 1.0); // 100% rates
        config.enabled = false;
        let engine = ChaosEngine::new(config);

        let result = engine.process_request(&[]).await;
        assert!(matches!(result, ChaosResult::Success));
    }

    #[tokio::test]
    async fn test_chaos_engine_high_error_rate() {
        let config = ChaosConfig::new(1.0, 0.0) // 100% error rate, 0% delay rate
            .with_status_codes(vec![503]);
        let engine = ChaosEngine::new(config);

        let mut error_count = 0;
        for _ in 0..10 {
            let result = engine.process_request(&[]).await;
            if let ChaosResult::Error { status_code, .. } = result {
                assert_eq!(status_code, 503);
                error_count += 1;
            }
        }

        // With 100% error rate, all requests should fail
        assert_eq!(error_count, 10);
    }

    #[tokio::test]
    async fn test_chaos_engine_high_delay_rate() {
        let config = ChaosConfig::new(0.0, 1.0) // 0% error rate, 100% delay rate
            .with_delay_range(100, 200);
        let engine = ChaosEngine::new(config);

        let mut delay_count = 0;
        for _ in 0..10 {
            let result = engine.process_request(&[]).await;
            if let ChaosResult::Delay { delay_ms } = result {
                assert!((100..=200).contains(&delay_ms));
                delay_count += 1;
            }
        }

        // With 100% delay rate, all requests should be delayed
        assert_eq!(delay_count, 10);
    }

    #[tokio::test]
    async fn test_chaos_engine_update_config() {
        let config = ChaosConfig::new(0.5, 0.5);
        let engine = ChaosEngine::new(config);

        let new_config = ChaosConfig::new(0.2, 0.8).with_delay_range(50, 100);

        engine.update_config(new_config).await;

        let updated = engine.get_config().await;
        assert_eq!(updated.error_rate, 0.2);
        assert_eq!(updated.delay_rate, 0.8);
        assert_eq!(updated.min_delay_ms, 50);
        assert_eq!(updated.max_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_chaos_engine_statistics() {
        let config = ChaosConfig::new(0.3, 0.4).with_delay_range(100, 500).with_timeouts(2000);

        let engine = ChaosEngine::new(config);
        let stats = engine.get_statistics().await;

        assert!(stats.enabled);
        assert_eq!(stats.error_rate, 0.3);
        assert_eq!(stats.delay_rate, 0.4);
        assert_eq!(stats.min_delay_ms, 100);
        assert_eq!(stats.max_delay_ms, 500);
        assert!(stats.inject_timeouts);
    }

    #[tokio::test]
    async fn test_chaos_result_variants() {
        let success = ChaosResult::Success;
        assert!(matches!(success, ChaosResult::Success));

        let error = ChaosResult::Error {
            status_code: 500,
            message: "Error".to_string(),
        };
        if let ChaosResult::Error { status_code, .. } = error {
            assert_eq!(status_code, 500);
        } else {
            panic!("Expected Error variant");
        }

        let delay = ChaosResult::Delay { delay_ms: 100 };
        if let ChaosResult::Delay { delay_ms } = delay {
            assert_eq!(delay_ms, 100);
        } else {
            panic!("Expected Delay variant");
        }
    }
}
