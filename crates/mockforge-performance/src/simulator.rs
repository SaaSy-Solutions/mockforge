//! Performance Simulator
//!
//! Main orchestrator for performance mode, combining RPS control,
//! bottleneck simulation, and latency recording.

use crate::bottleneck::{BottleneckConfig, BottleneckSimulator};
use crate::controller::{RpsController, RpsProfile};
use crate::latency::{LatencyAnalyzer, LatencyRecorder};
use crate::metrics::{PerformanceMetrics, PerformanceSnapshot};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Simulator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatorConfig {
    /// Initial RPS
    pub initial_rps: f64,
    /// RPS profile
    pub rps_profile: Option<RpsProfile>,
    /// Bottleneck configurations
    pub bottlenecks: Vec<BottleneckConfig>,
    /// Maximum latency samples to keep
    pub max_latency_samples: usize,
    /// Maximum age of latency samples (seconds)
    pub max_latency_age_seconds: u64,
}

impl SimulatorConfig {
    /// Create a new simulator configuration
    pub fn new(initial_rps: f64) -> Self {
        Self {
            initial_rps,
            rps_profile: None,
            bottlenecks: Vec::new(),
            max_latency_samples: 10000,
            max_latency_age_seconds: 300, // 5 minutes
        }
    }

    /// Set RPS profile
    pub fn with_rps_profile(mut self, profile: RpsProfile) -> Self {
        self.rps_profile = Some(profile);
        self
    }

    /// Add bottleneck
    pub fn with_bottleneck(mut self, bottleneck: BottleneckConfig) -> Self {
        self.bottlenecks.push(bottleneck);
        self
    }
}

/// Performance simulator
///
/// Orchestrates RPS control, bottleneck simulation, and latency recording.
#[derive(Debug, Clone)]
pub struct PerformanceSimulator {
    /// RPS controller
    rps_controller: Arc<RpsController>,
    /// Bottleneck simulator
    bottleneck_simulator: Arc<BottleneckSimulator>,
    /// Latency recorder
    latency_recorder: Arc<LatencyRecorder>,
    /// Latency analyzer
    latency_analyzer: Arc<LatencyAnalyzer>,
    /// Configuration
    config: Arc<RwLock<SimulatorConfig>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
}

impl PerformanceSimulator {
    /// Create a new performance simulator
    pub fn new(config: SimulatorConfig) -> Self {
        let rps_controller = Arc::new(RpsController::new(config.initial_rps));
        let bottleneck_simulator = Arc::new(BottleneckSimulator::new());
        let latency_recorder = Arc::new(LatencyRecorder::new(
            config.max_latency_samples,
            config.max_latency_age_seconds,
        ));
        let latency_analyzer = Arc::new(LatencyAnalyzer::new(latency_recorder.clone()));

        // Set up bottlenecks
        for bottleneck in &config.bottlenecks {
            let simulator = bottleneck_simulator.clone();
            let bottleneck = bottleneck.clone();
            tokio::spawn(async move {
                simulator.add_bottleneck(bottleneck).await;
            });
        }

        Self {
            rps_controller,
            bottleneck_simulator,
            latency_recorder,
            latency_analyzer,
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the simulator
    pub async fn start(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);

        info!("Performance simulator started");

        // Start RPS profile execution if configured
        let config = self.config.read().await;
        if let Some(ref profile) = config.rps_profile {
            let controller = self.rps_controller.clone();
            let profile = profile.clone();
            tokio::spawn(async move {
                Self::execute_rps_profile(controller, profile).await;
            });
        }
    }

    /// Stop the simulator
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        drop(is_running);

        self.bottleneck_simulator.clear_bottlenecks().await;
        info!("Performance simulator stopped");
    }

    /// Check if simulator is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Execute RPS profile
    async fn execute_rps_profile(controller: Arc<RpsController>, profile: RpsProfile) {
        info!("Executing RPS profile: {}", profile.name);

        for stage in profile.stages {
            if let Some(ref name) = stage.name {
                info!(
                    "RPS stage: {} - {} RPS for {}s",
                    name, stage.target_rps, stage.duration_secs
                );
            }

            controller.set_target_rps(stage.target_rps).await;

            if stage.duration_secs > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(stage.duration_secs)).await;
            } else {
                // Infinite duration - wait until stopped
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Process a request through the simulator
    ///
    /// This should be called for each request to apply RPS control,
    /// bottleneck simulation, and latency recording.
    pub async fn process_request(&self, endpoint: &str, method: &str) -> Result<(), anyhow::Error> {
        // Wait for RPS slot
        self.rps_controller.wait_for_slot().await;

        // Apply bottlenecks
        let bottleneck_delay_ms = self.bottleneck_simulator.apply_bottlenecks(endpoint).await;

        // Record request
        self.rps_controller.record_request().await;

        // Record latency (will be updated when response is received)
        // For now, we record the bottleneck delay
        if bottleneck_delay_ms > 0 {
            self.latency_recorder
                .record(
                    bottleneck_delay_ms,
                    Some(endpoint.to_string()),
                    Some(method.to_string()),
                    None,
                    None,
                )
                .await;
        }

        Ok(())
    }

    /// Record request completion with latency
    pub async fn record_completion(
        &self,
        endpoint: &str,
        method: &str,
        latency_ms: u64,
        status_code: u16,
        error: Option<String>,
    ) {
        self.latency_recorder
            .record(
                latency_ms,
                Some(endpoint.to_string()),
                Some(method.to_string()),
                Some(status_code),
                error,
            )
            .await;
    }

    /// Get current performance snapshot
    pub async fn get_snapshot(&self) -> PerformanceSnapshot {
        let stats = self.latency_analyzer.calculate_stats().await;
        let current_rps = self.rps_controller.get_current_rps().await;
        let target_rps = self.rps_controller.get_target_rps().await;

        let mut metrics = PerformanceMetrics::new();
        metrics.update_from_latency_stats(&stats, current_rps, target_rps);

        // Get active bottlenecks
        let bottlenecks = self.bottleneck_simulator.get_bottlenecks().await;
        let active_bottlenecks: Vec<String> =
            bottlenecks.iter().map(|b| format!("{:?}", b.bottleneck_type)).collect();

        PerformanceSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            metrics,
            active_bottlenecks,
        }
    }

    /// Get RPS controller
    pub fn rps_controller(&self) -> &Arc<RpsController> {
        &self.rps_controller
    }

    /// Get bottleneck simulator
    pub fn bottleneck_simulator(&self) -> &Arc<BottleneckSimulator> {
        &self.bottleneck_simulator
    }

    /// Get latency analyzer
    pub fn latency_analyzer(&self) -> &Arc<LatencyAnalyzer> {
        &self.latency_analyzer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_simulator() {
        let config = SimulatorConfig::new(10.0);
        let simulator = PerformanceSimulator::new(config);

        simulator.start().await;
        assert!(simulator.is_running().await);

        simulator.process_request("/api/users", "GET").await.unwrap();

        let snapshot = simulator.get_snapshot().await;
        // total_requests is u64, so >= 0 is always true - removed redundant assertion

        simulator.stop().await;
        assert!(!simulator.is_running().await);
    }
}
