//! Performance Metrics
//!
//! Tracks and aggregates performance metrics during load simulation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Current RPS
    pub current_rps: f64,
    /// Target RPS
    pub target_rps: f64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// P95 latency (ms)
    pub p95_latency_ms: u64,
    /// P99 latency (ms)
    pub p99_latency_ms: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Metrics by endpoint
    pub endpoint_metrics: HashMap<String, EndpointMetrics>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Endpoint-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointMetrics {
    /// Request count
    pub request_count: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// P95 latency (ms)
    pub p95_latency_ms: u64,
    /// P99 latency (ms)
    pub p99_latency_ms: u64,
    /// Error count
    pub error_count: u64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

/// Performance snapshot
///
/// A snapshot of performance metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Metrics
    pub metrics: PerformanceMetrics,
    /// Active bottlenecks
    pub active_bottlenecks: Vec<String>,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            current_rps: 0.0,
            target_rps: 0.0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            error_rate: 0.0,
            endpoint_metrics: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Update from latency statistics
    pub fn update_from_latency_stats(
        &mut self,
        stats: &crate::latency::LatencyStats,
        current_rps: f64,
        target_rps: f64,
    ) {
        self.total_requests = stats.count as u64;
        self.current_rps = current_rps;
        self.target_rps = target_rps;
        self.avg_latency_ms = stats.avg;
        self.p95_latency_ms = stats.p95;
        self.p99_latency_ms = stats.p99;
        self.error_rate = stats.error_rate;
        self.timestamp = Utc::now();
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointMetrics {
    /// Create new endpoint metrics
    pub fn new() -> Self {
        Self {
            request_count: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            error_count: 0,
            error_rate: 0.0,
        }
    }
}

impl Default for EndpointMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_new() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.current_rps, 0.0);
        assert_eq!(metrics.target_rps, 0.0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
        assert_eq!(metrics.p95_latency_ms, 0);
        assert_eq!(metrics.p99_latency_ms, 0);
        assert_eq!(metrics.error_rate, 0.0);
        assert!(metrics.endpoint_metrics.is_empty());
    }

    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();
        assert_eq!(metrics.total_requests, 0);
    }

    #[test]
    fn test_performance_metrics_clone() {
        let mut metrics = PerformanceMetrics::new();
        metrics.total_requests = 100;
        metrics.current_rps = 50.5;

        let cloned = metrics.clone();
        assert_eq!(cloned.total_requests, 100);
        assert_eq!(cloned.current_rps, 50.5);
    }

    #[test]
    fn test_performance_metrics_debug() {
        let metrics = PerformanceMetrics::new();
        let debug = format!("{:?}", metrics);
        assert!(debug.contains("PerformanceMetrics"));
    }

    #[test]
    fn test_performance_metrics_serialize() {
        let metrics = PerformanceMetrics::new();
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"total_requests\":0"));
        assert!(json.contains("\"current_rps\":0.0"));
    }

    #[test]
    fn test_performance_metrics_deserialize() {
        let json = r#"{
            "total_requests": 100,
            "successful_requests": 95,
            "failed_requests": 5,
            "current_rps": 10.5,
            "target_rps": 15.0,
            "avg_latency_ms": 50.0,
            "p95_latency_ms": 100,
            "p99_latency_ms": 150,
            "error_rate": 0.05,
            "endpoint_metrics": {},
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;

        let metrics: PerformanceMetrics = serde_json::from_str(json).unwrap();
        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.successful_requests, 95);
        assert_eq!(metrics.failed_requests, 5);
        assert_eq!(metrics.current_rps, 10.5);
        assert_eq!(metrics.target_rps, 15.0);
    }

    #[test]
    fn test_performance_metrics_update_from_latency_stats() {
        let mut metrics = PerformanceMetrics::new();
        let stats = crate::latency::LatencyStats {
            count: 100,
            min: 10,
            max: 200,
            avg: 50.0,
            median: 45.0,
            p95: 150,
            p99: 180,
            error_rate: 0.1,
        };

        metrics.update_from_latency_stats(&stats, 25.0, 30.0);

        assert_eq!(metrics.total_requests, 100);
        assert_eq!(metrics.current_rps, 25.0);
        assert_eq!(metrics.target_rps, 30.0);
        assert_eq!(metrics.avg_latency_ms, 50.0);
        assert_eq!(metrics.p95_latency_ms, 150);
        assert_eq!(metrics.p99_latency_ms, 180);
        assert_eq!(metrics.error_rate, 0.1);
    }

    #[test]
    fn test_endpoint_metrics_new() {
        let metrics = EndpointMetrics::new();
        assert_eq!(metrics.request_count, 0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
        assert_eq!(metrics.p95_latency_ms, 0);
        assert_eq!(metrics.p99_latency_ms, 0);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.error_rate, 0.0);
    }

    #[test]
    fn test_endpoint_metrics_default() {
        let metrics = EndpointMetrics::default();
        assert_eq!(metrics.request_count, 0);
    }

    #[test]
    fn test_endpoint_metrics_clone() {
        let mut metrics = EndpointMetrics::new();
        metrics.request_count = 50;
        metrics.avg_latency_ms = 25.5;

        let cloned = metrics.clone();
        assert_eq!(cloned.request_count, 50);
        assert_eq!(cloned.avg_latency_ms, 25.5);
    }

    #[test]
    fn test_endpoint_metrics_debug() {
        let metrics = EndpointMetrics::new();
        let debug = format!("{:?}", metrics);
        assert!(debug.contains("EndpointMetrics"));
    }

    #[test]
    fn test_endpoint_metrics_serialize() {
        let metrics = EndpointMetrics::new();
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"request_count\":0"));
        assert!(json.contains("\"error_rate\":0.0"));
    }

    #[test]
    fn test_performance_snapshot_clone() {
        let snapshot = PerformanceSnapshot {
            id: "test-id".to_string(),
            timestamp: Utc::now(),
            metrics: PerformanceMetrics::new(),
            active_bottlenecks: vec!["Network".to_string()],
        };

        let cloned = snapshot.clone();
        assert_eq!(cloned.id, "test-id");
        assert_eq!(cloned.active_bottlenecks.len(), 1);
    }

    #[test]
    fn test_performance_snapshot_debug() {
        let snapshot = PerformanceSnapshot {
            id: "test-snapshot".to_string(),
            timestamp: Utc::now(),
            metrics: PerformanceMetrics::new(),
            active_bottlenecks: vec![],
        };

        let debug = format!("{:?}", snapshot);
        assert!(debug.contains("PerformanceSnapshot"));
        assert!(debug.contains("test-snapshot"));
    }

    #[test]
    fn test_performance_snapshot_serialize() {
        let snapshot = PerformanceSnapshot {
            id: "snap-1".to_string(),
            timestamp: Utc::now(),
            metrics: PerformanceMetrics::new(),
            active_bottlenecks: vec!["Cpu".to_string(), "Memory".to_string()],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"id\":\"snap-1\""));
        assert!(json.contains("Cpu"));
        assert!(json.contains("Memory"));
    }

    #[test]
    fn test_performance_metrics_with_endpoint_metrics() {
        let mut metrics = PerformanceMetrics::new();

        let mut endpoint_metrics = EndpointMetrics::new();
        endpoint_metrics.request_count = 50;
        endpoint_metrics.avg_latency_ms = 100.0;

        metrics.endpoint_metrics.insert("/api/users".to_string(), endpoint_metrics);

        assert_eq!(metrics.endpoint_metrics.len(), 1);
        assert!(metrics.endpoint_metrics.contains_key("/api/users"));
        assert_eq!(metrics.endpoint_metrics.get("/api/users").unwrap().request_count, 50);
    }
}
