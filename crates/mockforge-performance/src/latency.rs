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

    // LatencySample tests
    #[test]
    fn test_latency_sample_clone() {
        let sample = LatencySample {
            timestamp: Utc::now(),
            latency_ms: 100,
            endpoint: Some("/api/users".to_string()),
            method: Some("GET".to_string()),
            status_code: Some(200),
            error: None,
        };

        let cloned = sample.clone();
        assert_eq!(sample.latency_ms, cloned.latency_ms);
        assert_eq!(sample.endpoint, cloned.endpoint);
    }

    #[test]
    fn test_latency_sample_debug() {
        let sample = LatencySample {
            timestamp: Utc::now(),
            latency_ms: 150,
            endpoint: None,
            method: None,
            status_code: None,
            error: None,
        };

        let debug = format!("{:?}", sample);
        assert!(debug.contains("LatencySample"));
        assert!(debug.contains("150"));
    }

    #[test]
    fn test_latency_sample_serialize() {
        let sample = LatencySample {
            timestamp: Utc::now(),
            latency_ms: 200,
            endpoint: Some("/api/test".to_string()),
            method: Some("POST".to_string()),
            status_code: Some(201),
            error: None,
        };

        let json = serde_json::to_string(&sample).unwrap();
        assert!(json.contains("\"latency_ms\":200"));
        assert!(json.contains("POST"));
    }

    #[test]
    fn test_latency_sample_with_error() {
        let sample = LatencySample {
            timestamp: Utc::now(),
            latency_ms: 500,
            endpoint: Some("/api/users".to_string()),
            method: Some("GET".to_string()),
            status_code: Some(500),
            error: Some("Internal Server Error".to_string()),
        };

        assert!(sample.error.is_some());
        assert_eq!(sample.status_code, Some(500));
    }

    // LatencyRecorder tests
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
    async fn test_latency_recorder_debug() {
        let recorder = LatencyRecorder::new(100, 60);
        let debug = format!("{:?}", recorder);
        assert!(debug.contains("LatencyRecorder"));
    }

    #[tokio::test]
    async fn test_latency_recorder_clone() {
        let recorder = LatencyRecorder::new(100, 60);
        let _cloned = recorder.clone();
    }

    #[tokio::test]
    async fn test_latency_recorder_sample_count() {
        let recorder = LatencyRecorder::new(100, 300);

        assert_eq!(recorder.sample_count().await, 0);

        recorder.record(100, None, None, None, None).await;
        assert_eq!(recorder.sample_count().await, 1);

        recorder.record(200, None, None, None, None).await;
        assert_eq!(recorder.sample_count().await, 2);
    }

    #[tokio::test]
    async fn test_latency_recorder_clear() {
        let recorder = LatencyRecorder::new(100, 300);

        recorder.record(100, None, None, None, None).await;
        recorder.record(200, None, None, None, None).await;

        assert_eq!(recorder.sample_count().await, 2);

        recorder.clear().await;

        assert_eq!(recorder.sample_count().await, 0);
    }

    #[tokio::test]
    async fn test_latency_recorder_get_samples_for_endpoint() {
        let recorder = LatencyRecorder::new(100, 300);

        recorder.record(100, Some("/api/users".to_string()), None, None, None).await;
        recorder.record(150, Some("/api/users".to_string()), None, None, None).await;
        recorder.record(200, Some("/api/orders".to_string()), None, None, None).await;

        let user_samples = recorder.get_samples_for_endpoint("/api/users").await;
        assert_eq!(user_samples.len(), 2);

        let order_samples = recorder.get_samples_for_endpoint("/api/orders").await;
        assert_eq!(order_samples.len(), 1);

        let unknown_samples = recorder.get_samples_for_endpoint("/api/unknown").await;
        assert_eq!(unknown_samples.len(), 0);
    }

    #[tokio::test]
    async fn test_latency_recorder_get_samples_in_range() {
        let recorder = LatencyRecorder::new(100, 300);

        let now = Utc::now();

        recorder.record(100, None, None, None, None).await;
        recorder.record(200, None, None, None, None).await;

        let start = now - chrono::Duration::seconds(1);
        let end = now + chrono::Duration::seconds(1);

        let samples = recorder.get_samples_in_range(start, end).await;
        assert_eq!(samples.len(), 2);
    }

    #[tokio::test]
    async fn test_latency_recorder_max_samples() {
        let recorder = LatencyRecorder::new(5, 300);

        for i in 0..10 {
            recorder.record(i * 10, None, None, None, None).await;
        }

        let samples = recorder.get_samples().await;
        assert!(samples.len() <= 5);
    }

    // LatencyAnalyzer tests
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

    #[test]
    fn test_latency_analyzer_debug() {
        let recorder = Arc::new(LatencyRecorder::new(100, 60));
        let analyzer = LatencyAnalyzer::new(recorder);
        let debug = format!("{:?}", analyzer);
        assert!(debug.contains("LatencyAnalyzer"));
    }

    #[test]
    fn test_latency_analyzer_clone() {
        let recorder = Arc::new(LatencyRecorder::new(100, 60));
        let analyzer = LatencyAnalyzer::new(recorder);
        let _cloned = analyzer.clone();
    }

    #[tokio::test]
    async fn test_latency_analyzer_empty_stats() {
        let recorder = Arc::new(LatencyRecorder::new(100, 300));
        let analyzer = LatencyAnalyzer::new(recorder);

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.count, 0);
        assert_eq!(stats.min, 0);
        assert_eq!(stats.max, 0);
        assert_eq!(stats.avg, 0.0);
    }

    #[tokio::test]
    async fn test_latency_analyzer_calculate_stats_for_endpoint() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        recorder.record(100, Some("/api/users".to_string()), None, None, None).await;
        recorder.record(200, Some("/api/users".to_string()), None, None, None).await;
        recorder.record(500, Some("/api/orders".to_string()), None, None, None).await;

        let user_stats = analyzer.calculate_stats_for_endpoint("/api/users").await;
        assert_eq!(user_stats.count, 2);
        assert_eq!(user_stats.min, 100);
        assert_eq!(user_stats.max, 200);
    }

    #[tokio::test]
    async fn test_latency_analyzer_error_rate() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        // 3 successful requests
        recorder.record(100, None, None, Some(200), None).await;
        recorder.record(100, None, None, Some(200), None).await;
        recorder.record(100, None, None, Some(200), None).await;

        // 2 failed requests
        recorder.record(100, None, None, Some(500), None).await;
        recorder.record(100, None, None, Some(404), None).await;

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.count, 5);
        assert_eq!(stats.error_rate, 0.4); // 2/5 = 0.4
    }

    #[tokio::test]
    async fn test_latency_analyzer_error_rate_with_error_message() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        recorder.record(100, None, None, None, None).await;
        recorder.record(100, None, None, None, Some("Timeout".to_string())).await;

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.error_rate, 0.5); // 1/2 = 0.5
    }

    // LatencyStats tests
    #[test]
    fn test_latency_stats_default() {
        let stats = LatencyStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.min, 0);
        assert_eq!(stats.max, 0);
        assert_eq!(stats.avg, 0.0);
        assert_eq!(stats.median, 0.0);
        assert_eq!(stats.p95, 0);
        assert_eq!(stats.p99, 0);
        assert_eq!(stats.error_rate, 0.0);
    }

    #[test]
    fn test_latency_stats_clone() {
        let stats = LatencyStats {
            count: 100,
            min: 10,
            max: 500,
            avg: 150.0,
            median: 140.0,
            p95: 400,
            p99: 480,
            error_rate: 0.05,
        };

        let cloned = stats.clone();
        assert_eq!(stats.count, cloned.count);
        assert_eq!(stats.min, cloned.min);
        assert_eq!(stats.avg, cloned.avg);
    }

    #[test]
    fn test_latency_stats_debug() {
        let stats = LatencyStats::default();
        let debug = format!("{:?}", stats);
        assert!(debug.contains("LatencyStats"));
    }

    #[test]
    fn test_latency_stats_serialize() {
        let stats = LatencyStats {
            count: 50,
            min: 10,
            max: 200,
            avg: 100.0,
            median: 95.0,
            p95: 180,
            p99: 195,
            error_rate: 0.02,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"count\":50"));
        assert!(json.contains("\"p95\":180"));
    }

    #[tokio::test]
    async fn test_latency_analyzer_percentiles() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        // Record 100 samples from 1 to 100
        for i in 1..=100 {
            recorder.record(i, None, None, None, None).await;
        }

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.count, 100);
        assert_eq!(stats.min, 1);
        assert_eq!(stats.max, 100);
        // p95 should be around 95
        assert!(stats.p95 >= 90);
        // p99 should be around 99
        assert!(stats.p99 >= 95);
    }

    #[tokio::test]
    async fn test_latency_analyzer_median_odd_count() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        // 5 samples - median should be the middle value
        for latency in [10, 20, 30, 40, 50] {
            recorder.record(latency, None, None, None, None).await;
        }

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.median, 30.0);
    }

    #[tokio::test]
    async fn test_latency_analyzer_median_even_count() {
        let recorder = Arc::new(LatencyRecorder::new(1000, 300));
        let analyzer = LatencyAnalyzer::new(recorder.clone());

        // 4 samples - median should be average of middle two
        for latency in [10, 20, 30, 40] {
            recorder.record(latency, None, None, None, None).await;
        }

        let stats = analyzer.calculate_stats().await;
        assert_eq!(stats.median, 25.0); // (20+30)/2
    }
}
