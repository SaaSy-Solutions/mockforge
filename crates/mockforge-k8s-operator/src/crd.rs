//! Custom Resource Definitions for Chaos Orchestrations

use chrono::{DateTime, Utc};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ChaosOrchestration CRD
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "mockforge.io",
    version = "v1",
    kind = "ChaosOrchestration",
    plural = "chaosorchestrations",
    shortname = "co",
    shortname = "chaos",
    namespaced,
    status = "ChaosOrchestrationStatus",
    printcolumn = r#"{"name":"Phase", "type":"string", "jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Progress", "type":"string", "jsonPath":".status.progress"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ChaosOrchestrationSpec {
    /// Name of the orchestration
    pub name: String,

    /// Description of what this orchestration tests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Cron schedule for automatic execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,

    /// Steps to execute
    pub steps: Vec<OrchestrationStep>,

    /// Global variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, serde_json::Value>,

    /// Lifecycle hooks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<OrchestrationHook>,

    /// Assertions to validate
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertions: Vec<OrchestrationAssertion>,

    /// Enable execution reporting
    #[serde(default = "default_true")]
    pub enable_reporting: bool,

    /// Target Kubernetes services
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_services: Vec<TargetService>,
}

/// Orchestration step
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationStep {
    /// Step name
    pub name: String,

    /// Scenario type to execute
    pub scenario: String,

    /// Duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,

    /// Delay before starting
    #[serde(default)]
    pub delay_before_seconds: u64,

    /// Continue on failure
    #[serde(default)]
    pub continue_on_failure: bool,

    /// Step-specific parameters
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Orchestration hook
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationHook {
    /// Hook name
    pub name: String,

    /// Hook type
    #[serde(rename = "type")]
    pub hook_type: String,

    /// Actions to execute
    pub actions: Vec<HashMap<String, serde_json::Value>>,
}

/// Orchestration assertion
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationAssertion {
    /// Assertion type
    #[serde(rename = "type")]
    pub assertion_type: String,

    /// Expected value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<serde_json::Value>,

    /// Comparison operator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
}

/// Target service configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TargetService {
    /// Service name
    pub name: String,

    /// Service namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Label selector
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub selector: HashMap<String, String>,
}

/// ChaosOrchestration status
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChaosOrchestrationStatus {
    /// Execution phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<OrchestrationPhase>,

    /// Current iteration
    #[serde(default)]
    pub current_iteration: u32,

    /// Current step index
    #[serde(default)]
    pub current_step: u32,

    /// Total number of steps
    #[serde(default)]
    pub total_steps: u32,

    /// Progress (0.0 - 1.0)
    #[serde(default)]
    pub progress: f64,

    /// Execution start time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub start_time: Option<DateTime<Utc>>,

    /// Execution end time
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub end_time: Option<DateTime<Utc>>,

    /// Failed steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_steps: Vec<String>,

    /// Last scheduled time (for cron)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub last_scheduled_time: Option<DateTime<Utc>>,

    /// Status conditions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// Orchestration execution phase
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
pub enum OrchestrationPhase {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
}

/// Status condition
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    /// Condition type
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Condition status
    pub status: ConditionStatus,

    /// Last transition time
    #[schemars(with = "String")]
    pub last_transition_time: DateTime<Utc>,

    /// Reason for the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Condition status
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
pub enum ConditionStatus {
    True,
    False,
    Unknown,
}

fn default_true() -> bool {
    true
}

/// ChaosScenario CRD
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "mockforge.io",
    version = "v1",
    kind = "ChaosScenario",
    plural = "chaosscenarios",
    shortname = "cs",
    namespaced,
    status = "ChaosScenarioStatus",
    printcolumn = r#"{"name":"Type", "type":"string", "jsonPath":".spec.type"}"#,
    printcolumn = r#"{"name":"Active", "type":"boolean", "jsonPath":".status.active"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ChaosScenarioSpec {
    /// Scenario name
    pub name: String,

    /// Scenario type
    #[serde(rename = "type")]
    pub scenario_type: String,

    /// Configuration
    #[serde(default)]
    pub config: ScenarioConfig,

    /// Duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
}

/// Scenario configuration
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_rate: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_load: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_pressure: Option<bool>,
}

/// ChaosScenario status
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChaosScenarioStatus {
    /// Whether the scenario is currently active
    #[serde(default)]
    pub active: bool,

    /// When the scenario was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub applied_at: Option<DateTime<Utc>>,

    /// Affected pods
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_pods: Vec<String>,
}

impl Default for ChaosOrchestrationStatus {
    fn default() -> Self {
        Self {
            phase: Some(OrchestrationPhase::Pending),
            current_iteration: 0,
            current_step: 0,
            total_steps: 0,
            progress: 0.0,
            start_time: None,
            end_time: None,
            failed_steps: Vec::new(),
            last_scheduled_time: None,
            conditions: Vec::new(),
        }
    }
}

impl ChaosOrchestrationStatus {
    /// Add a condition to the status
    pub fn add_condition(
        &mut self,
        condition_type: String,
        status: ConditionStatus,
        reason: Option<String>,
        message: Option<String>,
    ) {
        // Remove existing condition of same type
        self.conditions.retain(|c| c.condition_type != condition_type);

        self.conditions.push(Condition {
            condition_type,
            status,
            last_transition_time: Utc::now(),
            reason,
            message,
        });
    }

    /// Check if a condition exists and is true
    pub fn has_condition(&self, condition_type: &str) -> bool {
        self.conditions
            .iter()
            .any(|c| c.condition_type == condition_type && c.status == ConditionStatus::True)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // OrchestrationPhase tests
    #[test]
    fn test_orchestration_phase_pending() {
        assert_eq!(OrchestrationPhase::Pending, OrchestrationPhase::Pending);
    }

    #[test]
    fn test_orchestration_phase_running() {
        assert_eq!(OrchestrationPhase::Running, OrchestrationPhase::Running);
    }

    #[test]
    fn test_orchestration_phase_completed() {
        assert_eq!(OrchestrationPhase::Completed, OrchestrationPhase::Completed);
    }

    #[test]
    fn test_orchestration_phase_failed() {
        assert_eq!(OrchestrationPhase::Failed, OrchestrationPhase::Failed);
    }

    #[test]
    fn test_orchestration_phase_paused() {
        assert_eq!(OrchestrationPhase::Paused, OrchestrationPhase::Paused);
    }

    #[test]
    fn test_orchestration_phase_debug() {
        let phase = OrchestrationPhase::Running;
        let debug = format!("{:?}", phase);
        assert!(debug.contains("Running"));
    }

    #[test]
    fn test_orchestration_phase_clone() {
        let phase = OrchestrationPhase::Completed;
        let cloned = phase.clone();
        assert_eq!(phase, cloned);
    }

    // ConditionStatus tests
    #[test]
    fn test_condition_status_true() {
        assert_eq!(ConditionStatus::True, ConditionStatus::True);
    }

    #[test]
    fn test_condition_status_false() {
        assert_eq!(ConditionStatus::False, ConditionStatus::False);
    }

    #[test]
    fn test_condition_status_unknown() {
        assert_eq!(ConditionStatus::Unknown, ConditionStatus::Unknown);
    }

    #[test]
    fn test_condition_status_debug() {
        let status = ConditionStatus::True;
        let debug = format!("{:?}", status);
        assert!(debug.contains("True"));
    }

    #[test]
    fn test_condition_status_clone() {
        let status = ConditionStatus::False;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    // ChaosOrchestrationStatus tests
    #[test]
    fn test_chaos_orchestration_status_default() {
        let status = ChaosOrchestrationStatus::default();
        assert_eq!(status.phase, Some(OrchestrationPhase::Pending));
        assert_eq!(status.current_iteration, 0);
        assert_eq!(status.current_step, 0);
        assert_eq!(status.total_steps, 0);
        assert_eq!(status.progress, 0.0);
        assert!(status.start_time.is_none());
        assert!(status.end_time.is_none());
        assert!(status.failed_steps.is_empty());
        assert!(status.last_scheduled_time.is_none());
        assert!(status.conditions.is_empty());
    }

    #[test]
    fn test_add_condition() {
        let mut status = ChaosOrchestrationStatus::default();
        status.add_condition(
            "Ready".to_string(),
            ConditionStatus::True,
            Some("Initialized".to_string()),
            Some("All systems ready".to_string()),
        );

        assert_eq!(status.conditions.len(), 1);
        assert_eq!(status.conditions[0].condition_type, "Ready");
        assert_eq!(status.conditions[0].status, ConditionStatus::True);
        assert_eq!(status.conditions[0].reason, Some("Initialized".to_string()));
        assert_eq!(status.conditions[0].message, Some("All systems ready".to_string()));
    }

    #[test]
    fn test_add_condition_replaces_existing() {
        let mut status = ChaosOrchestrationStatus::default();

        // Add initial condition
        status.add_condition(
            "Ready".to_string(),
            ConditionStatus::False,
            Some("NotReady".to_string()),
            None,
        );

        // Replace with new condition
        status.add_condition(
            "Ready".to_string(),
            ConditionStatus::True,
            Some("Ready".to_string()),
            Some("Now ready".to_string()),
        );

        // Should still be just one condition
        assert_eq!(status.conditions.len(), 1);
        assert_eq!(status.conditions[0].status, ConditionStatus::True);
        assert_eq!(status.conditions[0].reason, Some("Ready".to_string()));
    }

    #[test]
    fn test_add_multiple_conditions() {
        let mut status = ChaosOrchestrationStatus::default();

        status.add_condition("Ready".to_string(), ConditionStatus::True, None, None);

        status.add_condition("Progressing".to_string(), ConditionStatus::True, None, None);

        status.add_condition("Available".to_string(), ConditionStatus::False, None, None);

        assert_eq!(status.conditions.len(), 3);
    }

    #[test]
    fn test_has_condition_true() {
        let mut status = ChaosOrchestrationStatus::default();
        status.add_condition("Ready".to_string(), ConditionStatus::True, None, None);

        assert!(status.has_condition("Ready"));
    }

    #[test]
    fn test_has_condition_false() {
        let mut status = ChaosOrchestrationStatus::default();
        status.add_condition("Ready".to_string(), ConditionStatus::False, None, None);

        assert!(!status.has_condition("Ready"));
    }

    #[test]
    fn test_has_condition_unknown() {
        let mut status = ChaosOrchestrationStatus::default();
        status.add_condition("Ready".to_string(), ConditionStatus::Unknown, None, None);

        assert!(!status.has_condition("Ready"));
    }

    #[test]
    fn test_has_condition_nonexistent() {
        let status = ChaosOrchestrationStatus::default();
        assert!(!status.has_condition("NonExistent"));
    }

    // ScenarioConfig tests
    #[test]
    fn test_scenario_config_default() {
        let config = ScenarioConfig::default();
        assert!(config.latency_ms.is_none());
        assert!(config.error_rate.is_none());
        assert!(config.packet_loss.is_none());
        assert!(config.cpu_load.is_none());
        assert!(config.memory_pressure.is_none());
    }

    #[test]
    fn test_scenario_config_with_values() {
        let config = ScenarioConfig {
            latency_ms: Some(100),
            error_rate: Some(0.05),
            packet_loss: Some(0.01),
            cpu_load: Some(0.8),
            memory_pressure: Some(true),
        };

        assert_eq!(config.latency_ms, Some(100));
        assert_eq!(config.error_rate, Some(0.05));
        assert_eq!(config.packet_loss, Some(0.01));
        assert_eq!(config.cpu_load, Some(0.8));
        assert_eq!(config.memory_pressure, Some(true));
    }

    #[test]
    fn test_scenario_config_clone() {
        let config = ScenarioConfig {
            latency_ms: Some(50),
            error_rate: Some(0.1),
            ..Default::default()
        };

        let cloned = config.clone();
        assert_eq!(config.latency_ms, cloned.latency_ms);
        assert_eq!(config.error_rate, cloned.error_rate);
    }

    // ChaosScenarioStatus tests
    #[test]
    fn test_chaos_scenario_status_default() {
        let status = ChaosScenarioStatus::default();
        assert!(!status.active);
        assert!(status.applied_at.is_none());
        assert!(status.affected_pods.is_empty());
    }

    #[test]
    fn test_chaos_scenario_status_active() {
        let status = ChaosScenarioStatus {
            active: true,
            applied_at: Some(Utc::now()),
            affected_pods: vec!["pod-1".to_string(), "pod-2".to_string()],
        };

        assert!(status.active);
        assert!(status.applied_at.is_some());
        assert_eq!(status.affected_pods.len(), 2);
    }

    // OrchestrationStep tests
    #[test]
    fn test_orchestration_step_basic() {
        let step = OrchestrationStep {
            name: "test-step".to_string(),
            scenario: "latency".to_string(),
            duration_seconds: Some(60),
            delay_before_seconds: 0,
            continue_on_failure: false,
            parameters: HashMap::new(),
        };

        assert_eq!(step.name, "test-step");
        assert_eq!(step.scenario, "latency");
        assert_eq!(step.duration_seconds, Some(60));
        assert_eq!(step.delay_before_seconds, 0);
        assert!(!step.continue_on_failure);
    }

    #[test]
    fn test_orchestration_step_with_parameters() {
        let mut params = HashMap::new();
        params.insert("latency_ms".to_string(), serde_json::json!(100));

        let step = OrchestrationStep {
            name: "latency-step".to_string(),
            scenario: "latency".to_string(),
            duration_seconds: None,
            delay_before_seconds: 5,
            continue_on_failure: true,
            parameters: params,
        };

        assert_eq!(step.parameters.len(), 1);
        assert_eq!(step.parameters["latency_ms"], serde_json::json!(100));
    }

    // OrchestrationHook tests
    #[test]
    fn test_orchestration_hook() {
        let hook = OrchestrationHook {
            name: "pre-run".to_string(),
            hook_type: "before".to_string(),
            actions: vec![],
        };

        assert_eq!(hook.name, "pre-run");
        assert_eq!(hook.hook_type, "before");
        assert!(hook.actions.is_empty());
    }

    // OrchestrationAssertion tests
    #[test]
    fn test_orchestration_assertion() {
        let assertion = OrchestrationAssertion {
            assertion_type: "status_code".to_string(),
            expected_value: Some(serde_json::json!(200)),
            operator: Some("equals".to_string()),
        };

        assert_eq!(assertion.assertion_type, "status_code");
        assert_eq!(assertion.expected_value, Some(serde_json::json!(200)));
        assert_eq!(assertion.operator, Some("equals".to_string()));
    }

    // TargetService tests
    #[test]
    fn test_target_service_basic() {
        let service = TargetService {
            name: "my-service".to_string(),
            namespace: Some("default".to_string()),
            selector: HashMap::new(),
        };

        assert_eq!(service.name, "my-service");
        assert_eq!(service.namespace, Some("default".to_string()));
        assert!(service.selector.is_empty());
    }

    #[test]
    fn test_target_service_with_selector() {
        let mut selector = HashMap::new();
        selector.insert("app".to_string(), "my-app".to_string());

        let service = TargetService {
            name: "my-service".to_string(),
            namespace: None,
            selector,
        };

        assert_eq!(service.selector.len(), 1);
        assert_eq!(service.selector["app"], "my-app");
    }

    // Condition tests
    #[test]
    fn test_condition_creation() {
        let condition = Condition {
            condition_type: "Ready".to_string(),
            status: ConditionStatus::True,
            last_transition_time: Utc::now(),
            reason: Some("Ready".to_string()),
            message: Some("Service is ready".to_string()),
        };

        assert_eq!(condition.condition_type, "Ready");
        assert_eq!(condition.status, ConditionStatus::True);
        assert_eq!(condition.reason, Some("Ready".to_string()));
    }

    // default_true helper function test
    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }

    // Serialization tests
    #[test]
    fn test_orchestration_phase_serialize() {
        let json = serde_json::to_string(&OrchestrationPhase::Running).unwrap();
        assert!(json.contains("Running"));
    }

    #[test]
    fn test_condition_status_serialize() {
        let json = serde_json::to_string(&ConditionStatus::True).unwrap();
        assert!(json.contains("True"));
    }

    #[test]
    fn test_scenario_config_serialize() {
        let config = ScenarioConfig {
            latency_ms: Some(100),
            error_rate: Some(0.05),
            packet_loss: None,
            cpu_load: None,
            memory_pressure: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("latencyMs"));
        assert!(json.contains("errorRate"));
    }
}
