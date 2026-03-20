use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::reinforcement_learning::{RemediationAction, SystemState};

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
}

/// Metric type for prediction
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum MetricType {
    ErrorRate,
    Latency,
    CpuUsage,
    MemoryUsage,
    RequestRate,
    FailureCount,
}

/// Time series for a metric
#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub metric: MetricType,
    pub data: VecDeque<DataPoint>,
    pub max_size: usize,
}

impl TimeSeries {
    pub fn new(metric: MetricType, max_size: usize) -> Self {
        Self {
            metric,
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn add(&mut self, point: DataPoint) {
        if self.data.len() >= self.max_size {
            self.data.pop_front();
        }
        self.data.push_back(point);
    }

    pub fn values(&self) -> Vec<f64> {
        self.data.iter().map(|p| p.value).collect()
    }

    /// Calculate moving average
    pub fn moving_average(&self, window: usize) -> Vec<f64> {
        let values = self.values();
        if values.len() < window {
            return vec![];
        }

        let mut averages = Vec::new();
        for i in 0..=(values.len() - window) {
            let sum: f64 = values[i..i + window].iter().sum();
            averages.push(sum / window as f64);
        }
        averages
    }

    /// Calculate exponential moving average
    pub fn exponential_moving_average(&self, alpha: f64) -> Vec<f64> {
        let values = self.values();
        if values.is_empty() {
            return vec![];
        }

        let mut ema = Vec::new();
        ema.push(values[0]);

        for i in 1..values.len() {
            let e = alpha * values[i] + (1.0 - alpha) * ema[i - 1];
            ema.push(e);
        }

        ema
    }

    /// Simple linear regression for trend
    pub fn linear_trend(&self) -> Option<(f64, f64)> {
        let values = self.values();
        let n = values.len();

        if n < 2 {
            return None;
        }

        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let y = values;

        let x_mean = x.iter().sum::<f64>() / n as f64;
        let y_mean = y.iter().sum::<f64>() / n as f64;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for i in 0..n {
            numerator += (x[i] - x_mean) * (y[i] - y_mean);
            denominator += (x[i] - x_mean).powi(2);
        }

        if denominator == 0.0 {
            return None;
        }

        let slope = numerator / denominator;
        let intercept = y_mean - slope * x_mean;

        Some((slope, intercept))
    }

    /// Predict next N values using linear trend
    pub fn predict_linear(&self, steps: usize) -> Vec<f64> {
        if let Some((slope, intercept)) = self.linear_trend() {
            let current_x = self.data.len() as f64;
            (0..steps).map(|i| slope * (current_x + i as f64) + intercept).collect()
        } else {
            vec![]
        }
    }
}

/// Anomaly detection using statistical methods
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    threshold_multiplier: f64, // Standard deviations for anomaly
}

impl AnomalyDetector {
    pub fn new(threshold_multiplier: f64) -> Self {
        Self {
            threshold_multiplier,
        }
    }

    /// Detect anomalies using z-score
    pub fn detect_zscore(&self, series: &TimeSeries) -> Vec<(usize, f64)> {
        let values = series.values();
        if values.len() < 2 {
            return vec![];
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return vec![];
        }

        let mut anomalies = Vec::new();
        for (i, value) in values.iter().enumerate() {
            let z_score = (value - mean).abs() / std_dev;
            if z_score > self.threshold_multiplier {
                anomalies.push((i, *value));
            }
        }

        anomalies
    }

    /// Detect anomalies using IQR method
    pub fn detect_iqr(&self, series: &TimeSeries) -> Vec<(usize, f64)> {
        let mut values = series.values();
        if values.len() < 4 {
            return vec![];
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let q1_idx = values.len() / 4;
        let q3_idx = (values.len() * 3) / 4;

        let q1 = values[q1_idx];
        let q3 = values[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        let original_values = series.values();
        let mut anomalies = Vec::new();

        for (i, value) in original_values.iter().enumerate() {
            if *value < lower_bound || *value > upper_bound {
                anomalies.push((i, *value));
            }
        }

        anomalies
    }
}

/// Failure prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePrediction {
    pub metric: MetricType,
    pub current_value: f64,
    pub predicted_value: f64,
    pub time_to_failure: Option<std::time::Duration>,
    pub confidence: f64,
    pub threshold: f64,
    pub recommended_actions: Vec<RemediationAction>,
}

/// Predictive remediation engine
pub struct PredictiveRemediationEngine {
    time_series: Arc<RwLock<HashMap<MetricType, TimeSeries>>>,
    anomaly_detector: AnomalyDetector,
    prediction_horizon: usize, // Number of steps to predict ahead
    thresholds: HashMap<MetricType, f64>,
}

impl PredictiveRemediationEngine {
    pub fn new(prediction_horizon: usize) -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert(MetricType::ErrorRate, 50.0);
        thresholds.insert(MetricType::Latency, 80.0);
        thresholds.insert(MetricType::CpuUsage, 85.0);
        thresholds.insert(MetricType::MemoryUsage, 90.0);
        thresholds.insert(MetricType::FailureCount, 5.0);

        Self {
            time_series: Arc::new(RwLock::new(HashMap::new())),
            anomaly_detector: AnomalyDetector::new(3.0),
            prediction_horizon,
            thresholds,
        }
    }

    /// Record a metric value
    pub async fn record(&self, metric: MetricType, value: f64) {
        let mut series_map = self.time_series.write().await;

        series_map
            .entry(metric.clone())
            .or_insert_with(|| TimeSeries::new(metric, 1000))
            .add(DataPoint {
                timestamp: chrono::Utc::now(),
                value,
            });
    }

    /// Predict failures for all metrics
    pub async fn predict_failures(&self) -> Vec<FailurePrediction> {
        let series_map = self.time_series.read().await;
        let mut predictions = Vec::new();

        for (metric, series) in series_map.iter() {
            if let Some(prediction) = self.predict_failure_for_metric(metric, series).await {
                predictions.push(prediction);
            }
        }

        predictions
    }

    /// Predict failure for a specific metric
    async fn predict_failure_for_metric(
        &self,
        metric: &MetricType,
        series: &TimeSeries,
    ) -> Option<FailurePrediction> {
        if series.data.is_empty() {
            return None;
        }

        let current_value = series.data.back()?.value;
        let threshold = *self.thresholds.get(metric)?;

        // Predict future values
        let predictions = series.predict_linear(self.prediction_horizon);
        if predictions.is_empty() {
            return None;
        }

        // Find when threshold will be crossed
        let mut time_to_failure = None;
        let mut predicted_value = current_value;

        for (i, pred) in predictions.iter().enumerate() {
            if *pred > threshold && time_to_failure.is_none() {
                // Assume 1 minute per step
                time_to_failure = Some(std::time::Duration::from_secs((i as u64 + 1) * 60));
                predicted_value = *pred;
                break;
            }
        }

        // Calculate confidence based on trend strength
        let confidence = if let Some((slope, _)) = series.linear_trend() {
            (slope.abs() * 10.0).min(1.0)
        } else {
            0.0
        };

        // Recommend actions based on metric type
        let recommended_actions = self.recommend_actions(metric, predicted_value, threshold);

        Some(FailurePrediction {
            metric: metric.clone(),
            current_value,
            predicted_value,
            time_to_failure,
            confidence,
            threshold,
            recommended_actions,
        })
    }

    /// Recommend remediation actions
    fn recommend_actions(
        &self,
        metric: &MetricType,
        predicted_value: f64,
        threshold: f64,
    ) -> Vec<RemediationAction> {
        if predicted_value <= threshold {
            return vec![];
        }

        match metric {
            MetricType::ErrorRate => vec![
                RemediationAction::EnableCircuitBreaker,
                RemediationAction::RestartService,
            ],
            MetricType::Latency => {
                vec![RemediationAction::ClearCache, RemediationAction::ScaleUp(2)]
            }
            MetricType::CpuUsage | MetricType::MemoryUsage => {
                vec![
                    RemediationAction::ScaleUp(2),
                    RemediationAction::RestrictTraffic,
                ]
            }
            MetricType::FailureCount => vec![
                RemediationAction::RollbackDeployment,
                RemediationAction::RestartService,
            ],
            MetricType::RequestRate => vec![
                RemediationAction::ScaleUp(4),
                RemediationAction::RestrictTraffic,
            ],
        }
    }

    /// Detect anomalies in metrics
    pub async fn detect_anomalies(&self) -> HashMap<MetricType, Vec<(usize, f64)>> {
        let series_map = self.time_series.read().await;
        let mut anomalies = HashMap::new();

        for (metric, series) in series_map.iter() {
            let detected = self.anomaly_detector.detect_zscore(series);
            if !detected.is_empty() {
                anomalies.insert(metric.clone(), detected);
            }
        }

        anomalies
    }

    /// Get current system state
    pub async fn get_system_state(&self) -> SystemState {
        let series_map = self.time_series.read().await;

        let error_rate = series_map
            .get(&MetricType::ErrorRate)
            .and_then(|s| s.data.back())
            .map(|p| p.value as u8)
            .unwrap_or(0);

        let latency_level = series_map
            .get(&MetricType::Latency)
            .and_then(|s| s.data.back())
            .map(|p| p.value as u8)
            .unwrap_or(0);

        let cpu_usage = series_map
            .get(&MetricType::CpuUsage)
            .and_then(|s| s.data.back())
            .map(|p| p.value as u8)
            .unwrap_or(0);

        let memory_usage = series_map
            .get(&MetricType::MemoryUsage)
            .and_then(|s| s.data.back())
            .map(|p| p.value as u8)
            .unwrap_or(0);

        let active_failures = series_map
            .get(&MetricType::FailureCount)
            .and_then(|s| s.data.back())
            .map(|p| p.value as u8)
            .unwrap_or(0);

        let service_health = if error_rate > 80 || active_failures > 5 {
            "critical".to_string()
        } else if error_rate > 50 || latency_level > 70 {
            "degraded".to_string()
        } else {
            "healthy".to_string()
        };

        SystemState {
            error_rate,
            latency_level,
            cpu_usage,
            memory_usage,
            active_failures,
            service_health,
        }
    }

    /// Proactive remediation: apply actions before failure
    pub async fn proactive_remediate(&self) -> Vec<RemediationAction> {
        let predictions = self.predict_failures().await;
        let mut actions = Vec::new();

        for prediction in predictions {
            // Only remediate if failure is imminent (< 5 minutes) and confidence is high
            if let Some(ttf) = prediction.time_to_failure {
                if ttf.as_secs() < 300 && prediction.confidence > 0.6 {
                    actions.extend(prediction.recommended_actions);
                }
            }
        }

        // Deduplicate actions
        actions.sort_by_key(|a| format!("{:?}", a));
        actions.dedup();

        actions
    }
}

/// Trend analyzer for long-term patterns
pub struct TrendAnalyzer {
    engine: Arc<PredictiveRemediationEngine>,
}

impl TrendAnalyzer {
    pub fn new(engine: Arc<PredictiveRemediationEngine>) -> Self {
        Self { engine }
    }

    /// Analyze trends across all metrics
    pub async fn analyze_trends(&self) -> TrendReport {
        let series_map = self.engine.time_series.read().await;
        let mut trends = HashMap::new();

        for (metric, series) in series_map.iter() {
            if let Some((slope, _)) = series.linear_trend() {
                let direction = if slope > 0.1 {
                    TrendDirection::Increasing
                } else if slope < -0.1 {
                    TrendDirection::Decreasing
                } else {
                    TrendDirection::Stable
                };

                trends.insert(
                    metric.clone(),
                    MetricTrend {
                        direction,
                        slope,
                        confidence: (slope.abs() * 10.0).min(1.0),
                    },
                );
            }
        }

        TrendReport { trends }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendReport {
    pub trends: HashMap<MetricType, MetricTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTrend {
    pub direction: TrendDirection,
    pub slope: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_time_series() {
        let mut series = TimeSeries::new(MetricType::ErrorRate, 100);

        for i in 0..50 {
            series.add(DataPoint {
                timestamp: chrono::Utc::now(),
                value: i as f64,
            });
        }

        let values = series.values();
        assert_eq!(values.len(), 50);

        let trend = series.linear_trend();
        assert!(trend.is_some());
    }

    #[tokio::test]
    async fn test_prediction() {
        let engine = PredictiveRemediationEngine::new(10);

        // Simulate increasing error rate
        for i in 0..20 {
            engine.record(MetricType::ErrorRate, (i * 5) as f64).await;
        }

        let predictions = engine.predict_failures().await;
        assert!(!predictions.is_empty());
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let mut series = TimeSeries::new(MetricType::CpuUsage, 100);

        // Normal values
        for _ in 0..50 {
            series.add(DataPoint {
                timestamp: chrono::Utc::now(),
                value: 50.0,
            });
        }

        // Anomaly
        series.add(DataPoint {
            timestamp: chrono::Utc::now(),
            value: 200.0,
        });

        let detector = AnomalyDetector::new(3.0);
        let anomalies = detector.detect_zscore(&series);

        assert!(!anomalies.is_empty());
    }
}
