//! RPS (Requests Per Second) Controller
//!
//! Controls the rate of requests to simulate load at a specific RPS.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::debug;

/// RPS (Requests Per Second) controller
///
/// Controls request rate to maintain a target RPS.
#[derive(Debug, Clone)]
pub struct RpsController {
    /// Target RPS
    target_rps: Arc<RwLock<f64>>,
    /// Current RPS (calculated)
    current_rps: Arc<RwLock<f64>>,
    /// Request counter
    request_count: Arc<RwLock<u64>>,
    /// Last reset time
    last_reset: Arc<RwLock<Instant>>,
    /// Minimum interval between requests (calculated from RPS)
    min_interval: Arc<RwLock<Duration>>,
}

impl RpsController {
    /// Create a new RPS controller
    pub fn new(target_rps: f64) -> Self {
        let min_interval = if target_rps > 0.0 {
            Duration::from_secs_f64(1.0 / target_rps)
        } else {
            Duration::from_secs(0)
        };

        Self {
            target_rps: Arc::new(RwLock::new(target_rps)),
            current_rps: Arc::new(RwLock::new(0.0)),
            request_count: Arc::new(RwLock::new(0)),
            last_reset: Arc::new(RwLock::new(Instant::now())),
            min_interval: Arc::new(RwLock::new(min_interval)),
        }
    }

    /// Set target RPS
    pub async fn set_target_rps(&self, rps: f64) {
        let mut target = self.target_rps.write().await;
        *target = rps;

        let min_interval = if rps > 0.0 {
            Duration::from_secs_f64(1.0 / rps)
        } else {
            Duration::from_secs(0)
        };

        let mut interval = self.min_interval.write().await;
        *interval = min_interval;

        debug!("RPS controller: target RPS set to {}", rps);
    }

    /// Get target RPS
    pub async fn get_target_rps(&self) -> f64 {
        *self.target_rps.read().await
    }

    /// Get current RPS (calculated over last second)
    pub async fn get_current_rps(&self) -> f64 {
        *self.current_rps.read().await
    }

    /// Wait for next request slot (rate limiting)
    ///
    /// This will sleep if necessary to maintain the target RPS.
    pub async fn wait_for_slot(&self) {
        let min_interval = *self.min_interval.read().await;
        if min_interval.is_zero() {
            return; // No rate limiting
        }

        // Simple rate limiting: sleep for minimum interval
        sleep(min_interval).await;
    }

    /// Record a request
    ///
    /// Updates the current RPS calculation.
    pub async fn record_request(&self) {
        let mut count = self.request_count.write().await;
        *count += 1;

        // Calculate current RPS over last second
        let now = Instant::now();
        let mut last_reset = self.last_reset.write().await;
        let elapsed = now.duration_since(*last_reset);

        if elapsed >= Duration::from_secs(1) {
            // Reset and calculate RPS
            let rps = *count as f64 / elapsed.as_secs_f64();
            let mut current = self.current_rps.write().await;
            *current = rps;

            *count = 0;
            *last_reset = now;

            debug!("RPS controller: current RPS = {:.2}", rps);
        }
    }

    /// Get request count since last reset
    pub async fn get_request_count(&self) -> u64 {
        *self.request_count.read().await
    }
}

/// RPS Profile for dynamic RPS changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpsProfile {
    /// Profile name
    pub name: String,
    /// RPS stages (time in seconds, target RPS)
    pub stages: Vec<RpsStage>,
}

/// RPS Stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpsStage {
    /// Duration in seconds
    pub duration_secs: u64,
    /// Target RPS for this stage
    pub target_rps: f64,
    /// Stage name/description
    pub name: Option<String>,
}

impl RpsProfile {
    /// Create a simple constant RPS profile
    pub fn constant(rps: f64) -> Self {
        Self {
            name: format!("Constant {} RPS", rps),
            stages: vec![RpsStage {
                duration_secs: 0, // Infinite
                target_rps: rps,
                name: Some("Constant".to_string()),
            }],
        }
    }

    /// Create a ramp-up profile
    pub fn ramp_up(start_rps: f64, end_rps: f64, duration_secs: u64) -> Self {
        let steps = (duration_secs / 10).max(1); // 10 second steps
        let rps_step = (end_rps - start_rps) / steps as f64;

        let mut stages = Vec::new();
        for i in 0..steps {
            let current_rps = start_rps + (i as f64 * rps_step);
            stages.push(RpsStage {
                duration_secs: 10,
                target_rps: current_rps,
                name: Some(format!("Ramp {} -> {}", current_rps, current_rps + rps_step)),
            });
        }

        Self {
            name: format!("Ramp up {} -> {} RPS", start_rps, end_rps),
            stages,
        }
    }

    /// Create a spike profile
    pub fn spike(base_rps: f64, spike_rps: f64, spike_duration_secs: u64) -> Self {
        Self {
            name: format!("Spike {} -> {} RPS", base_rps, spike_rps),
            stages: vec![
                RpsStage {
                    duration_secs: 30,
                    target_rps: base_rps,
                    name: Some("Base".to_string()),
                },
                RpsStage {
                    duration_secs: spike_duration_secs,
                    target_rps: spike_rps,
                    name: Some("Spike".to_string()),
                },
                RpsStage {
                    duration_secs: 30,
                    target_rps: base_rps,
                    name: Some("Recovery".to_string()),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rps_controller() {
        let controller = RpsController::new(10.0);
        assert_eq!(controller.get_target_rps().await, 10.0);

        // Record some requests
        for _ in 0..5 {
            controller.record_request().await;
        }

        // Should have recorded requests
        assert!(controller.get_request_count().await > 0);
    }

    #[test]
    fn test_rps_profile_constant() {
        let profile = RpsProfile::constant(100.0);
        assert_eq!(profile.stages.len(), 1);
        assert_eq!(profile.stages[0].target_rps, 100.0);
    }

    #[test]
    fn test_rps_profile_ramp_up() {
        let profile = RpsProfile::ramp_up(10.0, 100.0, 60);
        assert!(!profile.stages.is_empty());
        assert_eq!(profile.stages[0].target_rps, 10.0);
    }

    #[test]
    fn test_rps_profile_spike() {
        let profile = RpsProfile::spike(50.0, 200.0, 10);
        assert_eq!(profile.stages.len(), 3);
        assert_eq!(profile.stages[0].target_rps, 50.0);
        assert_eq!(profile.stages[1].target_rps, 200.0);
        assert_eq!(profile.stages[2].target_rps, 50.0);
    }
}
