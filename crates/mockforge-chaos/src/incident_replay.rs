//! Incident Replay - Convert production incident timelines into replayable chaos scenarios
//!
//! This module provides functionality to ingest production incident timelines (sequence of
//! status codes, latency spikes, error rates) and auto-generate chaos scenarios that
//! reproduce the incident conditions in mock environments.

use crate::config::{ChaosConfig, FaultInjectionConfig, LatencyConfig};
use crate::scenario_orchestrator::{OrchestratedScenario, ScenarioStep};
use crate::scenarios::ChaosScenario;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Production incident timeline
///
/// Represents a sequence of events during a production incident that can be
/// replayed as a chaos scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimeline {
    /// Unique incident identifier
    pub incident_id: String,
    /// Incident start time
    pub start_time: DateTime<Utc>,
    /// Incident end time
    pub end_time: DateTime<Utc>,
    /// Sequence of events during the incident
    pub events: Vec<IncidentEvent>,
    /// Additional metadata about the incident
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

/// An event in the incident timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentEvent {
    /// Event timestamp (relative to incident start or absolute)
    pub timestamp: DateTime<Utc>,
    /// Type of event
    pub event_type: IncidentEventType,
    /// Affected endpoint (if applicable)
    pub endpoint: Option<String>,
    /// HTTP method (if applicable)
    pub method: Option<String>,
    /// Additional event metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

/// Types of incident events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncidentEventType {
    /// Status code change event
    StatusCodeChange {
        /// HTTP status code
        code: u16,
        /// Rate/probability of this status code (0.0 to 1.0)
        rate: f64,
    },
    /// Latency spike event
    LatencySpike {
        /// Latency in milliseconds
        latency_ms: u64,
        /// Duration of the spike in seconds
        duration_seconds: Option<u64>,
    },
    /// Error rate increase event
    ErrorRateIncrease {
        /// Error rate (0.0 to 1.0)
        rate: f64,
        /// Error status codes
        error_codes: Option<Vec<u16>>,
    },
    /// Request pattern change event
    RequestPatternChange {
        /// Pattern description
        pattern: String,
        /// Request count change (positive = increase, negative = decrease)
        request_delta: Option<i64>,
    },
    /// Service degradation event
    ServiceDegradation {
        /// Degradation level (0.0 to 1.0, where 1.0 = complete failure)
        level: f64,
        /// Affected services/endpoints
        affected_services: Option<Vec<String>>,
    },
}

/// Incident replay generator
///
/// Converts incident timelines into replayable chaos scenarios.
pub struct IncidentReplayGenerator;

impl IncidentReplayGenerator {
    /// Create a new incident replay generator
    pub fn new() -> Self {
        Self
    }

    /// Generate a chaos scenario from an incident timeline
    ///
    /// # Arguments
    /// * `timeline` - The incident timeline to convert
    ///
    /// # Returns
    /// An orchestrated scenario that replays the incident
    pub fn generate_scenario(&self, timeline: &IncidentTimeline) -> OrchestratedScenario {
        let mut steps = Vec::new();
        let incident_duration = (timeline.end_time - timeline.start_time).num_seconds() as u64;

        // Group events by time windows to create scenario steps
        let time_windows = self.group_events_by_time_window(timeline);

        for (window_start, window_events) in time_windows {
            // Calculate delay before this step
            let delay_seconds = if window_start > timeline.start_time {
                (window_start - timeline.start_time).num_seconds() as u64
            } else {
                0
            };

            // Create chaos config for this time window
            let event_refs: Vec<&IncidentEvent> = window_events.iter().collect();
            let chaos_config = self.create_chaos_config_for_events(&event_refs);

            // Create scenario for this window
            let scenario =
                ChaosScenario::new(format!("incident_window_{}", delay_seconds), chaos_config)
                    .with_duration(self.calculate_window_duration(&event_refs, timeline));

            // Create scenario step
            let step = ScenarioStep::new(format!("step_at_{}s", delay_seconds), scenario)
                .with_delay_before(delay_seconds);

            steps.push(step);
        }

        // Create orchestrated scenario
        let mut scenario = OrchestratedScenario::new(format!("replay_{}", timeline.incident_id))
            .with_description(format!(
                "Replay of incident {} from {} to {}",
                timeline.incident_id, timeline.start_time, timeline.end_time
            ))
            .with_tags(vec!["incident-replay".to_string(), timeline.incident_id.clone()]);

        // Add steps
        for step in steps {
            scenario = scenario.add_step(step);
        }

        scenario
    }

    /// Group events by time windows
    ///
    /// Groups events into time windows (e.g., 30-second windows) to create
    /// discrete scenario steps.
    fn group_events_by_time_window(
        &self,
        timeline: &IncidentTimeline,
    ) -> Vec<(DateTime<Utc>, Vec<IncidentEvent>)> {
        let window_size_seconds = 30; // 30-second windows
        let mut windows: Vec<(DateTime<Utc>, Vec<IncidentEvent>)> = Vec::new();

        let mut current_window_start = timeline.start_time;
        let mut current_window_events = Vec::new();

        for event in &timeline.events {
            // Calculate which window this event belongs to
            let event_offset = (event.timestamp - timeline.start_time).num_seconds();
            let window_index = (event_offset / window_size_seconds) as i64;
            let window_start =
                timeline.start_time + Duration::seconds(window_index * window_size_seconds);

            if window_start != current_window_start {
                // Save current window and start new one
                if !current_window_events.is_empty() {
                    windows.push((current_window_start, current_window_events));
                }
                current_window_start = window_start;
                current_window_events = Vec::new();
            }

            current_window_events.push(event.clone());
        }

        // Add final window
        if !current_window_events.is_empty() {
            windows.push((current_window_start, current_window_events));
        }

        windows
    }

    /// Create chaos config for a set of events
    fn create_chaos_config_for_events<'a>(&self, events: &[&'a IncidentEvent]) -> ChaosConfig {
        let mut error_rate = 0.0_f64;
        let mut delay_rate = 0.0_f64;
        let mut min_delay_ms = 0_u64;
        let mut max_delay_ms = 0_u64;
        let mut status_codes = Vec::new();
        let mut inject_timeouts = false;

        for event in events {
            match &event.event_type {
                IncidentEventType::StatusCodeChange { code, rate } => {
                    error_rate = error_rate.max(*rate);
                    status_codes.push(*code);
                }
                IncidentEventType::LatencySpike { latency_ms, .. } => {
                    delay_rate = 1.0; // Always inject delay during spike
                    min_delay_ms = min_delay_ms.max(*latency_ms);
                    max_delay_ms = max_delay_ms.max(*latency_ms);
                }
                IncidentEventType::ErrorRateIncrease {
                    rate,
                    error_codes: codes,
                } => {
                    error_rate = error_rate.max(*rate);
                    if let Some(codes) = codes {
                        status_codes.extend(codes.iter().copied());
                    } else {
                        // Default error codes if not specified
                        status_codes.extend(vec![500, 502, 503, 504]);
                    }
                }
                IncidentEventType::ServiceDegradation { level, .. } => {
                    // Map degradation level to error rate
                    error_rate = error_rate.max(*level);
                    if *level > 0.8 {
                        inject_timeouts = true;
                    }
                }
                _ => {
                    // Other event types don't directly map to chaos config
                }
            }
        }

        // Ensure we have default status codes if none specified
        if status_codes.is_empty() {
            status_codes = vec![500, 502, 503, 504];
        }

        // Ensure delay range is valid
        if max_delay_ms == 0 && min_delay_ms > 0 {
            max_delay_ms = min_delay_ms;
        }

        // Clamp rates
        let error_rate = error_rate.min(1.0).max(0.0);
        let delay_rate = delay_rate.min(1.0).max(0.0);

        // Build latency config if needed
        let latency_config = if delay_rate > 0.0 && max_delay_ms > 0 {
            Some(LatencyConfig {
                enabled: true,
                fixed_delay_ms: if min_delay_ms == max_delay_ms {
                    Some(min_delay_ms.max(100))
                } else {
                    None
                },
                random_delay_range_ms: if min_delay_ms != max_delay_ms {
                    Some((min_delay_ms.max(100), max_delay_ms.max(min_delay_ms.max(100))))
                } else {
                    None
                },
                jitter_percent: 0.0,
                probability: delay_rate,
            })
        } else {
            None
        };

        // Build fault injection config if needed
        let fault_config = if error_rate > 0.0 && !status_codes.is_empty() {
            Some(FaultInjectionConfig {
                enabled: true,
                http_errors: status_codes,
                http_error_probability: error_rate,
                connection_errors: false,
                connection_error_probability: 0.0,
                timeout_errors: inject_timeouts,
                timeout_ms: 5000,
                timeout_probability: if inject_timeouts { error_rate } else { 0.0 },
                partial_responses: false,
                partial_response_probability: 0.0,
                payload_corruption: false,
                payload_corruption_probability: 0.0,
                corruption_type: crate::config::CorruptionType::None,
                error_pattern: None,
                mockai_enabled: false,
            })
        } else {
            None
        };

        ChaosConfig {
            enabled: true,
            latency: latency_config,
            fault_injection: fault_config,
            rate_limit: None,
            traffic_shaping: None,
            circuit_breaker: None,
            bulkhead: None,
        }
    }

    /// Calculate duration for a time window
    fn calculate_window_duration(
        &self,
        events: &[&IncidentEvent],
        timeline: &IncidentTimeline,
    ) -> u64 {
        // Find the maximum duration specified in events
        let mut max_duration = 30; // Default 30 seconds

        for event in events {
            if let IncidentEventType::LatencySpike {
                duration_seconds: Some(duration),
                ..
            } = &event.event_type
            {
                max_duration = max_duration.max(*duration);
            }
        }

        max_duration
    }

    /// Import incident timeline from JSON
    pub fn import_from_json(&self, json: &str) -> Result<IncidentTimeline, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Import incident timeline from YAML
    pub fn import_from_yaml(&self, yaml: &str) -> Result<IncidentTimeline, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Export scenario to JSON
    pub fn export_scenario_to_json(
        &self,
        scenario: &OrchestratedScenario,
    ) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(scenario)
    }

    /// Export scenario to YAML
    pub fn export_scenario_to_yaml(
        &self,
        scenario: &OrchestratedScenario,
    ) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(scenario)
    }
}

impl Default for IncidentReplayGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Import adapter for external incident formats
pub struct IncidentFormatAdapter;

impl IncidentFormatAdapter {
    /// Convert PagerDuty incident format to IncidentTimeline
    pub fn from_pagerduty(pagerduty_data: &Value) -> Result<IncidentTimeline, String> {
        // Extract incident data from PagerDuty format
        // This is a simplified implementation - real implementation would parse
        // PagerDuty's actual API response format
        let incident_id = pagerduty_data
            .get("incident")
            .and_then(|i| i.get("id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| "Missing incident.id".to_string())?
            .to_string();

        let created_at = pagerduty_data
            .get("incident")
            .and_then(|i| i.get("created_at"))
            .and_then(|ts| ts.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or_else(|| "Missing or invalid incident.created_at".to_string())?;

        let resolved_at = pagerduty_data
            .get("incident")
            .and_then(|i| i.get("resolved_at"))
            .and_then(|ts| ts.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());

        // Extract events from PagerDuty log entries or metrics
        let events = Self::extract_pagerduty_events(pagerduty_data)?;

        Ok(IncidentTimeline {
            incident_id,
            start_time: created_at,
            end_time: resolved_at,
            events,
            metadata: HashMap::new(),
        })
    }

    /// Extract events from PagerDuty data
    fn extract_pagerduty_events(pagerduty_data: &Value) -> Result<Vec<IncidentEvent>, String> {
        let mut events = Vec::new();

        // Try to extract from log entries
        if let Some(log_entries) = pagerduty_data.get("log_entries").and_then(|l| l.as_array()) {
            for entry in log_entries {
                if let Some(timestamp_str) = entry.get("created_at").and_then(|t| t.as_str()) {
                    if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_str) {
                        let timestamp = timestamp.with_timezone(&Utc);

                        // Try to extract event type from entry
                        if let Some(summary) = entry.get("summary").and_then(|s| s.as_str()) {
                            // Simple heuristic: look for error patterns
                            if summary.to_lowercase().contains("error") {
                                events.push(IncidentEvent {
                                    timestamp,
                                    event_type: IncidentEventType::ErrorRateIncrease {
                                        rate: 0.5, // Default rate
                                        error_codes: Some(vec![500]),
                                    },
                                    endpoint: None,
                                    method: None,
                                    metadata: HashMap::new(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    /// Convert Datadog incident format to IncidentTimeline
    pub fn from_datadog(datadog_data: &Value) -> Result<IncidentTimeline, String> {
        // Extract incident data from Datadog format
        let incident_id = datadog_data
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| "Missing id".to_string())?
            .to_string();

        let created_at = datadog_data
            .get("created")
            .and_then(|ts| ts.as_i64())
            .map(|ts| DateTime::from_timestamp(ts / 1000, 0).unwrap_or_else(Utc::now))
            .ok_or_else(|| "Missing or invalid created timestamp".to_string())?;

        let resolved_at = datadog_data
            .get("resolved")
            .and_then(|ts| ts.as_i64())
            .map(|ts| DateTime::from_timestamp(ts / 1000, 0).unwrap_or_else(Utc::now))
            .unwrap_or_else(|| Utc::now());

        // Extract events from Datadog metrics or logs
        let events = Self::extract_datadog_events(datadog_data)?;

        Ok(IncidentTimeline {
            incident_id,
            start_time: created_at,
            end_time: resolved_at,
            events,
            metadata: HashMap::new(),
        })
    }

    /// Extract events from Datadog data
    fn extract_datadog_events(datadog_data: &Value) -> Result<Vec<IncidentEvent>, String> {
        let mut events = Vec::new();

        // Try to extract from metrics
        if let Some(metrics) = datadog_data.get("metrics").and_then(|m| m.as_array()) {
            for metric in metrics {
                if let Some(points) = metric.get("points").and_then(|p| p.as_array()) {
                    for point in points {
                        if let Some((timestamp, value)) = point
                            .as_array()
                            .and_then(|arr| Some((arr.get(0)?.as_i64()?, arr.get(1)?.as_f64()?)))
                        {
                            let timestamp = DateTime::from_timestamp(timestamp, 0)
                                .unwrap_or_else(|| Utc::now());

                            // Check metric name to determine event type
                            if let Some(metric_name) = metric.get("metric").and_then(|m| m.as_str())
                            {
                                if metric_name.contains("latency")
                                    || metric_name.contains("duration")
                                {
                                    events.push(IncidentEvent {
                                        timestamp,
                                        event_type: IncidentEventType::LatencySpike {
                                            latency_ms: (value * 1000.0) as u64,
                                            duration_seconds: None,
                                        },
                                        endpoint: None,
                                        method: None,
                                        metadata: HashMap::new(),
                                    });
                                } else if metric_name.contains("error")
                                    || metric_name.contains("status")
                                {
                                    events.push(IncidentEvent {
                                        timestamp,
                                        event_type: IncidentEventType::ErrorRateIncrease {
                                            rate: value.min(1.0).max(0.0),
                                            error_codes: None,
                                        },
                                        endpoint: None,
                                        method: None,
                                        metadata: HashMap::new(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }
}
