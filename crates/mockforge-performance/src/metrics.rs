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
