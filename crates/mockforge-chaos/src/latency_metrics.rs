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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_latency_metrics_tracker_new() {
        let tracker = LatencyMetricsTracker::new();
        let samples = tracker.get_samples();
        assert_eq!(samples.len(), 0);
    }

    #[test]
    fn test_latency_metrics_tracker_default() {
        let tracker = LatencyMetricsTracker::default();
        let samples = tracker.get_samples();
        assert_eq!(samples.len(), 0);
    }

    #[test]
    fn test_record_single_latency() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);

        let samples = tracker.get_samples();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].latency_ms, 100);
    }

    #[test]
    fn test_record_multiple_latencies() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);
        tracker.record_latency(200);
        tracker.record_latency(150);

        let samples = tracker.get_samples();
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].latency_ms, 100);
        assert_eq!(samples[1].latency_ms, 200);
        assert_eq!(samples[2].latency_ms, 150);
    }

    #[test]
    fn test_clear_samples() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);
        tracker.record_latency(200);

        assert_eq!(tracker.get_samples().len(), 2);

        tracker.clear();
        assert_eq!(tracker.get_samples().len(), 0);
    }

    #[test]
    fn test_get_stats_empty() {
        let tracker = LatencyMetricsTracker::new();
        let stats = tracker.get_stats();

        assert_eq!(stats.count, 0);
        assert_eq!(stats.min_ms, 0);
        assert_eq!(stats.max_ms, 0);
        assert_eq!(stats.avg_ms, 0.0);
        assert_eq!(stats.p50_ms, 0);
        assert_eq!(stats.p95_ms, 0);
        assert_eq!(stats.p99_ms, 0);
    }

    #[test]
    fn test_get_stats_single_sample() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);

        let stats = tracker.get_stats();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.min_ms, 100);
        assert_eq!(stats.max_ms, 100);
        assert_eq!(stats.avg_ms, 100.0);
        assert_eq!(stats.p50_ms, 100);
        assert_eq!(stats.p95_ms, 100);
        assert_eq!(stats.p99_ms, 100);
    }

    #[test]
    fn test_get_stats_multiple_samples() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);
        tracker.record_latency(200);
        tracker.record_latency(150);
        tracker.record_latency(300);
        tracker.record_latency(50);

        let stats = tracker.get_stats();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.min_ms, 50);
        assert_eq!(stats.max_ms, 300);
        assert_eq!(stats.avg_ms, 160.0);
    }

    #[test]
    fn test_get_stats_percentiles() {
        let tracker = LatencyMetricsTracker::new();
        // Add 100 samples from 1 to 100
        for i in 1..=100 {
            tracker.record_latency(i);
        }

        let stats = tracker.get_stats();
        assert_eq!(stats.count, 100);
        assert_eq!(stats.min_ms, 1);
        assert_eq!(stats.max_ms, 100);
        // For 100 samples [1..=100], p50 = arr[50] = 51 (0-indexed)
        // p95 = arr[95] = 96, p99 = arr[99] = 100
        assert_eq!(stats.p50_ms, 51); // Median (index 50)
        assert_eq!(stats.p95_ms, 96); // 95th percentile (index 95)
        assert_eq!(stats.p99_ms, 100); // 99th percentile (index 99)
    }

    #[test]
    fn test_latency_sample_serialize() {
        let sample = LatencySample {
            timestamp: 1234567890,
            latency_ms: 100,
        };

        let json = serde_json::to_value(&sample).unwrap();
        assert_eq!(json["timestamp"], 1234567890u64);
        assert_eq!(json["latency_ms"], 100);
    }

    #[test]
    fn test_latency_sample_deserialize() {
        let json = serde_json::json!({
            "timestamp": 1234567890u64,
            "latency_ms": 200
        });

        let sample: LatencySample = serde_json::from_value(json).unwrap();
        assert_eq!(sample.timestamp, 1234567890);
        assert_eq!(sample.latency_ms, 200);
    }

    #[test]
    fn test_latency_stats_serialize() {
        let stats = LatencyStats {
            count: 100,
            min_ms: 10,
            max_ms: 500,
            avg_ms: 150.5,
            p50_ms: 140,
            p95_ms: 450,
            p99_ms: 490,
        };

        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["count"], 100);
        assert_eq!(json["min_ms"], 10);
        assert_eq!(json["max_ms"], 500);
        assert_eq!(json["avg_ms"], 150.5);
        assert_eq!(json["p50_ms"], 140);
        assert_eq!(json["p95_ms"], 450);
        assert_eq!(json["p99_ms"], 490);
    }

    #[test]
    fn test_latency_stats_deserialize() {
        let json = serde_json::json!({
            "count": 50,
            "min_ms": 20,
            "max_ms": 300,
            "avg_ms": 120.3,
            "p50_ms": 110,
            "p95_ms": 280,
            "p99_ms": 295
        });

        let stats: LatencyStats = serde_json::from_value(json).unwrap();
        assert_eq!(stats.count, 50);
        assert_eq!(stats.min_ms, 20);
        assert_eq!(stats.max_ms, 300);
        assert_eq!(stats.avg_ms, 120.3);
        assert_eq!(stats.p50_ms, 110);
        assert_eq!(stats.p95_ms, 280);
        assert_eq!(stats.p99_ms, 295);
    }

    #[test]
    fn test_get_samples_in_range() {
        let tracker = LatencyMetricsTracker::new();

        let now =
            SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

        // Record samples with different timestamps
        tracker.record_latency(100);
        thread::sleep(Duration::from_millis(10));
        tracker.record_latency(200);
        thread::sleep(Duration::from_millis(10));
        tracker.record_latency(300);

        let all_samples = tracker.get_samples();
        assert_eq!(all_samples.len(), 3);

        // Get samples in a range that should include all
        let start = now - 1000;
        let end = now + 1000;
        let range_samples = tracker.get_samples_in_range(start, end);
        assert_eq!(range_samples.len(), 3);
    }

    #[test]
    fn test_get_samples_in_range_empty() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);

        // Query a range in the past that shouldn't include any samples
        let samples = tracker.get_samples_in_range(0, 1000);
        assert_eq!(samples.len(), 0);
    }

    #[test]
    fn test_tracker_clone() {
        let tracker1 = LatencyMetricsTracker::new();
        tracker1.record_latency(100);

        let tracker2 = tracker1.clone();
        let samples = tracker2.get_samples();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].latency_ms, 100);

        // Both trackers should share the same underlying data
        tracker2.record_latency(200);
        let samples1 = tracker1.get_samples();
        assert_eq!(samples1.len(), 2);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;

        let tracker = Arc::new(LatencyMetricsTracker::new());
        let mut handles = vec![];

        // Spawn multiple threads that record latencies
        for i in 0..5 {
            let tracker_clone = tracker.clone();
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    tracker_clone.record_latency((i * 10 + j) as u64);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 50 samples total
        let samples = tracker.get_samples();
        assert_eq!(samples.len(), 50);
    }

    #[test]
    fn test_edge_case_zero_latency() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(0);

        let stats = tracker.get_stats();
        assert_eq!(stats.min_ms, 0);
        assert_eq!(stats.max_ms, 0);
        assert_eq!(stats.avg_ms, 0.0);
    }

    #[test]
    fn test_edge_case_large_latency() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(u64::MAX);

        let stats = tracker.get_stats();
        assert_eq!(stats.min_ms, u64::MAX);
        assert_eq!(stats.max_ms, u64::MAX);
    }

    #[test]
    fn test_avg_calculation_precision() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);
        tracker.record_latency(200);
        tracker.record_latency(300);

        let stats = tracker.get_stats();
        assert_eq!(stats.avg_ms, 200.0);
    }

    #[test]
    fn test_percentile_calculation_small_dataset() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(100);
        tracker.record_latency(200);

        let stats = tracker.get_stats();
        assert_eq!(stats.count, 2);
        assert!(stats.p50_ms >= 100 && stats.p50_ms <= 200);
    }

    #[test]
    fn test_samples_ordering() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(300);
        tracker.record_latency(100);
        tracker.record_latency(200);

        let samples = tracker.get_samples();
        // Samples should be returned in the order they were recorded
        assert_eq!(samples[0].latency_ms, 300);
        assert_eq!(samples[1].latency_ms, 100);
        assert_eq!(samples[2].latency_ms, 200);
    }

    #[test]
    fn test_stats_sorted_internally() {
        let tracker = LatencyMetricsTracker::new();
        tracker.record_latency(300);
        tracker.record_latency(100);
        tracker.record_latency(200);

        let stats = tracker.get_stats();
        // Stats should use sorted values
        assert_eq!(stats.min_ms, 100);
        assert_eq!(stats.max_ms, 300);
        assert_eq!(stats.p50_ms, 200); // Median of [100, 200, 300]
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_sample() {
        let original = LatencySample {
            timestamp: 1234567890,
            latency_ms: 150,
        };

        let json = serde_json::to_value(&original).unwrap();
        let deserialized: LatencySample = serde_json::from_value(json).unwrap();

        assert_eq!(original.timestamp, deserialized.timestamp);
        assert_eq!(original.latency_ms, deserialized.latency_ms);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_stats() {
        let original = LatencyStats {
            count: 100,
            min_ms: 10,
            max_ms: 500,
            avg_ms: 150.5,
            p50_ms: 140,
            p95_ms: 450,
            p99_ms: 490,
        };

        let json = serde_json::to_value(&original).unwrap();
        let deserialized: LatencyStats = serde_json::from_value(json).unwrap();

        assert_eq!(original.count, deserialized.count);
        assert_eq!(original.min_ms, deserialized.min_ms);
        assert_eq!(original.max_ms, deserialized.max_ms);
        assert_eq!(original.avg_ms, deserialized.avg_ms);
    }
}
