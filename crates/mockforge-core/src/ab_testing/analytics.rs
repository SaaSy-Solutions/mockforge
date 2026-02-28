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
            && test_config.start_time.is_none_or(|t| t <= chrono::Utc::now())
            && test_config.end_time.is_none_or(|t| t >= chrono::Utc::now());

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

    /// Calculate statistical significance using a two-proportion z-test
    ///
    /// Compares the best and worst performing variants using the standard
    /// z-test for two proportions. Returns a confidence percentage (0-100).
    /// A value >= 95.0 is conventionally considered statistically significant.
    pub fn statistical_significance(&self) -> f64 {
        if self.variant_analytics.len() < 2 {
            return 0.0;
        }

        let variants: Vec<&VariantAnalytics> = self.variant_analytics.values().collect();
        if variants.len() < 2 {
            return 0.0;
        }

        // Find the best and worst performing variants
        let best = variants
            .iter()
            .max_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();
        let worst = variants
            .iter()
            .min_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        let n1 = best.request_count as f64;
        let n2 = worst.request_count as f64;

        // Need a minimum sample size for meaningful results
        if n1 < 5.0 || n2 < 5.0 {
            return 0.0;
        }

        let p1 = best.success_rate();
        let p2 = worst.success_rate();

        // Pooled proportion under null hypothesis (H0: p1 == p2)
        let pooled = (best.success_count as f64 + worst.success_count as f64) / (n1 + n2);

        // Guard against zero variance (all successes or all failures)
        if pooled <= 0.0 || pooled >= 1.0 {
            return 0.0;
        }

        // Standard error of the difference
        let se = (pooled * (1.0 - pooled) * (1.0 / n1 + 1.0 / n2)).sqrt();
        if se < f64::EPSILON {
            return 0.0;
        }

        // Z-score
        let z = (p1 - p2).abs() / se;

        // Convert z-score to confidence percentage using the standard normal CDF approximation
        // Using the Abramowitz and Stegun approximation for the normal CDF
        let confidence = z_to_confidence(z) * 100.0;

        confidence.min(100.0)
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

/// Convert a z-score to a confidence level (two-tailed) using the standard normal CDF.
///
/// Uses the Abramowitz and Stegun approximation (formula 26.2.17) for the
/// standard normal cumulative distribution function, which is accurate to ~1e-5.
fn z_to_confidence(z: f64) -> f64 {
    // For a two-tailed test, confidence = 1 - 2 * (1 - Phi(|z|))
    let z_abs = z.abs();

    // Abramowitz and Stegun constants
    let p = 0.2316419;
    let b1 = 0.319381530;
    let b2 = -0.356563782;
    let b3 = 1.781477937;
    let b4 = -1.821255978;
    let b5 = 1.330274429;

    let t = 1.0 / (1.0 + p * z_abs);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let pdf = (-0.5 * z_abs * z_abs).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 1.0 - pdf * (b1 * t + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5);

    // Two-tailed confidence: probability that the difference is real
    1.0 - 2.0 * (1.0 - cdf)
}
