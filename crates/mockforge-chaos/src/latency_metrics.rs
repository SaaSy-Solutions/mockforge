//! Latency metrics tracking for real-time visualization

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::SystemTime;

/// Single latency sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySample {
    /// Timestamp in milliseconds since epoch
    pub timestamp: u64,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// Latency metrics tracker
/// Tracks recent latency samples for real-time visualization
#[derive(Debug, Clone)]
pub struct LatencyMetricsTracker {
    /// Recent latency samples (max 1000 samples or 5 minutes)
    samples: Arc<RwLock<VecDeque<LatencySample>>>,
    /// Maximum number of samples to keep
    max_samples: usize,
    /// Maximum age of samples in seconds (5 minutes)
    max_age_seconds: u64,
}

impl LatencyMetricsTracker {
    /// Create a new latency metrics tracker
    pub fn new() -> Self {
        Self {
            samples: Arc::new(RwLock::new(VecDeque::new())),
            max_samples: 1000,
            max_age_seconds: 300, // 5 minutes
        }
    }

    /// Record a latency sample
    pub fn record_latency(&self, latency_ms: u64) {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let sample = LatencySample {
            timestamp: now,
            latency_ms,
        };

        let mut samples = self.samples.write();
        samples.push_back(sample);

        // Clean up old samples
        self.cleanup_old_samples(&mut samples);
    }

    /// Get all latency samples within the time window
    pub fn get_samples(&self) -> Vec<LatencySample> {
        let mut samples = self.samples.write();
        self.cleanup_old_samples(&mut samples);
        samples.iter().cloned().collect()
    }

    /// Get samples within a time range
    pub fn get_samples_in_range(&self, start_ms: u64, end_ms: u64) -> Vec<LatencySample> {
        let samples = self.samples.read();
        samples
            .iter()
            .filter(|s| s.timestamp >= start_ms && s.timestamp <= end_ms)
            .cloned()
            .collect()
    }

    /// Clean up old samples
    fn cleanup_old_samples(&self, samples: &mut VecDeque<LatencySample>) {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let cutoff = now.saturating_sub(self.max_age_seconds * 1000);

        // Remove samples older than cutoff
        while samples.front().map(|s| s.timestamp < cutoff).unwrap_or(false) {
            samples.pop_front();
        }

        // Limit to max_samples
        while samples.len() > self.max_samples {
            samples.pop_front();
        }
    }

    /// Clear all samples
    pub fn clear(&self) {
        let mut samples = self.samples.write();
        samples.clear();
    }

    /// Get statistics about current samples
    pub fn get_stats(&self) -> LatencyStats {
        let samples = self.get_samples();
        if samples.is_empty() {
            return LatencyStats {
                count: 0,
                min_ms: 0,
                max_ms: 0,
                avg_ms: 0.0,
                p50_ms: 0,
                p95_ms: 0,
                p99_ms: 0,
            };
        }

        let mut latencies: Vec<u64> = samples.iter().map(|s| s.latency_ms).collect();
        latencies.sort();

        let count = latencies.len();
        let min_ms = latencies[0];
        let max_ms = latencies[count - 1];
        let sum: u64 = latencies.iter().sum();
        let avg_ms = sum as f64 / count as f64;

        let p50_ms = latencies[count / 2];
        let p95_ms = latencies[(count * 95) / 100];
        let p99_ms = latencies[(count * 99) / 100];

        LatencyStats {
            count,
            min_ms,
            max_ms,
            avg_ms,
            p50_ms,
            p95_ms,
            p99_ms,
        }
    }
}

impl Default for LatencyMetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Latency statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Number of samples
    pub count: usize,
    /// Minimum latency in ms
    pub min_ms: u64,
    /// Maximum latency in ms
    pub max_ms: u64,
    /// Average latency in ms
    pub avg_ms: f64,
    /// 50th percentile (median) latency in ms
    pub p50_ms: u64,
    /// 95th percentile latency in ms
    pub p95_ms: u64,
    /// 99th percentile latency in ms
    pub p99_ms: u64,
}
