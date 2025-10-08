//! Alert system for chaos events

use crate::{
    analytics::{ChaosImpact, MetricsBucket, TimeBucket},
    scenario_recorder::ChaosEvent,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertType {
    /// High event rate detected
    HighEventRate {
        events_per_minute: usize,
        threshold: usize,
    },
    /// High latency detected
    HighLatency {
        avg_latency_ms: f64,
        threshold_ms: u64,
    },
    /// High fault rate detected
    HighFaultRate {
        faults_per_minute: usize,
        threshold: usize,
    },
    /// Rate limit violations
    RateLimitViolations {
        violations_per_minute: usize,
        threshold: usize,
    },
    /// Endpoint under stress
    EndpointStress {
        endpoint: String,
        events_per_minute: usize,
        threshold: usize,
    },
    /// Chaos impact high
    HighImpact {
        severity_score: f64,
        threshold: f64,
    },
    /// Custom alert
    Custom {
        message: String,
        metadata: HashMap<String, String>,
    },
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: String,
    /// Alert timestamp
    pub timestamp: DateTime<Utc>,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert type and details
    pub alert_type: AlertType,
    /// Alert message
    pub message: String,
    /// Is this alert resolved
    pub resolved: bool,
    /// Resolution timestamp
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Alert {
    /// Create a new alert
    pub fn new(severity: AlertSeverity, alert_type: AlertType, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            severity,
            alert_type,
            message: message.into(),
            resolved: false,
            resolved_at: None,
        }
    }

    /// Resolve this alert
    pub fn resolve(&mut self) {
        self.resolved = true;
        self.resolved_at = Some(Utc::now());
    }
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Is this rule enabled
    pub enabled: bool,
    /// Severity for alerts from this rule
    pub severity: AlertSeverity,
    /// Rule type
    pub rule_type: AlertRuleType,
}

/// Alert rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertRuleType {
    /// Alert on event rate threshold
    EventRateThreshold {
        threshold: usize,
        window_minutes: i64,
    },
    /// Alert on latency threshold
    LatencyThreshold {
        threshold_ms: u64,
        window_minutes: i64,
    },
    /// Alert on fault rate threshold
    FaultRateThreshold {
        threshold: usize,
        window_minutes: i64,
    },
    /// Alert on rate limit violations
    RateLimitThreshold {
        threshold: usize,
        window_minutes: i64,
    },
    /// Alert on endpoint stress
    EndpointThreshold {
        endpoint: String,
        threshold: usize,
        window_minutes: i64,
    },
    /// Alert on chaos impact score
    ImpactThreshold {
        threshold: f64,
        window_minutes: i64,
    },
}

impl AlertRule {
    /// Create a new alert rule
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        severity: AlertSeverity,
        rule_type: AlertRuleType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            enabled: true,
            severity,
            rule_type,
        }
    }

    /// Check if this rule should fire based on metrics
    pub fn evaluate(&self, metrics: &[MetricsBucket]) -> Option<Alert> {
        if !self.enabled || metrics.is_empty() {
            return None;
        }

        match &self.rule_type {
            AlertRuleType::EventRateThreshold { threshold, .. } => {
                let total_events: usize = metrics.iter().map(|m| m.total_events).sum();
                let events_per_minute = total_events / metrics.len().max(1);

                if events_per_minute > *threshold {
                    Some(Alert::new(
                        self.severity,
                        AlertType::HighEventRate {
                            events_per_minute,
                            threshold: *threshold,
                        },
                        format!(
                            "High chaos event rate detected: {} events/min (threshold: {})",
                            events_per_minute, threshold
                        ),
                    ))
                } else {
                    None
                }
            }
            AlertRuleType::LatencyThreshold { threshold_ms, .. } => {
                let avg_latency: f64 = metrics.iter().map(|m| m.avg_latency_ms).sum::<f64>()
                    / metrics.len() as f64;

                if avg_latency > *threshold_ms as f64 {
                    Some(Alert::new(
                        self.severity,
                        AlertType::HighLatency {
                            avg_latency_ms: avg_latency,
                            threshold_ms: *threshold_ms,
                        },
                        format!(
                            "High latency detected: {:.0}ms (threshold: {}ms)",
                            avg_latency, threshold_ms
                        ),
                    ))
                } else {
                    None
                }
            }
            AlertRuleType::FaultRateThreshold { threshold, .. } => {
                let total_faults: usize = metrics.iter().map(|m| m.total_faults).sum();
                let faults_per_minute = total_faults / metrics.len().max(1);

                if faults_per_minute > *threshold {
                    Some(Alert::new(
                        self.severity,
                        AlertType::HighFaultRate {
                            faults_per_minute,
                            threshold: *threshold,
                        },
                        format!(
                            "High fault injection rate detected: {} faults/min (threshold: {})",
                            faults_per_minute, threshold
                        ),
                    ))
                } else {
                    None
                }
            }
            AlertRuleType::RateLimitThreshold { threshold, .. } => {
                let total_violations: usize =
                    metrics.iter().map(|m| m.rate_limit_violations).sum();
                let violations_per_minute = total_violations / metrics.len().max(1);

                if violations_per_minute > *threshold {
                    Some(Alert::new(
                        self.severity,
                        AlertType::RateLimitViolations {
                            violations_per_minute,
                            threshold: *threshold,
                        },
                        format!(
                            "High rate limit violations: {} violations/min (threshold: {})",
                            violations_per_minute, threshold
                        ),
                    ))
                } else {
                    None
                }
            }
            AlertRuleType::EndpointThreshold {
                endpoint,
                threshold,
                ..
            } => {
                let endpoint_events: usize = metrics
                    .iter()
                    .map(|m| m.affected_endpoints.get(endpoint).copied().unwrap_or(0))
                    .sum();
                let events_per_minute = endpoint_events / metrics.len().max(1);

                if events_per_minute > *threshold {
                    Some(Alert::new(
                        self.severity,
                        AlertType::EndpointStress {
                            endpoint: endpoint.clone(),
                            events_per_minute,
                            threshold: *threshold,
                        },
                        format!(
                            "Endpoint '{}' under chaos stress: {} events/min (threshold: {})",
                            endpoint, events_per_minute, threshold
                        ),
                    ))
                } else {
                    None
                }
            }
            AlertRuleType::ImpactThreshold { threshold, .. } => {
                // This would need ChaosImpact from analytics
                // For now, return None - would be implemented with full integration
                None
            }
        }
    }
}

/// Alert handler trait
pub trait AlertHandler: Send + Sync {
    /// Handle an alert
    fn handle(&self, alert: &Alert);
}

/// Console alert handler (logs to console)
pub struct ConsoleAlertHandler;

impl AlertHandler for ConsoleAlertHandler {
    fn handle(&self, alert: &Alert) {
        match alert.severity {
            AlertSeverity::Info => info!("[ALERT] {}: {}", alert.id, alert.message),
            AlertSeverity::Warning => warn!("[ALERT] {}: {}", alert.id, alert.message),
            AlertSeverity::Critical => {
                tracing::error!("[ALERT] {}: {}", alert.id, alert.message)
            }
        }
    }
}

/// Alert manager
pub struct AlertManager {
    /// Alert rules
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Alert history
    alert_history: Arc<RwLock<Vec<Alert>>>,
    /// Alert handlers
    handlers: Arc<RwLock<Vec<Box<dyn AlertHandler>>>>,
    /// Maximum history size
    max_history: usize,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(vec![Box::new(ConsoleAlertHandler)])),
            max_history: 1000,
        }
    }

    /// Add an alert rule
    pub fn add_rule(&self, rule: AlertRule) {
        let id = rule.id.clone();
        let mut rules = self.rules.write().unwrap();
        rules.insert(id.clone(), rule);
        info!("Added alert rule: {}", id);
    }

    /// Remove an alert rule
    pub fn remove_rule(&self, id: &str) -> Option<AlertRule> {
        let mut rules = self.rules.write().unwrap();
        let removed = rules.remove(id);
        if removed.is_some() {
            info!("Removed alert rule: {}", id);
        }
        removed
    }

    /// Enable/disable a rule
    pub fn set_rule_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let mut rules = self.rules.write().unwrap();
        if let Some(rule) = rules.get_mut(id) {
            rule.enabled = enabled;
            info!("Alert rule '{}' {}", id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(format!("Rule '{}' not found", id))
        }
    }

    /// Get all rules
    pub fn get_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules.read().unwrap();
        rules.values().cloned().collect()
    }

    /// Evaluate all rules against metrics
    pub fn evaluate_rules(&self, metrics: &[MetricsBucket]) {
        let rules = self.rules.read().unwrap();

        for rule in rules.values() {
            if let Some(alert) = rule.evaluate(metrics) {
                self.fire_alert(alert);
            }
        }
    }

    /// Fire an alert
    pub fn fire_alert(&self, alert: Alert) {
        debug!("Firing alert: {} - {}", alert.id, alert.message);

        // Store in active alerts
        {
            let mut active = self.active_alerts.write().unwrap();
            active.insert(alert.id.clone(), alert.clone());
        }

        // Add to history
        {
            let mut history = self.alert_history.write().unwrap();
            history.push(alert.clone());

            // Trim history if needed
            if history.len() > self.max_history {
                let excess = history.len() - self.max_history;
                history.drain(0..excess);
            }
        }

        // Notify handlers
        let handlers = self.handlers.read().unwrap();
        for handler in handlers.iter() {
            handler.handle(&alert);
        }
    }

    /// Resolve an alert
    pub fn resolve_alert(&self, alert_id: &str) -> Result<(), String> {
        let mut alert = {
            let mut active = self.active_alerts.write().unwrap();
            active.remove(alert_id)
        };

        if let Some(ref mut alert_ref) = alert {
            alert_ref.resolve();

            // Update in history
            let mut history = self.alert_history.write().unwrap();
            if let Some(historical_alert) = history.iter_mut().find(|a| a.id == alert_id) {
                *historical_alert = alert_ref.clone();
            }

            info!("Resolved alert: {}", alert_id);
            Ok(())
        } else {
            Err(format!("Alert '{}' not found", alert_id))
        }
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        let active = self.active_alerts.read().unwrap();
        active.values().cloned().collect()
    }

    /// Get alert history
    pub fn get_alert_history(&self, limit: Option<usize>) -> Vec<Alert> {
        let history = self.alert_history.read().unwrap();
        let mut alerts: Vec<_> = history.clone();

        if let Some(limit) = limit {
            alerts.truncate(limit);
        }

        alerts
    }

    /// Add a custom alert handler
    pub fn add_handler(&self, handler: Box<dyn AlertHandler>) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push(handler);
    }

    /// Clear all alerts
    pub fn clear_alerts(&self) {
        let mut active = self.active_alerts.write().unwrap();
        let mut history = self.alert_history.write().unwrap();
        active.clear();
        history.clear();
        info!("Cleared all alerts");
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::MetricsBucket;

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            AlertSeverity::Warning,
            AlertType::HighEventRate {
                events_per_minute: 100,
                threshold: 50,
            },
            "Test alert",
        );

        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.resolved);
    }

    #[test]
    fn test_alert_resolve() {
        let mut alert = Alert::new(
            AlertSeverity::Info,
            AlertType::Custom {
                message: "test".to_string(),
                metadata: HashMap::new(),
            },
            "Test",
        );

        alert.resolve();
        assert!(alert.resolved);
        assert!(alert.resolved_at.is_some());
    }

    #[test]
    fn test_alert_rule_evaluation() {
        let rule = AlertRule::new(
            "test_rule",
            "Test Rule",
            AlertSeverity::Warning,
            AlertRuleType::EventRateThreshold {
                threshold: 50,
                window_minutes: 1,
            },
        );

        let mut bucket = MetricsBucket::new(Utc::now(), TimeBucket::Minute);
        bucket.total_events = 100;

        let alert = rule.evaluate(&[bucket]);
        assert!(alert.is_some());

        let alert = alert.unwrap();
        assert_eq!(alert.severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_alert_manager() {
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "test_rule",
            "Test Rule",
            AlertSeverity::Info,
            AlertRuleType::EventRateThreshold {
                threshold: 10,
                window_minutes: 1,
            },
        );

        manager.add_rule(rule);

        let rules = manager.get_rules();
        assert_eq!(rules.len(), 1);

        manager.remove_rule("test_rule");
        let rules = manager.get_rules();
        assert_eq!(rules.len(), 0);
    }
}
