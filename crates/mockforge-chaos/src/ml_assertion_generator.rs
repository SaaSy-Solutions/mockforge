//! ML-based assertion generation from historical data
//!
//! Analyzes historical orchestration execution data to automatically generate
//! meaningful assertions based on observed patterns and anomalies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Historical execution data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDataPoint {
    pub timestamp: DateTime<Utc>,
    pub orchestration_id: String,
    pub step_id: String,
    pub metrics: HashMap<String, f64>,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

/// Statistical summary of metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p95: f64,
    pub p99: f64,
    pub sample_count: usize,
}

/// Generated assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAssertion {
    pub id: String,
    pub assertion_type: AssertionType,
    pub path: String,
    pub operator: AssertionOperator,
    pub value: f64,
    pub confidence: f64,
    pub rationale: String,
    pub based_on_samples: usize,
    pub created_at: DateTime<Utc>,
}

/// Type of assertion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    MetricThreshold,
    SuccessRate,
    Duration,
    ErrorRate,
    Custom,
}

/// Assertion operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    InRange,
    NotInRange,
}

/// Assertion generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionGeneratorConfig {
    /// Minimum number of samples required
    pub min_samples: usize,
    /// Confidence threshold (0.0 - 1.0)
    pub min_confidence: f64,
    /// Standard deviations for threshold detection
    pub std_dev_multiplier: f64,
    /// Use percentiles for threshold calculation
    pub use_percentiles: bool,
    /// Percentile to use for upper bounds
    pub upper_percentile: f64,
    /// Percentile to use for lower bounds
    pub lower_percentile: f64,
}

impl Default for AssertionGeneratorConfig {
    fn default() -> Self {
        Self {
            min_samples: 10,
            min_confidence: 0.7,
            std_dev_multiplier: 2.0,
            use_percentiles: true,
            upper_percentile: 95.0,
            lower_percentile: 5.0,
        }
    }
}

/// ML-based assertion generator
pub struct AssertionGenerator {
    config: AssertionGeneratorConfig,
    historical_data: Vec<ExecutionDataPoint>,
}

impl AssertionGenerator {
    /// Create a new assertion generator
    pub fn new(config: AssertionGeneratorConfig) -> Self {
        Self {
            config,
            historical_data: Vec::new(),
        }
    }

    /// Add historical data
    pub fn add_data(&mut self, data: ExecutionDataPoint) {
        self.historical_data.push(data);
    }

    /// Add multiple data points
    pub fn add_bulk_data(&mut self, data: Vec<ExecutionDataPoint>) {
        self.historical_data.extend(data);
    }

    /// Generate assertions based on historical data
    pub fn generate_assertions(&self) -> Result<Vec<GeneratedAssertion>, String> {
        if self.historical_data.len() < self.config.min_samples {
            return Err(format!(
                "Insufficient data: need at least {} samples, have {}",
                self.config.min_samples,
                self.historical_data.len()
            ));
        }

        let mut assertions = Vec::new();

        // Group data by orchestration and step
        let grouped_data = self.group_data_by_step();

        for ((orch_id, step_id), data_points) in grouped_data {
            if data_points.len() < self.config.min_samples {
                continue;
            }

            // Generate duration assertions
            assertions.extend(self.generate_duration_assertions(&orch_id, &step_id, &data_points)?);

            // Generate success rate assertions
            assertions.extend(self.generate_success_rate_assertions(&orch_id, &step_id, &data_points)?);

            // Generate metric assertions
            assertions.extend(self.generate_metric_assertions(&orch_id, &step_id, &data_points)?);

            // Generate error rate assertions
            assertions.extend(self.generate_error_rate_assertions(&orch_id, &step_id, &data_points)?);
        }

        Ok(assertions)
    }

    /// Group data by step
    fn group_data_by_step(&self) -> HashMap<(String, String), Vec<ExecutionDataPoint>> {
        let mut grouped: HashMap<(String, String), Vec<ExecutionDataPoint>> = HashMap::new();

        for data_point in &self.historical_data {
            let key = (data_point.orchestration_id.clone(), data_point.step_id.clone());
            grouped.entry(key).or_default().push(data_point.clone());
        }

        grouped
    }

    /// Generate duration assertions
    fn generate_duration_assertions(
        &self,
        orch_id: &str,
        step_id: &str,
        data: &[ExecutionDataPoint],
    ) -> Result<Vec<GeneratedAssertion>, String> {
        let durations: Vec<f64> = data.iter().map(|d| d.duration_ms as f64).collect();
        let stats = Self::calculate_stats(&durations);

        let mut assertions = Vec::new();

        // Generate P95 duration assertion
        if self.config.use_percentiles {
            let threshold = stats.p95;
            let confidence = self.calculate_confidence(&durations, threshold);

            if confidence >= self.config.min_confidence {
                assertions.push(GeneratedAssertion {
                    id: format!("duration_{}_{}", orch_id, step_id),
                    assertion_type: AssertionType::Duration,
                    path: format!("{}.{}.duration", orch_id, step_id),
                    operator: AssertionOperator::LessThanOrEqual,
                    value: threshold,
                    confidence,
                    rationale: format!(
                        "Based on P95 of historical data: {:.2}ms (mean: {:.2}ms, std: {:.2}ms)",
                        threshold, stats.mean, stats.std_dev
                    ),
                    based_on_samples: data.len(),
                    created_at: Utc::now(),
                });
            }
        }

        Ok(assertions)
    }

    /// Generate success rate assertions
    fn generate_success_rate_assertions(
        &self,
        orch_id: &str,
        step_id: &str,
        data: &[ExecutionDataPoint],
    ) -> Result<Vec<GeneratedAssertion>, String> {
        let success_count = data.iter().filter(|d| d.success).count();
        let total_count = data.len();
        let success_rate = success_count as f64 / total_count as f64;

        let mut assertions = Vec::new();

        // Only generate if success rate is consistently high
        if success_rate >= 0.9 {
            let confidence = success_rate;

            assertions.push(GeneratedAssertion {
                id: format!("success_rate_{}_{}", orch_id, step_id),
                assertion_type: AssertionType::SuccessRate,
                path: format!("{}.{}.success_rate", orch_id, step_id),
                operator: AssertionOperator::GreaterThanOrEqual,
                value: success_rate * 0.95, // Allow 5% deviation
                confidence,
                rationale: format!(
                    "Based on historical success rate: {:.2}% ({}/{} successful executions)",
                    success_rate * 100.0,
                    success_count,
                    total_count
                ),
                based_on_samples: total_count,
                created_at: Utc::now(),
            });
        }

        Ok(assertions)
    }

    /// Generate metric assertions
    fn generate_metric_assertions(
        &self,
        orch_id: &str,
        step_id: &str,
        data: &[ExecutionDataPoint],
    ) -> Result<Vec<GeneratedAssertion>, String> {
        let mut assertions = Vec::new();

        // Collect all metric names
        let mut all_metrics: HashMap<String, Vec<f64>> = HashMap::new();
        for data_point in data {
            for (metric_name, value) in &data_point.metrics {
                all_metrics.entry(metric_name.clone()).or_default().push(*value);
            }
        }

        // Generate assertions for each metric
        for (metric_name, values) in all_metrics {
            if values.len() < self.config.min_samples {
                continue;
            }

            let stats = Self::calculate_stats(&values);

            if self.config.use_percentiles {
                // Upper bound assertion (P95)
                let upper_threshold = stats.p95;
                let confidence = self.calculate_confidence(&values, upper_threshold);

                if confidence >= self.config.min_confidence {
                    assertions.push(GeneratedAssertion {
                        id: format!("metric_{}_{}_{}_upper", orch_id, step_id, metric_name),
                        assertion_type: AssertionType::MetricThreshold,
                        path: format!("{}.{}.metrics.{}", orch_id, step_id, metric_name),
                        operator: AssertionOperator::LessThanOrEqual,
                        value: upper_threshold,
                        confidence,
                        rationale: format!(
                            "Metric '{}' typically below {:.2} (P95: {:.2}, mean: {:.2}, std: {:.2})",
                            metric_name, upper_threshold, stats.p95, stats.mean, stats.std_dev
                        ),
                        based_on_samples: values.len(),
                        created_at: Utc::now(),
                    });
                }
            }
        }

        Ok(assertions)
    }

    /// Generate error rate assertions
    fn generate_error_rate_assertions(
        &self,
        orch_id: &str,
        step_id: &str,
        data: &[ExecutionDataPoint],
    ) -> Result<Vec<GeneratedAssertion>, String> {
        let error_count = data.iter().filter(|d| !d.success).count();
        let total_count = data.len();
        let error_rate = error_count as f64 / total_count as f64;

        let mut assertions = Vec::new();

        // Generate assertion if error rate is consistently low
        if error_rate <= 0.1 {
            assertions.push(GeneratedAssertion {
                id: format!("error_rate_{}_{}", orch_id, step_id),
                assertion_type: AssertionType::ErrorRate,
                path: format!("{}.{}.error_rate", orch_id, step_id),
                operator: AssertionOperator::LessThanOrEqual,
                value: (error_rate * 1.5).min(0.2), // Allow 50% increase, max 20%
                confidence: 1.0 - error_rate,
                rationale: format!(
                    "Based on historical error rate: {:.2}% ({}/{} failures)",
                    error_rate * 100.0,
                    error_count,
                    total_count
                ),
                based_on_samples: total_count,
                created_at: Utc::now(),
            });
        }

        Ok(assertions)
    }

    /// Calculate statistics for a set of values
    fn calculate_stats(values: &[f64]) -> MetricStats {
        if values.is_empty() {
            return MetricStats {
                mean: 0.0,
                median: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
                p95: 0.0,
                p99: 0.0,
                sample_count: 0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
        let median = sorted[sorted.len() / 2];
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];

        let variance = sorted
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / sorted.len() as f64;
        let std_dev = variance.sqrt();

        let p95_idx = ((sorted.len() as f64) * 0.95) as usize;
        let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];

        MetricStats {
            mean,
            median,
            std_dev,
            min,
            max,
            p95,
            p99,
            sample_count: sorted.len(),
        }
    }

    /// Calculate confidence for a threshold
    fn calculate_confidence(&self, values: &[f64], threshold: f64) -> f64 {
        let within_threshold = values.iter().filter(|&&v| v <= threshold).count();
        within_threshold as f64 / values.len() as f64
    }

    /// Get data count
    pub fn data_count(&self) -> usize {
        self.historical_data.len()
    }

    /// Clear historical data
    pub fn clear_data(&mut self) {
        self.historical_data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample_data(count: usize) -> Vec<ExecutionDataPoint> {
        (0..count)
            .map(|i| {
                let mut metrics = HashMap::new();
                metrics.insert("latency_ms".to_string(), 100.0 + (i % 20) as f64);
                metrics.insert("error_rate".to_string(), 0.01 + (i % 5) as f64 * 0.001);

                ExecutionDataPoint {
                    timestamp: Utc::now(),
                    orchestration_id: "orch-1".to_string(),
                    step_id: "step-1".to_string(),
                    metrics,
                    success: i % 10 != 0, // 90% success rate
                    duration_ms: 100 + (i % 50) as u64,
                    error_message: if i % 10 == 0 {
                        Some("Test error".to_string())
                    } else {
                        None
                    },
                }
            })
            .collect()
    }

    #[test]
    fn test_generator_creation() {
        let config = AssertionGeneratorConfig::default();
        let generator = AssertionGenerator::new(config);
        assert_eq!(generator.data_count(), 0);
    }

    #[test]
    fn test_add_data() {
        let config = AssertionGeneratorConfig::default();
        let mut generator = AssertionGenerator::new(config);

        let data = create_sample_data(1);
        generator.add_data(data[0].clone());

        assert_eq!(generator.data_count(), 1);
    }

    #[test]
    fn test_generate_assertions() {
        let config = AssertionGeneratorConfig::default();
        let mut generator = AssertionGenerator::new(config);

        let data = create_sample_data(50);
        generator.add_bulk_data(data);

        let assertions = generator.generate_assertions().unwrap();
        assert!(!assertions.is_empty());

        // Should have duration, success rate, and metric assertions
        assert!(assertions.iter().any(|a| a.assertion_type == AssertionType::Duration));
        assert!(assertions.iter().any(|a| a.assertion_type == AssertionType::SuccessRate));
    }

    #[test]
    fn test_insufficient_data() {
        let config = AssertionGeneratorConfig::default();
        let mut generator = AssertionGenerator::new(config);

        let data = create_sample_data(5);
        generator.add_bulk_data(data);

        let result = generator.generate_assertions();
        assert!(result.is_err());
    }

    #[test]
    fn test_stats_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = AssertionGenerator::calculate_stats(&values);

        assert_eq!(stats.mean, 5.5);
        assert_eq!(stats.median, 6.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 10.0);
        assert_eq!(stats.sample_count, 10);
    }
}
