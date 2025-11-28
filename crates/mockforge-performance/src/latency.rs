//! Latency Recording and Analysis
//!
//! Records request latencies and provides analysis capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Latency sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySample {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Endpoint/path
    pub endpoint: Option<String>,
    /// HTTP method
    pub method: Option<String>,
    /// Status code
    pub status_code: Option<u16>,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Latency recorder
///
/// Records latency samples and maintains statistics.
#[derive(Debug, Clone)]
pub struct LatencyRecorder {
    /// Latency samples
    samples: Arc<RwLock<VecDeque<LatencySample>>>,
    /// Maximum number of samples to keep
    max_samples: usize,
    /// Maximum age of samples in seconds
    max_age_seconds: u64,
}

impl LatencyRecorder {
    /// Create a new latency recorder
    pub fn new(max_samples: usize, max_age_seconds: u64) -> Self {
        Self {
            samples: Arc::new(RwLock::new(VecDeque::new())),
            max_samples,
            max_age_seconds,
        }
    }

    /// Record a latency sample
    pub async fn record(
        &self,
        latency_ms: u64,
        endpoint: Option<String>,
        method: Option<String>,
        status_code: Option<u16>,
        error: Option<String>,
    ) {
        let sample = LatencySample {
            timestamp: Utc::now(),
            latency_ms,
            endpoint,
            method,
            status_code,
            error,
        };

        let mut samples = self.samples.write().await;
        samples.push_back(sample);

        // Clean up old samples
        self.cleanup_old_samples(&mut samples).await;
    }

    /// Get all samples
    pub async fn get_samples(&self) -> Vec<LatencySample> {
        let mut samples = self.samples.write().await;
        self.cleanup_old_samples(&mut samples).await;
        samples.iter().cloned().collect()
    }

    /// Get samples for a specific endpoint
    pub async fn get_samples_for_endpoint(&self, endpoint: &str) -> Vec<LatencySample> {
        let samples = self.get_samples().await;
        samples
            .into_iter()
            .filter(|s| s.endpoint.as_ref().map(|e| e == endpoint).unwrap_or(false))
            .collect()
    }

    /// Get samples within a time range
    pub async fn get_samples_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<LatencySample> {
        let samples = self.get_samples().await;
        samples
            .into_iter()
            .filter(|s| s.timestamp >= start && s.timestamp <= end)
            .collect()
    }

    /// Clean up old samples
    async fn cleanup_old_samples(&self, samples: &mut VecDeque<LatencySample>) {
        let now = Utc::now();
        let cutoff = now
            .checked_sub_signed(chrono::Duration::seconds(self.max_age_seconds as i64))
            .unwrap_or(now);

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
    pub async fn clear(&self) {
        let mut samples = self.samples.write().await;
        samples.clear();
    }

    /// Get sample count
    pub async fn sample_count(&self) -> usize {
        let samples = self.samples.read().await;
        samples.len()
    }
}

/// Latency analyzer
///
/// Analyzes latency samples and provides statistics.
#[derive(Debug, Clone)]
pub struct LatencyAnalyzer {
    recorder: Arc<LatencyRecorder>,
}

impl LatencyAnalyzer {
    /// Create a new latency analyzer
    pub fn new(recorder: Arc<LatencyRecorder>) -> Self {
        Self { recorder }
    }

    /// Calculate latency statistics
    pub async fn calculate_stats(&self) -> LatencyStats {
        let samples = self.recorder.get_samples().await;
        self.calculate_stats_from_samples(&samples)
    }

    /// Calculate statistics for a specific endpoint
    pub async fn calculate_stats_for_endpoint(&self, endpoint: &str) -> LatencyStats {
        let samples = self.recorder.get_samples_for_endpoint(endpoint).await;
        self.calculate_stats_from_samples(&samples)
    }

    /// Calculate statistics from samples
    fn calculate_stats_from_samples(&self, samples: &[LatencySample]) -> LatencyStats {
        if samples.is_empty() {
            return LatencyStats::default();
        }

        let mut latencies: Vec<u64> = samples.iter().map(|s| s.latency_ms).collect();
        latencies.sort();

        let count = latencies.len();
        let sum: u64 = latencies.iter().sum();
        let avg = sum as f64 / count as f64;

        let min = latencies[0];
        let max = latencies[count - 1];
        let median = if count % 2 == 0 {
            (latencies[count / 2 - 1] + latencies[count / 2]) as f64 / 2.0
        } else {
            latencies[count / 2] as f64
        };

        let p95 = if count > 0 {
            latencies[(count as f64 * 0.95) as usize]
        } else {
            0
        };

        let p99 = if count > 0 {
            latencies[(count as f64 * 0.99) as usize]
        } else {
            0
        };

        // Calculate error rate
        let error_count = samples
            .iter()
            .filter(|s| s.error.is_some() || s.status_code.map(|c| c >= 400).unwrap_or(false))
            .count();
        let error_rate = error_count as f64 / count as f64;

        LatencyStats {
            count,
            min,
            max,
            avg,
            median,
            p95,
            p99,
            error_rate,
        }
    }
}

/// Latency statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LatencyStats {
    /// Sample count
    pub count: usize,
    /// Minimum latency (ms)
    pub min: u64,
    /// Maximum latency (ms)
    pub max: u64,
    /// Average latency (ms)
    pub avg: f64,
    /// Median latency (ms)
    pub median: f64,
    /// P95 latency (ms)
    pub p95: u64,
    /// P99 latency (ms)
    pub p99: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_latency_recorder() {
        let recorder = LatencyRecorder::new(1000, 300);

        recorder
            .record(100, Some("/api/users".to_string()), Some("GET".to_string()), Some(200), None)
            .await;
        recorder
            .record(150, Some("/api/users".to_string()), Some("GET".to_string()), Some(200), None)
            .await;

        let samples = recorder.get_samples().await;
        assert_eq!(samples.len(), 2);
    }

    #[tokio::test]
    async fn test_latency_analyzer() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        // Record some samples
        for latency in [100, 150, 200, 250, 300] {
            recorder.record(latency, None, None, None, None).await;
        }

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, 100);
        assert_eq!(stats.max, 300);
    }
}
