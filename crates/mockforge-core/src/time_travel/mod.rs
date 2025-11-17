//! # Time Travel / Temporal Testing Module
//!
//! This module provides time travel capabilities for testing time-dependent behavior.
//! It allows you to:
//! - Simulate time progression without waiting
//! - Schedule responses to be returned at specific virtual times
//! - Test time-based state transitions (e.g., token expiry, session timeouts)
//! - Control time flow for deterministic testing

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Virtual clock that can be manipulated for testing time-dependent behavior
#[derive(Debug, Clone)]
pub struct VirtualClock {
    /// The current virtual time (None means use real time)
    current_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Whether time travel is enabled
    enabled: Arc<RwLock<bool>>,
    /// Time scale factor (1.0 = real time, 2.0 = 2x speed, 0.5 = half speed)
    scale_factor: Arc<RwLock<f64>>,
    /// Baseline real time when virtual time was set (for scaled time)
    baseline_real_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualClock {
    /// Create a new virtual clock (disabled by default, uses real time)
    pub fn new() -> Self {
        Self {
            current_time: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(false)),
            scale_factor: Arc::new(RwLock::new(1.0)),
            baseline_real_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new virtual clock with time travel enabled at a specific time
    pub fn new_at(time: DateTime<Utc>) -> Self {
        let clock = Self::new();
        clock.enable_and_set(time);
        clock
    }

    /// Enable time travel and set the current virtual time
    pub fn enable_and_set(&self, time: DateTime<Utc>) {
        let mut current = self.current_time.write().unwrap();
        *current = Some(time);

        let mut enabled = self.enabled.write().unwrap();
        *enabled = true;

        let mut baseline = self.baseline_real_time.write().unwrap();
        *baseline = Some(Utc::now());

        info!("Time travel enabled at {}", time);
    }

    /// Disable time travel and return to using real time
    pub fn disable(&self) {
        let mut enabled = self.enabled.write().unwrap();
        *enabled = false;

        let mut current = self.current_time.write().unwrap();
        *current = None;

        let mut baseline = self.baseline_real_time.write().unwrap();
        *baseline = None;

        info!("Time travel disabled, using real time");
    }

    /// Check if time travel is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// Get the current time (virtual or real)
    pub fn now(&self) -> DateTime<Utc> {
        let enabled = *self.enabled.read().unwrap();

        if !enabled {
            return Utc::now();
        }

        let current = self.current_time.read().unwrap();
        let scale = *self.scale_factor.read().unwrap();

        if let Some(virtual_time) = *current {
            // If scale factor is 1.0, just return the virtual time
            if (scale - 1.0).abs() < f64::EPSILON {
                return virtual_time;
            }

            // If scale factor is different, calculate scaled time
            let baseline = self.baseline_real_time.read().unwrap();
            if let Some(baseline_real) = *baseline {
                let elapsed_real = Utc::now() - baseline_real;
                let elapsed_scaled =
                    Duration::milliseconds((elapsed_real.num_milliseconds() as f64 * scale) as i64);
                return virtual_time + elapsed_scaled;
            }

            virtual_time
        } else {
            Utc::now()
        }
    }

    /// Advance time by a duration
    pub fn advance(&self, duration: Duration) {
        let enabled = *self.enabled.read().unwrap();
        if !enabled {
            warn!("Cannot advance time: time travel is not enabled");
            return;
        }

        let mut current = self.current_time.write().unwrap();
        if let Some(time) = *current {
            let new_time = time + duration;
            *current = Some(new_time);

            // Update baseline to current real time
            let mut baseline = self.baseline_real_time.write().unwrap();
            *baseline = Some(Utc::now());

            info!("Time advanced by {} to {}", duration, new_time);
        }
    }

    /// Set the time scale factor (1.0 = real time, 2.0 = 2x speed, etc.)
    pub fn set_scale(&self, factor: f64) {
        if factor <= 0.0 {
            warn!("Invalid scale factor: {}, must be positive", factor);
            return;
        }

        let mut scale = self.scale_factor.write().unwrap();
        *scale = factor;

        // Update baseline to current real time
        let mut baseline = self.baseline_real_time.write().unwrap();
        *baseline = Some(Utc::now());

        info!("Time scale set to {}x", factor);
    }

    /// Get the current time scale factor
    pub fn get_scale(&self) -> f64 {
        *self.scale_factor.read().unwrap()
    }

    /// Reset time travel to real time
    pub fn reset(&self) {
        self.disable();
        info!("Time travel reset to real time");
    }

    /// Set the virtual time to a specific point
    pub fn set_time(&self, time: DateTime<Utc>) {
        let enabled = *self.enabled.read().unwrap();
        if !enabled {
            self.enable_and_set(time);
            return;
        }

        let mut current = self.current_time.write().unwrap();
        *current = Some(time);

        // Update baseline to current real time
        let mut baseline = self.baseline_real_time.write().unwrap();
        *baseline = Some(Utc::now());

        info!("Virtual time set to {}", time);
    }

    /// Get time travel status
    pub fn status(&self) -> TimeTravelStatus {
        TimeTravelStatus {
            enabled: self.is_enabled(),
            current_time: if self.is_enabled() {
                Some(self.now())
            } else {
                None
            },
            scale_factor: self.get_scale(),
            real_time: Utc::now(),
        }
    }
}

/// Status information for time travel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelStatus {
    /// Whether time travel is enabled
    pub enabled: bool,
    /// Current virtual time (None if using real time)
    pub current_time: Option<DateTime<Utc>>,
    /// Time scale factor
    pub scale_factor: f64,
    /// Current real time
    pub real_time: DateTime<Utc>,
}

/// Configuration for time travel features
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelConfig {
    /// Whether time travel is enabled by default
    #[serde(default)]
    pub enabled: bool,
    /// Initial virtual time (if enabled)
    #[cfg_attr(feature = "schema", schemars(with = "Option<String>"))]
    pub initial_time: Option<DateTime<Utc>>,
    /// Initial time scale factor
    #[serde(default = "default_scale")]
    pub scale_factor: f64,
    /// Whether to enable scheduled responses
    #[serde(default = "default_true")]
    pub enable_scheduling: bool,
}

fn default_scale() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

impl Default for TimeTravelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            initial_time: None,
            scale_factor: 1.0,
            enable_scheduling: true,
        }
    }
}

/// Schedule manager for time-based response scheduling
#[derive(Debug, Clone)]
pub struct ResponseScheduler {
    /// Virtual clock reference
    clock: Arc<VirtualClock>,
    /// Scheduled responses (sorted by trigger time)
    scheduled: Arc<RwLock<BTreeMap<DateTime<Utc>, Vec<ScheduledResponse>>>>,
    /// Named schedules for easy reference
    named_schedules: Arc<RwLock<HashMap<String, String>>>,
}

/// A scheduled response that will be returned at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledResponse {
    /// Unique identifier for this scheduled response
    pub id: String,
    /// When this response should be returned
    pub trigger_time: DateTime<Utc>,
    /// The response body
    pub body: serde_json::Value,
    /// HTTP status code
    #[serde(default = "default_status")]
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional name/label
    pub name: Option<String>,
    /// Whether this should repeat
    #[serde(default)]
    pub repeat: Option<RepeatConfig>,
}

fn default_status() -> u16 {
    200
}

/// Configuration for repeating scheduled responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatConfig {
    /// Interval between repeats
    pub interval: Duration,
    /// Maximum number of repeats (None = infinite)
    pub max_count: Option<usize>,
}

impl ResponseScheduler {
    /// Create a new response scheduler
    pub fn new(clock: Arc<VirtualClock>) -> Self {
        Self {
            clock,
            scheduled: Arc::new(RwLock::new(BTreeMap::new())),
            named_schedules: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Schedule a response to be returned at a specific time
    pub fn schedule(&self, response: ScheduledResponse) -> Result<String, String> {
        let id = if response.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            response.id.clone()
        };

        let mut scheduled = self.scheduled.write().unwrap();
        scheduled.entry(response.trigger_time).or_default().push(response.clone());

        if let Some(name) = &response.name {
            let mut named = self.named_schedules.write().unwrap();
            named.insert(name.clone(), id.clone());
        }

        info!("Scheduled response {} for {}", id, response.trigger_time);
        Ok(id)
    }

    /// Get responses that should be triggered at the current time
    pub fn get_due_responses(&self) -> Vec<ScheduledResponse> {
        let now = self.clock.now();
        let mut scheduled = self.scheduled.write().unwrap();
        let mut due = Vec::new();

        // Get all times up to now
        let times_to_process: Vec<DateTime<Utc>> =
            scheduled.range(..=now).map(|(time, _)| *time).collect();

        for time in times_to_process {
            if let Some(responses) = scheduled.remove(&time) {
                for response in responses {
                    due.push(response.clone());

                    // Handle repeating responses
                    if let Some(repeat_config) = &response.repeat {
                        let next_time = time + repeat_config.interval;

                        // Check if we should schedule another repeat
                        let should_repeat = if let Some(max) = repeat_config.max_count {
                            // Track repeat count (simplified - in production use a counter)
                            max > 1
                        } else {
                            true
                        };

                        if should_repeat {
                            let mut next_response = response.clone();
                            next_response.trigger_time = next_time;
                            if let Some(ref mut repeat) = next_response.repeat {
                                if let Some(ref mut count) = repeat.max_count {
                                    *count -= 1;
                                }
                            }

                            scheduled.entry(next_time).or_default().push(next_response);
                        }
                    }
                }
            }
        }

        debug!("Found {} due responses at {}", due.len(), now);
        due
    }

    /// Remove a scheduled response by ID
    pub fn cancel(&self, id: &str) -> bool {
        let mut scheduled = self.scheduled.write().unwrap();

        for responses in scheduled.values_mut() {
            if let Some(pos) = responses.iter().position(|r| r.id == id) {
                responses.remove(pos);
                info!("Cancelled scheduled response {}", id);
                return true;
            }
        }

        false
    }

    /// Clear all scheduled responses
    pub fn clear_all(&self) {
        let mut scheduled = self.scheduled.write().unwrap();
        scheduled.clear();

        let mut named = self.named_schedules.write().unwrap();
        named.clear();

        info!("Cleared all scheduled responses");
    }

    /// Get all scheduled responses
    pub fn list_scheduled(&self) -> Vec<ScheduledResponse> {
        let scheduled = self.scheduled.read().unwrap();
        scheduled.values().flat_map(|v| v.iter().cloned()).collect()
    }

    /// Get count of scheduled responses
    pub fn count(&self) -> usize {
        let scheduled = self.scheduled.read().unwrap();
        scheduled.values().map(|v| v.len()).sum()
    }
}

/// Time travel scenario snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeScenario {
    /// Scenario name
    pub name: String,
    /// Whether time travel is enabled
    pub enabled: bool,
    /// Current virtual time (if enabled)
    pub current_time: Option<DateTime<Utc>>,
    /// Time scale factor
    pub scale_factor: f64,
    /// Scheduled responses (if any)
    #[serde(default)]
    pub scheduled_responses: Vec<ScheduledResponse>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Description (optional)
    #[serde(default)]
    pub description: Option<String>,
}

impl TimeScenario {
    /// Create a new scenario from current time travel state
    pub fn from_manager(manager: &TimeTravelManager, name: String) -> Self {
        let status = manager.clock().status();
        let scheduled = manager.scheduler().list_scheduled();

        Self {
            name,
            enabled: status.enabled,
            current_time: status.current_time,
            scale_factor: status.scale_factor,
            scheduled_responses: scheduled,
            created_at: Utc::now(),
            description: None,
        }
    }

    /// Apply this scenario to a time travel manager
    pub fn apply_to_manager(&self, manager: &TimeTravelManager) {
        if self.enabled {
            if let Some(time) = self.current_time {
                manager.enable_and_set(time);
            } else {
                manager.enable_and_set(Utc::now());
            }
            manager.set_scale(self.scale_factor);
        } else {
            manager.disable();
        }

        // Clear existing scheduled responses and add scenario ones
        manager.scheduler().clear_all();
        for response in &self.scheduled_responses {
            let _ = manager.scheduler().schedule(response.clone());
        }
    }
}

/// Global virtual clock registry for automatic time travel detection
///
/// This registry allows modules throughout MockForge to automatically detect
/// if time travel is enabled and use the virtual clock when available.
/// Modules can call `get_global_clock()` or `now()` to get the current time,
/// which will automatically use virtual time if enabled, or real time otherwise.
static GLOBAL_CLOCK_REGISTRY: Lazy<Arc<RwLock<Option<Arc<VirtualClock>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Register a virtual clock with the global registry
///
/// This should be called when a `TimeTravelManager` is created to make
/// the virtual clock available throughout the application.
pub fn register_global_clock(clock: Arc<VirtualClock>) {
    let mut registry = GLOBAL_CLOCK_REGISTRY.write().unwrap();
    *registry = Some(clock);
    info!("Virtual clock registered globally");
}

/// Unregister the global virtual clock
///
/// This should be called when time travel is disabled or the manager is dropped.
pub fn unregister_global_clock() {
    let mut registry = GLOBAL_CLOCK_REGISTRY.write().unwrap();
    *registry = None;
    info!("Virtual clock unregistered globally");
}

/// Get the global virtual clock if one is registered
///
/// Returns `None` if time travel is not enabled or no clock is registered.
pub fn get_global_clock() -> Option<Arc<VirtualClock>> {
    let registry = GLOBAL_CLOCK_REGISTRY.read().unwrap();
    registry.clone()
}

/// Get the current time, automatically using virtual clock if available
///
/// This is a convenience function that checks the global registry and returns
/// virtual time if time travel is enabled, or real time otherwise.
/// This allows modules to automatically respect time travel without needing
/// to explicitly pass a clock reference.
pub fn now() -> DateTime<Utc> {
    if let Some(clock) = get_global_clock() {
        clock.now()
    } else {
        Utc::now()
    }
}

/// Check if time travel is currently enabled globally
///
/// Returns `true` if a virtual clock is registered and enabled.
pub fn is_time_travel_enabled() -> bool {
    if let Some(clock) = get_global_clock() {
        clock.is_enabled()
    } else {
        false
    }
}

/// Global time travel manager
pub struct TimeTravelManager {
    /// Virtual clock
    clock: Arc<VirtualClock>,
    /// Response scheduler
    scheduler: Arc<ResponseScheduler>,
    /// Cron scheduler for recurring events
    cron_scheduler: Arc<cron::CronScheduler>,
}

impl TimeTravelManager {
    /// Create a new time travel manager
    ///
    /// The virtual clock is automatically registered with the global registry
    /// so it can be detected by other modules (e.g., auth, session expiration).
    pub fn new(config: TimeTravelConfig) -> Self {
        let clock = Arc::new(VirtualClock::new());

        if config.enabled {
            if let Some(initial_time) = config.initial_time {
                clock.enable_and_set(initial_time);
            } else {
                clock.enable_and_set(Utc::now());
            }
            clock.set_scale(config.scale_factor);
            // Register with global registry for automatic detection
            register_global_clock(clock.clone());
        }

        let scheduler = Arc::new(ResponseScheduler::new(clock.clone()));
        let cron_scheduler = Arc::new(cron::CronScheduler::new(clock.clone()));

        Self {
            clock,
            scheduler,
            cron_scheduler,
        }
    }

    /// Get the virtual clock
    pub fn clock(&self) -> Arc<VirtualClock> {
        self.clock.clone()
    }

    /// Get the response scheduler
    pub fn scheduler(&self) -> Arc<ResponseScheduler> {
        self.scheduler.clone()
    }

    /// Get the cron scheduler
    pub fn cron_scheduler(&self) -> Arc<cron::CronScheduler> {
        self.cron_scheduler.clone()
    }

    /// Get the current time (respects virtual clock if enabled)
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Save current state as a scenario
    pub fn save_scenario(&self, name: String) -> TimeScenario {
        TimeScenario::from_manager(self, name)
    }

    /// Load and apply a scenario
    pub fn load_scenario(&self, scenario: &TimeScenario) {
        scenario.apply_to_manager(self);
    }

    /// Enable time travel and set the current virtual time
    ///
    /// This method wraps the clock's enable_and_set and updates the global registry.
    pub fn enable_and_set(&self, time: DateTime<Utc>) {
        self.clock.enable_and_set(time);
        register_global_clock(self.clock.clone());
    }

    /// Disable time travel and return to using real time
    ///
    /// This method wraps the clock's disable and updates the global registry.
    pub fn disable(&self) {
        self.clock.disable();
        unregister_global_clock();
    }

    /// Advance time by a duration
    ///
    /// This method wraps the clock's advance for convenience.
    pub fn advance(&self, duration: Duration) {
        self.clock.advance(duration);
    }

    /// Set the virtual time to a specific point
    ///
    /// This method wraps the clock's set_time for convenience.
    pub fn set_time(&self, time: DateTime<Utc>) {
        self.clock.set_time(time);
        if self.clock.is_enabled() {
            register_global_clock(self.clock.clone());
        }
    }

    /// Set the time scale factor
    ///
    /// This method wraps the clock's set_scale for convenience.
    pub fn set_scale(&self, factor: f64) {
        self.clock.set_scale(factor);
    }
}

impl Drop for TimeTravelManager {
    fn drop(&mut self) {
        // Unregister when manager is dropped
        unregister_global_clock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_clock_creation() {
        let clock = VirtualClock::new();
        assert!(!clock.is_enabled());
    }

    #[test]
    fn test_virtual_clock_enable() {
        let clock = VirtualClock::new();
        let test_time = Utc::now();
        clock.enable_and_set(test_time);

        assert!(clock.is_enabled());
        let now = clock.now();
        assert!((now - test_time).num_seconds().abs() < 1);
    }

    #[test]
    fn test_virtual_clock_advance() {
        let clock = VirtualClock::new();
        let test_time = Utc::now();
        clock.enable_and_set(test_time);

        clock.advance(Duration::hours(2));
        let now = clock.now();

        assert!((now - test_time - Duration::hours(2)).num_seconds().abs() < 1);
    }

    #[test]
    fn test_virtual_clock_scale() {
        let clock = VirtualClock::new();
        clock.set_scale(2.0);
        assert_eq!(clock.get_scale(), 2.0);
    }

    #[test]
    fn test_response_scheduler() {
        let clock = Arc::new(VirtualClock::new());
        let test_time = Utc::now();
        clock.enable_and_set(test_time);

        let scheduler = ResponseScheduler::new(clock.clone());

        let response = ScheduledResponse {
            id: "test-1".to_string(),
            trigger_time: test_time + Duration::seconds(10),
            body: serde_json::json!({"message": "Hello"}),
            status: 200,
            headers: HashMap::new(),
            name: Some("test".to_string()),
            repeat: None,
        };

        let id = scheduler.schedule(response).unwrap();
        assert_eq!(id, "test-1");
        assert_eq!(scheduler.count(), 1);
    }

    #[test]
    fn test_scheduled_response_triggering() {
        let clock = Arc::new(VirtualClock::new());
        let test_time = Utc::now();
        clock.enable_and_set(test_time);

        let scheduler = ResponseScheduler::new(clock.clone());

        let response = ScheduledResponse {
            id: "test-1".to_string(),
            trigger_time: test_time + Duration::seconds(10),
            body: serde_json::json!({"message": "Hello"}),
            status: 200,
            headers: HashMap::new(),
            name: None,
            repeat: None,
        };

        scheduler.schedule(response).unwrap();

        // Should not be due yet
        let due = scheduler.get_due_responses();
        assert_eq!(due.len(), 0);

        // Advance time
        clock.advance(Duration::seconds(15));

        // Should be due now
        let due = scheduler.get_due_responses();
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn test_time_travel_config() {
        let config = TimeTravelConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.scale_factor, 1.0);
        assert!(config.enable_scheduling);
    }

    #[test]
    fn test_time_travel_manager() {
        let config = TimeTravelConfig {
            enabled: true,
            initial_time: Some(Utc::now()),
            scale_factor: 1.0,
            enable_scheduling: true,
        };

        let manager = TimeTravelManager::new(config);
        assert!(manager.clock().is_enabled());
    }

    #[test]
    fn test_one_month_later_scenario() {
        let clock = Arc::new(VirtualClock::new());
        let initial_time = Utc::now();
        clock.enable_and_set(initial_time);

        // Advance by 1 month (30 days)
        clock.advance(Duration::days(30));

        let final_time = clock.now();
        let elapsed = final_time - initial_time;

        // Should be approximately 30 days
        assert!(elapsed.num_days() >= 29 && elapsed.num_days() <= 31);
    }

    #[test]
    fn test_scenario_save_and_load() {
        let config = TimeTravelConfig {
            enabled: true,
            initial_time: Some(Utc::now()),
            scale_factor: 2.0,
            enable_scheduling: true,
        };

        let manager = TimeTravelManager::new(config);

        // Advance time
        manager.clock().advance(Duration::hours(24));

        // Save scenario
        let scenario = manager.save_scenario("test-scenario".to_string());
        assert_eq!(scenario.name, "test-scenario");
        assert!(scenario.enabled);
        assert_eq!(scenario.scale_factor, 2.0);
        assert!(scenario.current_time.is_some());

        // Create new manager and load scenario
        let new_config = TimeTravelConfig::default();
        let new_manager = TimeTravelManager::new(new_config);

        // Load scenario
        new_manager.load_scenario(&scenario);

        // Verify state was restored
        assert!(new_manager.clock().is_enabled());
        assert_eq!(new_manager.clock().get_scale(), 2.0);
        if let Some(saved_time) = scenario.current_time {
            let loaded_time = new_manager.clock().now();
            // Times should be very close (within 1 second)
            assert!((loaded_time - saved_time).num_seconds().abs() < 1);
        }
    }

    #[test]
    fn test_duration_parsing_month_year() {
        // Test that month and year durations work
        let clock = Arc::new(VirtualClock::new());
        let initial_time = Utc::now();
        clock.enable_and_set(initial_time);

        // Advance by 1 month (should be ~30 days)
        clock.advance(Duration::days(30));
        let after_month = clock.now();
        let month_elapsed = after_month - initial_time;
        assert!(month_elapsed.num_days() >= 29 && month_elapsed.num_days() <= 31);

        // Reset and advance by 1 year (should be ~365 days)
        clock.set_time(initial_time);
        clock.advance(Duration::days(365));
        let after_year = clock.now();
        let year_elapsed = after_year - initial_time;
        assert!(year_elapsed.num_days() >= 364 && year_elapsed.num_days() <= 366);
    }
}

// Cron scheduler module
pub mod cron;

// Re-export cron types
pub use cron::{CronJob, CronJobAction, CronScheduler};
