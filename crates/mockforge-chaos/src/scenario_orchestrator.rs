//! Scenario orchestration for composing and chaining chaos scenarios

use crate::{
    config::ChaosConfig,
    scenarios::ChaosScenario,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Scenario step in an orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    /// Step name
    pub name: String,
    /// Scenario to execute
    pub scenario: ChaosScenario,
    /// Duration in seconds (0 = use scenario's duration)
    pub duration_seconds: Option<u64>,
    /// Delay before starting this step (in seconds)
    pub delay_before_seconds: u64,
    /// Continue on failure
    pub continue_on_failure: bool,
}

impl ScenarioStep {
    /// Create a new scenario step
    pub fn new(name: impl Into<String>, scenario: ChaosScenario) -> Self {
        Self {
            name: name.into(),
            scenario,
            duration_seconds: None,
            delay_before_seconds: 0,
            continue_on_failure: false,
        }
    }

    /// Set duration
    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration_seconds = Some(seconds);
        self
    }

    /// Set delay before step
    pub fn with_delay_before(mut self, seconds: u64) -> Self {
        self.delay_before_seconds = seconds;
        self
    }

    /// Set continue on failure
    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }
}

/// Orchestrated scenario composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratedScenario {
    /// Orchestration name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Steps to execute in order
    pub steps: Vec<ScenarioStep>,
    /// Execute steps in parallel
    pub parallel: bool,
    /// Loop the orchestration
    pub loop_orchestration: bool,
    /// Maximum iterations (0 = infinite)
    pub max_iterations: usize,
    /// Tags
    pub tags: Vec<String>,
}

impl OrchestratedScenario {
    /// Create a new orchestrated scenario
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            steps: Vec::new(),
            parallel: false,
            loop_orchestration: false,
            max_iterations: 1,
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a step
    pub fn add_step(mut self, step: ScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Execute steps in parallel
    pub fn with_parallel_execution(mut self) -> Self {
        self.parallel = true;
        self
    }

    /// Loop the orchestration
    pub fn with_loop(mut self, max_iterations: usize) -> Self {
        self.loop_orchestration = true;
        self.max_iterations = max_iterations;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
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
}

/// Orchestration execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationStatus {
    /// Orchestration name
    pub name: String,
    /// Current iteration
    pub current_iteration: usize,
    /// Current step index
    pub current_step: usize,
    /// Total steps
    pub total_steps: usize,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Is currently running
    pub is_running: bool,
    /// Failed steps
    pub failed_steps: Vec<String>,
    /// Progress (0.0 - 1.0)
    pub progress: f64,
}

/// Scenario orchestrator
pub struct ScenarioOrchestrator {
    /// Current orchestration status
    status: Arc<RwLock<Option<OrchestrationStatus>>>,
    /// Active config (current step's config)
    active_config: Arc<RwLock<Option<ChaosConfig>>>,
    /// Control channel
    control_tx: Option<mpsc::Sender<OrchestrationControl>>,
}

/// Orchestration control commands
enum OrchestrationControl {
    Pause,
    Resume,
    Stop,
    SkipStep,
}

impl ScenarioOrchestrator {
    /// Create a new orchestrator
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(None)),
            active_config: Arc::new(RwLock::new(None)),
            control_tx: None,
        }
    }

    /// Execute an orchestrated scenario
    pub async fn execute(&mut self, orchestrated: OrchestratedScenario) -> Result<(), String> {
        // Check if already running
        {
            let status = self.status.read().unwrap();
            if status.is_some() {
                return Err("Orchestration already in progress".to_string());
            }
        }

        let orchestration_name = orchestrated.name.clone();
        let total_steps = orchestrated.steps.len();

        if total_steps == 0 {
            return Err("No steps to execute".to_string());
        }

        info!(
            "Starting orchestration '{}' ({} steps, parallel: {})",
            orchestration_name, total_steps, orchestrated.parallel
        );

        // Initialize status
        {
            let mut status = self.status.write().unwrap();
            *status = Some(OrchestrationStatus {
                name: orchestration_name.clone(),
                current_iteration: 1,
                current_step: 0,
                total_steps,
                started_at: Utc::now(),
                is_running: true,
                failed_steps: Vec::new(),
                progress: 0.0,
            });
        }

        // Create control channel
        let (control_tx, mut control_rx) = mpsc::channel::<OrchestrationControl>(10);
        self.control_tx = Some(control_tx);

        // Clone Arc for the async task
        let status_arc = Arc::clone(&self.status);
        let config_arc = Arc::clone(&self.active_config);

        // Spawn orchestration task
        tokio::spawn(async move {
            Self::orchestration_task(
                orchestrated,
                status_arc,
                config_arc,
                &mut control_rx,
            )
            .await;
        });

        Ok(())
    }

    /// Orchestration task (runs in background)
    async fn orchestration_task(
        orchestrated: OrchestratedScenario,
        status: Arc<RwLock<Option<OrchestrationStatus>>>,
        active_config: Arc<RwLock<Option<ChaosConfig>>>,
        control_rx: &mut mpsc::Receiver<OrchestrationControl>,
    ) {
        let max_iterations = if orchestrated.loop_orchestration {
            orchestrated.max_iterations
        } else {
            1
        };

        let mut iteration = 1;

        loop {
            // Check if reached max iterations
            if max_iterations > 0 && iteration > max_iterations {
                break;
            }

            info!(
                "Starting iteration {}/{} of orchestration '{}'",
                iteration,
                if max_iterations > 0 {
                    max_iterations.to_string()
                } else {
                    "∞".to_string()
                },
                orchestrated.name
            );

            // Update iteration
            Self::update_status(&status, |s| s.current_iteration = iteration);

            if orchestrated.parallel {
                // Execute steps in parallel
                Self::execute_steps_parallel(&orchestrated, &status, &active_config).await;
            } else {
                // Execute steps sequentially
                if !Self::execute_steps_sequential(
                    &orchestrated,
                    &status,
                    &active_config,
                    control_rx,
                )
                .await
                {
                    // Stopped by control command
                    break;
                }
            }

            iteration += 1;

            // Check if should loop
            if !orchestrated.loop_orchestration {
                break;
            }
        }

        info!("Orchestration '{}' completed", orchestrated.name);
        Self::clear_status(&status);
        Self::clear_config(&active_config);
    }

    /// Execute steps sequentially
    async fn execute_steps_sequential(
        orchestrated: &OrchestratedScenario,
        status: &Arc<RwLock<Option<OrchestrationStatus>>>,
        active_config: &Arc<RwLock<Option<ChaosConfig>>>,
        control_rx: &mut mpsc::Receiver<OrchestrationControl>,
    ) -> bool {
        for (index, step) in orchestrated.steps.iter().enumerate() {
            // Check for control commands
            if let Ok(cmd) = control_rx.try_recv() {
                match cmd {
                    OrchestrationControl::Pause => {
                        info!("Orchestration paused");
                        // Wait for resume or stop
                        if let Some(cmd) = control_rx.recv().await {
                            match cmd {
                                OrchestrationControl::Resume => {
                                    info!("Orchestration resumed");
                                }
                                OrchestrationControl::Stop => {
                                    info!("Orchestration stopped");
                                    return false;
                                }
                                _ => {}
                            }
                        }
                    }
                    OrchestrationControl::Stop => {
                        info!("Orchestration stopped");
                        return false;
                    }
                    OrchestrationControl::SkipStep => {
                        info!("Skipping step: {}", step.name);
                        continue;
                    }
                    _ => {}
                }
            }

            // Execute step
            let success = Self::execute_step(step, status, active_config).await;

            if !success && !step.continue_on_failure {
                warn!("Step '{}' failed, stopping orchestration", step.name);
                Self::update_status(status, |s| s.failed_steps.push(step.name.clone()));
                return false;
            }

            // Update progress
            Self::update_status(status, |s| {
                s.current_step = index + 1;
                s.progress = (index + 1) as f64 / orchestrated.steps.len() as f64;
            });
        }

        true
    }

    /// Execute steps in parallel
    async fn execute_steps_parallel(
        orchestrated: &OrchestratedScenario,
        status: &Arc<RwLock<Option<OrchestrationStatus>>>,
        active_config: &Arc<RwLock<Option<ChaosConfig>>>,
    ) {
        let mut handles = Vec::new();

        for step in &orchestrated.steps {
            let step_clone = step.clone();
            let status_clone = Arc::clone(status);
            let config_clone = Arc::clone(active_config);

            let handle = tokio::spawn(async move {
                Self::execute_step(&step_clone, &status_clone, &config_clone).await
            });

            handles.push(handle);
        }

        // Wait for all steps to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Execute a single step
    async fn execute_step(
        step: &ScenarioStep,
        status: &Arc<RwLock<Option<OrchestrationStatus>>>,
        active_config: &Arc<RwLock<Option<ChaosConfig>>>,
    ) -> bool {
        info!("Executing step: {}", step.name);

        // Delay before step
        if step.delay_before_seconds > 0 {
            debug!("Waiting {}s before step '{}'", step.delay_before_seconds, step.name);
            sleep(std::time::Duration::from_secs(step.delay_before_seconds)).await;
        }

        // Apply the step's chaos config
        {
            let mut config = active_config.write().unwrap();
            *config = Some(step.scenario.chaos_config.clone());
        }

        // Determine duration
        let duration = step
            .duration_seconds
            .or(Some(step.scenario.duration_seconds))
            .unwrap_or(0);

        if duration > 0 {
            debug!("Running step '{}' for {}s", step.name, duration);
            sleep(std::time::Duration::from_secs(duration)).await;
        }

        info!("Completed step: {}", step.name);
        true
    }

    /// Update status
    fn update_status<F>(status: &Arc<RwLock<Option<OrchestrationStatus>>>, f: F)
    where
        F: FnOnce(&mut OrchestrationStatus),
    {
        let mut status_guard = status.write().unwrap();
        if let Some(ref mut s) = *status_guard {
            f(s);
        }
    }

    /// Clear status
    fn clear_status(status: &Arc<RwLock<Option<OrchestrationStatus>>>) {
        let mut status_guard = status.write().unwrap();
        *status_guard = None;
    }

    /// Clear config
    fn clear_config(config: &Arc<RwLock<Option<ChaosConfig>>>) {
        let mut config_guard = config.write().unwrap();
        *config_guard = None;
    }

    /// Get current orchestration status
    pub fn get_status(&self) -> Option<OrchestrationStatus> {
        self.status.read().unwrap().clone()
    }

    /// Get currently active chaos config
    pub fn get_active_config(&self) -> Option<ChaosConfig> {
        self.active_config.read().unwrap().clone()
    }

    /// Check if orchestration is running
    pub fn is_running(&self) -> bool {
        self.status.read().unwrap().is_some()
    }

    /// Stop orchestration
    pub async fn stop(&self) -> Result<(), String> {
        if let Some(ref tx) = self.control_tx {
            tx.send(OrchestrationControl::Stop)
                .await
                .map_err(|e| format!("Failed to stop: {}", e))?;
            Ok(())
        } else {
            Err("No orchestration in progress".to_string())
        }
    }
}

impl Default for ScenarioOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_step_creation() {
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let step = ScenarioStep::new("step1", scenario);

        assert_eq!(step.name, "step1");
        assert_eq!(step.delay_before_seconds, 0);
        assert!(!step.continue_on_failure);
    }

    #[test]
    fn test_orchestrated_scenario_creation() {
        let orchestrated = OrchestratedScenario::new("test_orchestration");

        assert_eq!(orchestrated.name, "test_orchestration");
        assert_eq!(orchestrated.steps.len(), 0);
        assert!(!orchestrated.parallel);
        assert!(!orchestrated.loop_orchestration);
    }

    #[test]
    fn test_add_steps() {
        let scenario1 = ChaosScenario::new("scenario1", ChaosConfig::default());
        let scenario2 = ChaosScenario::new("scenario2", ChaosConfig::default());

        let step1 = ScenarioStep::new("step1", scenario1);
        let step2 = ScenarioStep::new("step2", scenario2);

        let orchestrated = OrchestratedScenario::new("test")
            .add_step(step1)
            .add_step(step2);

        assert_eq!(orchestrated.steps.len(), 2);
    }

    #[test]
    fn test_json_export_import() {
        let scenario = ChaosScenario::new("test", ChaosConfig::default());
        let step = ScenarioStep::new("step1", scenario);

        let orchestrated = OrchestratedScenario::new("test_orchestration")
            .with_description("Test description")
            .add_step(step);

        let json = orchestrated.to_json().unwrap();
        let imported = OrchestratedScenario::from_json(&json).unwrap();

        assert_eq!(imported.name, "test_orchestration");
        assert_eq!(imported.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = ScenarioOrchestrator::new();
        assert!(!orchestrator.is_running());
    }
}
