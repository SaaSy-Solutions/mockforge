//! Analytics for A/B testing
//!
//! This module provides analytics and reporting functionality for A/B tests.

use crate::ab_testing::types::{ABTestConfig, VariantAnalytics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary report for an A/B test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestReport {
    /// Test configuration
    pub test_config: ABTestConfig,
    /// Analytics for each variant
    pub variant_analytics: HashMap<String, VariantAnalytics>,
    /// Total requests across all variants
    pub total_requests: u64,
    /// Test start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Test end time (if ended)
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the test is currently active
    pub is_active: bool,
}

impl ABTestReport {
    /// Create a new A/B test report
    pub fn new(
        test_config: ABTestConfig,
        variant_analytics: HashMap<String, VariantAnalytics>,
    ) -> Self {
        let total_requests: u64 = variant_analytics.values().map(|a| a.request_count).sum();
        let is_active = test_config.enabled
            && test_config.start_time.map_or(true, |t| t <= chrono::Utc::now())
            && test_config.end_time.map_or(true, |t| t >= chrono::Utc::now());

        Self {
            test_config,
            variant_analytics,
            total_requests,
            start_time: None,
            end_time: None,
            is_active,
        }
    }

    /// Get the best performing variant (highest success rate)
    pub fn best_variant(&self) -> Option<&VariantAnalytics> {
        self.variant_analytics.values().max_by(|a, b| {
            a.success_rate()
                .partial_cmp(&b.success_rate())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get the worst performing variant (lowest success rate)
    pub fn worst_variant(&self) -> Option<&VariantAnalytics> {
        self.variant_analytics.values().min_by(|a, b| {
            a.success_rate()
                .partial_cmp(&b.success_rate())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Calculate statistical significance (simplified - would need proper statistical test in production)
    pub fn statistical_significance(&self) -> f64 {
        // Simplified calculation - in production, use proper statistical tests
        // like chi-square or t-test
        if self.variant_analytics.len() < 2 {
            return 0.0;
        }

        let variants: Vec<&VariantAnalytics> = self.variant_analytics.values().collect();
        if variants.len() < 2 {
            return 0.0;
        }

        // Simple comparison of success rates
        let success_rates: Vec<f64> = variants.iter().map(|v| v.success_rate()).collect();
        let max_rate = success_rates.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_rate = success_rates.iter().fold(1.0f64, |a, &b| a.min(b));

        // Difference between best and worst
        (max_rate - min_rate) * 100.0
    }
}

/// Comparison between two variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantComparison {
    /// First variant ID
    pub variant_a_id: String,
    /// Second variant ID
    pub variant_b_id: String,
    /// Success rate difference (A - B)
    pub success_rate_diff: f64,
    /// Response time difference in milliseconds (A - B)
    pub response_time_diff_ms: f64,
    /// Error rate difference (A - B)
    pub error_rate_diff: f64,
    /// Request count difference (A - B)
    pub request_count_diff: i64,
}

impl VariantComparison {
    /// Create a comparison between two variants
    pub fn new(variant_a: &VariantAnalytics, variant_b: &VariantAnalytics) -> Self {
        Self {
            variant_a_id: variant_a.variant_id.clone(),
            variant_b_id: variant_b.variant_id.clone(),
            success_rate_diff: variant_a.success_rate() - variant_b.success_rate(),
            response_time_diff_ms: variant_a.avg_response_time_ms - variant_b.avg_response_time_ms,
            error_rate_diff: variant_a.error_rate() - variant_b.error_rate(),
            request_count_diff: variant_a.request_count as i64 - variant_b.request_count as i64,
        }
    }
}
