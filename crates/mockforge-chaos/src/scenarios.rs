//! Chaos engineering scenarios for orchestrating complex failure patterns

use crate::config::{ChaosConfig, FaultInjectionConfig, LatencyConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

/// A chaos engineering scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosScenario {
    /// Scenario name
    pub name: String,
    /// Scenario description
    pub description: Option<String>,
    /// Chaos configuration to apply
    pub chaos_config: ChaosConfig,
    /// Duration in seconds (0 = infinite)
    pub duration_seconds: u64,
    /// Start time (None = start immediately)
    pub start_time: Option<DateTime<Utc>>,
    /// End time (None = run indefinitely or until duration expires)
    pub end_time: Option<DateTime<Utc>>,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl ChaosScenario {
    /// Create a new chaos scenario
    pub fn new(name: impl Into<String>, chaos_config: ChaosConfig) -> Self {
        Self {
            name: name.into(),
            description: None,
            chaos_config,
            duration_seconds: 0,
            start_time: None,
            end_time: None,
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration_seconds = seconds;
        self
    }

    /// Set start time
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Check if scenario is currently active
    pub fn is_active(&self) -> bool {
        let now = Utc::now();

        // Check start time
        if let Some(start) = self.start_time {
            if now < start {
                return false;
            }
        }

        // Check end time
        if let Some(end) = self.end_time {
            if now > end {
                return false;
            }
        }

        true
    }
}

/// Predefined chaos scenarios
pub struct PredefinedScenarios;

impl PredefinedScenarios {
    /// Network degradation scenario (high latency, packet loss)
    pub fn network_degradation() -> ChaosScenario {
        ChaosScenario::new(
            "network_degradation",
            ChaosConfig {
                enabled: true,
                latency: Some(LatencyConfig {
                    enabled: true,
                    fixed_delay_ms: Some(500),
                    random_delay_range_ms: None,
                    jitter_percent: 20.0,
                    probability: 0.8,
                }),
                traffic_shaping: Some(crate::config::TrafficShapingConfig {
                    enabled: true,
                    packet_loss_percent: 5.0,
                    bandwidth_limit_bps: 100_000, // 100KB/s
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .with_description("Simulates degraded network conditions with high latency and packet loss")
        .with_tags(vec!["network".to_string(), "latency".to_string()])
    }

    /// Service instability scenario (random errors)
    pub fn service_instability() -> ChaosScenario {
        ChaosScenario::new(
            "service_instability",
            ChaosConfig {
                enabled: true,
                fault_injection: Some(FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![500, 502, 503, 504],
                    http_error_probability: 0.2,
                    timeout_errors: true,
                    timeout_probability: 0.1,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .with_description("Simulates an unstable service with random errors and timeouts")
        .with_tags(vec!["service".to_string(), "errors".to_string()])
    }

    /// Cascading failure scenario (combined failures)
    pub fn cascading_failure() -> ChaosScenario {
        ChaosScenario::new(
            "cascading_failure",
            ChaosConfig {
                enabled: true,
                latency: Some(LatencyConfig {
                    enabled: true,
                    fixed_delay_ms: None,
                    random_delay_range_ms: Some((1000, 5000)),
                    jitter_percent: 30.0,
                    probability: 0.7,
                }),
                fault_injection: Some(FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![500, 503],
                    http_error_probability: 0.3,
                    timeout_errors: true,
                    timeout_probability: 0.2,
                    connection_errors: true,
                    connection_error_probability: 0.1,
                    ..Default::default()
                }),
                rate_limit: Some(crate::config::RateLimitConfig {
                    enabled: true,
                    requests_per_second: 10,
                    burst_size: 2,
                    per_ip: true,
                    per_endpoint: false,
                }),
                ..Default::default()
            },
        )
        .with_description("Simulates a cascading failure with multiple simultaneous issues")
        .with_tags(vec!["critical".to_string(), "cascading".to_string()])
    }

    /// Peak traffic scenario (rate limiting stress test)
    pub fn peak_traffic() -> ChaosScenario {
        ChaosScenario::new(
            "peak_traffic",
            ChaosConfig {
                enabled: true,
                rate_limit: Some(crate::config::RateLimitConfig {
                    enabled: true,
                    requests_per_second: 50,
                    burst_size: 10,
                    per_ip: false,
                    per_endpoint: true,
                }),
                ..Default::default()
            },
        )
        .with_description("Simulates peak traffic conditions with aggressive rate limiting")
        .with_tags(vec!["traffic".to_string(), "load".to_string()])
    }

    /// Slow backend scenario (consistent high latency)
    pub fn slow_backend() -> ChaosScenario {
        ChaosScenario::new(
            "slow_backend",
            ChaosConfig {
                enabled: true,
                latency: Some(LatencyConfig {
                    enabled: true,
                    fixed_delay_ms: Some(2000),
                    random_delay_range_ms: None,
                    jitter_percent: 10.0,
                    probability: 1.0,
                }),
                ..Default::default()
            },
        )
        .with_description("Simulates a consistently slow backend service")
        .with_tags(vec!["latency".to_string(), "performance".to_string()])
    }
}

/// Scenario engine for managing active chaos scenarios
pub struct ScenarioEngine {
    active_scenarios: Arc<RwLock<HashMap<String, ChaosScenario>>>,
}

impl ScenarioEngine {
    /// Create a new scenario engine
    pub fn new() -> Self {
        Self {
            active_scenarios: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a scenario
    pub fn start_scenario(&self, scenario: ChaosScenario) {
        let name = scenario.name.clone();
        info!("Starting chaos scenario: {}", name);

        let mut scenarios = self.active_scenarios.write().unwrap();
        scenarios.insert(name, scenario);
    }

    /// Stop a scenario by name
    pub fn stop_scenario(&self, name: &str) -> bool {
        info!("Stopping chaos scenario: {}", name);

        let mut scenarios = self.active_scenarios.write().unwrap();
        scenarios.remove(name).is_some()
    }

    /// Stop all scenarios
    pub fn stop_all_scenarios(&self) {
        info!("Stopping all chaos scenarios");

        let mut scenarios = self.active_scenarios.write().unwrap();
        scenarios.clear();
    }

    /// Get active scenarios
    pub fn get_active_scenarios(&self) -> Vec<ChaosScenario> {
        let scenarios = self.active_scenarios.read().unwrap();
        scenarios.values().cloned().collect()
    }

    /// Get a specific scenario
    pub fn get_scenario(&self, name: &str) -> Option<ChaosScenario> {
        let scenarios = self.active_scenarios.read().unwrap();
        scenarios.get(name).cloned()
    }

    /// Get merged chaos config from all active scenarios
    pub fn get_merged_config(&self) -> Option<ChaosConfig> {
        let scenarios = self.active_scenarios.read().unwrap();

        if scenarios.is_empty() {
            return None;
        }

        // For simplicity, use the first active scenario's config
        // In a more sophisticated implementation, you could merge configs
        scenarios.values()
            .find(|s| s.is_active())
            .map(|s| s.chaos_config.clone())
    }

    /// Clean up expired scenarios
    pub fn cleanup_expired(&self) {
        debug!("Cleaning up expired scenarios");

        let mut scenarios = self.active_scenarios.write().unwrap();
        scenarios.retain(|name, scenario| {
            let active = scenario.is_active();
            if !active {
                info!("Removing expired scenario: {}", name);
            }
            active
        });
    }
}

impl Default for ScenarioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_creation() {
        let scenario = PredefinedScenarios::network_degradation();
        assert_eq!(scenario.name, "network_degradation");
        assert!(scenario.chaos_config.enabled);
        assert!(scenario.chaos_config.latency.is_some());
    }

    #[test]
    fn test_scenario_engine() {
        let engine = ScenarioEngine::new();

        let scenario = PredefinedScenarios::service_instability();
        engine.start_scenario(scenario.clone());

        let active = engine.get_active_scenarios();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "service_instability");

        assert!(engine.stop_scenario("service_instability"));
        assert_eq!(engine.get_active_scenarios().len(), 0);
    }

    #[test]
    fn test_predefined_scenarios() {
        let scenarios = vec![
            PredefinedScenarios::network_degradation(),
            PredefinedScenarios::service_instability(),
            PredefinedScenarios::cascading_failure(),
            PredefinedScenarios::peak_traffic(),
            PredefinedScenarios::slow_backend(),
        ];

        for scenario in scenarios {
            assert!(!scenario.name.is_empty());
            assert!(scenario.chaos_config.enabled);
        }
    }
}
