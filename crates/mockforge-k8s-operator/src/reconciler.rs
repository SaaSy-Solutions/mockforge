//! Reconciler for ChaosOrchestration resources

use crate::crd::{ChaosOrchestration, ChaosOrchestrationStatus, OrchestrationPhase, ConditionStatus};
use crate::{OperatorError, Result};
use kube::{Api, Client, ResourceExt};
use kube::api::{Patch, PatchParams};
use tracing::{debug, info, warn, error};
use serde_json::json;
use mockforge_chaos::{ScenarioOrchestrator, OrchestratedScenario, ScenarioStep, ChaosScenario, ChaosConfig};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reconciler for ChaosOrchestration resources
pub struct Reconciler {
    client: Client,
    orchestrators: Arc<RwLock<std::collections::HashMap<String, ScenarioOrchestrator>>>,
}

impl Reconciler {
    /// Create a new reconciler
    pub fn new(client: Client) -> Self {
        Self {
            client,
            orchestrators: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Reconcile a ChaosOrchestration resource
    pub async fn reconcile(&self, orchestration: Arc<ChaosOrchestration>, namespace: &str) -> Result<()> {
        let name = orchestration.name_any();
        info!("Reconciling ChaosOrchestration: {}/{}", namespace, name);

        let api: Api<ChaosOrchestration> = Api::namespaced(self.client.clone(), namespace);

        // Get current status
        let current_status = orchestration.status.clone().unwrap_or_default();

        match current_status.phase {
            Some(OrchestrationPhase::Pending) | None => {
                // Start orchestration
                self.start_orchestration(orchestration.clone(), &api).await?;
            }
            Some(OrchestrationPhase::Running) => {
                // Check orchestration status and update
                self.update_running_orchestration(orchestration.clone(), &api).await?;
            }
            Some(OrchestrationPhase::Completed) | Some(OrchestrationPhase::Failed) => {
                // Check if should be restarted (e.g., scheduled execution)
                if let Some(schedule) = &orchestration.spec.schedule {
                    self.handle_scheduled_execution(orchestration.clone(), &api, schedule).await?;
                }
            }
            Some(OrchestrationPhase::Paused) => {
                // Orchestration is paused, do nothing
                debug!("Orchestration {}/{} is paused", namespace, name);
            }
        }

        Ok(())
    }

    /// Start an orchestration
    async fn start_orchestration(
        &self,
        orchestration: Arc<ChaosOrchestration>,
        api: &Api<ChaosOrchestration>,
    ) -> Result<()> {
        let name = orchestration.name_any();
        info!("Starting orchestration: {}", name);

        // Convert CRD to OrchestratedScenario
        let orchestrated = self.crd_to_orchestrated(&orchestration.spec)?;

        // Create orchestrator
        let mut orchestrator = ScenarioOrchestrator::new();

        // Execute orchestration in background
        let orchestration_clone = orchestrated.clone();
        let orchestrator_result = orchestrator.execute(orchestration_clone).await;

        if let Err(e) = orchestrator_result {
            error!("Failed to start orchestration: {}", e);

            // Update status to failed
            self.update_status(
                api,
                &name,
                ChaosOrchestrationStatus {
                    phase: Some(OrchestrationPhase::Failed),
                    ..Default::default()
                },
            ).await?;

            return Err(OperatorError::Orchestration(e));
        }

        // Store orchestrator
        let mut orchestrators = self.orchestrators.write().await;
        orchestrators.insert(name.clone(), orchestrator);
        drop(orchestrators);

        // Update status to running
        let mut status = ChaosOrchestrationStatus {
            phase: Some(OrchestrationPhase::Running),
            total_steps: orchestrated.steps.len() as u32,
            start_time: Some(chrono::Utc::now()),
            ..Default::default()
        };
        status.add_condition(
            "Started".to_string(),
            ConditionStatus::True,
            Some("OrchestrationStarted".to_string()),
            Some("Orchestration has been started".to_string()),
        );

        self.update_status(api, &name, status).await?;

        Ok(())
    }

    /// Update running orchestration status
    async fn update_running_orchestration(
        &self,
        orchestration: Arc<ChaosOrchestration>,
        api: &Api<ChaosOrchestration>,
    ) -> Result<()> {
        let name = orchestration.name_any();

        let orchestrators = self.orchestrators.read().await;
        let orchestrator = orchestrators.get(&name);

        if let Some(orch) = orchestrator {
            // Get current status from orchestrator
            if let Some(orch_status) = orch.get_status() {
                let phase = if orch_status.is_running {
                    OrchestrationPhase::Running
                } else {
                    if orch_status.failed_steps.is_empty() {
                        OrchestrationPhase::Completed
                    } else {
                        OrchestrationPhase::Failed
                    }
                };

                let status = ChaosOrchestrationStatus {
                    phase: Some(phase.clone()),
                    current_iteration: orch_status.current_iteration as u32,
                    current_step: orch_status.current_step as u32,
                    total_steps: orch_status.total_steps as u32,
                    progress: orch_status.progress,
                    start_time: Some(orch_status.started_at),
                    end_time: if !orch_status.is_running {
                        Some(chrono::Utc::now())
                    } else {
                        None
                    },
                    failed_steps: orch_status.failed_steps.clone(),
                    ..orchestration.status.clone().unwrap_or_default()
                };

                drop(orchestrators);
                self.update_status(api, &name, status).await?;

                // Clean up orchestrator if completed or failed
                if phase == OrchestrationPhase::Completed || phase == OrchestrationPhase::Failed {
                    let mut orchestrators = self.orchestrators.write().await;
                    orchestrators.remove(&name);
                }
            }
        } else {
            warn!("Orchestrator not found for running orchestration: {}", name);
        }

        Ok(())
    }

    /// Handle scheduled execution
    async fn handle_scheduled_execution(
        &self,
        orchestration: Arc<ChaosOrchestration>,
        api: &Api<ChaosOrchestration>,
        schedule: &str,
    ) -> Result<()> {
        let name = orchestration.name_any();

        // Parse cron schedule and check if should run
        // This is a simplified version - production would use a cron parser
        let should_run = self.should_run_scheduled(schedule, &orchestration)?;

        if should_run {
            info!("Running scheduled orchestration: {}", name);

            // Reset status to pending to trigger re-execution
            let status = ChaosOrchestrationStatus {
                phase: Some(OrchestrationPhase::Pending),
                last_scheduled_time: Some(chrono::Utc::now()),
                ..Default::default()
            };

            self.update_status(api, &name, status).await?;
        }

        Ok(())
    }

    /// Check if scheduled orchestration should run
    fn should_run_scheduled(
        &self,
        _schedule: &str,
        orchestration: &ChaosOrchestration,
    ) -> Result<bool> {
        // Simplified implementation - just check if enough time has passed
        if let Some(status) = &orchestration.status {
            if let Some(last_scheduled) = status.last_scheduled_time {
                let elapsed = chrono::Utc::now() - last_scheduled;
                // Run if more than 1 hour has passed (simplified)
                Ok(elapsed.num_hours() >= 1)
            } else {
                Ok(true) // Never run before
            }
        } else {
            Ok(true)
        }
    }

    /// Convert CRD spec to OrchestratedScenario
    fn crd_to_orchestrated(&self, spec: &crate::crd::ChaosOrchestrationSpec) -> Result<OrchestratedScenario> {
        let mut orchestrated = OrchestratedScenario::new(&spec.name);

        if let Some(desc) = &spec.description {
            orchestrated = orchestrated.with_description(desc);
        }

        // Convert steps
        for step_spec in &spec.steps {
            let config = self.build_chaos_config(&step_spec.parameters)?;
            let scenario = ChaosScenario::new(&step_spec.scenario, config);

            let mut step = ScenarioStep::new(&step_spec.name, scenario);

            if let Some(duration) = step_spec.duration_seconds {
                step = step.with_duration(duration);
            }

            if step_spec.delay_before_seconds > 0 {
                step = step.with_delay_before(step_spec.delay_before_seconds);
            }

            if step_spec.continue_on_failure {
                step = step.continue_on_failure();
            }

            orchestrated = orchestrated.add_step(step);
        }

        Ok(orchestrated)
    }

    /// Build ChaosConfig from parameters
    fn build_chaos_config(
        &self,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ChaosConfig> {
        // Start with default config
        let mut config = ChaosConfig::default();

        // Apply parameters
        if let Some(latency) = parameters.get("latency_ms") {
            if let Some(latency_val) = latency.as_u64() {
                config.latency = Some(mockforge_chaos::LatencyConfig {
                    min_ms: latency_val,
                    max_ms: latency_val,
                });
            }
        }

        if let Some(error_rate) = parameters.get("error_rate") {
            if let Some(rate) = error_rate.as_f64() {
                config.fault_injection = Some(mockforge_chaos::FaultInjectionConfig {
                    error_rate: rate,
                    ..Default::default()
                });
            }
        }

        Ok(config)
    }

    /// Update orchestration status
    async fn update_status(
        &self,
        api: &Api<ChaosOrchestration>,
        name: &str,
        status: ChaosOrchestrationStatus,
    ) -> Result<()> {
        let patch = json!({
            "status": status
        });

        api.patch_status(
            name,
            &PatchParams::default(),
            &Patch::Merge(&patch),
        ).await?;

        debug!("Updated status for {}: {:?}", name, status.phase);

        Ok(())
    }

    /// Handle orchestration deletion
    pub async fn cleanup(&self, name: &str) -> Result<()> {
        info!("Cleaning up orchestration: {}", name);

        // Stop orchestrator if running
        let mut orchestrators = self.orchestrators.write().await;
        if let Some(orchestrator) = orchestrators.remove(name) {
            if let Err(e) = orchestrator.stop().await {
                warn!("Error stopping orchestrator {}: {}", name, e);
            }
        }

        Ok(())
    }
}
