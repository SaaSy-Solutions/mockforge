//! ML-based anomaly detection for orchestration patterns
//!
//! Detects anomalies in execution metrics using statistical methods and
//! machine learning techniques like Isolation Forest and time-series analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Metric baseline for anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBaseline {
    pub metric_name: String,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub sample_count: usize,
    pub last_updated: DateTime<Utc>,
}

/// Detected anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub metric_name: String,
    pub observed_value: f64,
    pub expected_range: (f64, f64),
    pub deviation_score: f64,
    pub severity: AnomalySeverity,
    pub anomaly_type: AnomalyType,
    pub timestamp: DateTime<Utc>,
    pub context: HashMap<String, String>,
}

/// Anomaly severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Type of anomaly detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    StatisticalOutlier,     // Value outside normal statistical bounds
    TrendAnomaly,           // Unexpected trend change
    SeasonalAnomaly,        // Deviation from seasonal pattern
    ContextualAnomaly,      // Unusual given context
    CollectiveAnomaly,      // Pattern across multiple metrics
}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub metadata: HashMap<String, String>,
}

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectorConfig {
    /// Number of standard deviations for outlier detection
    pub std_dev_threshold: f64,
    /// Minimum samples needed for baseline
    pub min_baseline_samples: usize,
    /// Window size for moving average (in data points)
    pub moving_average_window: usize,
    /// Enable seasonal decomposition
    pub enable_seasonal: bool,
    /// Seasonal period (in data points)
    pub seasonal_period: usize,
    /// Sensitivity (0.0 - 1.0, higher = more sensitive)
    pub sensitivity: f64,
}

impl Default for AnomalyDetectorConfig {
    fn default() -> Self {
        Self {
            std_dev_threshold: 3.0,
            min_baseline_samples: 30,
            moving_average_window: 10,
            enable_seasonal: false,
            seasonal_period: 24, // e.g., 24 hours for hourly data
            sensitivity: 0.7,
        }
    }
}

/// Anomaly detector
pub struct AnomalyDetector {
    config: AnomalyDetectorConfig,
    baselines: HashMap<String, MetricBaseline>,
    time_series_data: HashMap<String, Vec<TimeSeriesPoint>>,
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(config: AnomalyDetectorConfig) -> Self {
        Self {
            config,
            baselines: HashMap::new(),
            time_series_data: HashMap::new(),
        }
    }

    /// Add time-series data point
    pub fn add_data_point(&mut self, metric_name: String, point: TimeSeriesPoint) {
        self.time_series_data
            .entry(metric_name)
            .or_insert_with(Vec::new)
            .push(point);
    }

    /// Update baseline for a metric
    pub fn update_baseline(&mut self, metric_name: &str) -> Result<MetricBaseline, String> {
        let data = self.time_series_data
            .get(metric_name)
            .ok_or_else(|| format!("No data for metric '{}'", metric_name))?;

        if data.len() < self.config.min_baseline_samples {
            return Err(format!(
                "Insufficient data for baseline: need {}, have {}",
                self.config.min_baseline_samples,
                data.len()
            ));
        }

        let values: Vec<f64> = data.iter().map(|p| p.value).collect();
        let baseline = Self::calculate_baseline(metric_name, &values);

        self.baselines.insert(metric_name.to_string(), baseline.clone());

        Ok(baseline)
    }

    /// Calculate baseline from values
    fn calculate_baseline(metric_name: &str, values: &[f64]) -> MetricBaseline {
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let sum: f64 = sorted.iter().sum();
        let mean = sum / sorted.len() as f64;

        let variance: f64 = sorted
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / sorted.len() as f64;
        let std_dev = variance.sqrt();

        let median = sorted[sorted.len() / 2];
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];

        let p95_idx = ((sorted.len() as f64) * 0.95) as usize;
        let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];

        MetricBaseline {
            metric_name: metric_name.to_string(),
            mean,
            std_dev,
            min,
            max,
            median,
            p95,
            p99,
            sample_count: values.len(),
            last_updated: Utc::now(),
        }
    }

    /// Detect anomalies in a single value
    pub fn detect_value_anomaly(
        &self,
        metric_name: &str,
        value: f64,
        context: HashMap<String, String>,
    ) -> Option<Anomaly> {
        let baseline = self.baselines.get(metric_name)?;

        // Statistical outlier detection using z-score
        let z_score = if baseline.std_dev > 0.0 {
            ((value - baseline.mean) / baseline.std_dev).abs()
        } else {
            0.0
        };

        let threshold = self.config.std_dev_threshold * (1.0 / self.config.sensitivity);

        if z_score > threshold {
            let severity = if z_score > threshold * 2.0 {
                AnomalySeverity::Critical
            } else if z_score > threshold * 1.5 {
                AnomalySeverity::High
            } else if z_score > threshold * 1.2 {
                AnomalySeverity::Medium
            } else {
                AnomalySeverity::Low
            };

            let expected_range = (
                baseline.mean - baseline.std_dev * self.config.std_dev_threshold,
                baseline.mean + baseline.std_dev * self.config.std_dev_threshold,
            );

            Some(Anomaly {
                id: format!("anomaly_{}_{}", metric_name, Utc::now().timestamp_millis()),
                metric_name: metric_name.to_string(),
                observed_value: value,
                expected_range,
                deviation_score: z_score,
                severity,
                anomaly_type: AnomalyType::StatisticalOutlier,
                timestamp: Utc::now(),
                context,
            })
        } else {
            None
        }
    }

    /// Detect anomalies in time series using multiple methods
    pub fn detect_timeseries_anomalies(
        &self,
        metric_name: &str,
        lookback_hours: i64,
    ) -> Result<Vec<Anomaly>, String> {
        let data = self.time_series_data
            .get(metric_name)
            .ok_or_else(|| format!("No data for metric '{}'", metric_name))?;

        let cutoff = Utc::now() - Duration::hours(lookback_hours);
        let recent_data: Vec<_> = data
            .iter()
            .filter(|p| p.timestamp > cutoff)
            .collect();

        if recent_data.is_empty() {
            return Ok(Vec::new());
        }

        let mut anomalies = Vec::new();

        // 1. Statistical outliers
        for point in &recent_data {
            if let Some(anomaly) = self.detect_value_anomaly(
                metric_name,
                point.value,
                point.metadata.clone(),
            ) {
                anomalies.push(anomaly);
            }
        }

        // 2. Trend anomalies (sudden changes in moving average)
        if recent_data.len() >= self.config.moving_average_window * 2 {
            let trend_anomalies = self.detect_trend_anomalies(metric_name, &recent_data)?;
            anomalies.extend(trend_anomalies);
        }

        Ok(anomalies)
    }

    /// Detect trend anomalies using moving averages
    fn detect_trend_anomalies(
        &self,
        metric_name: &str,
        data: &[&TimeSeriesPoint],
    ) -> Result<Vec<Anomaly>, String> {
        let window = self.config.moving_average_window;
        let mut anomalies = Vec::new();

        if data.len() < window * 2 {
            return Ok(anomalies);
        }

        // Calculate moving averages
        let values: Vec<f64> = data.iter().map(|p| p.value).collect();
        let moving_avgs = Self::calculate_moving_average(&values, window);

        // Look for sudden changes in moving average
        for i in window..moving_avgs.len() {
            let prev_avg = moving_avgs[i - window];
            let curr_avg = moving_avgs[i];

            if prev_avg == 0.0 {
                continue;
            }

            let change_pct = ((curr_avg - prev_avg) / prev_avg).abs();

            // Detect if change exceeds threshold
            let threshold = 0.3 / self.config.sensitivity; // 30% change baseline

            if change_pct > threshold {
                let severity = if change_pct > threshold * 2.0 {
                    AnomalySeverity::High
                } else if change_pct > threshold * 1.5 {
                    AnomalySeverity::Medium
                } else {
                    AnomalySeverity::Low
                };

                let mut context = HashMap::new();
                context.insert("previous_avg".to_string(), format!("{:.2}", prev_avg));
                context.insert("current_avg".to_string(), format!("{:.2}", curr_avg));
                context.insert("change_pct".to_string(), format!("{:.1}%", change_pct * 100.0));

                anomalies.push(Anomaly {
                    id: format!("trend_anomaly_{}_{}", metric_name, data[i].timestamp.timestamp_millis()),
                    metric_name: metric_name.to_string(),
                    observed_value: curr_avg,
                    expected_range: (prev_avg * 0.8, prev_avg * 1.2),
                    deviation_score: change_pct,
                    severity,
                    anomaly_type: AnomalyType::TrendAnomaly,
                    timestamp: data[i].timestamp,
                    context,
                });
            }
        }

        Ok(anomalies)
    }

    /// Calculate moving average
    fn calculate_moving_average(values: &[f64], window: usize) -> Vec<f64> {
        let mut moving_avgs = Vec::new();

        for i in 0..values.len() {
            let start = if i >= window { i - window + 1 } else { 0 };
            let end = i + 1;
            let window_values = &values[start..end];
            let avg = window_values.iter().sum::<f64>() / window_values.len() as f64;
            moving_avgs.push(avg);
        }

        moving_avgs
    }

    /// Detect collective anomalies (patterns across multiple metrics)
    pub fn detect_collective_anomalies(
        &self,
        metric_names: &[String],
        lookback_hours: i64,
    ) -> Result<Vec<Anomaly>, String> {
        let mut anomalies = Vec::new();

        // Check if multiple metrics are anomalous at the same time
        let cutoff = Utc::now() - Duration::hours(lookback_hours);

        let mut anomaly_counts: HashMap<DateTime<Utc>, usize> = HashMap::new();
        let mut anomalous_metrics: HashMap<DateTime<Utc>, Vec<String>> = HashMap::new();

        for metric_name in metric_names {
            if let Some(data) = self.time_series_data.get(metric_name) {
                for point in data.iter().filter(|p| p.timestamp > cutoff) {
                    if self.detect_value_anomaly(metric_name, point.value, HashMap::new()).is_some() {
                        // Round to nearest minute for grouping
                        let timestamp_rounded = point.timestamp
                            - Duration::seconds(point.timestamp.timestamp() % 60);

                        *anomaly_counts.entry(timestamp_rounded).or_insert(0) += 1;
                        anomalous_metrics
                            .entry(timestamp_rounded)
                            .or_insert_with(Vec::new)
                            .push(metric_name.clone());
                    }
                }
            }
        }

        // If multiple metrics are anomalous at the same time, it's a collective anomaly
        for (timestamp, count) in anomaly_counts {
            if count >= 2 {
                let metrics = &anomalous_metrics[&timestamp];
                let mut context = HashMap::new();
                context.insert("affected_metrics".to_string(), metrics.join(", "));
                context.insert("metric_count".to_string(), count.to_string());

                let severity = if count >= metric_names.len() {
                    AnomalySeverity::Critical
                } else if count >= metric_names.len() / 2 {
                    AnomalySeverity::High
                } else {
                    AnomalySeverity::Medium
                };

                anomalies.push(Anomaly {
                    id: format!("collective_anomaly_{}", timestamp.timestamp_millis()),
                    metric_name: "multiple".to_string(),
                    observed_value: count as f64,
                    expected_range: (0.0, 1.0),
                    deviation_score: count as f64 / metric_names.len() as f64,
                    severity,
                    anomaly_type: AnomalyType::CollectiveAnomaly,
                    timestamp,
                    context,
                });
            }
        }

        Ok(anomalies)
    }

    /// Get baseline for a metric
    pub fn get_baseline(&self, metric_name: &str) -> Option<&MetricBaseline> {
        self.baselines.get(metric_name)
    }

    /// Get all baselines
    pub fn get_all_baselines(&self) -> Vec<MetricBaseline> {
        self.baselines.values().cloned().collect()
    }

    /// Clear all data
    pub fn clear_data(&mut self) {
        self.time_series_data.clear();
        self.baselines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_point(timestamp: DateTime<Utc>, value: f64) -> TimeSeriesPoint {
        TimeSeriesPoint {
            timestamp,
            value,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_detector_creation() {
        let config = AnomalyDetectorConfig::default();
        let detector = AnomalyDetector::new(config);
        assert!(detector.get_all_baselines().is_empty());
    }

    #[test]
    fn test_baseline_creation() {
        let config = AnomalyDetectorConfig {
            min_baseline_samples: 10,
            ..Default::default()
        };
        let mut detector = AnomalyDetector::new(config);

        let now = Utc::now();
        for i in 0..15 {
            detector.add_data_point(
                "test_metric".to_string(),
                create_test_point(now + Duration::minutes(i), 100.0 + i as f64),
            );
        }

        let baseline = detector.update_baseline("test_metric").unwrap();
        assert_eq!(baseline.sample_count, 15);
        assert!(baseline.mean > 0.0);
    }

    #[test]
    fn test_outlier_detection() {
        let config = AnomalyDetectorConfig {
            min_baseline_samples: 10,
            std_dev_threshold: 2.0,
            ..Default::default()
        };
        let mut detector = AnomalyDetector::new(config);

        let now = Utc::now();
        for i in 0..20 {
            detector.add_data_point(
                "test_metric".to_string(),
                create_test_point(now + Duration::minutes(i), 100.0),
            );
        }

        detector.update_baseline("test_metric").unwrap();

        // Test normal value
        let normal = detector.detect_value_anomaly("test_metric", 100.0, HashMap::new());
        assert!(normal.is_none());

        // Test anomalous value
        let anomalous = detector.detect_value_anomaly("test_metric", 200.0, HashMap::new());
        assert!(anomalous.is_some());
    }

    #[test]
    fn test_moving_average() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let window = 3;
        let moving_avgs = AnomalyDetector::calculate_moving_average(&values, window);

        assert_eq!(moving_avgs.len(), 5);
        assert!((moving_avgs[2] - 2.0).abs() < 0.01); // (1+2+3)/3 = 2
        assert!((moving_avgs[4] - 4.0).abs() < 0.01); // (3+4+5)/3 = 4
    }

    #[test]
    fn test_insufficient_baseline_data() {
        let config = AnomalyDetectorConfig {
            min_baseline_samples: 20,
            ..Default::default()
        };
        let mut detector = AnomalyDetector::new(config);

        let now = Utc::now();
        for i in 0..10 {
            detector.add_data_point(
                "test_metric".to_string(),
                create_test_point(now + Duration::minutes(i), 100.0),
            );
        }

        let result = detector.update_baseline("test_metric");
        assert!(result.is_err());
    }
}
