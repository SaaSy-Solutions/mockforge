//! A/B testing framework for chaos engineering strategies
//!
//! Compare different chaos configurations and strategies to determine
//! which approach is most effective for testing system resilience.

use crate::analytics::ChaosAnalytics;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A/B test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    /// Test name
    pub name: String,
    /// Description
    pub description: String,
    /// Variant A (control)
    pub variant_a: TestVariant,
    /// Variant B (treatment)
    pub variant_b: TestVariant,
    /// Test duration
    pub duration_minutes: i64,
    /// Traffic split (0.0 - 1.0, percentage for variant B)
    pub traffic_split: f64,
    /// Success criteria
    pub success_criteria: SuccessCriteria,
    /// Minimum sample size per variant
    pub min_sample_size: usize,
}

/// Test variant (A or B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestVariant {
    /// Variant name
    pub name: String,
    /// Chaos configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Scenario to run (optional)
    pub scenario: Option<String>,
    /// Description
    pub description: String,
}

/// Success criteria for A/B test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    /// Primary metric to optimize
    pub primary_metric: MetricType,
    /// Secondary metrics to track
    pub secondary_metrics: Vec<MetricType>,
    /// Minimum improvement threshold (0.0 - 1.0)
    pub min_improvement: f64,
    /// Statistical significance level (e.g., 0.95 for 95%)
    pub significance_level: f64,
    /// Maximum acceptable degradation in secondary metrics
    pub max_secondary_degradation: f64,
}

/// Metric type for comparison
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    ErrorRate,
    LatencyP50,
    LatencyP95,
    LatencyP99,
    SuccessRate,
    RecoveryTime,
    ResilienceScore,
    ChaosEffectiveness,
    FaultDetectionRate,
}

/// A/B test status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ABTestStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

/// A/B test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    /// Test ID
    pub id: String,
    /// Test configuration
    pub config: ABTestConfig,
    /// Test status
    pub status: ABTestStatus,
    /// Start time
    pub started_at: Option<DateTime<Utc>>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Variant A results
    pub variant_a_results: Option<VariantResults>,
    /// Variant B results
    pub variant_b_results: Option<VariantResults>,
    /// Test conclusion
    pub conclusion: Option<TestConclusion>,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Results for a test variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResults {
    /// Variant name
    pub variant_name: String,
    /// Number of requests/tests
    pub sample_size: usize,
    /// Metrics
    pub metrics: VariantMetrics,
    /// Chaos events recorded
    pub chaos_events: usize,
    /// Duration
    pub duration_ms: u64,
    /// Success rate
    pub success_rate: f64,
}

/// Metrics for a variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantMetrics {
    pub error_rate: f64,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub avg_latency: f64,
    pub success_rate: f64,
    pub recovery_time_ms: f64,
    pub resilience_score: f64,
    pub chaos_effectiveness: f64,
    pub fault_detection_rate: f64,
}

/// Test conclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConclusion {
    /// Winner variant (A or B)
    pub winner: String,
    /// Statistical significance achieved
    pub statistically_significant: bool,
    /// P-value
    pub p_value: f64,
    /// Improvement percentage for primary metric
    pub improvement_pct: f64,
    /// Detailed comparison
    pub comparison: MetricComparison,
    /// Recommendation
    pub recommendation: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
}

/// Detailed metric comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    /// Primary metric comparison
    pub primary: SingleMetricComparison,
    /// Secondary metrics comparison
    pub secondary: Vec<SingleMetricComparison>,
}

/// Single metric comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleMetricComparison {
    pub metric: MetricType,
    pub variant_a_value: f64,
    pub variant_b_value: f64,
    pub difference: f64,
    pub difference_pct: f64,
    pub winner: String,
    pub significant: bool,
}

/// A/B testing engine
pub struct ABTestingEngine {
    tests: Arc<RwLock<HashMap<String, ABTest>>>,
    #[allow(dead_code)]
    analytics: Arc<ChaosAnalytics>,
    max_concurrent_tests: usize,
}

impl ABTestingEngine {
    /// Create a new A/B testing engine
    pub fn new(analytics: Arc<ChaosAnalytics>) -> Self {
        Self {
            tests: Arc::new(RwLock::new(HashMap::new())),
            analytics,
            max_concurrent_tests: 5,
        }
    }

    /// Create a new A/B test
    pub fn create_test(&self, config: ABTestConfig) -> Result<String, String> {
        // Validate config
        if config.traffic_split < 0.0 || config.traffic_split > 1.0 {
            return Err("Traffic split must be between 0.0 and 1.0".to_string());
        }

        if config.success_criteria.min_improvement < 0.0 {
            return Err("Minimum improvement must be non-negative".to_string());
        }

        // Check concurrent limit
        let tests = self.tests.read();
        let running_tests = tests.values().filter(|t| t.status == ABTestStatus::Running).count();

        if running_tests >= self.max_concurrent_tests {
            return Err(format!(
                "Maximum concurrent tests ({}) reached",
                self.max_concurrent_tests
            ));
        }
        drop(tests);

        let test = ABTest {
            id: format!("abtest-{}", Uuid::new_v4()),
            config,
            status: ABTestStatus::Draft,
            started_at: None,
            ended_at: None,
            variant_a_results: None,
            variant_b_results: None,
            conclusion: None,
            created_at: Utc::now(),
        };

        let test_id = test.id.clone();

        let mut tests = self.tests.write();
        tests.insert(test_id.clone(), test);

        Ok(test_id)
    }

    /// Start an A/B test
    pub fn start_test(&self, test_id: &str) -> Result<(), String> {
        let mut tests = self.tests.write();
        let test = tests.get_mut(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status != ABTestStatus::Draft {
            return Err("Test must be in Draft status to start".to_string());
        }

        test.status = ABTestStatus::Running;
        test.started_at = Some(Utc::now());

        Ok(())
    }

    /// Stop an A/B test and analyze results
    pub fn stop_test(&self, test_id: &str) -> Result<TestConclusion, String> {
        let mut tests = self.tests.write();
        let test = tests.get_mut(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status != ABTestStatus::Running {
            return Err("Test must be running to stop".to_string());
        }

        test.status = ABTestStatus::Completed;
        test.ended_at = Some(Utc::now());

        // Analyze results
        let conclusion = self.analyze_results(test)?;
        test.conclusion = Some(conclusion.clone());

        Ok(conclusion)
    }

    /// Record variant execution results
    pub fn record_variant_result(
        &self,
        test_id: &str,
        variant: &str,
        results: VariantResults,
    ) -> Result<(), String> {
        let mut tests = self.tests.write();
        let test = tests.get_mut(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status != ABTestStatus::Running {
            return Err("Test must be running to record results".to_string());
        }

        if variant == "A" {
            test.variant_a_results = Some(results);
        } else if variant == "B" {
            test.variant_b_results = Some(results);
        } else {
            return Err("Invalid variant name".to_string());
        }

        Ok(())
    }

    /// Analyze test results and determine winner
    fn analyze_results(&self, test: &ABTest) -> Result<TestConclusion, String> {
        let variant_a = test
            .variant_a_results
            .as_ref()
            .ok_or_else(|| "Variant A results not available".to_string())?;
        let variant_b = test
            .variant_b_results
            .as_ref()
            .ok_or_else(|| "Variant B results not available".to_string())?;

        // Check minimum sample size
        if variant_a.sample_size < test.config.min_sample_size
            || variant_b.sample_size < test.config.min_sample_size
        {
            return Err("Insufficient sample size for analysis".to_string());
        }

        // Compare primary metric
        let primary =
            self.compare_metric(&test.config.success_criteria.primary_metric, variant_a, variant_b);

        // Compare secondary metrics
        let secondary: Vec<SingleMetricComparison> = test
            .config
            .success_criteria
            .secondary_metrics
            .iter()
            .map(|metric| self.compare_metric(metric, variant_a, variant_b))
            .collect();

        // Determine winner
        let winner = if primary.variant_b_value > primary.variant_a_value {
            "B".to_string()
        } else {
            "A".to_string()
        };

        // Calculate improvement
        let improvement_pct = if primary.variant_a_value > 0.0 {
            ((primary.variant_b_value - primary.variant_a_value) / primary.variant_a_value) * 100.0
        } else {
            0.0
        };

        // Check if statistically significant
        let p_value =
            self.calculate_p_value(variant_a.sample_size, variant_b.sample_size, &primary);
        let statistically_significant =
            p_value < (1.0 - test.config.success_criteria.significance_level);

        // Check secondary metrics for degradation
        let secondary_degraded = secondary.iter().any(|comp| {
            comp.winner == "A"
                && comp.difference_pct.abs()
                    > test.config.success_criteria.max_secondary_degradation
        });

        // Generate recommendation
        let recommendation = if !statistically_significant {
            format!("Results are not statistically significant (p-value: {:.4}). Consider running the test longer or with more traffic.", p_value)
        } else if secondary_degraded {
            format!("Variant {} shows improvement in primary metric but degrades secondary metrics beyond acceptable threshold.", winner)
        } else if improvement_pct >= test.config.success_criteria.min_improvement {
            format!(
                "Variant {} is the clear winner with {:.2}% improvement in {:?}.",
                winner, improvement_pct, test.config.success_criteria.primary_metric
            )
        } else {
            format!("Variants show similar performance. Improvement ({:.2}%) below minimum threshold ({:.2}%).", improvement_pct, test.config.success_criteria.min_improvement)
        };

        // Calculate confidence
        let confidence = if statistically_significant && !secondary_degraded {
            test.config.success_criteria.significance_level
        } else if statistically_significant {
            test.config.success_criteria.significance_level * 0.7
        } else {
            1.0 - p_value
        };

        Ok(TestConclusion {
            winner,
            statistically_significant,
            p_value,
            improvement_pct,
            comparison: MetricComparison { primary, secondary },
            recommendation,
            confidence,
        })
    }

    /// Compare a single metric between variants
    fn compare_metric(
        &self,
        metric: &MetricType,
        variant_a: &VariantResults,
        variant_b: &VariantResults,
    ) -> SingleMetricComparison {
        let (a_value, b_value) = match metric {
            MetricType::ErrorRate => (variant_a.metrics.error_rate, variant_b.metrics.error_rate),
            MetricType::LatencyP50 => {
                (variant_a.metrics.latency_p50, variant_b.metrics.latency_p50)
            }
            MetricType::LatencyP95 => {
                (variant_a.metrics.latency_p95, variant_b.metrics.latency_p95)
            }
            MetricType::LatencyP99 => {
                (variant_a.metrics.latency_p99, variant_b.metrics.latency_p99)
            }
            MetricType::SuccessRate => {
                (variant_a.metrics.success_rate, variant_b.metrics.success_rate)
            }
            MetricType::RecoveryTime => {
                (variant_a.metrics.recovery_time_ms, variant_b.metrics.recovery_time_ms)
            }
            MetricType::ResilienceScore => {
                (variant_a.metrics.resilience_score, variant_b.metrics.resilience_score)
            }
            MetricType::ChaosEffectiveness => {
                (variant_a.metrics.chaos_effectiveness, variant_b.metrics.chaos_effectiveness)
            }
            MetricType::FaultDetectionRate => {
                (variant_a.metrics.fault_detection_rate, variant_b.metrics.fault_detection_rate)
            }
        };

        let difference = b_value - a_value;
        let difference_pct = if a_value > 0.0 {
            (difference / a_value) * 100.0
        } else {
            0.0
        };

        // For error rate and latency, lower is better
        let winner = match metric {
            MetricType::ErrorRate
            | MetricType::LatencyP50
            | MetricType::LatencyP95
            | MetricType::LatencyP99
            | MetricType::RecoveryTime => {
                if b_value < a_value {
                    "B"
                } else {
                    "A"
                }
            }
            _ => {
                if b_value > a_value {
                    "B"
                } else {
                    "A"
                }
            }
        };

        SingleMetricComparison {
            metric: metric.clone(),
            variant_a_value: a_value,
            variant_b_value: b_value,
            difference,
            difference_pct,
            winner: winner.to_string(),
            significant: difference_pct.abs() > 5.0, // Simple threshold
        }
    }

    /// Calculate p-value (simplified t-test approximation)
    fn calculate_p_value(
        &self,
        n_a: usize,
        n_b: usize,
        comparison: &SingleMetricComparison,
    ) -> f64 {
        // Simplified statistical significance calculation
        // In a real implementation, this would use proper statistical tests

        let pooled_n = (n_a + n_b) as f64;
        let effect_size = comparison.difference_pct.abs() / 100.0;

        // Rough approximation: larger sample sizes and larger effect sizes = lower p-value
        let p_value = 1.0 / (1.0 + pooled_n * effect_size);

        p_value.clamp(0.001, 0.999)
    }

    /// Get test by ID
    pub fn get_test(&self, test_id: &str) -> Option<ABTest> {
        let tests = self.tests.read();
        tests.get(test_id).cloned()
    }

    /// Get all tests
    pub fn get_all_tests(&self) -> Vec<ABTest> {
        let tests = self.tests.read();
        tests.values().cloned().collect()
    }

    /// Get running tests
    pub fn get_running_tests(&self) -> Vec<ABTest> {
        let tests = self.tests.read();
        tests.values().filter(|t| t.status == ABTestStatus::Running).cloned().collect()
    }

    /// Delete a test
    pub fn delete_test(&self, test_id: &str) -> Result<(), String> {
        let mut tests = self.tests.write();
        let test = tests.get(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status == ABTestStatus::Running {
            return Err("Cannot delete running test".to_string());
        }

        tests.remove(test_id);
        Ok(())
    }

    /// Pause a running test
    pub fn pause_test(&self, test_id: &str) -> Result<(), String> {
        let mut tests = self.tests.write();
        let test = tests.get_mut(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status != ABTestStatus::Running {
            return Err("Only running tests can be paused".to_string());
        }

        test.status = ABTestStatus::Paused;
        Ok(())
    }

    /// Resume a paused test
    pub fn resume_test(&self, test_id: &str) -> Result<(), String> {
        let mut tests = self.tests.write();
        let test = tests.get_mut(test_id).ok_or_else(|| "Test not found".to_string())?;

        if test.status != ABTestStatus::Paused {
            return Err("Only paused tests can be resumed".to_string());
        }

        test.status = ABTestStatus::Running;
        Ok(())
    }

    /// Get test statistics
    pub fn get_stats(&self) -> ABTestStats {
        let tests = self.tests.read();

        let total = tests.len();
        let running = tests.values().filter(|t| t.status == ABTestStatus::Running).count();
        let completed = tests.values().filter(|t| t.status == ABTestStatus::Completed).count();
        let cancelled = tests.values().filter(|t| t.status == ABTestStatus::Cancelled).count();

        let successful_tests = tests
            .values()
            .filter(|t| {
                t.status == ABTestStatus::Completed
                    && t.conclusion.as_ref().is_some_and(|c| c.statistically_significant)
            })
            .count();

        ABTestStats {
            total_tests: total,
            running_tests: running,
            completed_tests: completed,
            cancelled_tests: cancelled,
            successful_tests,
            avg_improvement: self.calculate_avg_improvement(&tests),
        }
    }

    fn calculate_avg_improvement(&self, tests: &HashMap<String, ABTest>) -> f64 {
        let improvements: Vec<f64> = tests
            .values()
            .filter_map(|t| {
                if t.status == ABTestStatus::Completed {
                    t.conclusion.as_ref().map(|c| c.improvement_pct)
                } else {
                    None
                }
            })
            .collect();

        if improvements.is_empty() {
            0.0
        } else {
            improvements.iter().sum::<f64>() / improvements.len() as f64
        }
    }
}

impl Default for ABTestingEngine {
    fn default() -> Self {
        Self::new(Arc::new(ChaosAnalytics::new()))
    }
}

/// A/B test statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestStats {
    pub total_tests: usize,
    pub running_tests: usize,
    pub completed_tests: usize,
    pub cancelled_tests: usize,
    pub successful_tests: usize,
    pub avg_improvement: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let analytics = Arc::new(ChaosAnalytics::new());
        let engine = ABTestingEngine::new(analytics);
        let stats = engine.get_stats();
        assert_eq!(stats.total_tests, 0);
    }

    #[test]
    fn test_create_test() {
        let analytics = Arc::new(ChaosAnalytics::new());
        let engine = ABTestingEngine::new(analytics);

        let config = ABTestConfig {
            name: "Test 1".to_string(),
            description: "Test description".to_string(),
            variant_a: TestVariant {
                name: "Control".to_string(),
                config: HashMap::new(),
                scenario: None,
                description: "Control variant".to_string(),
            },
            variant_b: TestVariant {
                name: "Treatment".to_string(),
                config: HashMap::new(),
                scenario: None,
                description: "Treatment variant".to_string(),
            },
            duration_minutes: 60,
            traffic_split: 0.5,
            success_criteria: SuccessCriteria {
                primary_metric: MetricType::ErrorRate,
                secondary_metrics: vec![],
                min_improvement: 0.1,
                significance_level: 0.95,
                max_secondary_degradation: 10.0,
            },
            min_sample_size: 100,
        };

        let result = engine.create_test(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_traffic_split() {
        let analytics = Arc::new(ChaosAnalytics::new());
        let engine = ABTestingEngine::new(analytics);

        let config = ABTestConfig {
            name: "Test".to_string(),
            description: "Test".to_string(),
            variant_a: TestVariant {
                name: "A".to_string(),
                config: HashMap::new(),
                scenario: None,
                description: "".to_string(),
            },
            variant_b: TestVariant {
                name: "B".to_string(),
                config: HashMap::new(),
                scenario: None,
                description: "".to_string(),
            },
            duration_minutes: 60,
            traffic_split: 1.5,
            success_criteria: SuccessCriteria {
                primary_metric: MetricType::ErrorRate,
                secondary_metrics: vec![],
                min_improvement: 0.1,
                significance_level: 0.95,
                max_secondary_degradation: 10.0,
            },
            min_sample_size: 100,
        };

        let result = engine.create_test(config);
        assert!(result.is_err());
    }
}
