//! Trend analysis for orchestration metrics over time

use crate::pdf::ExecutionReport;
use crate::{ReportingError, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Improving,
    Degrading,
    Stable,
    Volatile,
}

/// Trend report for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendReport {
    pub metric_name: String,
    pub trend: TrendDirection,
    pub change_percentage: f64,
    pub current_value: f64,
    pub previous_value: f64,
    pub average_value: f64,
    pub std_deviation: f64,
    pub data_points: Vec<DataPoint>,
    pub forecast: Vec<ForecastPoint>,
    pub anomalies: Vec<AnomalyPoint>,
}

/// Historical data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Forecasted data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPoint {
    pub timestamp: DateTime<Utc>,
    pub predicted_value: f64,
    pub confidence_interval: (f64, f64),
}

/// Anomaly point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub severity: String,
}

/// Regression result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
}

/// Trend analyzer
pub struct TrendAnalyzer {
    historical_reports: Vec<ExecutionReport>,
}

impl TrendAnalyzer {
    /// Create a new trend analyzer
    pub fn new() -> Self {
        Self {
            historical_reports: Vec::new(),
        }
    }

    /// Add historical report
    pub fn add_report(&mut self, report: ExecutionReport) {
        self.historical_reports.push(report);
        // Keep sorted by time
        self.historical_reports.sort_by_key(|r| r.start_time);
    }

    /// Analyze trends for a metric
    pub fn analyze_metric(&self, metric_name: &str) -> Result<TrendReport> {
        if self.historical_reports.is_empty() {
            return Err(ReportingError::Analysis("No historical data available".to_string()));
        }

        // Extract metric values
        let data_points = self.extract_metric_values(metric_name)?;

        if data_points.is_empty() {
            return Err(ReportingError::Analysis(format!("No data for metric: {}", metric_name)));
        }

        // Calculate statistics
        let values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();
        let average_value = values.iter().sum::<f64>() / values.len() as f64;

        let variance =
            values.iter().map(|v| (v - average_value).powi(2)).sum::<f64>() / values.len() as f64;
        let std_deviation = variance.sqrt();

        // Calculate trend
        let regression = self.linear_regression(&data_points);
        let trend = self.determine_trend(&regression, std_deviation);

        // Calculate change percentage
        let current_value = data_points.last().unwrap().value;
        let previous_value = if data_points.len() > 1 {
            data_points[data_points.len() - 2].value
        } else {
            current_value
        };

        let change_percentage = if previous_value != 0.0 {
            ((current_value - previous_value) / previous_value) * 100.0
        } else {
            0.0
        };

        // Detect anomalies
        let anomalies = self.detect_anomalies(&data_points, average_value, std_deviation);

        // Generate forecast
        let forecast = self.generate_forecast(&regression, &data_points, 5);

        Ok(TrendReport {
            metric_name: metric_name.to_string(),
            trend,
            change_percentage,
            current_value,
            previous_value,
            average_value,
            std_deviation,
            data_points,
            forecast,
            anomalies,
        })
    }

    /// Extract metric values from reports
    fn extract_metric_values(&self, metric_name: &str) -> Result<Vec<DataPoint>> {
        let mut data_points = Vec::new();

        for report in &self.historical_reports {
            let value = match metric_name {
                "error_rate" => report.metrics.error_rate,
                "avg_latency" => report.metrics.avg_latency_ms,
                "p95_latency" => report.metrics.p95_latency_ms,
                "p99_latency" => report.metrics.p99_latency_ms,
                "total_requests" => report.metrics.total_requests as f64,
                "failed_requests" => report.metrics.failed_requests as f64,
                "success_rate" => {
                    if report.metrics.total_requests > 0 {
                        report.metrics.successful_requests as f64
                            / report.metrics.total_requests as f64
                    } else {
                        0.0
                    }
                }
                _ => {
                    return Err(ReportingError::Analysis(format!(
                        "Unknown metric: {}",
                        metric_name
                    )))
                }
            };

            data_points.push(DataPoint {
                timestamp: report.start_time,
                value,
            });
        }

        Ok(data_points)
    }

    /// Perform linear regression
    fn linear_regression(&self, data_points: &[DataPoint]) -> RegressionResult {
        if data_points.len() < 2 {
            return RegressionResult {
                slope: 0.0,
                intercept: 0.0,
                r_squared: 0.0,
            };
        }

        let n = data_points.len() as f64;

        // Convert timestamps to x values (days since first point)
        let x_values: Vec<f64> = data_points
            .iter()
            .map(|dp| (dp.timestamp - data_points[0].timestamp).num_seconds() as f64 / 86400.0)
            .collect();

        let y_values: Vec<f64> = data_points.iter().map(|dp| dp.value).collect();

        let sum_x: f64 = x_values.iter().sum();
        let sum_y: f64 = y_values.iter().sum();
        let sum_xy: f64 = x_values.iter().zip(&y_values).map(|(x, y)| x * y).sum();
        let sum_xx: f64 = x_values.iter().map(|x| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate R-squared
        let mean_y = sum_y / n;
        let ss_tot: f64 = y_values.iter().map(|y| (y - mean_y).powi(2)).sum();
        let ss_res: f64 = x_values
            .iter()
            .zip(&y_values)
            .map(|(x, y)| {
                let predicted = slope * x + intercept;
                (y - predicted).powi(2)
            })
            .sum();

        let r_squared = if ss_tot > 0.0 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };

        RegressionResult {
            slope,
            intercept,
            r_squared,
        }
    }

    /// Determine trend direction
    fn determine_trend(&self, regression: &RegressionResult, std_dev: f64) -> TrendDirection {
        let slope_threshold = std_dev * 0.1;

        if regression.r_squared < 0.5 {
            // Low correlation - volatile
            TrendDirection::Volatile
        } else if regression.slope.abs() < slope_threshold {
            // Minimal change - stable
            TrendDirection::Stable
        } else if regression.slope > 0.0 {
            // Positive slope - for error rates this is degrading
            TrendDirection::Degrading
        } else {
            // Negative slope - for error rates this is improving
            TrendDirection::Improving
        }
    }

    /// Detect anomalies using statistical methods
    fn detect_anomalies(
        &self,
        data_points: &[DataPoint],
        mean: f64,
        std_dev: f64,
    ) -> Vec<AnomalyPoint> {
        let mut anomalies = Vec::new();
        let threshold = 2.0; // 2 standard deviations

        for point in data_points {
            let z_score = ((point.value - mean) / std_dev).abs();

            if z_score > threshold {
                let severity = if z_score > 3.0 { "high" } else { "medium" };

                anomalies.push(AnomalyPoint {
                    timestamp: point.timestamp,
                    value: point.value,
                    severity: severity.to_string(),
                });
            }
        }

        anomalies
    }

    /// Generate forecast using linear regression
    fn generate_forecast(
        &self,
        regression: &RegressionResult,
        data_points: &[DataPoint],
        periods: usize,
    ) -> Vec<ForecastPoint> {
        let mut forecast = Vec::new();

        if data_points.is_empty() {
            return forecast;
        }

        let last_timestamp = data_points.last().unwrap().timestamp;
        let first_timestamp = data_points[0].timestamp;

        for i in 1..=periods {
            let future_timestamp = last_timestamp + Duration::days(i as i64);
            let days_from_start =
                (future_timestamp - first_timestamp).num_seconds() as f64 / 86400.0;

            let predicted_value = regression.slope * days_from_start + regression.intercept;

            // Simple confidence interval (±2 std errors)
            let std_error = 0.1; // Simplified - should be calculated from residuals
            let confidence_interval =
                (predicted_value - 2.0 * std_error, predicted_value + 2.0 * std_error);

            forecast.push(ForecastPoint {
                timestamp: future_timestamp,
                predicted_value,
                confidence_interval,
            });
        }

        forecast
    }

    /// Get all available metrics
    pub fn available_metrics(&self) -> Vec<String> {
        vec![
            "error_rate".to_string(),
            "avg_latency".to_string(),
            "p95_latency".to_string(),
            "p99_latency".to_string(),
            "total_requests".to_string(),
            "failed_requests".to_string(),
            "success_rate".to_string(),
        ]
    }

    /// Analyze all metrics
    pub fn analyze_all_metrics(&self) -> Result<Vec<TrendReport>> {
        let mut reports = Vec::new();

        for metric in self.available_metrics() {
            if let Ok(report) = self.analyze_metric(&metric) {
                reports.push(report);
            }
        }

        Ok(reports)
    }
}

impl Default for TrendAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::ReportMetrics;

    fn create_test_report(i: i64, avg_latency: f64, error_rate: f64) -> ExecutionReport {
        ExecutionReport {
            orchestration_name: "test".to_string(),
            start_time: Utc::now() - Duration::days(10 - i),
            end_time: Utc::now() - Duration::days(10 - i),
            duration_seconds: 100,
            status: "Completed".to_string(),
            total_steps: 5,
            completed_steps: 5,
            failed_steps: 0,
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 980,
                failed_requests: 20,
                avg_latency_ms: avg_latency,
                p95_latency_ms: 200.0,
                p99_latency_ms: 300.0,
                error_rate,
            },
            failures: vec![],
            recommendations: vec![],
        }
    }

    #[test]
    fn test_trend_analyzer() {
        let mut analyzer = TrendAnalyzer::new();

        for i in 0..10 {
            let report = create_test_report(i, 100.0 + i as f64 * 5.0, 0.02);
            analyzer.add_report(report);
        }

        let trend = analyzer.analyze_metric("avg_latency").unwrap();
        assert_eq!(trend.metric_name, "avg_latency");
        assert!(trend.data_points.len() >= 10);
    }

    #[test]
    fn test_trend_analyzer_new() {
        let analyzer = TrendAnalyzer::new();
        assert!(analyzer.historical_reports.is_empty());
    }

    #[test]
    fn test_trend_analyzer_default() {
        let analyzer = TrendAnalyzer::default();
        assert!(analyzer.historical_reports.is_empty());
    }

    #[test]
    fn test_trend_direction_enum_serialize() {
        let improving = TrendDirection::Improving;
        let json = serde_json::to_string(&improving).unwrap();
        assert_eq!(json, "\"improving\"");

        let degrading = TrendDirection::Degrading;
        let json = serde_json::to_string(&degrading).unwrap();
        assert_eq!(json, "\"degrading\"");

        let stable = TrendDirection::Stable;
        let json = serde_json::to_string(&stable).unwrap();
        assert_eq!(json, "\"stable\"");

        let volatile = TrendDirection::Volatile;
        let json = serde_json::to_string(&volatile).unwrap();
        assert_eq!(json, "\"volatile\"");
    }

    #[test]
    fn test_analyze_no_historical_data() {
        let analyzer = TrendAnalyzer::new();
        let result = analyzer.analyze_metric("error_rate");
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_unknown_metric() {
        let mut analyzer = TrendAnalyzer::new();
        analyzer.add_report(create_test_report(0, 100.0, 0.02));

        let result = analyzer.analyze_metric("unknown_metric");
        assert!(result.is_err());
    }

    #[test]
    fn test_available_metrics() {
        let analyzer = TrendAnalyzer::new();
        let metrics = analyzer.available_metrics();

        assert!(metrics.contains(&"error_rate".to_string()));
        assert!(metrics.contains(&"avg_latency".to_string()));
        assert!(metrics.contains(&"p95_latency".to_string()));
        assert!(metrics.contains(&"p99_latency".to_string()));
        assert!(metrics.contains(&"total_requests".to_string()));
        assert!(metrics.contains(&"failed_requests".to_string()));
        assert!(metrics.contains(&"success_rate".to_string()));
    }

    #[test]
    fn test_analyze_all_metrics() {
        let mut analyzer = TrendAnalyzer::new();

        for i in 0..5 {
            analyzer.add_report(create_test_report(i, 100.0 + i as f64 * 2.0, 0.02));
        }

        let reports = analyzer.analyze_all_metrics().unwrap();
        assert!(!reports.is_empty());
    }

    #[test]
    fn test_trend_report_clone() {
        let report = TrendReport {
            metric_name: "error_rate".to_string(),
            trend: TrendDirection::Stable,
            change_percentage: 0.0,
            current_value: 0.02,
            previous_value: 0.02,
            average_value: 0.02,
            std_deviation: 0.001,
            data_points: vec![],
            forecast: vec![],
            anomalies: vec![],
        };

        let cloned = report.clone();
        assert_eq!(report.metric_name, cloned.metric_name);
        assert_eq!(report.trend, cloned.trend);
    }

    #[test]
    fn test_data_point_clone() {
        let point = DataPoint {
            timestamp: Utc::now(),
            value: 100.0,
        };

        let cloned = point.clone();
        assert_eq!(point.timestamp, cloned.timestamp);
        assert_eq!(point.value, cloned.value);
    }

    #[test]
    fn test_forecast_point_clone() {
        let point = ForecastPoint {
            timestamp: Utc::now(),
            predicted_value: 105.0,
            confidence_interval: (100.0, 110.0),
        };

        let cloned = point.clone();
        assert_eq!(point.predicted_value, cloned.predicted_value);
        assert_eq!(point.confidence_interval, cloned.confidence_interval);
    }

    #[test]
    fn test_anomaly_point_clone() {
        let point = AnomalyPoint {
            timestamp: Utc::now(),
            value: 500.0,
            severity: "high".to_string(),
        };

        let cloned = point.clone();
        assert_eq!(point.value, cloned.value);
        assert_eq!(point.severity, cloned.severity);
    }

    #[test]
    fn test_regression_result_clone() {
        let result = RegressionResult {
            slope: 1.5,
            intercept: 100.0,
            r_squared: 0.95,
        };

        let cloned = result.clone();
        assert_eq!(result.slope, cloned.slope);
        assert_eq!(result.r_squared, cloned.r_squared);
    }

    #[test]
    fn test_trend_degrading() {
        let mut analyzer = TrendAnalyzer::new();

        // Create reports with increasing error rate (degrading)
        for i in 0..10 {
            let report = create_test_report(i, 100.0, 0.01 + i as f64 * 0.02);
            analyzer.add_report(report);
        }

        let trend = analyzer.analyze_metric("error_rate").unwrap();
        assert!(matches!(trend.trend, TrendDirection::Degrading | TrendDirection::Volatile));
    }

    #[test]
    fn test_trend_stable() {
        let mut analyzer = TrendAnalyzer::new();

        // Create reports with stable metrics
        for i in 0..10 {
            let report = create_test_report(i, 100.0, 0.02);
            analyzer.add_report(report);
        }

        let trend = analyzer.analyze_metric("error_rate").unwrap();
        // With no variation, should be stable or volatile (depending on r_squared)
        assert!(matches!(trend.trend, TrendDirection::Stable | TrendDirection::Volatile));
    }

    #[test]
    fn test_forecast_generation() {
        let mut analyzer = TrendAnalyzer::new();

        for i in 0..10 {
            let report = create_test_report(i, 100.0 + i as f64 * 5.0, 0.02);
            analyzer.add_report(report);
        }

        let trend = analyzer.analyze_metric("avg_latency").unwrap();
        assert!(!trend.forecast.is_empty());
        assert_eq!(trend.forecast.len(), 5); // Default forecast periods
    }

    #[test]
    fn test_trend_report_serialize() {
        let mut analyzer = TrendAnalyzer::new();

        for i in 0..5 {
            analyzer.add_report(create_test_report(i, 100.0, 0.02));
        }

        let trend = analyzer.analyze_metric("error_rate").unwrap();
        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("metric_name"));
        assert!(json.contains("trend"));
    }

    #[test]
    fn test_single_report_analysis() {
        let mut analyzer = TrendAnalyzer::new();
        analyzer.add_report(create_test_report(0, 100.0, 0.02));

        let trend = analyzer.analyze_metric("error_rate").unwrap();
        assert_eq!(trend.data_points.len(), 1);
        // With single point, change_percentage should be 0
        assert_eq!(trend.change_percentage, 0.0);
    }

    #[test]
    fn test_success_rate_metric() {
        let mut analyzer = TrendAnalyzer::new();

        for i in 0..5 {
            analyzer.add_report(create_test_report(i, 100.0, 0.02));
        }

        let trend = analyzer.analyze_metric("success_rate").unwrap();
        assert_eq!(trend.metric_name, "success_rate");
        // Success rate should be ~0.98 (980/1000)
        assert!(trend.current_value > 0.9);
    }

    #[test]
    fn test_reports_sorted_by_time() {
        let mut analyzer = TrendAnalyzer::new();

        // Add reports out of order
        analyzer.add_report(create_test_report(5, 100.0, 0.02));
        analyzer.add_report(create_test_report(0, 100.0, 0.02));
        analyzer.add_report(create_test_report(3, 100.0, 0.02));

        // Reports should be sorted by time
        let times: Vec<_> = analyzer.historical_reports.iter().map(|r| r.start_time).collect();
        for i in 1..times.len() {
            assert!(times[i] >= times[i - 1]);
        }
    }
}
