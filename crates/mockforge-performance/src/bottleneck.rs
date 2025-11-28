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
}
