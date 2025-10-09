//! Scenario replay engine for reproducing recorded chaos scenarios

use crate::scenario_recorder::{ChaosEvent, ChaosEventType, RecordedScenario};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Replay speed modes
#[derive(Debug, Clone, Copy)]
pub enum ReplaySpeed {
    /// Real-time (1x speed)
    RealTime,
    /// Custom speed multiplier (e.g., 2.0 = 2x faster)
    Custom(f64),
    /// As fast as possible (no delays)
    Fast,
}

impl ReplaySpeed {
    /// Calculate delay based on speed
    pub fn calculate_delay(&self, original_delay_ms: u64) -> u64 {
        match self {
            ReplaySpeed::RealTime => original_delay_ms,
            ReplaySpeed::Custom(multiplier) => {
                ((original_delay_ms as f64) / multiplier) as u64
            }
            ReplaySpeed::Fast => 0,
        }
    }
}

/// Replay options
#[derive(Debug, Clone)]
pub struct ReplayOptions {
    /// Speed of replay
    pub speed: ReplaySpeed,
    /// Loop the replay
    pub loop_replay: bool,
    /// Skip initial delay
    pub skip_initial_delay: bool,
    /// Filter events by type
    pub event_type_filter: Option<Vec<String>>,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            speed: ReplaySpeed::RealTime,
            loop_replay: false,
            skip_initial_delay: false,
            event_type_filter: None,
        }
    }
}

/// Replay status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStatus {
    /// Scenario name
    pub scenario_name: String,
    /// Current event index
    pub current_event: usize,
    /// Total events
    pub total_events: usize,
    /// Replay start time
    pub started_at: DateTime<Utc>,
    /// Is currently playing
    pub is_playing: bool,
    /// Is paused
    pub is_paused: bool,
    /// Replay progress (0.0 - 1.0)
    pub progress: f64,
}

/// Scenario replay engine
pub struct ScenarioReplayEngine {
    /// Current replay status
    status: Arc<RwLock<Option<ReplayStatus>>>,
    /// Replay control channel
    control_tx: Option<mpsc::Sender<ReplayControl>>,
}

/// Replay control commands
enum ReplayControl {
    Pause,
    Resume,
    Stop,
    Skip(usize),
}

impl ScenarioReplayEngine {
    /// Create a new replay engine
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(None)),
            control_tx: None,
        }
    }

    /// Start replaying a recorded scenario
    pub async fn replay(
        &mut self,
        recorded: RecordedScenario,
        options: ReplayOptions,
    ) -> Result<(), String> {
        // Check if already replaying
        {
            let status = self.status.read();
            if status.is_some() {
                return Err("Replay already in progress".to_string());
            }
        }

        let scenario_name = recorded.scenario.name.clone();
        let total_events = recorded.events.len();

        info!(
            "Starting replay of scenario '{}' ({} events, speed: {:?})",
            scenario_name, total_events, options.speed
        );

        // Initialize status
        {
            let mut status = self.status.write();
            *status = Some(ReplayStatus {
                scenario_name: scenario_name.clone(),
                current_event: 0,
                total_events,
                started_at: Utc::now(),
                is_playing: true,
                is_paused: false,
                progress: 0.0,
            });
        }

        // Create control channel
        let (control_tx, mut control_rx) = mpsc::channel::<ReplayControl>(10);
        self.control_tx = Some(control_tx);

        // Clone Arc for the async task
        let status_arc = Arc::clone(&self.status);

        // Spawn replay task
        tokio::spawn(async move {
            Self::replay_task(
                recorded,
                options,
                status_arc,
                &mut control_rx,
            )
            .await;
        });

        Ok(())
    }

    /// Replay task (runs in background)
    async fn replay_task(
        recorded: RecordedScenario,
        options: ReplayOptions,
        status: Arc<RwLock<Option<ReplayStatus>>>,
        control_rx: &mut mpsc::Receiver<ReplayControl>,
    ) {
        let events = recorded.events;
        let total_events = events.len();

        if total_events == 0 {
            warn!("No events to replay");
            return;
        }

        loop {
            // Iterate through events
            for (index, event) in events.iter().enumerate() {
                // Check for control commands
                if let Ok(cmd) = control_rx.try_recv() {
                    match cmd {
                        ReplayControl::Pause => {
                            info!("Replay paused");
                            Self::update_status(&status, |s| s.is_paused = true);

                            // Wait for resume or stop
                            if let Some(cmd) = control_rx.recv().await {
                                match cmd {
                                    ReplayControl::Resume => {
                                        info!("Replay resumed");
                                        Self::update_status(&status, |s| s.is_paused = false);
                                    }
                                    ReplayControl::Stop => {
                                        info!("Replay stopped");
                                        Self::clear_status(&status);
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        ReplayControl::Stop => {
                            info!("Replay stopped");
                            Self::clear_status(&status);
                            return;
                        }
                        ReplayControl::Skip(n) => {
                            info!("Skipping {} events", n);
                            // Skip is handled by the outer loop
                        }
                        _ => {}
                    }
                }

                // Apply event type filter
                if let Some(ref filters) = options.event_type_filter {
                    let event_type = Self::get_event_type_name(&event.event_type);
                    if !filters.contains(&event_type) {
                        continue;
                    }
                }

                // Calculate delay from previous event
                if index > 0 && !options.skip_initial_delay {
                    let prev_event = &events[index - 1];
                    let delay_ms = event
                        .timestamp
                        .signed_duration_since(prev_event.timestamp)
                        .num_milliseconds() as u64;

                    let adjusted_delay = options.speed.calculate_delay(delay_ms);

                    if adjusted_delay > 0 {
                        debug!("Waiting {}ms before next event", adjusted_delay);
                        sleep(std::time::Duration::from_millis(adjusted_delay)).await;
                    }
                }

                // Replay the event
                Self::replay_event(event).await;

                // Update status
                Self::update_status(&status, |s| {
                    s.current_event = index + 1;
                    s.progress = (index + 1) as f64 / total_events as f64;
                });

                debug!(
                    "Replayed event {}/{}: {:?}",
                    index + 1,
                    total_events,
                    event.event_type
                );
            }

            // Check if should loop
            if !options.loop_replay {
                break;
            }

            info!("Looping replay from beginning");
            Self::update_status(&status, |s| {
                s.current_event = 0;
                s.progress = 0.0;
            });
        }

        info!("Replay completed");
        Self::clear_status(&status);
    }

    /// Replay a single event
    async fn replay_event(event: &ChaosEvent) {
        match &event.event_type {
            ChaosEventType::LatencyInjection { delay_ms, endpoint } => {
                debug!(
                    "Replaying latency injection: {}ms{}",
                    delay_ms,
                    endpoint
                        .as_ref()
                        .map(|e| format!(" on {}", e))
                        .unwrap_or_default()
                );
                sleep(std::time::Duration::from_millis(*delay_ms)).await;
            }
            ChaosEventType::FaultInjection { fault_type, endpoint } => {
                debug!(
                    "Replaying fault injection: {}{}",
                    fault_type,
                    endpoint
                        .as_ref()
                        .map(|e| format!(" on {}", e))
                        .unwrap_or_default()
                );
            }
            ChaosEventType::RateLimitExceeded { client_ip, endpoint } => {
                debug!(
                    "Replaying rate limit exceeded: client={:?}, endpoint={:?}",
                    client_ip, endpoint
                );
            }
            ChaosEventType::TrafficShaping { action, bytes } => {
                debug!("Replaying traffic shaping: {} ({} bytes)", action, bytes);
            }
            ChaosEventType::ProtocolEvent { protocol, event, details } => {
                debug!(
                    "Replaying protocol event: {} - {} ({:?})",
                    protocol, event, details
                );
            }
            ChaosEventType::ScenarioTransition {
                from_scenario,
                to_scenario,
            } => {
                debug!(
                    "Replaying scenario transition: {:?} -> {}",
                    from_scenario, to_scenario
                );
            }
        }
    }

    /// Get event type name as string
    fn get_event_type_name(event_type: &ChaosEventType) -> String {
        match event_type {
            ChaosEventType::LatencyInjection { .. } => "LatencyInjection".to_string(),
            ChaosEventType::FaultInjection { .. } => "FaultInjection".to_string(),
            ChaosEventType::RateLimitExceeded { .. } => "RateLimitExceeded".to_string(),
            ChaosEventType::TrafficShaping { .. } => "TrafficShaping".to_string(),
            ChaosEventType::ProtocolEvent { .. } => "ProtocolEvent".to_string(),
            ChaosEventType::ScenarioTransition { .. } => "ScenarioTransition".to_string(),
        }
    }

    /// Update status
    fn update_status<F>(status: &Arc<RwLock<Option<ReplayStatus>>>, f: F)
    where
        F: FnOnce(&mut ReplayStatus),
    {
        let mut status_guard = status.write();
        if let Some(ref mut s) = *status_guard {
            f(s);
        }
    }

    /// Clear status
    fn clear_status(status: &Arc<RwLock<Option<ReplayStatus>>>) {
        let mut status_guard = status.write();
        *status_guard = None;
    }

    /// Pause replay
    pub async fn pause(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(ReplayControl::Pause)
                .await
                .map_err(|e| format!("Failed to pause: {}", e))?;
            Ok(())
        } else {
            Err("No replay in progress".to_string())
        }
    }

    /// Resume replay
    pub async fn resume(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(ReplayControl::Resume)
                .await
                .map_err(|e| format!("Failed to resume: {}", e))?;
            Ok(())
        } else {
            Err("No replay in progress".to_string())
        }
    }

    /// Stop replay
    pub async fn stop(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(ReplayControl::Stop)
                .await
                .map_err(|e| format!("Failed to stop: {}", e))?;
            Ok(())
        } else {
            Err("No replay in progress".to_string())
        }
    }

    /// Get current replay status
    pub fn get_status(&self) -> Option<ReplayStatus> {
        self.status.read().clone()
    }

    /// Check if replay is in progress
    pub fn is_replaying(&self) -> bool {
        self.status.read().is_some()
    }
}

impl Default for ScenarioReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[tokio::test]
    async fn test_replay_speed_calculation() {
        let real_time = ReplaySpeed::RealTime;
        assert_eq!(real_time.calculate_delay(100), 100);

        let double_speed = ReplaySpeed::Custom(2.0);
        assert_eq!(double_speed.calculate_delay(100), 50);

        let fast = ReplaySpeed::Fast;
        assert_eq!(fast.calculate_delay(100), 0);
    }

    #[tokio::test]
    async fn test_replay_engine_creation() {
        let engine = ScenarioReplayEngine::new();
        assert!(!engine.is_replaying());
    }

    #[test]
    fn test_replay_options_default() {
        let options = ReplayOptions::default();
        assert!(!options.loop_replay);
        assert!(!options.skip_initial_delay);
    }
}
