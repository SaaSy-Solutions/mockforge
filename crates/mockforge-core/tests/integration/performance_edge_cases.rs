//! Edge case tests for performance module
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for performance metrics collection.

use mockforge_core::performance::PerformanceMetrics;
use std::time::Duration;

/// Test PerformanceMetrics new (tested through get_summary)
#[tokio::test]
async fn test_performance_metrics_new() {
    let metrics = PerformanceMetrics::new();
    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 0);
    assert_eq!(summary.active_requests, 0);
    assert_eq!(summary.cache_hits, 0);
    assert_eq!(summary.cache_misses, 0);
    assert_eq!(summary.memory_usage_bytes, 0);
    assert_eq!(summary.error_count, 0);
}

/// Test PerformanceMetrics default (tested through get_summary)
#[tokio::test]
async fn test_performance_metrics_default() {
    let metrics = PerformanceMetrics::default();
    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 0);
}

/// Test PerformanceMetrics increment_active_requests
#[tokio::test]
async fn test_performance_metrics_increment_active_requests() {
    let metrics = PerformanceMetrics::new();
    assert_eq!(metrics.increment_active_requests(), 0);
    let summary = metrics.get_summary().await;
    assert_eq!(summary.active_requests, 1);

    assert_eq!(metrics.increment_active_requests(), 1);
    let summary2 = metrics.get_summary().await;
    assert_eq!(summary2.active_requests, 2);
}

/// Test PerformanceMetrics decrement_active_requests
#[tokio::test]
async fn test_performance_metrics_decrement_active_requests() {
    let metrics = PerformanceMetrics::new();
    metrics.increment_active_requests();
    metrics.increment_active_requests();

    assert_eq!(metrics.decrement_active_requests(), 2);
    let summary = metrics.get_summary().await;
    assert_eq!(summary.active_requests, 1);

    assert_eq!(metrics.decrement_active_requests(), 1);
    let summary2 = metrics.get_summary().await;
    assert_eq!(summary2.active_requests, 0);
}

/// Test PerformanceMetrics record_cache_hit
#[tokio::test]
async fn test_performance_metrics_record_cache_hit() {
    let metrics = PerformanceMetrics::new();
    metrics.record_cache_hit();
    metrics.record_cache_hit();

    let summary = metrics.get_summary().await;
    assert_eq!(summary.cache_hits, 2);
}

/// Test PerformanceMetrics record_cache_miss
#[tokio::test]
async fn test_performance_metrics_record_cache_miss() {
    let metrics = PerformanceMetrics::new();
    metrics.record_cache_miss();
    metrics.record_cache_miss();

    let summary = metrics.get_summary().await;
    assert_eq!(summary.cache_misses, 2);
}

/// Test PerformanceMetrics record_error
#[tokio::test]
async fn test_performance_metrics_record_error() {
    let metrics = PerformanceMetrics::new();
    metrics.record_error();
    metrics.record_error();

    let summary = metrics.get_summary().await;
    assert_eq!(summary.error_count, 2);
}

/// Test PerformanceMetrics update_memory_usage
#[tokio::test]
async fn test_performance_metrics_update_memory_usage() {
    let metrics = PerformanceMetrics::new();
    metrics.update_memory_usage(1024);
    let summary = metrics.get_summary().await;
    assert_eq!(summary.memory_usage_bytes, 1024);

    metrics.update_memory_usage(2048);
    let summary2 = metrics.get_summary().await;
    assert_eq!(summary2.memory_usage_bytes, 2048);
}

/// Test PerformanceMetrics update_memory_usage with zero
#[tokio::test]
async fn test_performance_metrics_update_memory_usage_zero() {
    let metrics = PerformanceMetrics::new();
    metrics.update_memory_usage(0);
    let summary = metrics.get_summary().await;
    assert_eq!(summary.memory_usage_bytes, 0);
}

/// Test PerformanceMetrics update_memory_usage with large value
#[tokio::test]
async fn test_performance_metrics_update_memory_usage_large() {
    let metrics = PerformanceMetrics::new();
    metrics.update_memory_usage(u64::MAX);
    let summary = metrics.get_summary().await;
    assert_eq!(summary.memory_usage_bytes, u64::MAX);
}

/// Test PerformanceMetrics record_request_duration with empty
#[tokio::test]
async fn test_performance_metrics_record_request_duration_empty() {
    let metrics = PerformanceMetrics::new();
    metrics.record_request_duration(Duration::from_millis(100)).await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 1);
}

/// Test PerformanceMetrics record_request_duration multiple
#[tokio::test]
async fn test_performance_metrics_record_request_duration_multiple() {
    let metrics = PerformanceMetrics::new();

    for i in 0..10 {
        metrics.record_request_duration(Duration::from_millis(i * 10)).await;
    }

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 10);
}

/// Test PerformanceMetrics record_request_duration with zero duration
#[tokio::test]
async fn test_performance_metrics_record_request_duration_zero() {
    let metrics = PerformanceMetrics::new();
    metrics.record_request_duration(Duration::from_secs(0)).await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 1);
}

/// Test PerformanceMetrics record_request_duration with very long duration
#[tokio::test]
async fn test_performance_metrics_record_request_duration_long() {
    let metrics = PerformanceMetrics::new();
    metrics.record_request_duration(Duration::from_secs(3600)).await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 1);
}

/// Test PerformanceMetrics get_summary with no data
#[tokio::test]
async fn test_performance_metrics_get_summary_empty() {
    let metrics = PerformanceMetrics::new();
    let summary = metrics.get_summary().await;

    assert_eq!(summary.total_requests, 0);
    assert_eq!(summary.active_requests, 0);
    assert_eq!(summary.cache_hit_rate, 0.0);
    assert_eq!(summary.error_rate, 0.0);
    assert!(summary.p50_duration.is_none());
    assert!(summary.p95_duration.is_none());
    assert!(summary.p99_duration.is_none());
    assert!(summary.avg_duration.is_none());
}

/// Test PerformanceMetrics get_summary with data
#[tokio::test]
async fn test_performance_metrics_get_summary_with_data() {
    let metrics = PerformanceMetrics::new();

    metrics.record_request_duration(Duration::from_millis(100)).await;
    metrics.record_request_duration(Duration::from_millis(200)).await;
    metrics.record_request_duration(Duration::from_millis(300)).await;

    metrics.record_cache_hit();
    metrics.record_cache_miss();

    metrics.record_error();

    let summary = metrics.get_summary().await;

    assert_eq!(summary.total_requests, 3);
    assert_eq!(summary.cache_hit_rate, 0.5);
    assert!((summary.error_rate - 0.333).abs() < 0.01);
    assert!(summary.p50_duration.is_some());
    assert!(summary.p95_duration.is_some());
    assert!(summary.p99_duration.is_some());
    assert!(summary.avg_duration.is_some());
}

/// Test PerformanceMetrics get_summary cache hit rate calculation
#[tokio::test]
async fn test_performance_metrics_get_summary_cache_hit_rate() {
    let metrics = PerformanceMetrics::new();

    // 3 hits, 1 miss = 75% hit rate
    metrics.record_cache_hit();
    metrics.record_cache_hit();
    metrics.record_cache_hit();
    metrics.record_cache_miss();

    let summary = metrics.get_summary().await;
    assert!((summary.cache_hit_rate - 0.75).abs() < 0.01);
}

/// Test PerformanceMetrics get_summary error rate calculation
#[tokio::test]
async fn test_performance_metrics_get_summary_error_rate() {
    let metrics = PerformanceMetrics::new();

    // 2 errors out of 5 requests = 40% error rate
    for _ in 0..5 {
        metrics.record_request_duration(Duration::from_millis(100)).await;
    }
    metrics.record_error();
    metrics.record_error();

    let summary = metrics.get_summary().await;
    assert!((summary.error_rate - 0.4).abs() < 0.01);
}

/// Test PerformanceMetrics increment_custom_counter
#[tokio::test]
async fn test_performance_metrics_increment_custom_counter() {
    let metrics = PerformanceMetrics::new();

    // Test that increment_custom_counter doesn't panic
    metrics.increment_custom_counter("test_counter").await;
    metrics.increment_custom_counter("test_counter").await;
    metrics.increment_custom_counter("other_counter").await;

    // Verify it completes successfully (no way to check custom counters from summary)
    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 0); // Custom counters don't affect request count
}

/// Test PerformanceMetrics increment_custom_counter with empty name
#[tokio::test]
async fn test_performance_metrics_increment_custom_counter_empty_name() {
    let metrics = PerformanceMetrics::new();

    // Test that empty name doesn't panic
    metrics.increment_custom_counter("").await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 0);
}

/// Test PerformanceMetrics record_request_duration limits to 1000
#[tokio::test]
async fn test_performance_metrics_record_request_duration_limit() {
    let metrics = PerformanceMetrics::new();

    // Record more than 1000 durations
    for i in 0..1500 {
        metrics.record_request_duration(Duration::from_millis(i as u64)).await;
    }

    let summary = metrics.get_summary().await;
    assert_eq!(summary.total_requests, 1500);
    // Should only keep last 1000 in memory
    // The summary should still calculate correctly
}

/// Test PerformanceMetrics active_requests can go negative (edge case)
#[tokio::test]
async fn test_performance_metrics_active_requests_negative() {
    let metrics = PerformanceMetrics::new();

    // Decrement without incrementing first
    metrics.decrement_active_requests();

    // Should handle gracefully (atomic operations wrap around)
    let summary = metrics.get_summary().await;
    // Note: This will be a large number due to underflow, but shouldn't panic
    // We just verify it doesn't crash
    let _ = summary.active_requests;
}

/// Test PerformanceMetrics percentiles with single value
#[tokio::test]
async fn test_performance_metrics_percentiles_single() {
    let metrics = PerformanceMetrics::new();

    metrics.record_request_duration(Duration::from_millis(100)).await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.p50_duration, Some(Duration::from_millis(100)));
    assert_eq!(summary.p95_duration, Some(Duration::from_millis(100)));
    assert_eq!(summary.p99_duration, Some(Duration::from_millis(100)));
}

/// Test PerformanceMetrics percentiles with multiple values
#[tokio::test]
async fn test_performance_metrics_percentiles_multiple() {
    let metrics = PerformanceMetrics::new();

    // Record 100 durations from 1ms to 100ms
    for i in 1..=100 {
        metrics.record_request_duration(Duration::from_millis(i)).await;
    }

    let summary = metrics.get_summary().await;
    assert!(summary.p50_duration.is_some());
    assert!(summary.p95_duration.is_some());
    assert!(summary.p99_duration.is_some());

    // P50 should be around 50ms
    if let Some(p50) = summary.p50_duration {
        assert!(p50.as_millis() >= 45 && p50.as_millis() <= 55);
    }
}

/// Test PerformanceMetrics average duration calculation
#[tokio::test]
async fn test_performance_metrics_average_duration() {
    let metrics = PerformanceMetrics::new();

    metrics.record_request_duration(Duration::from_millis(100)).await;
    metrics.record_request_duration(Duration::from_millis(200)).await;
    metrics.record_request_duration(Duration::from_millis(300)).await;

    let summary = metrics.get_summary().await;
    assert_eq!(summary.avg_duration, Some(Duration::from_millis(200)));
}
