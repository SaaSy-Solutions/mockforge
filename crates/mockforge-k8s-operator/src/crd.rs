//! Custom Resource Definitions for Chaos Orchestrations

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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
    pub start_time: Option<DateTime<Utc>>,

    /// Execution end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Failed steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_steps: Vec<String>,

    /// Last scheduled time (for cron)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub fn add_condition(&mut self, condition_type: String, status: ConditionStatus, reason: Option<String>, message: Option<String>) {
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
