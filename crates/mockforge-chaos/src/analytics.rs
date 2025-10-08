//! Chaos analytics and metrics aggregation

use crate::{
    scenario_recorder::{ChaosEvent, ChaosEventType},
    scenarios::ChaosScenario,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Time bucket for aggregated metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeBucket {
    /// 1-minute buckets
    Minute,
    /// 5-minute buckets
    FiveMinutes,
    /// 1-hour buckets
    Hour,
    /// 1-day buckets
    Day,
}

impl TimeBucket {
    /// Get duration for this bucket
    pub fn duration(&self) -> Duration {
        match self {
            TimeBucket::Minute => Duration::minutes(1),
            TimeBucket::FiveMinutes => Duration::minutes(5),
            TimeBucket::Hour => Duration::hours(1),
            TimeBucket::Day => Duration::days(1),
        }
    }

    /// Round timestamp to bucket boundary
    pub fn round_timestamp(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        let duration_secs = self.duration().num_seconds();
        let timestamp_secs = timestamp.timestamp();
        let rounded_secs = (timestamp_secs / duration_secs) * duration_secs;

        DateTime::from_timestamp(rounded_secs, 0).unwrap_or(timestamp)
    }
}

/// Aggregated chaos metrics for a time bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsBucket {
    /// Bucket timestamp
    pub timestamp: DateTime<Utc>,
    /// Bucket size
    pub bucket: TimeBucket,
    /// Total events in this bucket
    pub total_events: usize,
    /// Events by type
    pub events_by_type: HashMap<String, usize>,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Max latency (ms)
    pub max_latency_ms: u64,
    /// Min latency (ms)
    pub min_latency_ms: u64,
    /// Total faults injected
    pub total_faults: usize,
    /// Faults by type
    pub faults_by_type: HashMap<String, usize>,
    /// Rate limit violations
    pub rate_limit_violations: usize,
    /// Traffic shaping events
    pub traffic_shaping_events: usize,
    /// Protocol events
    pub protocol_events: HashMap<String, usize>,
    /// Affected endpoints
    pub affected_endpoints: HashMap<String, usize>,
}

impl MetricsBucket {
    /// Create a new empty metrics bucket
    pub fn new(timestamp: DateTime<Utc>, bucket: TimeBucket) -> Self {
        Self {
            timestamp: bucket.round_timestamp(timestamp),
            bucket,
            total_events: 0,
            events_by_type: HashMap::new(),
            avg_latency_ms: 0.0,
            max_latency_ms: 0,
            min_latency_ms: u64::MAX,
            total_faults: 0,
            faults_by_type: HashMap::new(),
            rate_limit_violations: 0,
            traffic_shaping_events: 0,
            protocol_events: HashMap::new(),
            affected_endpoints: HashMap::new(),
        }
    }

    /// Add an event to this bucket
    pub fn add_event(&mut self, event: &ChaosEvent) {
        self.total_events += 1;

        // Count by event type
        let event_type_name = Self::event_type_name(&event.event_type);
        *self.events_by_type.entry(event_type_name).or_insert(0) += 1;

        // Process specific event types
        match &event.event_type {
            ChaosEventType::LatencyInjection { delay_ms, endpoint } => {
                // Update latency stats
                self.update_latency_stats(*delay_ms);

                // Track affected endpoint
                if let Some(ep) = endpoint {
                    *self.affected_endpoints.entry(ep.clone()).or_insert(0) += 1;
                }
            }
            ChaosEventType::FaultInjection { fault_type, endpoint } => {
                self.total_faults += 1;
                *self.faults_by_type.entry(fault_type.clone()).or_insert(0) += 1;

                if let Some(ep) = endpoint {
                    *self.affected_endpoints.entry(ep.clone()).or_insert(0) += 1;
                }
            }
            ChaosEventType::RateLimitExceeded { endpoint, .. } => {
                self.rate_limit_violations += 1;

                if let Some(ep) = endpoint {
                    *self.affected_endpoints.entry(ep.clone()).or_insert(0) += 1;
                }
            }
            ChaosEventType::TrafficShaping { .. } => {
                self.traffic_shaping_events += 1;
            }
            ChaosEventType::ProtocolEvent { protocol, .. } => {
                *self.protocol_events.entry(protocol.clone()).or_insert(0) += 1;
            }
            ChaosEventType::ScenarioTransition { .. } => {
                // Just counted in total_events
            }
        }
    }

    /// Update latency statistics
    fn update_latency_stats(&mut self, delay_ms: u64) {
        // Update min/max
        self.max_latency_ms = self.max_latency_ms.max(delay_ms);
        self.min_latency_ms = self.min_latency_ms.min(delay_ms);

        // Update average (incremental)
        let n = self.events_by_type.get("LatencyInjection").copied().unwrap_or(1);
        self.avg_latency_ms = ((self.avg_latency_ms * (n - 1) as f64) + delay_ms as f64) / n as f64;
    }

    /// Get event type name as string
    fn event_type_name(event_type: &ChaosEventType) -> String {
        match event_type {
            ChaosEventType::LatencyInjection { .. } => "LatencyInjection".to_string(),
            ChaosEventType::FaultInjection { .. } => "FaultInjection".to_string(),
            ChaosEventType::RateLimitExceeded { .. } => "RateLimitExceeded".to_string(),
            ChaosEventType::TrafficShaping { .. } => "TrafficShaping".to_string(),
            ChaosEventType::ProtocolEvent { .. } => "ProtocolEvent".to_string(),
            ChaosEventType::ScenarioTransition { .. } => "ScenarioTransition".to_string(),
        }
    }
}

/// Chaos impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosImpact {
    /// Analysis period
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,

    /// Total chaos events
    pub total_events: usize,

    /// Impact severity (0.0 - 1.0)
    /// Based on event frequency, latency, and fault rate
    pub severity_score: f64,

    /// Most affected endpoints
    pub top_affected_endpoints: Vec<(String, usize)>,

    /// Chaos distribution
    pub event_distribution: HashMap<String, usize>,

    /// Average system degradation percentage
    pub avg_degradation_percent: f64,

    /// Peak chaos time
    pub peak_chaos_time: Option<DateTime<Utc>>,
    pub peak_chaos_events: usize,
}

impl ChaosImpact {
    /// Calculate impact from metrics buckets
    pub fn from_buckets(buckets: &[MetricsBucket]) -> Self {
        if buckets.is_empty() {
            return Self::empty();
        }

        let start_time = buckets.first().unwrap().timestamp;
        let end_time = buckets.last().unwrap().timestamp;

        let mut total_events = 0;
        let mut endpoint_counts: HashMap<String, usize> = HashMap::new();
        let mut event_distribution: HashMap<String, usize> = HashMap::new();
        let mut peak_chaos_events = 0;
        let mut peak_chaos_time = None;

        for bucket in buckets {
            total_events += bucket.total_events;

            // Track peak
            if bucket.total_events > peak_chaos_events {
                peak_chaos_events = bucket.total_events;
                peak_chaos_time = Some(bucket.timestamp);
            }

            // Aggregate endpoint counts
            for (endpoint, count) in &bucket.affected_endpoints {
                *endpoint_counts.entry(endpoint.clone()).or_insert(0) += count;
            }

            // Aggregate event distribution
            for (event_type, count) in &bucket.events_by_type {
                *event_distribution.entry(event_type.clone()).or_insert(0) += count;
            }
        }

        // Calculate severity score (0.0 - 1.0)
        let avg_events_per_bucket = total_events as f64 / buckets.len() as f64;
        let severity_score = (avg_events_per_bucket / 100.0).min(1.0); // Normalize to 0-1

        // Get top affected endpoints
        let mut top_affected: Vec<_> = endpoint_counts.into_iter().collect();
        top_affected.sort_by(|a, b| b.1.cmp(&a.1));
        top_affected.truncate(10); // Top 10

        // Calculate degradation (simplified: based on latency and faults)
        let avg_degradation_percent = severity_score * 100.0;

        Self {
            start_time,
            end_time,
            total_events,
            severity_score,
            top_affected_endpoints: top_affected,
            event_distribution,
            avg_degradation_percent,
            peak_chaos_time,
            peak_chaos_events,
        }
    }

    /// Create empty impact analysis
    fn empty() -> Self {
        Self {
            start_time: Utc::now(),
            end_time: Utc::now(),
            total_events: 0,
            severity_score: 0.0,
            top_affected_endpoints: vec![],
            event_distribution: HashMap::new(),
            avg_degradation_percent: 0.0,
            peak_chaos_time: None,
            peak_chaos_events: 0,
        }
    }
}

/// Chaos analytics engine
pub struct ChaosAnalytics {
    /// Metrics buckets by time
    buckets: Arc<RwLock<HashMap<(DateTime<Utc>, TimeBucket), MetricsBucket>>>,
    /// Maximum buckets to retain
    max_buckets: usize,
}

impl ChaosAnalytics {
    /// Create a new analytics engine
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            max_buckets: 1440, // 24 hours of minute buckets
        }
    }

    /// Set maximum buckets to retain
    pub fn with_max_buckets(mut self, max: usize) -> Self {
        self.max_buckets = max;
        self
    }

    /// Record an event
    pub fn record_event(&self, event: &ChaosEvent, bucket_size: TimeBucket) {
        let bucket_timestamp = bucket_size.round_timestamp(event.timestamp);
        let key = (bucket_timestamp, bucket_size);

        let mut buckets = self.buckets.write().unwrap();

        // Get or create bucket
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| MetricsBucket::new(bucket_timestamp, bucket_size));

        bucket.add_event(event);

        // Cleanup old buckets if needed
        if buckets.len() > self.max_buckets {
            self.cleanup_old_buckets(&mut buckets);
        }
    }

    /// Get metrics for a time range
    pub fn get_metrics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size: TimeBucket,
    ) -> Vec<MetricsBucket> {
        let buckets = self.buckets.read().unwrap();

        let mut result: Vec<_> = buckets
            .iter()
            .filter(|((timestamp, size), _)| {
                *size == bucket_size && *timestamp >= start && *timestamp <= end
            })
            .map(|(_, bucket)| bucket.clone())
            .collect();

        result.sort_by_key(|b| b.timestamp);
        result
    }

    /// Get chaos impact analysis
    pub fn get_impact_analysis(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size: TimeBucket,
    ) -> ChaosImpact {
        let buckets = self.get_metrics(start, end, bucket_size);
        ChaosImpact::from_buckets(&buckets)
    }

    /// Get current metrics (last N minutes)
    pub fn get_current_metrics(&self, minutes: i64, bucket_size: TimeBucket) -> Vec<MetricsBucket> {
        let end = Utc::now();
        let start = end - Duration::minutes(minutes);
        self.get_metrics(start, end, bucket_size)
    }

    /// Cleanup old buckets
    fn cleanup_old_buckets(&self, buckets: &mut HashMap<(DateTime<Utc>, TimeBucket), MetricsBucket>) {
        if buckets.len() <= self.max_buckets {
            return;
        }

        // Find oldest buckets and remove them
        let mut timestamps: Vec<_> = buckets.keys().map(|(ts, _)| *ts).collect();
        timestamps.sort();

        let keep_from = timestamps.len().saturating_sub(self.max_buckets);
        let remove_before = timestamps.get(keep_from).copied().unwrap_or(Utc::now());

        buckets.retain(|(ts, _), _| *ts >= remove_before);
    }

    /// Clear all analytics data
    pub fn clear(&self) {
        let mut buckets = self.buckets.write().unwrap();
        buckets.clear();
    }
}

impl Default for ChaosAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use std::collections::HashMap;

    #[test]
    fn test_time_bucket_rounding() {
        let timestamp = DateTime::parse_from_rfc3339("2025-10-07T12:34:56Z")
            .unwrap()
            .with_timezone(&Utc);

        let minute_bucket = TimeBucket::Minute;
        let rounded = minute_bucket.round_timestamp(timestamp);

        // Should round down to 12:34:00
        assert_eq!(rounded.minute(), 34);
        assert_eq!(rounded.second(), 0);
    }

    #[test]
    fn test_metrics_bucket_creation() {
        let timestamp = Utc::now();
        let bucket = MetricsBucket::new(timestamp, TimeBucket::Minute);

        assert_eq!(bucket.total_events, 0);
        assert_eq!(bucket.min_latency_ms, u64::MAX);
        assert_eq!(bucket.max_latency_ms, 0);
    }

    #[test]
    fn test_add_event_to_bucket() {
        let mut bucket = MetricsBucket::new(Utc::now(), TimeBucket::Minute);

        let event = ChaosEvent {
            timestamp: Utc::now(),
            event_type: ChaosEventType::LatencyInjection {
                delay_ms: 100,
                endpoint: Some("/api/test".to_string()),
            },
            metadata: HashMap::new(),
        };

        bucket.add_event(&event);

        assert_eq!(bucket.total_events, 1);
        assert_eq!(bucket.avg_latency_ms, 100.0);
        assert_eq!(bucket.max_latency_ms, 100);
        assert_eq!(bucket.min_latency_ms, 100);
        assert_eq!(bucket.affected_endpoints.get("/api/test"), Some(&1));
    }

    #[test]
    fn test_analytics_record_event() {
        let analytics = ChaosAnalytics::new();

        let event = ChaosEvent {
            timestamp: Utc::now(),
            event_type: ChaosEventType::LatencyInjection {
                delay_ms: 100,
                endpoint: None,
            },
            metadata: HashMap::new(),
        };

        analytics.record_event(&event, TimeBucket::Minute);

        let metrics = analytics.get_current_metrics(1, TimeBucket::Minute);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_events, 1);
    }

    #[test]
    fn test_chaos_impact_empty() {
        let impact = ChaosImpact::from_buckets(&[]);
        assert_eq!(impact.total_events, 0);
        assert_eq!(impact.severity_score, 0.0);
    }
}
