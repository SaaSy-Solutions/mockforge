//! Bottleneck Simulation
//!
//! Simulates various types of bottlenecks to observe system behavior under stress.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::debug;

/// Bottleneck type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BottleneckType {
    /// CPU bottleneck (simulated with busy-wait)
    Cpu,
    /// Memory bottleneck (simulated with allocation)
    Memory,
    /// Network bottleneck (simulated with delay)
    Network,
    /// I/O bottleneck (simulated with delay)
    Io,
    /// Database bottleneck (simulated with delay)
    Database,
}

/// Bottleneck configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckConfig {
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,
    /// Severity (0.0-1.0, where 1.0 is maximum bottleneck)
    pub severity: f64,
    /// Affected endpoint pattern (None = all endpoints)
    pub endpoint_pattern: Option<String>,
    /// Duration in seconds (None = indefinite)
    pub duration_secs: Option<u64>,
}

impl BottleneckConfig {
    /// Create a new bottleneck configuration
    pub fn new(bottleneck_type: BottleneckType, severity: f64) -> Self {
        Self {
            bottleneck_type,
            severity,
            endpoint_pattern: None,
            duration_secs: None,
        }
    }

    /// Set endpoint pattern
    pub fn with_endpoint_pattern(mut self, pattern: String) -> Self {
        self.endpoint_pattern = Some(pattern);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_secs: u64) -> Self {
        self.duration_secs = Some(duration_secs);
        self
    }
}

/// Bottleneck simulator
///
/// Simulates various types of bottlenecks.
#[derive(Debug, Clone)]
pub struct BottleneckSimulator {
    /// Active bottlenecks
    bottlenecks: Arc<RwLock<Vec<BottleneckConfig>>>,
}

impl BottleneckSimulator {
    /// Create a new bottleneck simulator
    pub fn new() -> Self {
        Self {
            bottlenecks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a bottleneck
    pub async fn add_bottleneck(&self, config: BottleneckConfig) {
        let bottleneck_type = config.bottleneck_type;
        let mut bottlenecks = self.bottlenecks.write().await;
        bottlenecks.push(config);
        debug!("Bottleneck added: {:?}", bottleneck_type);
    }

    /// Remove all bottlenecks
    pub async fn clear_bottlenecks(&self) {
        let mut bottlenecks = self.bottlenecks.write().await;
        bottlenecks.clear();
        debug!("All bottlenecks cleared");
    }

    /// Get active bottlenecks
    pub async fn get_bottlenecks(&self) -> Vec<BottleneckConfig> {
        let bottlenecks = self.bottlenecks.read().await;
        bottlenecks.clone()
    }

    /// Apply bottlenecks for a request
    ///
    /// Returns the total delay in milliseconds.
    pub async fn apply_bottlenecks(&self, endpoint: &str) -> u64 {
        let bottlenecks = self.bottlenecks.read().await;
        let mut total_delay_ms = 0u64;

        for bottleneck in bottlenecks.iter() {
            // Check if endpoint matches pattern
            if let Some(ref pattern) = bottleneck.endpoint_pattern {
                if !endpoint.contains(pattern) {
                    continue;
                }
            }

            // Calculate delay based on bottleneck type and severity
            let delay_ms = match bottleneck.bottleneck_type {
                BottleneckType::Cpu => {
                    // CPU bottleneck: busy-wait
                    let cpu_time_ms = (bottleneck.severity * 100.0) as u64;
                    self.simulate_cpu_bottleneck(cpu_time_ms).await;
                    0 // CPU bottleneck doesn't add delay, it uses CPU time
                }
                BottleneckType::Memory => {
                    // Memory bottleneck: allocation
                    let memory_mb = (bottleneck.severity * 100.0) as usize;
                    self.simulate_memory_bottleneck(memory_mb).await;
                    0 // Memory bottleneck doesn't add delay
                }
                BottleneckType::Network => {
                    // Network bottleneck: delay
                    (bottleneck.severity * 500.0) as u64
                }
                BottleneckType::Io => {
                    // I/O bottleneck: delay
                    (bottleneck.severity * 300.0) as u64
                }
                BottleneckType::Database => {
                    // Database bottleneck: delay
                    (bottleneck.severity * 400.0) as u64
                }
            };

            total_delay_ms += delay_ms;
        }

        if total_delay_ms > 0 {
            sleep(Duration::from_millis(total_delay_ms)).await;
        }

        total_delay_ms
    }

    /// Simulate CPU bottleneck (busy-wait)
    async fn simulate_cpu_bottleneck(&self, duration_ms: u64) {
        let start = std::time::Instant::now();
        let duration = Duration::from_millis(duration_ms);

        // Busy-wait to simulate CPU load
        while start.elapsed() < duration {
            // Spin loop
            std::hint::spin_loop();
        }
    }

    /// Simulate memory bottleneck (allocation)
    async fn simulate_memory_bottleneck(&self, size_mb: usize) {
        // Allocate memory to simulate memory pressure
        let _memory: Vec<u8> = vec![0; size_mb * 1024 * 1024];
        // Memory is dropped when function returns
    }
}

impl Default for BottleneckSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottleneck_type_clone() {
        let bt = BottleneckType::Cpu;
        let cloned = bt.clone();
        assert_eq!(bt, cloned);
    }

    #[test]
    fn test_bottleneck_type_debug() {
        let bt = BottleneckType::Memory;
        let debug = format!("{:?}", bt);
        assert!(debug.contains("Memory"));
    }

    #[test]
    fn test_bottleneck_type_serialize() {
        let bt = BottleneckType::Network;
        let json = serde_json::to_string(&bt).unwrap();
        assert_eq!(json, "\"network\"");
    }

    #[test]
    fn test_bottleneck_type_deserialize() {
        let bt: BottleneckType = serde_json::from_str("\"database\"").unwrap();
        assert_eq!(bt, BottleneckType::Database);
    }

    #[test]
    fn test_bottleneck_type_serialize_all_variants() {
        assert_eq!(serde_json::to_string(&BottleneckType::Cpu).unwrap(), "\"cpu\"");
        assert_eq!(serde_json::to_string(&BottleneckType::Memory).unwrap(), "\"memory\"");
        assert_eq!(serde_json::to_string(&BottleneckType::Network).unwrap(), "\"network\"");
        assert_eq!(serde_json::to_string(&BottleneckType::Io).unwrap(), "\"io\"");
        assert_eq!(serde_json::to_string(&BottleneckType::Database).unwrap(), "\"database\"");
    }

    #[test]
    fn test_bottleneck_type_copy() {
        let bt = BottleneckType::Io;
        let copied: BottleneckType = bt;
        assert_eq!(bt, copied);
    }

    #[test]
    fn test_bottleneck_config_new() {
        let config = BottleneckConfig::new(BottleneckType::Cpu, 0.5);
        assert_eq!(config.bottleneck_type, BottleneckType::Cpu);
        assert_eq!(config.severity, 0.5);
        assert!(config.endpoint_pattern.is_none());
        assert!(config.duration_secs.is_none());
    }

    #[test]
    fn test_bottleneck_config_with_endpoint_pattern() {
        let config = BottleneckConfig::new(BottleneckType::Network, 0.8)
            .with_endpoint_pattern("/api/users".to_string());
        assert_eq!(config.endpoint_pattern, Some("/api/users".to_string()));
    }

    #[test]
    fn test_bottleneck_config_with_duration() {
        let config = BottleneckConfig::new(BottleneckType::Database, 0.3).with_duration(60);
        assert_eq!(config.duration_secs, Some(60));
    }

    #[test]
    fn test_bottleneck_config_builder_chain() {
        let config = BottleneckConfig::new(BottleneckType::Memory, 0.7)
            .with_endpoint_pattern("/api/orders".to_string())
            .with_duration(120);

        assert_eq!(config.bottleneck_type, BottleneckType::Memory);
        assert_eq!(config.severity, 0.7);
        assert_eq!(config.endpoint_pattern, Some("/api/orders".to_string()));
        assert_eq!(config.duration_secs, Some(120));
    }

    #[test]
    fn test_bottleneck_config_clone() {
        let config = BottleneckConfig::new(BottleneckType::Io, 0.4).with_duration(30);
        let cloned = config.clone();
        assert_eq!(config.bottleneck_type, cloned.bottleneck_type);
        assert_eq!(config.severity, cloned.severity);
    }

    #[test]
    fn test_bottleneck_config_debug() {
        let config = BottleneckConfig::new(BottleneckType::Cpu, 0.9);
        let debug = format!("{:?}", config);
        assert!(debug.contains("BottleneckConfig"));
        assert!(debug.contains("Cpu"));
    }

    #[test]
    fn test_bottleneck_config_serialize() {
        let config = BottleneckConfig::new(BottleneckType::Network, 0.5);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"bottleneck_type\":\"network\""));
        assert!(json.contains("\"severity\":0.5"));
    }

    #[test]
    fn test_bottleneck_simulator_new() {
        let simulator = BottleneckSimulator::new();
        let debug = format!("{:?}", simulator);
        assert!(debug.contains("BottleneckSimulator"));
    }

    #[test]
    fn test_bottleneck_simulator_default() {
        let simulator = BottleneckSimulator::default();
        let debug = format!("{:?}", simulator);
        assert!(debug.contains("BottleneckSimulator"));
    }

    #[test]
    fn test_bottleneck_simulator_clone() {
        let simulator = BottleneckSimulator::new();
        let _cloned = simulator.clone();
    }

    #[tokio::test]
    async fn test_bottleneck_simulator() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Network, 0.5)
            .with_endpoint_pattern("/api/users".to_string());

        simulator.add_bottleneck(config).await;

        let bottlenecks = simulator.get_bottlenecks().await;
        assert_eq!(bottlenecks.len(), 1);
    }

    #[tokio::test]
    async fn test_bottleneck_simulator_clear() {
        let simulator = BottleneckSimulator::new();

        simulator.add_bottleneck(BottleneckConfig::new(BottleneckType::Cpu, 0.5)).await;
        simulator
            .add_bottleneck(BottleneckConfig::new(BottleneckType::Memory, 0.3))
            .await;

        let bottlenecks = simulator.get_bottlenecks().await;
        assert_eq!(bottlenecks.len(), 2);

        simulator.clear_bottlenecks().await;

        let bottlenecks = simulator.get_bottlenecks().await;
        assert!(bottlenecks.is_empty());
    }

    #[tokio::test]
    async fn test_bottleneck_simulator_multiple_bottlenecks() {
        let simulator = BottleneckSimulator::new();

        simulator
            .add_bottleneck(BottleneckConfig::new(BottleneckType::Network, 0.2))
            .await;
        simulator.add_bottleneck(BottleneckConfig::new(BottleneckType::Io, 0.3)).await;
        simulator
            .add_bottleneck(BottleneckConfig::new(BottleneckType::Database, 0.4))
            .await;

        let bottlenecks = simulator.get_bottlenecks().await;
        assert_eq!(bottlenecks.len(), 3);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Network, 0.1);
        simulator.add_bottleneck(config).await;

        let start = std::time::Instant::now();
        simulator.apply_bottlenecks("/api/test").await;
        let elapsed = start.elapsed();

        // Should have added some delay
        assert!(elapsed.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_with_pattern_match() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Network, 0.1)
            .with_endpoint_pattern("/api/users".to_string());
        simulator.add_bottleneck(config).await;

        let start = std::time::Instant::now();
        let delay = simulator.apply_bottlenecks("/api/users/123").await;
        let elapsed = start.elapsed();

        // Should have applied delay because endpoint contains pattern
        assert!(elapsed.as_millis() > 0 || delay > 0);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_with_pattern_no_match() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Network, 0.5)
            .with_endpoint_pattern("/api/users".to_string());
        simulator.add_bottleneck(config).await;

        let start = std::time::Instant::now();
        let delay = simulator.apply_bottlenecks("/api/orders").await;
        let elapsed = start.elapsed();

        // Should not have applied delay because endpoint doesn't match
        assert_eq!(delay, 0);
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_io() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Io, 0.1);
        simulator.add_bottleneck(config).await;

        let start = std::time::Instant::now();
        let delay = simulator.apply_bottlenecks("/api/test").await;
        let elapsed = start.elapsed();

        // I/O bottleneck adds delay
        assert!(delay > 0 || elapsed.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_database() {
        let simulator = BottleneckSimulator::new();

        let config = BottleneckConfig::new(BottleneckType::Database, 0.1);
        simulator.add_bottleneck(config).await;

        let start = std::time::Instant::now();
        let delay = simulator.apply_bottlenecks("/api/test").await;
        let elapsed = start.elapsed();

        // Database bottleneck adds delay
        assert!(delay > 0 || elapsed.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_cpu() {
        let simulator = BottleneckSimulator::new();

        // Very low severity to keep test fast
        let config = BottleneckConfig::new(BottleneckType::Cpu, 0.01);
        simulator.add_bottleneck(config).await;

        // CPU bottleneck returns 0 delay but uses CPU time
        let delay = simulator.apply_bottlenecks("/api/test").await;
        assert_eq!(delay, 0); // CPU doesn't add delay, it uses CPU time
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_memory() {
        let simulator = BottleneckSimulator::new();

        // Very low severity to keep test fast
        let config = BottleneckConfig::new(BottleneckType::Memory, 0.01);
        simulator.add_bottleneck(config).await;

        // Memory bottleneck returns 0 delay
        let delay = simulator.apply_bottlenecks("/api/test").await;
        assert_eq!(delay, 0);
    }

    #[tokio::test]
    async fn test_apply_bottlenecks_no_bottlenecks() {
        let simulator = BottleneckSimulator::new();

        let start = std::time::Instant::now();
        let delay = simulator.apply_bottlenecks("/api/test").await;
        let elapsed = start.elapsed();

        // No bottlenecks, should be fast
        assert_eq!(delay, 0);
        assert!(elapsed.as_millis() < 10);
    }
}
