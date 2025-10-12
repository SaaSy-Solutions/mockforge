//! Advanced analytics engine for chaos engineering
//!
//! Provides predictive analytics, anomaly detection, and intelligent insights.

use crate::{
    analytics::{ChaosAnalytics, MetricsBucket, TimeBucket},
    scenario_recorder::ChaosEvent,
};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Anomaly detected in chaos patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Anomaly ID
    pub id: String,
    /// Detection time
    pub detected_at: DateTime<Utc>,
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
    /// Description
    pub description: String,
    /// Affected metrics
    pub affected_metrics: Vec<String>,
    /// Suggested actions
    pub suggested_actions: Vec<String>,
}

/// Types of anomalies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    /// Sudden spike in events
    EventSpike,
    /// Unusual latency patterns
    LatencyAnomaly,
    /// High error rate
    HighErrorRate,
    /// Resource exhaustion pattern
    ResourceExhaustion,
    /// Cascading failures
    CascadingFailure,
    /// Unexpected quiet period
    UnexpectedQuiet,
}

/// Predictive insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveInsight {
    /// Insight ID
    pub id: String,
    /// Generated at
    pub generated_at: DateTime<Utc>,
    /// Predicted metric
    pub metric: String,
    /// Predicted value
    pub predicted_value: f64,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Time horizon
    pub time_horizon_minutes: i64,
    /// Recommendation
    pub recommendation: String,
}

/// Trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Metric name
    pub metric: String,
    /// Analysis period
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    /// Trend direction
    pub trend: TrendDirection,
    /// Rate of change
    pub rate_of_change: f64,
    /// Statistical confidence
    pub confidence: f64,
    /// Data points
    pub data_points: Vec<DataPoint>,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

/// Data point for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Correlation analysis between metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationAnalysis {
    /// Metric A
    pub metric_a: String,
    /// Metric B
    pub metric_b: String,
    /// Correlation coefficient (-1.0 to 1.0)
    pub correlation: f64,
    /// Statistical significance
    pub p_value: f64,
    /// Interpretation
    pub interpretation: String,
}

/// System health score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthScore {
    /// Overall score (0.0 - 100.0)
    pub overall_score: f64,
    /// Component scores
    pub components: HashMap<String, f64>,
    /// Factors affecting score
    pub factors: Vec<HealthFactor>,
    /// Calculated at
    pub calculated_at: DateTime<Utc>,
}

/// Factor affecting health score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthFactor {
    pub name: String,
    pub impact: f64, // Positive or negative impact on score
    pub description: String,
}

/// Advanced analytics engine
pub struct AdvancedAnalyticsEngine {
    /// Base analytics
    base_analytics: Arc<ChaosAnalytics>,
    /// Detected anomalies
    anomalies: Arc<RwLock<Vec<Anomaly>>>,
    /// Historical events for pattern learning
    event_history: Arc<RwLock<VecDeque<ChaosEvent>>>,
    /// Maximum events to retain
    max_history_size: usize,
    /// Anomaly detection threshold
    anomaly_threshold: f64,
}

impl AdvancedAnalyticsEngine {
    /// Create a new advanced analytics engine
    pub fn new(base_analytics: Arc<ChaosAnalytics>) -> Self {
        Self {
            base_analytics,
            anomalies: Arc::new(RwLock::new(Vec::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size: 10000,
            anomaly_threshold: 0.7,
        }
    }

    /// Set maximum history size
    pub fn with_max_history(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    /// Set anomaly detection threshold
    pub fn with_anomaly_threshold(mut self, threshold: f64) -> Self {
        self.anomaly_threshold = threshold;
        self
    }

    /// Record and analyze an event
    pub fn record_event(&self, event: ChaosEvent) {
        // Add to base analytics
        self.base_analytics.record_event(&event, TimeBucket::Minute);

        // Add to history
        {
            let mut history = self.event_history.write();
            history.push_back(event.clone());

            // Trim history if needed
            while history.len() > self.max_history_size {
                history.pop_front();
            }
        }

        // Check for anomalies
        self.detect_anomalies();
    }

    /// Detect anomalies in recent data
    pub fn detect_anomalies(&self) {
        let now = Utc::now();
        let recent_start = now - Duration::minutes(5);

        let recent_metrics = self.base_analytics.get_metrics(recent_start, now, TimeBucket::Minute);

        if recent_metrics.is_empty() {
            return;
        }

        // Calculate baseline from older data
        let baseline_start = now - Duration::minutes(30);
        let baseline_end = now - Duration::minutes(10);
        let baseline_metrics =
            self.base_analytics
                .get_metrics(baseline_start, baseline_end, TimeBucket::Minute);

        if baseline_metrics.is_empty() {
            return;
        }

        // Detect event spikes
        self.detect_event_spike(&recent_metrics, &baseline_metrics);

        // Detect latency anomalies
        self.detect_latency_anomaly(&recent_metrics, &baseline_metrics);

        // Detect high error rates
        self.detect_high_error_rate(&recent_metrics);
    }

    /// Detect event spikes
    fn detect_event_spike(&self, recent: &[MetricsBucket], baseline: &[MetricsBucket]) {
        let recent_avg =
            recent.iter().map(|b| b.total_events).sum::<usize>() as f64 / recent.len() as f64;
        let baseline_avg =
            baseline.iter().map(|b| b.total_events).sum::<usize>() as f64 / baseline.len() as f64;

        if baseline_avg > 0.0 {
            let spike_ratio = recent_avg / baseline_avg;

            if spike_ratio > 2.0 {
                let severity = (spike_ratio - 1.0).min(1.0);

                if severity >= self.anomaly_threshold {
                    let anomaly = Anomaly {
                        id: format!("event_spike_{}", Utc::now().timestamp()),
                        detected_at: Utc::now(),
                        anomaly_type: AnomalyType::EventSpike,
                        severity,
                        description: format!(
                            "Event rate spiked {:.1}x above baseline",
                            spike_ratio
                        ),
                        affected_metrics: vec!["total_events".to_string()],
                        suggested_actions: vec![
                            "Review recent configuration changes".to_string(),
                            "Check orchestration step frequency".to_string(),
                        ],
                    };

                    let mut anomalies = self.anomalies.write();
                    anomalies.push(anomaly);
                }
            }
        }
    }

    /// Detect latency anomalies
    fn detect_latency_anomaly(&self, recent: &[MetricsBucket], baseline: &[MetricsBucket]) {
        let recent_avg = recent.iter().map(|b| b.avg_latency_ms).sum::<f64>() / recent.len() as f64;
        let baseline_avg =
            baseline.iter().map(|b| b.avg_latency_ms).sum::<f64>() / baseline.len() as f64;

        if baseline_avg > 0.0 {
            let latency_ratio = recent_avg / baseline_avg;

            if !(0.5..=1.5).contains(&latency_ratio) {
                let severity = ((latency_ratio - 1.0).abs()).min(1.0);

                if severity >= self.anomaly_threshold {
                    let anomaly = Anomaly {
                        id: format!("latency_anomaly_{}", Utc::now().timestamp()),
                        detected_at: Utc::now(),
                        anomaly_type: AnomalyType::LatencyAnomaly,
                        severity,
                        description: format!(
                            "Latency changed {:.1}x from baseline ({:.1}ms vs {:.1}ms)",
                            latency_ratio, recent_avg, baseline_avg
                        ),
                        affected_metrics: vec!["avg_latency_ms".to_string()],
                        suggested_actions: vec![
                            "Review latency injection settings".to_string(),
                            "Check network conditions".to_string(),
                        ],
                    };

                    let mut anomalies = self.anomalies.write();
                    anomalies.push(anomaly);
                }
            }
        }
    }

    /// Detect high error rates
    fn detect_high_error_rate(&self, recent: &[MetricsBucket]) {
        let total_events: usize = recent.iter().map(|b| b.total_events).sum();
        let total_faults: usize = recent.iter().map(|b| b.total_faults).sum();

        if total_events > 0 {
            let error_rate = total_faults as f64 / total_events as f64;

            if error_rate > 0.5 {
                let severity = error_rate;

                if severity >= self.anomaly_threshold {
                    let anomaly = Anomaly {
                        id: format!("high_error_rate_{}", Utc::now().timestamp()),
                        detected_at: Utc::now(),
                        anomaly_type: AnomalyType::HighErrorRate,
                        severity,
                        description: format!("Error rate at {:.1}%", error_rate * 100.0),
                        affected_metrics: vec![
                            "total_faults".to_string(),
                            "total_events".to_string(),
                        ],
                        suggested_actions: vec![
                            "Review fault injection settings".to_string(),
                            "Check system resilience".to_string(),
                        ],
                    };

                    let mut anomalies = self.anomalies.write();
                    anomalies.push(anomaly);
                }
            }
        }
    }

    /// Get recent anomalies
    pub fn get_anomalies(&self, since: DateTime<Utc>) -> Vec<Anomaly> {
        let anomalies = self.anomalies.read();
        anomalies.iter().filter(|a| a.detected_at >= since).cloned().collect()
    }

    /// Perform trend analysis on a metric
    pub fn analyze_trend(
        &self,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> TrendAnalysis {
        let buckets = self.base_analytics.get_metrics(start, end, TimeBucket::FiveMinutes);

        let data_points: Vec<DataPoint> = buckets
            .iter()
            .map(|b| {
                let value = match metric_name {
                    "total_events" => b.total_events as f64,
                    "avg_latency_ms" => b.avg_latency_ms,
                    "total_faults" => b.total_faults as f64,
                    "rate_limit_violations" => b.rate_limit_violations as f64,
                    _ => 0.0,
                };

                DataPoint {
                    timestamp: b.timestamp,
                    value,
                }
            })
            .collect();

        // Calculate trend using simple linear regression
        let (trend, rate) = self.calculate_trend(&data_points);

        TrendAnalysis {
            metric: metric_name.to_string(),
            start_time: start,
            end_time: end,
            trend,
            rate_of_change: rate,
            confidence: 0.85, // Simplified - in production use statistical calculation
            data_points,
        }
    }

    /// Calculate trend direction and rate
    fn calculate_trend(&self, data_points: &[DataPoint]) -> (TrendDirection, f64) {
        if data_points.len() < 2 {
            return (TrendDirection::Stable, 0.0);
        }

        // Simple moving average comparison
        let first_half: Vec<f64> =
            data_points[..data_points.len() / 2].iter().map(|p| p.value).collect();
        let second_half: Vec<f64> =
            data_points[data_points.len() / 2..].iter().map(|p| p.value).collect();

        let first_avg: f64 = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let second_avg: f64 = second_half.iter().sum::<f64>() / second_half.len() as f64;

        let rate = if first_avg > 0.0 {
            (second_avg - first_avg) / first_avg
        } else {
            0.0
        };

        let trend = if rate > 0.1 {
            TrendDirection::Increasing
        } else if rate < -0.1 {
            TrendDirection::Decreasing
        } else if rate.abs() < 0.05 {
            TrendDirection::Stable
        } else {
            TrendDirection::Volatile
        };

        (trend, rate)
    }

    /// Generate predictive insights
    pub fn generate_insights(&self) -> Vec<PredictiveInsight> {
        let mut insights = Vec::new();

        // Analyze recent trends
        let now = Utc::now();
        let lookback = now - Duration::hours(1);

        let trend = self.analyze_trend("total_events", lookback, now);

        // Predict future event rate
        if trend.trend == TrendDirection::Increasing {
            insights.push(PredictiveInsight {
                id: format!("prediction_{}", Utc::now().timestamp()),
                generated_at: Utc::now(),
                metric: "total_events".to_string(),
                predicted_value: trend.rate_of_change * 1.2, // Simplified prediction
                confidence: trend.confidence,
                time_horizon_minutes: 30,
                recommendation: "Event rate is increasing. Consider scaling resources or adjusting chaos parameters.".to_string(),
            });
        }

        insights
    }

    /// Calculate system health score
    pub fn calculate_health_score(&self) -> HealthScore {
        let now = Utc::now();
        let lookback = now - Duration::minutes(15);

        let impact = self.base_analytics.get_impact_analysis(lookback, now, TimeBucket::Minute);

        let mut components = HashMap::new();
        let mut factors = Vec::new();

        // Calculate component scores
        let event_score = (1.0 - impact.severity_score) * 100.0;
        components.insert("chaos_impact".to_string(), event_score);

        if impact.severity_score > 0.5 {
            factors.push(HealthFactor {
                name: "High chaos severity".to_string(),
                impact: -20.0,
                description: "System under significant chaos load".to_string(),
            });
        }

        // Check for recent anomalies
        let recent_anomalies = self.get_anomalies(lookback);
        let anomaly_score = (1.0 - (recent_anomalies.len() as f64 * 0.1)).max(0.0) * 100.0;
        components.insert("anomaly_score".to_string(), anomaly_score);

        if !recent_anomalies.is_empty() {
            factors.push(HealthFactor {
                name: "Anomalies detected".to_string(),
                impact: -(recent_anomalies.len() as f64 * 5.0),
                description: format!("{} anomalies detected", recent_anomalies.len()),
            });
        }

        // Calculate overall score
        let overall_score = components.values().sum::<f64>() / components.len() as f64;

        HealthScore {
            overall_score,
            components,
            factors,
            calculated_at: Utc::now(),
        }
    }

    /// Clear all analytics data
    pub fn clear(&self) {
        self.base_analytics.clear();
        let mut anomalies = self.anomalies.write();
        anomalies.clear();
        let mut history = self.event_history.write();
        history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_engine_creation() {
        let base = Arc::new(ChaosAnalytics::new());
        let engine = AdvancedAnalyticsEngine::new(base);

        assert_eq!(engine.max_history_size, 10000);
        assert_eq!(engine.anomaly_threshold, 0.7);
    }

    #[test]
    fn test_trend_direction() {
        let base = Arc::new(ChaosAnalytics::new());
        let engine = AdvancedAnalyticsEngine::new(base);

        let data_points = vec![
            DataPoint {
                timestamp: Utc::now(),
                value: 10.0,
            },
            DataPoint {
                timestamp: Utc::now(),
                value: 20.0,
            },
        ];

        let (trend, rate) = engine.calculate_trend(&data_points);
        assert_eq!(trend, TrendDirection::Increasing);
        assert!(rate > 0.0);
    }
}
