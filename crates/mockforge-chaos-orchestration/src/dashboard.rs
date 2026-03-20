//! Real-time dashboard WebSocket support

use crate::{
    alerts::{Alert, AlertManager},
    analytics::{ChaosAnalytics, ChaosImpact, MetricsBucket, TimeBucket},
    scenario_orchestrator::OrchestrationStatus,
    scenario_replay::ReplayStatus,
};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Dashboard update message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DashboardUpdate {
    /// Metrics update
    Metrics {
        timestamp: DateTime<Utc>,
        bucket: MetricsBucket,
    },
    /// Alert fired
    AlertFired { alert: Alert },
    /// Alert resolved
    AlertResolved { alert_id: String },
    /// Scenario status change
    ScenarioStatus {
        scenario_name: String,
        status: String,
        progress: Option<f64>,
    },
    /// Orchestration status
    OrchestrationStatus { status: Option<OrchestrationStatus> },
    /// Replay status
    ReplayStatus { status: Option<ReplayStatus> },
    /// Impact analysis update
    ImpactUpdate { impact: ChaosImpact },
    /// Schedule update
    ScheduleUpdate {
        schedule_id: String,
        next_execution: Option<DateTime<Utc>>,
    },
    /// Health check / keepalive
    Ping { timestamp: DateTime<Utc> },
}

/// Dashboard statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    /// Current timestamp
    pub timestamp: DateTime<Utc>,

    /// Total events in last hour
    pub events_last_hour: usize,

    /// Total events in last 24 hours
    pub events_last_day: usize,

    /// Average latency (ms)
    pub avg_latency_ms: f64,

    /// Total faults in last hour
    pub faults_last_hour: usize,

    /// Active alerts count
    pub active_alerts: usize,

    /// Total scheduled scenarios
    pub scheduled_scenarios: usize,

    /// Active orchestrations
    pub active_orchestrations: usize,

    /// Active replays
    pub active_replays: usize,

    /// Current chaos impact score (0.0 - 1.0)
    pub current_impact_score: f64,

    /// Top affected endpoints
    pub top_endpoints: Vec<(String, usize)>,
}

impl DashboardStats {
    /// Create empty stats
    pub fn empty() -> Self {
        Self {
            timestamp: Utc::now(),
            events_last_hour: 0,
            events_last_day: 0,
            avg_latency_ms: 0.0,
            faults_last_hour: 0,
            active_alerts: 0,
            scheduled_scenarios: 0,
            active_orchestrations: 0,
            active_replays: 0,
            current_impact_score: 0.0,
            top_endpoints: vec![],
        }
    }

    /// Calculate stats from analytics
    pub fn from_analytics(analytics: &ChaosAnalytics, alert_manager: &AlertManager) -> Self {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let one_day_ago = now - Duration::days(1);

        // Get metrics for last hour and day
        let hour_metrics = analytics.get_metrics(one_hour_ago, now, TimeBucket::Minute);
        let day_metrics = analytics.get_metrics(one_day_ago, now, TimeBucket::Hour);

        // Calculate statistics
        let events_last_hour: usize = hour_metrics.iter().map(|m| m.total_events).sum();
        let events_last_day: usize = day_metrics.iter().map(|m| m.total_events).sum();

        let avg_latency_ms = if !hour_metrics.is_empty() {
            hour_metrics.iter().map(|m| m.avg_latency_ms).sum::<f64>() / hour_metrics.len() as f64
        } else {
            0.0
        };

        let faults_last_hour: usize = hour_metrics.iter().map(|m| m.total_faults).sum();

        // Get active alerts
        let active_alerts = alert_manager.get_active_alerts().len();

        // Get impact analysis
        let impact = analytics.get_impact_analysis(one_hour_ago, now, TimeBucket::Minute);

        Self {
            timestamp: now,
            events_last_hour,
            events_last_day,
            avg_latency_ms,
            faults_last_hour,
            active_alerts,
            scheduled_scenarios: 0,   // Would be populated from scheduler
            active_orchestrations: 0, // Would be populated from orchestrator
            active_replays: 0,        // Would be populated from replay engine
            current_impact_score: impact.severity_score,
            top_endpoints: impact.top_affected_endpoints,
        }
    }
}

/// Dashboard manager
pub struct DashboardManager {
    /// Analytics engine
    analytics: Arc<ChaosAnalytics>,
    /// Alert manager
    alert_manager: Arc<AlertManager>,
    /// Update broadcaster
    update_tx: broadcast::Sender<DashboardUpdate>,
    /// Last stats snapshot
    last_stats: Arc<RwLock<DashboardStats>>,
}

impl DashboardManager {
    /// Create a new dashboard manager
    pub fn new(analytics: Arc<ChaosAnalytics>, alert_manager: Arc<AlertManager>) -> Self {
        let (update_tx, _) = broadcast::channel(100);

        Self {
            analytics,
            alert_manager,
            update_tx,
            last_stats: Arc::new(RwLock::new(DashboardStats::empty())),
        }
    }

    /// Subscribe to dashboard updates
    pub fn subscribe(&self) -> broadcast::Receiver<DashboardUpdate> {
        self.update_tx.subscribe()
    }

    /// Send a dashboard update
    pub fn send_update(&self, update: DashboardUpdate) {
        debug!("Sending dashboard update: {:?}", update);
        let _ = self.update_tx.send(update);
    }

    /// Broadcast metrics update
    pub fn broadcast_metrics(&self, bucket: MetricsBucket) {
        self.send_update(DashboardUpdate::Metrics {
            timestamp: Utc::now(),
            bucket,
        });
    }

    /// Broadcast alert
    pub fn broadcast_alert(&self, alert: Alert) {
        self.send_update(DashboardUpdate::AlertFired { alert });
    }

    /// Broadcast alert resolution
    pub fn broadcast_alert_resolved(&self, alert_id: String) {
        self.send_update(DashboardUpdate::AlertResolved { alert_id });
    }

    /// Broadcast scenario status
    pub fn broadcast_scenario_status(
        &self,
        scenario_name: String,
        status: String,
        progress: Option<f64>,
    ) {
        self.send_update(DashboardUpdate::ScenarioStatus {
            scenario_name,
            status,
            progress,
        });
    }

    /// Broadcast impact update
    pub fn broadcast_impact(&self, impact: ChaosImpact) {
        self.send_update(DashboardUpdate::ImpactUpdate { impact });
    }

    /// Send ping (keepalive)
    pub fn send_ping(&self) {
        self.send_update(DashboardUpdate::Ping {
            timestamp: Utc::now(),
        });
    }

    /// Get current statistics
    pub fn get_stats(&self) -> DashboardStats {
        let mut stats = self.last_stats.write();
        *stats = DashboardStats::from_analytics(&self.analytics, &self.alert_manager);
        stats.clone()
    }

    /// Get metrics for time range
    pub fn get_metrics_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size: TimeBucket,
    ) -> Vec<MetricsBucket> {
        self.analytics.get_metrics(start, end, bucket_size)
    }

    /// Get impact analysis
    pub fn get_impact_analysis(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> ChaosImpact {
        self.analytics.get_impact_analysis(start, end, TimeBucket::Minute)
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alert_manager.get_active_alerts()
    }

    /// Get alert history
    pub fn get_alert_history(&self, limit: Option<usize>) -> Vec<Alert> {
        self.alert_manager.get_alert_history(limit)
    }

    /// Start background update loop
    pub async fn start_update_loop(&self, interval_seconds: u64) {
        let analytics = Arc::clone(&self.analytics);
        let _alert_manager = Arc::clone(&self.alert_manager);
        let update_tx = self.update_tx.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_seconds));

            loop {
                interval.tick().await;

                // Send ping
                let _ = update_tx.send(DashboardUpdate::Ping {
                    timestamp: Utc::now(),
                });

                // Calculate and broadcast impact
                let now = Utc::now();
                let one_hour_ago = now - Duration::hours(1);
                let impact = analytics.get_impact_analysis(one_hour_ago, now, TimeBucket::Minute);

                let _ = update_tx.send(DashboardUpdate::ImpactUpdate { impact });

                // Check for new metrics
                let recent_metrics = analytics.get_current_metrics(1, TimeBucket::Minute);
                if let Some(latest) = recent_metrics.last() {
                    let _ = update_tx.send(DashboardUpdate::Metrics {
                        timestamp: Utc::now(),
                        bucket: latest.clone(),
                    });
                }
            }
        });

        info!("Dashboard update loop started (interval: {}s)", interval_seconds);
    }
}

/// Dashboard query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct DashboardQuery {
    /// Start time (ISO 8601)
    pub start: Option<String>,
    /// End time (ISO 8601)
    pub end: Option<String>,
    /// Bucket size (minute, hour, day)
    pub bucket: Option<String>,
    /// Limit
    pub limit: Option<usize>,
}

impl DashboardQuery {
    /// Parse start time
    pub fn parse_start(&self) -> Option<DateTime<Utc>> {
        self.start
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)))
    }

    /// Parse end time
    pub fn parse_end(&self) -> Option<DateTime<Utc>> {
        self.end
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)))
    }

    /// Parse bucket size
    pub fn parse_bucket(&self) -> TimeBucket {
        match self.bucket.as_deref() {
            Some("minute") | Some("1m") => TimeBucket::Minute,
            Some("5minutes") | Some("5m") => TimeBucket::FiveMinutes,
            Some("hour") | Some("1h") => TimeBucket::Hour,
            Some("day") | Some("1d") => TimeBucket::Day,
            _ => TimeBucket::Minute,
        }
    }

    /// Get time range (defaults to last hour)
    pub fn get_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let end = self.parse_end().unwrap_or_else(Utc::now);
        let start = self.parse_start().unwrap_or_else(|| end - Duration::hours(1));
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_stats_empty() {
        let stats = DashboardStats::empty();
        assert_eq!(stats.events_last_hour, 0);
        assert_eq!(stats.active_alerts, 0);
    }

    #[test]
    fn test_dashboard_query_defaults() {
        let query = DashboardQuery {
            start: None,
            end: None,
            bucket: None,
            limit: None,
        };

        let (start, end) = query.get_range();
        assert!(end > start);
        assert_eq!(query.parse_bucket(), TimeBucket::Minute);
    }

    #[test]
    fn test_dashboard_query_parsing() {
        let query = DashboardQuery {
            start: Some("2025-10-07T12:00:00Z".to_string()),
            end: Some("2025-10-07T13:00:00Z".to_string()),
            bucket: Some("hour".to_string()),
            limit: Some(100),
        };

        let (start, end) = query.get_range();
        assert_eq!(start.to_rfc3339(), "2025-10-07T12:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2025-10-07T13:00:00+00:00");
        assert_eq!(query.parse_bucket(), TimeBucket::Hour);
    }

    #[tokio::test]
    async fn test_dashboard_manager_creation() {
        let analytics = Arc::new(ChaosAnalytics::new());
        let alert_manager = Arc::new(AlertManager::new());
        let manager = DashboardManager::new(analytics, alert_manager);

        let stats = manager.get_stats();
        assert_eq!(stats.events_last_hour, 0);
    }

    #[tokio::test]
    async fn test_dashboard_subscribe() {
        let analytics = Arc::new(ChaosAnalytics::new());
        let alert_manager = Arc::new(AlertManager::new());
        let manager = DashboardManager::new(analytics, alert_manager);

        let mut rx = manager.subscribe();

        manager.send_ping();

        let update = rx.recv().await.unwrap();
        match update {
            DashboardUpdate::Ping { .. } => {}
            _ => panic!("Expected Ping update"),
        }
    }
}
