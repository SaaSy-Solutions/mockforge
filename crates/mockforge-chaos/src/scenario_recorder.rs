//! Scenario recording and replay system

use crate::scenarios::ChaosScenario;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// A recorded chaos event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: ChaosEventType,
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

/// Types of chaos events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChaosEventType {
    /// Latency injection event
    LatencyInjection {
        delay_ms: u64,
        endpoint: Option<String>,
    },
    /// Fault injection event
    FaultInjection {
        fault_type: String,
        endpoint: Option<String>,
    },
    /// Rate limit event
    RateLimitExceeded {
        client_ip: Option<String>,
        endpoint: Option<String>,
    },
    /// Traffic shaping event
    TrafficShaping { action: String, bytes: usize },
    /// Protocol-specific event
    ProtocolEvent {
        protocol: String,
        event: String,
        details: HashMap<String, String>,
    },
    /// Scenario transition
    ScenarioTransition {
        from_scenario: Option<String>,
        to_scenario: String,
    },
}

/// Recorded scenario with events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedScenario {
    /// Scenario metadata
    pub scenario: ChaosScenario,
    /// Recorded events
    pub events: Vec<ChaosEvent>,
    /// Recording start time
    pub recording_started: DateTime<Utc>,
    /// Recording end time
    pub recording_ended: Option<DateTime<Utc>>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
}

impl RecordedScenario {
    /// Create a new recorded scenario
    pub fn new(scenario: ChaosScenario) -> Self {
        Self {
            scenario,
            events: Vec::new(),
            recording_started: Utc::now(),
            recording_ended: None,
            total_duration_ms: 0,
        }
    }

    /// Add an event to the recording
    pub fn add_event(&mut self, event: ChaosEvent) {
        self.events.push(event);
    }

    /// Finish recording
    pub fn finish(&mut self) {
        self.recording_ended = Some(Utc::now());
        self.total_duration_ms = self
            .recording_ended
            .unwrap()
            .signed_duration_since(self.recording_started)
            .num_milliseconds() as u64;
    }

    /// Get events within a time range
    pub fn events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&ChaosEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .collect()
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Import from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Save to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|s| s.to_str());

        let content = match extension {
            Some("yaml") | Some("yml") => {
                self.to_yaml().map_err(|e| std::io::Error::other(e.to_string()))?
            }
            _ => self.to_json().map_err(|e| std::io::Error::other(e.to_string()))?,
        };

        fs::write(path, content)?;
        info!("Saved recorded scenario to: {}", path.display());
        Ok(())
    }

    /// Load from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|s| s.to_str());

        let scenario = match extension {
            Some("yaml") | Some("yml") => {
                Self::from_yaml(&content).map_err(|e| std::io::Error::other(e.to_string()))?
            }
            _ => Self::from_json(&content).map_err(|e| std::io::Error::other(e.to_string()))?,
        };

        info!("Loaded recorded scenario from: {}", path.display());
        Ok(scenario)
    }
}

/// Scenario recorder
pub struct ScenarioRecorder {
    /// Current recording
    current_recording: Arc<RwLock<Option<RecordedScenario>>>,
    /// Completed recordings
    recordings: Arc<RwLock<Vec<RecordedScenario>>>,
    /// Maximum events to record (0 = unlimited)
    max_events: usize,
}

impl ScenarioRecorder {
    /// Create a new scenario recorder
    pub fn new() -> Self {
        Self {
            current_recording: Arc::new(RwLock::new(None)),
            recordings: Arc::new(RwLock::new(Vec::new())),
            max_events: 10000,
        }
    }

    /// Set maximum events to record
    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Start recording a scenario
    pub fn start_recording(&self, scenario: ChaosScenario) -> Result<(), String> {
        let mut current = self.current_recording.write();

        if current.is_some() {
            return Err("Recording already in progress".to_string());
        }

        info!("Started recording scenario: {}", scenario.name);
        *current = Some(RecordedScenario::new(scenario));
        Ok(())
    }

    /// Stop recording
    pub fn stop_recording(&self) -> Result<RecordedScenario, String> {
        let mut current = self.current_recording.write();

        if let Some(mut recording) = current.take() {
            recording.finish();
            info!(
                "Stopped recording scenario: {} ({} events, {}ms)",
                recording.scenario.name,
                recording.events.len(),
                recording.total_duration_ms
            );

            // Store in completed recordings
            let mut recordings = self.recordings.write();
            recordings.push(recording.clone());

            Ok(recording)
        } else {
            Err("No recording in progress".to_string())
        }
    }

    /// Record an event
    pub fn record_event(&self, event: ChaosEvent) {
        let mut current = self.current_recording.write();

        if let Some(recording) = current.as_mut() {
            // Check max events limit
            if self.max_events > 0 && recording.events.len() >= self.max_events {
                warn!("Max events limit ({}) reached, stopping recording", self.max_events);
                return;
            }

            recording.add_event(event);
            debug!("Recorded event (total: {})", recording.events.len());
        }
    }

    /// Check if recording is in progress
    pub fn is_recording(&self) -> bool {
        self.current_recording.read().is_some()
    }

    /// Get current recording (read-only)
    pub fn get_current_recording(&self) -> Option<RecordedScenario> {
        self.current_recording.read().clone()
    }

    /// Get all completed recordings
    pub fn get_recordings(&self) -> Vec<RecordedScenario> {
        self.recordings.read().clone()
    }

    /// Get recording by scenario name
    pub fn get_recording_by_name(&self, name: &str) -> Option<RecordedScenario> {
        self.recordings.read().iter().find(|r| r.scenario.name == name).cloned()
    }

    /// Clear all recordings
    pub fn clear_recordings(&self) {
        let mut recordings = self.recordings.write();
        recordings.clear();
        info!("Cleared all recordings");
    }
}

impl Default for ScenarioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorded_scenario_creation() {
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());
        let recording = RecordedScenario::new(scenario);

        assert_eq!(recording.scenario.name, "test");
        assert_eq!(recording.events.len(), 0);
        assert!(recording.recording_ended.is_none());
    }

    #[test]
    fn test_add_event() {
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());
        let mut recording = RecordedScenario::new(scenario);

        let event = ChaosEvent {
            timestamp: Utc::now(),
            event_type: ChaosEventType::LatencyInjection {
                delay_ms: 100,
                endpoint: Some("/api/test".to_string()),
            },
            metadata: HashMap::new(),
        };

        recording.add_event(event);
        assert_eq!(recording.events.len(), 1);
    }

    #[test]
    fn test_finish_recording() {
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());
        let mut recording = RecordedScenario::new(scenario);

        std::thread::sleep(std::time::Duration::from_millis(10));
        recording.finish();

        assert!(recording.recording_ended.is_some());
        assert!(recording.total_duration_ms >= 10);
    }

    #[test]
    fn test_recorder_start_stop() {
        let recorder = ScenarioRecorder::new();
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());

        assert!(!recorder.is_recording());

        recorder.start_recording(scenario).unwrap();
        assert!(recorder.is_recording());

        let recording = recorder.stop_recording().unwrap();
        assert!(!recorder.is_recording());
        assert_eq!(recording.scenario.name, "test");
    }

    #[test]
    fn test_record_event() {
        let recorder = ScenarioRecorder::new();
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());

        recorder.start_recording(scenario).unwrap();

        let event = ChaosEvent {
            timestamp: Utc::now(),
            event_type: ChaosEventType::LatencyInjection {
                delay_ms: 100,
                endpoint: None,
            },
            metadata: HashMap::new(),
        };

        recorder.record_event(event);

        let current = recorder.get_current_recording().unwrap();
        assert_eq!(current.events.len(), 1);
    }

    #[test]
    fn test_json_export_import() {
        let scenario = ChaosScenario::new("test", crate::ChaosConfig::default());
        let mut recording = RecordedScenario::new(scenario);

        let event = ChaosEvent {
            timestamp: Utc::now(),
            event_type: ChaosEventType::LatencyInjection {
                delay_ms: 100,
                endpoint: Some("/test".to_string()),
            },
            metadata: HashMap::new(),
        };

        recording.add_event(event);
        recording.finish();

        let json = recording.to_json().unwrap();
        let imported = RecordedScenario::from_json(&json).unwrap();

        assert_eq!(imported.scenario.name, "test");
        assert_eq!(imported.events.len(), 1);
    }
}
