//! Time-based scenario scheduling

use crate::scenarios::ChaosScenario;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep_until, Instant};
use tracing::{debug, info, warn};

/// Schedule type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ScheduleType {
    /// Run once at a specific time
    Once {
        at: DateTime<Utc>,
    },
    /// Run after a delay
    Delayed {
        delay_seconds: u64,
    },
    /// Run periodically with interval
    Periodic {
        interval_seconds: u64,
        /// Maximum executions (0 = infinite)
        max_executions: usize,
    },
    /// Run on a cron-like schedule (simplified)
    Cron {
        /// Hour (0-23)
        hour: Option<u8>,
        /// Minute (0-59)
        minute: Option<u8>,
        /// Day of week (0-6, 0 = Sunday)
        day_of_week: Option<u8>,
        /// Maximum executions (0 = infinite)
        max_executions: usize,
    },
}

/// Scheduled scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScenario {
    /// Unique ID
    pub id: String,
    /// Scenario to execute
    pub scenario: ChaosScenario,
    /// Schedule configuration
    pub schedule: ScheduleType,
    /// Is enabled
    pub enabled: bool,
    /// Number of times executed
    pub execution_count: usize,
    /// Last execution time
    pub last_executed: Option<DateTime<Utc>>,
    /// Next scheduled execution
    pub next_execution: Option<DateTime<Utc>>,
}

impl ScheduledScenario {
    /// Create a new scheduled scenario
    pub fn new(
        id: impl Into<String>,
        scenario: ChaosScenario,
        schedule: ScheduleType,
    ) -> Self {
        let mut scheduled = Self {
            id: id.into(),
            scenario,
            schedule,
            enabled: true,
            execution_count: 0,
            last_executed: None,
            next_execution: None,
        };

        scheduled.calculate_next_execution();
        scheduled
    }

    /// Calculate next execution time
    pub fn calculate_next_execution(&mut self) {
        let now = Utc::now();

        self.next_execution = match &self.schedule {
            ScheduleType::Once { at } => {
                if *at > now && self.execution_count == 0 {
                    Some(*at)
                } else {
                    None
                }
            }
            ScheduleType::Delayed { delay_seconds } => {
                if self.execution_count == 0 {
                    Some(now + Duration::seconds(*delay_seconds as i64))
                } else {
                    None
                }
            }
            ScheduleType::Periodic {
                interval_seconds,
                max_executions,
            } => {
                if *max_executions == 0 || self.execution_count < *max_executions {
                    Some(now + Duration::seconds(*interval_seconds as i64))
                } else {
                    None
                }
            }
            ScheduleType::Cron {
                hour,
                minute,
                day_of_week,
                max_executions,
            } => {
                if *max_executions > 0 && self.execution_count >= *max_executions {
                    None
                } else {
                    // Simplified cron calculation - just add 1 hour for next execution
                    // In a production system, you'd use a cron library
                    let next = now + Duration::hours(1);

                    // This is a simplified implementation
                    // For full cron support, integrate with a cron parsing library like `cron`
                    Some(next)
                }
            }
        };
    }

    /// Check if should execute now
    pub fn should_execute(&self) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(next) = self.next_execution {
            Utc::now() >= next
        } else {
            false
        }
    }

    /// Mark as executed
    pub fn mark_executed(&mut self) {
        self.execution_count += 1;
        self.last_executed = Some(Utc::now());
        self.calculate_next_execution();
    }
}

/// Scenario scheduler
pub struct ScenarioScheduler {
    /// Scheduled scenarios
    schedules: Arc<RwLock<HashMap<String, ScheduledScenario>>>,
    /// Execution callback channel
    execution_tx: Arc<RwLock<Option<mpsc::Sender<ScheduledScenario>>>>,
    /// Scheduler task handle
    task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ScenarioScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            schedules: Arc::new(RwLock::new(HashMap::new())),
            execution_tx: Arc::new(RwLock::new(None)),
            task_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a scheduled scenario
    pub fn add_schedule(&self, scheduled: ScheduledScenario) {
        let id = scheduled.id.clone();
        let mut schedules = self.schedules.write().unwrap();
        schedules.insert(id.clone(), scheduled);
        info!("Added scheduled scenario: {}", id);
    }

    /// Remove a scheduled scenario
    pub fn remove_schedule(&self, id: &str) -> Option<ScheduledScenario> {
        let mut schedules = self.schedules.write().unwrap();
        let removed = schedules.remove(id);
        if removed.is_some() {
            info!("Removed scheduled scenario: {}", id);
        }
        removed
    }

    /// Get a scheduled scenario
    pub fn get_schedule(&self, id: &str) -> Option<ScheduledScenario> {
        let schedules = self.schedules.read().unwrap();
        schedules.get(id).cloned()
    }

    /// Get all scheduled scenarios
    pub fn get_all_schedules(&self) -> Vec<ScheduledScenario> {
        let schedules = self.schedules.read().unwrap();
        schedules.values().cloned().collect()
    }

    /// Enable a scheduled scenario
    pub fn enable_schedule(&self, id: &str) -> Result<(), String> {
        let mut schedules = self.schedules.write().unwrap();
        if let Some(scheduled) = schedules.get_mut(id) {
            scheduled.enabled = true;
            scheduled.calculate_next_execution();
            info!("Enabled scheduled scenario: {}", id);
            Ok(())
        } else {
            Err(format!("Schedule '{}' not found", id))
        }
    }

    /// Disable a scheduled scenario
    pub fn disable_schedule(&self, id: &str) -> Result<(), String> {
        let mut schedules = self.schedules.write().unwrap();
        if let Some(scheduled) = schedules.get_mut(id) {
            scheduled.enabled = false;
            info!("Disabled scheduled scenario: {}", id);
            Ok(())
        } else {
            Err(format!("Schedule '{}' not found", id))
        }
    }

    /// Start the scheduler with a callback
    pub async fn start<F>(&self, callback: F)
    where
        F: Fn(ScheduledScenario) + Send + 'static,
    {
        // Check if already running
        {
            let task_handle = self.task_handle.read().unwrap();
            if task_handle.is_some() {
                warn!("Scheduler already running");
                return;
            }
        }

        info!("Starting scenario scheduler");

        let (tx, mut rx) = mpsc::channel::<ScheduledScenario>(100);

        // Store execution channel
        {
            let mut execution_tx = self.execution_tx.write().unwrap();
            *execution_tx = Some(tx);
        }

        // Spawn scheduler task
        let schedules = Arc::clone(&self.schedules);
        let handle = tokio::spawn(async move {
            Self::scheduler_task(schedules, rx, callback).await;
        });

        // Store task handle
        {
            let mut task_handle = self.task_handle.write().unwrap();
            *task_handle = Some(handle);
        }
    }

    /// Scheduler task (runs in background)
    async fn scheduler_task<F>(
        schedules: Arc<RwLock<HashMap<String, ScheduledScenario>>>,
        mut rx: mpsc::Receiver<ScheduledScenario>,
        callback: F,
    )
    where
        F: Fn(ScheduledScenario),
    {
        let mut interval = interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Check for scenarios to execute
                    let mut to_execute = Vec::new();

                    {
                        let mut schedules_guard = schedules.write().unwrap();

                        for (id, scheduled) in schedules_guard.iter_mut() {
                            if scheduled.should_execute() {
                                debug!("Triggering scheduled scenario: {}", id);
                                to_execute.push(scheduled.clone());
                                scheduled.mark_executed();
                            }
                        }
                    }

                    // Execute scenarios
                    for scheduled in to_execute {
                        info!("Executing scheduled scenario: {}", scheduled.id);
                        callback(scheduled);
                    }
                }

                Some(scheduled) = rx.recv() => {
                    // Manual execution request
                    info!("Manual execution of scheduled scenario: {}", scheduled.id);
                    callback(scheduled);
                }

                else => break,
            }
        }

        info!("Scheduler task stopped");
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        info!("Stopping scenario scheduler");

        // Clear execution channel
        {
            let mut execution_tx = self.execution_tx.write().unwrap();
            *execution_tx = None;
        }

        // Abort task
        let mut task_handle = self.task_handle.write().unwrap();
        if let Some(handle) = task_handle.take() {
            handle.abort();
        }
    }

    /// Manually trigger a scheduled scenario
    pub async fn trigger_now(&self, id: &str) -> Result<(), String> {
        let scheduled = {
            let schedules = self.schedules.read().unwrap();
            schedules.get(id).cloned()
        };

        if let Some(scheduled) = scheduled {
            let execution_tx = self.execution_tx.read().unwrap();
            if let Some(tx) = execution_tx.as_ref() {
                tx.send(scheduled)
                    .await
                    .map_err(|e| format!("Failed to trigger: {}", e))?;
                Ok(())
            } else {
                Err("Scheduler not started".to_string())
            }
        } else {
            Err(format!("Schedule '{}' not found", id))
        }
    }

    /// Get next scheduled execution
    pub fn get_next_execution(&self) -> Option<(String, DateTime<Utc>)> {
        let schedules = self.schedules.read().unwrap();
        schedules
            .iter()
            .filter_map(|(id, s)| s.next_execution.map(|t| (id.clone(), t)))
            .min_by_key(|(_, t)| *t)
    }
}

impl Default for ScenarioScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChaosConfig;

    #[test]
    fn test_scheduled_scenario_once() {
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let future_time = Utc::now() + Duration::hours(1);
        let schedule = ScheduleType::Once { at: future_time };

        let scheduled = ScheduledScenario::new("sched1", scenario, schedule);

        assert_eq!(scheduled.id, "sched1");
        assert!(scheduled.enabled);
        assert_eq!(scheduled.execution_count, 0);
        assert!(scheduled.next_execution.is_some());
    }

    #[test]
    fn test_scheduled_scenario_periodic() {
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let schedule = ScheduleType::Periodic {
            interval_seconds: 60,
            max_executions: 10,
        };

        let scheduled = ScheduledScenario::new("sched1", scenario, schedule);

        assert!(scheduled.next_execution.is_some());
    }

    #[test]
    fn test_scheduler_add_remove() {
        let scheduler = ScenarioScheduler::new();
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let schedule = ScheduleType::Delayed { delay_seconds: 10 };

        let scheduled = ScheduledScenario::new("sched1", scenario, schedule);

        scheduler.add_schedule(scheduled.clone());
        assert!(scheduler.get_schedule("sched1").is_some());

        let removed = scheduler.remove_schedule("sched1");
        assert!(removed.is_some());
        assert!(scheduler.get_schedule("sched1").is_none());
    }

    #[test]
    fn test_enable_disable() {
        let scheduler = ScenarioScheduler::new();
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let schedule = ScheduleType::Delayed { delay_seconds: 10 };

        let scheduled = ScheduledScenario::new("sched1", scenario, schedule);
        scheduler.add_schedule(scheduled);

        scheduler.disable_schedule("sched1").unwrap();
        let s = scheduler.get_schedule("sched1").unwrap();
        assert!(!s.enabled);

        scheduler.enable_schedule("sched1").unwrap();
        let s = scheduler.get_schedule("sched1").unwrap();
        assert!(s.enabled);
    }
}
