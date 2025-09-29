//! Performance monitoring and optimization utilities
//!
//! This module provides infrastructure for monitoring and optimizing
//! performance in MockForge applications.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance metrics collector
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// Request processing duration histogram
    request_durations: Arc<RwLock<Vec<Duration>>>,
    /// Total number of requests processed
    request_count: AtomicU64,
    /// Number of active concurrent requests
    active_requests: AtomicUsize,
    /// Cache hit/miss statistics
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    /// Memory usage tracking
    memory_usage_bytes: AtomicU64,
    /// Error rates
    error_count: AtomicU64,
    /// Custom metric counters
    custom_counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    /// Create a new performance metrics collector
    pub fn new() -> Self {
        Self {
            request_durations: Arc::new(RwLock::new(Vec::new())),
            request_count: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            memory_usage_bytes: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            custom_counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a request processing duration
    pub async fn record_request_duration(&self, duration: Duration) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        let mut durations = self.request_durations.write().await;
        durations.push(duration);
        
        // Keep only the last 1000 durations to prevent unbounded growth
        if durations.len() > 1000 {
            let drain_count = durations.len() - 1000;
            durations.drain(0..drain_count);
        }
    }

    /// Increment active request count
    pub fn increment_active_requests(&self) -> usize {
        self.active_requests.fetch_add(1, Ordering::Relaxed)
    }

    /// Decrement active request count
    pub fn decrement_active_requests(&self) -> usize {
        self.active_requests.fetch_sub(1, Ordering::Relaxed)
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Increment a custom counter
    pub async fn increment_custom_counter(&self, name: &str) {
        let mut counters = self.custom_counters.write().await;
        let counter = counters.entry(name.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Get performance summary
    pub async fn get_summary(&self) -> PerformanceSummary {
        let durations = self.request_durations.read().await;
        let total_requests = self.request_count.load(Ordering::Relaxed);
        let active_requests = self.active_requests.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let memory_usage = self.memory_usage_bytes.load(Ordering::Relaxed);
        let error_count = self.error_count.load(Ordering::Relaxed);

        // Calculate percentiles
        let mut sorted_durations: Vec<Duration> = durations.clone();
        sorted_durations.sort();

        let (p50, p95, p99) = if !sorted_durations.is_empty() {
            let p50_idx = sorted_durations.len() / 2;
            let p95_idx = (sorted_durations.len() * 95) / 100;
            let p99_idx = (sorted_durations.len() * 99) / 100;
            
            (
                sorted_durations.get(p50_idx).copied(),
                sorted_durations.get(p95_idx).copied(),
                sorted_durations.get(p99_idx).copied(),
            )
        } else {
            (None, None, None)
        };

        let avg_duration = if !sorted_durations.is_empty() {
            Some(Duration::from_nanos(
                sorted_durations.iter().map(|d| d.as_nanos() as u64).sum::<u64>() / sorted_durations.len() as u64
            ))
        } else {
            None
        };

        let cache_hit_rate = if cache_hits + cache_misses > 0 {
            (cache_hits as f64) / ((cache_hits + cache_misses) as f64)
        } else {
            0.0
        };

        let error_rate = if total_requests > 0 {
            (error_count as f64) / (total_requests as f64)
        } else {
            0.0
        };

        PerformanceSummary {
            total_requests,
            active_requests,
            avg_duration,
            p50_duration: p50,
            p95_duration: p95,
            p99_duration: p99,
            cache_hit_rate,
            cache_hits,
            cache_misses,
            memory_usage_bytes: memory_usage,
            error_count,
            error_rate,
        }
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.request_durations.write().await.clear();
        self.request_count.store(0, Ordering::Relaxed);
        self.active_requests.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.memory_usage_bytes.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.custom_counters.write().await.clear();
    }
}

/// Performance summary snapshot
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_requests: u64,
    pub active_requests: usize,
    pub avg_duration: Option<Duration>,
    pub p50_duration: Option<Duration>,
    pub p95_duration: Option<Duration>,
    pub p99_duration: Option<Duration>,
    pub cache_hit_rate: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub memory_usage_bytes: u64,
    pub error_count: u64,
    pub error_rate: f64,
}

/// Performance monitoring guard for automatic duration tracking
pub struct PerformanceGuard {
    start_time: Instant,
    metrics: Arc<PerformanceMetrics>,
    name: Option<String>,
}

impl PerformanceGuard {
    /// Create a new performance guard
    pub fn new(metrics: Arc<PerformanceMetrics>) -> Self {
        metrics.increment_active_requests();
        Self {
            start_time: Instant::now(),
            metrics,
            name: None,
        }
    }

    /// Create a named performance guard
    pub fn named(metrics: Arc<PerformanceMetrics>, name: String) -> Self {
        metrics.increment_active_requests();
        Self {
            start_time: Instant::now(),
            metrics,
            name: Some(name),
        }
    }

    /// Get the elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Drop for PerformanceGuard {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.metrics.decrement_active_requests();
        
        // Record duration asynchronously
        let metrics = self.metrics.clone();
        let name = self.name.clone();
        tokio::spawn(async move {
            metrics.record_request_duration(duration).await;
            if let Some(name) = name {
                metrics.increment_custom_counter(&format!("{}_count", name)).await;
            }
        });
    }
}

/// High-level performance monitoring wrapper
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    metrics: Arc<PerformanceMetrics>,
    enabled: bool,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(PerformanceMetrics::new()),
            enabled: true,
        }
    }

    /// Create a disabled performance monitor (no-op)
    pub fn disabled() -> Self {
        Self {
            metrics: Arc::new(PerformanceMetrics::new()),
            enabled: false,
        }
    }

    /// Enable or disable monitoring
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if monitoring is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start tracking an operation
    pub fn start_tracking(&self) -> Option<PerformanceGuard> {
        if self.enabled {
            Some(PerformanceGuard::new(self.metrics.clone()))
        } else {
            None
        }
    }

    /// Start tracking a named operation
    pub fn start_tracking_named(&self, name: &str) -> Option<PerformanceGuard> {
        if self.enabled {
            Some(PerformanceGuard::named(self.metrics.clone(), name.to_string()))
        } else {
            None
        }
    }

    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        if self.enabled {
            self.metrics.record_cache_hit();
        }
    }

    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        if self.enabled {
            self.metrics.record_cache_miss();
        }
    }

    /// Record an error
    pub fn record_error(&self) {
        if self.enabled {
            self.metrics.record_error();
        }
    }

    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: u64) {
        if self.enabled {
            self.metrics.update_memory_usage(bytes);
        }
    }

    /// Get performance summary
    pub async fn get_summary(&self) -> PerformanceSummary {
        self.metrics.get_summary().await
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        self.metrics.reset().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        // Record some sample data
        metrics.record_request_duration(Duration::from_millis(100)).await;
        metrics.record_request_duration(Duration::from_millis(200)).await;
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_error();
        metrics.update_memory_usage(1024);

        let summary = metrics.get_summary().await;
        
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.cache_hits, 1);
        assert_eq!(summary.cache_misses, 1);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.memory_usage_bytes, 1024);
        assert!((summary.cache_hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_performance_guard() {
        let monitor = PerformanceMonitor::new();
        
        {
            let _guard = monitor.start_tracking();
            sleep(Duration::from_millis(10)).await;
        }
        
        // Give time for async drop to complete
        sleep(Duration::from_millis(50)).await;
        
        let summary = monitor.get_summary().await;
        assert_eq!(summary.total_requests, 1);
        assert_eq!(summary.active_requests, 0);
    }

    #[tokio::test]
    async fn test_disabled_monitor() {
        let monitor = PerformanceMonitor::disabled();
        
        assert!(!monitor.is_enabled());
        assert!(monitor.start_tracking().is_none());
        
        monitor.record_cache_hit();
        monitor.record_error();
        
        let summary = monitor.get_summary().await;
        assert_eq!(summary.total_requests, 0);
        assert_eq!(summary.cache_hits, 0);
        assert_eq!(summary.error_count, 0);
    }
}