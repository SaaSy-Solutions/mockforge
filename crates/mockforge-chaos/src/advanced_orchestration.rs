//! Advanced orchestration features
//!
//! Provides conditional logic, variables, hooks, assertions, and reporting
//! for complex chaos engineering orchestrations.

use crate::{
    scenario_orchestrator::{OrchestratedScenario, ScenarioStep},
    scenarios::ChaosScenario,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use thiserror::Error;

/// Orchestration errors
#[derive(Error, Debug)]
pub enum OrchestrationError {
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Hook execution failed: {0}")]
    HookFailed(String),

    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    #[error("Condition evaluation failed: {0}")]
    ConditionFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Conditional expression for if/then logic
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Variable equals value
    Equals {
        variable: String,
        value: JsonValue,
    },
    /// Variable not equals value
    NotEquals {
        variable: String,
        value: JsonValue,
    },
    /// Variable greater than value
    GreaterThan {
        variable: String,
        value: f64,
    },
    /// Variable less than value
    LessThan {
        variable: String,
        value: f64,
    },
    /// Variable greater than or equal to value
    GreaterThanOrEqual {
        variable: String,
        value: f64,
    },
    /// Variable less than or equal to value
    LessThanOrEqual {
        variable: String,
        value: f64,
    },
    /// Variable exists
    Exists {
        variable: String,
    },
    /// AND condition
    And {
        conditions: Vec<Condition>,
    },
    /// OR condition
    Or {
        conditions: Vec<Condition>,
    },
    /// NOT condition
    Not {
        condition: Box<Condition>,
    },
    /// Previous step succeeded
    PreviousStepSucceeded,
    /// Previous step failed
    PreviousStepFailed,
    /// Metric threshold
    MetricThreshold {
        metric_name: String,
        operator: ComparisonOperator,
        threshold: f64,
    },
}

/// Comparison operator
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

impl Condition {
    /// Evaluate the condition
    pub fn evaluate(&self, context: &ExecutionContext) -> Result<bool, OrchestrationError> {
        match self {
            Condition::Equals { variable, value } => {
                let var_value = context.get_variable(variable)?;
                Ok(var_value == value)
            }
            Condition::NotEquals { variable, value } => {
                let var_value = context.get_variable(variable)?;
                Ok(var_value != value)
            }
            Condition::GreaterThan { variable, value } => {
                let var_value = context.get_variable(variable)?;
                if let Some(num) = var_value.as_f64() {
                    Ok(num > *value)
                } else {
                    Err(OrchestrationError::ConditionFailed(
                        format!("Variable {} is not a number", variable),
                    ))
                }
            }
            Condition::LessThan { variable, value } => {
                let var_value = context.get_variable(variable)?;
                if let Some(num) = var_value.as_f64() {
                    Ok(num < *value)
                } else {
                    Err(OrchestrationError::ConditionFailed(
                        format!("Variable {} is not a number", variable),
                    ))
                }
            }
            Condition::GreaterThanOrEqual { variable, value } => {
                let var_value = context.get_variable(variable)?;
                if let Some(num) = var_value.as_f64() {
                    Ok(num >= *value)
                } else {
                    Err(OrchestrationError::ConditionFailed(
                        format!("Variable {} is not a number", variable),
                    ))
                }
            }
            Condition::LessThanOrEqual { variable, value } => {
                let var_value = context.get_variable(variable)?;
                if let Some(num) = var_value.as_f64() {
                    Ok(num <= *value)
                } else {
                    Err(OrchestrationError::ConditionFailed(
                        format!("Variable {} is not a number", variable),
                    ))
                }
            }
            Condition::Exists { variable } => Ok(context.variables.contains_key(variable)),
            Condition::And { conditions } => {
                for cond in conditions {
                    if !cond.evaluate(context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or { conditions } => {
                for cond in conditions {
                    if cond.evaluate(context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::Not { condition } => Ok(!condition.evaluate(context)?),
            Condition::PreviousStepSucceeded => Ok(context.last_step_success),
            Condition::PreviousStepFailed => Ok(!context.last_step_success),
            Condition::MetricThreshold {
                metric_name,
                operator,
                threshold,
            } => {
                if let Some(value) = context.metrics.get(metric_name) {
                    Ok(match operator {
                        ComparisonOperator::Equals => (value - threshold).abs() < f64::EPSILON,
                        ComparisonOperator::NotEquals => (value - threshold).abs() >= f64::EPSILON,
                        ComparisonOperator::GreaterThan => value > threshold,
                        ComparisonOperator::LessThan => value < threshold,
                        ComparisonOperator::GreaterThanOrEqual => value >= threshold,
                        ComparisonOperator::LessThanOrEqual => value <= threshold,
                    })
                } else {
                    Err(OrchestrationError::ConditionFailed(format!(
                        "Metric {} not found",
                        metric_name
                    )))
                }
            }
        }
    }
}

/// Conditional step with if/then/else logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStep {
    /// Step name
    pub name: String,
    /// Condition to evaluate
    pub condition: Condition,
    /// Steps to execute if condition is true
    pub then_steps: Vec<AdvancedScenarioStep>,
    /// Steps to execute if condition is false (optional)
    pub else_steps: Vec<AdvancedScenarioStep>,
}

/// Hook type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Execute before step
    PreStep,
    /// Execute after step
    PostStep,
    /// Execute before orchestration
    PreOrchestration,
    /// Execute after orchestration
    PostOrchestration,
}

/// Hook action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookAction {
    /// Set variable
    SetVariable { name: String, value: JsonValue },
    /// Log message
    Log { message: String, level: LogLevel },
    /// HTTP request
    HttpRequest {
        url: String,
        method: String,
        body: Option<String>,
    },
    /// Execute command
    Command { command: String, args: Vec<String> },
    /// Record metric
    RecordMetric { name: String, value: f64 },
}

/// Log level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Hook name
    pub name: String,
    /// Hook type
    pub hook_type: HookType,
    /// Actions to perform
    pub actions: Vec<HookAction>,
    /// Condition (optional)
    pub condition: Option<Condition>,
}

impl Hook {
    /// Execute the hook
    pub async fn execute(&self, context: &mut ExecutionContext) -> Result<(), OrchestrationError> {
        // Check condition if present
        if let Some(condition) = &self.condition {
            if !condition.evaluate(context)? {
                return Ok(());
            }
        }

        for action in &self.actions {
            self.execute_action(action, context).await?;
        }

        Ok(())
    }

    /// Execute a single action
    async fn execute_action(
        &self,
        action: &HookAction,
        context: &mut ExecutionContext,
    ) -> Result<(), OrchestrationError> {
        match action {
            HookAction::SetVariable { name, value } => {
                context.set_variable(name.clone(), value.clone());
                Ok(())
            }
            HookAction::Log { message, level } => {
                use tracing::{debug, error, info, trace, warn};
                match level {
                    LogLevel::Trace => trace!("[Hook: {}] {}", self.name, message),
                    LogLevel::Debug => debug!("[Hook: {}] {}", self.name, message),
                    LogLevel::Info => info!("[Hook: {}] {}", self.name, message),
                    LogLevel::Warn => warn!("[Hook: {}] {}", self.name, message),
                    LogLevel::Error => error!("[Hook: {}] {}", self.name, message),
                }
                Ok(())
            }
            HookAction::HttpRequest { url, method, body } => {
                // In production, this would make actual HTTP requests
                // For now, just log
                tracing::info!(
                    "[Hook: {}] HTTP {} {} (body: {:?})",
                    self.name,
                    method,
                    url,
                    body
                );
                Ok(())
            }
            HookAction::Command { command, args } => {
                // In production, this would execute commands
                // For now, just log
                tracing::info!(
                    "[Hook: {}] Execute: {} {:?}",
                    self.name,
                    command,
                    args
                );
                Ok(())
            }
            HookAction::RecordMetric { name, value } => {
                context.record_metric(name.clone(), *value);
                Ok(())
            }
        }
    }
}

/// Assertion for validating expected outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assertion {
    /// Variable equals value
    VariableEquals {
        variable: String,
        expected: JsonValue,
    },
    /// Metric within range
    MetricInRange {
        metric: String,
        min: f64,
        max: f64,
    },
    /// Step completed successfully
    StepSucceeded { step_name: String },
    /// Step failed
    StepFailed { step_name: String },
    /// Custom condition
    Condition { condition: Condition },
}

impl Assertion {
    /// Validate the assertion
    pub fn validate(&self, context: &ExecutionContext) -> Result<bool, OrchestrationError> {
        match self {
            Assertion::VariableEquals { variable, expected } => {
                let value = context.get_variable(variable)?;
                Ok(value == expected)
            }
            Assertion::MetricInRange { metric, min, max } => {
                if let Some(value) = context.metrics.get(metric) {
                    Ok(*value >= *min && *value <= *max)
                } else {
                    Ok(false)
                }
            }
            Assertion::StepSucceeded { step_name } => {
                if let Some(result) = context.step_results.get(step_name) {
                    Ok(result.success)
                } else {
                    Ok(false)
                }
            }
            Assertion::StepFailed { step_name } => {
                if let Some(result) = context.step_results.get(step_name) {
                    Ok(!result.success)
                } else {
                    Ok(false)
                }
            }
            Assertion::Condition { condition } => condition.evaluate(context),
        }
    }
}

/// Advanced scenario step with conditionals, variables, hooks, and assertions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedScenarioStep {
    /// Base step
    #[serde(flatten)]
    pub base: ScenarioStep,

    /// Condition to execute this step (optional)
    pub condition: Option<Condition>,

    /// Pre-step hooks
    pub pre_hooks: Vec<Hook>,

    /// Post-step hooks
    pub post_hooks: Vec<Hook>,

    /// Assertions to validate after execution
    pub assertions: Vec<Assertion>,

    /// Variables to set before execution
    pub variables: HashMap<String, JsonValue>,

    /// Timeout in seconds (overrides duration)
    pub timeout_seconds: Option<u64>,

    /// Retry configuration
    pub retry: Option<RetryConfig>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: usize,
    /// Delay between retries (seconds)
    pub delay_seconds: u64,
    /// Exponential backoff
    pub exponential_backoff: bool,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step name
    pub step_name: String,
    /// Success status
    pub success: bool,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: DateTime<Utc>,
    /// Duration in seconds
    pub duration_seconds: f64,
    /// Error message if failed
    pub error: Option<String>,
    /// Assertion results
    pub assertion_results: Vec<AssertionResult>,
    /// Metrics collected during step
    pub metrics: HashMap<String, f64>,
}

/// Assertion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Assertion description
    pub description: String,
    /// Passed or failed
    pub passed: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Execution context with variables and state
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Variables
    pub variables: HashMap<String, JsonValue>,
    /// Metrics
    pub metrics: HashMap<String, f64>,
    /// Step results
    pub step_results: HashMap<String, StepResult>,
    /// Last step success status
    pub last_step_success: bool,
    /// Current iteration (for loops)
    pub iteration: usize,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            metrics: HashMap::new(),
            step_results: HashMap::new(),
            last_step_success: true,
            iteration: 0,
        }
    }

    /// Set a variable
    pub fn set_variable(&mut self, name: String, value: JsonValue) {
        self.variables.insert(name, value);
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Result<&JsonValue, OrchestrationError> {
        self.variables
            .get(name)
            .ok_or_else(|| OrchestrationError::VariableNotFound(name.to_string()))
    }

    /// Record a metric
    pub fn record_metric(&mut self, name: String, value: f64) {
        self.metrics.insert(name, value);
    }

    /// Record step result
    pub fn record_step_result(&mut self, result: StepResult) {
        self.last_step_success = result.success;
        self.step_results.insert(result.step_name.clone(), result);
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Advanced orchestrated scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedOrchestratedScenario {
    /// Base orchestration
    #[serde(flatten)]
    pub base: OrchestratedScenario,

    /// Advanced steps
    pub advanced_steps: Vec<AdvancedScenarioStep>,

    /// Conditional steps
    pub conditional_steps: Vec<ConditionalStep>,

    /// Global hooks
    pub hooks: Vec<Hook>,

    /// Global assertions
    pub assertions: Vec<Assertion>,

    /// Initial variables
    pub variables: HashMap<String, JsonValue>,

    /// Enable detailed reporting
    pub enable_reporting: bool,

    /// Report output path
    pub report_path: Option<String>,
}

impl AdvancedOrchestratedScenario {
    /// Create from base orchestration
    pub fn from_base(base: OrchestratedScenario) -> Self {
        Self {
            base,
            advanced_steps: Vec::new(),
            conditional_steps: Vec::new(),
            hooks: Vec::new(),
            assertions: Vec::new(),
            variables: HashMap::new(),
            enable_reporting: false,
            report_path: None,
        }
    }

    /// Add variable
    pub fn with_variable(mut self, name: String, value: JsonValue) -> Self {
        self.variables.insert(name, value);
        self
    }

    /// Add hook
    pub fn with_hook(mut self, hook: Hook) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Add assertion
    pub fn with_assertion(mut self, assertion: Assertion) -> Self {
        self.assertions.push(assertion);
        self
    }

    /// Enable reporting
    pub fn with_reporting(mut self, path: Option<String>) -> Self {
        self.enable_reporting = true;
        self.report_path = path;
        self
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, OrchestrationError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| OrchestrationError::SerializationError(e.to_string()))
    }

    /// Export to YAML
    pub fn to_yaml(&self) -> Result<String, OrchestrationError> {
        serde_yaml::to_string(self)
            .map_err(|e| OrchestrationError::SerializationError(e.to_string()))
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, OrchestrationError> {
        serde_json::from_str(json)
            .map_err(|e| OrchestrationError::SerializationError(e.to_string()))
    }

    /// Import from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, OrchestrationError> {
        serde_yaml::from_str(yaml)
            .map_err(|e| OrchestrationError::SerializationError(e.to_string()))
    }
}

/// Execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Orchestration name
    pub orchestration_name: String,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: DateTime<Utc>,
    /// Total duration in seconds
    pub total_duration_seconds: f64,
    /// Success status
    pub success: bool,
    /// Step results
    pub step_results: Vec<StepResult>,
    /// Assertion results
    pub assertion_results: Vec<AssertionResult>,
    /// Final variables
    pub final_variables: HashMap<String, JsonValue>,
    /// Final metrics
    pub final_metrics: HashMap<String, f64>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl ExecutionReport {
    /// Create a new report
    pub fn new(orchestration_name: String, start_time: DateTime<Utc>) -> Self {
        Self {
            orchestration_name,
            start_time,
            end_time: Utc::now(),
            total_duration_seconds: 0.0,
            success: true,
            step_results: Vec::new(),
            assertion_results: Vec::new(),
            final_variables: HashMap::new(),
            final_metrics: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Finalize the report
    pub fn finalize(mut self, context: &ExecutionContext) -> Self {
        self.end_time = Utc::now();
        self.total_duration_seconds = (self.end_time - self.start_time).num_milliseconds() as f64 / 1000.0;
        self.final_variables = context.variables.clone();
        self.final_metrics = context.metrics.clone();
        self.step_results = context.step_results.values().cloned().collect();
        self.success = self.step_results.iter().all(|r| r.success) && self.errors.is_empty();
        self
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, OrchestrationError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| OrchestrationError::SerializationError(e.to_string()))
    }

    /// Export to HTML
    pub fn to_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Chaos Orchestration Report: {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f5f5f5; padding: 20px; border-radius: 5px; }}
        .success {{ color: green; }}
        .failure {{ color: red; }}
        table {{ border-collapse: collapse; width: 100%; margin: 20px 0; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background: #f5f5f5; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Chaos Orchestration Report</h1>
        <h2>{}</h2>
        <p><strong>Status:</strong> <span class="{}">{}</span></p>
        <p><strong>Duration:</strong> {:.2} seconds</p>
        <p><strong>Start Time:</strong> {}</p>
        <p><strong>End Time:</strong> {}</p>
    </div>

    <h2>Step Results</h2>
    <table>
        <tr>
            <th>Step</th>
            <th>Status</th>
            <th>Duration (s)</th>
            <th>Assertions</th>
        </tr>
        {}
    </table>

    <h2>Metrics</h2>
    <table>
        <tr>
            <th>Metric</th>
            <th>Value</th>
        </tr>
        {}
    </table>
</body>
</html>"#,
            self.orchestration_name,
            self.orchestration_name,
            if self.success { "success" } else { "failure" },
            if self.success { "SUCCESS" } else { "FAILURE" },
            self.total_duration_seconds,
            self.start_time,
            self.end_time,
            self.step_results
                .iter()
                .map(|r| format!(
                    "<tr><td>{}</td><td class=\"{}\">{}</td><td>{:.2}</td><td>{}/{}</td></tr>",
                    r.step_name,
                    if r.success { "success" } else { "failure" },
                    if r.success { "SUCCESS" } else { "FAILURE" },
                    r.duration_seconds,
                    r.assertion_results.iter().filter(|a| a.passed).count(),
                    r.assertion_results.len()
                ))
                .collect::<Vec<_>>()
                .join("\n        "),
            self.final_metrics
                .iter()
                .map(|(k, v)| format!("<tr><td>{}</td><td>{:.2}</td></tr>", k, v))
                .collect::<Vec<_>>()
                .join("\n        ")
        )
    }
}

/// Orchestration library for storing and sharing orchestrations
#[derive(Debug, Clone)]
pub struct OrchestrationLibrary {
    /// Storage for orchestrations
    orchestrations: Arc<RwLock<HashMap<String, AdvancedOrchestratedScenario>>>,
}

impl OrchestrationLibrary {
    /// Create a new library
    pub fn new() -> Self {
        Self {
            orchestrations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an orchestration
    pub fn store(&self, name: String, orchestration: AdvancedOrchestratedScenario) {
        let mut orch = self.orchestrations.write().unwrap();
        orch.insert(name, orchestration);
    }

    /// Retrieve an orchestration
    pub fn retrieve(&self, name: &str) -> Option<AdvancedOrchestratedScenario> {
        let orch = self.orchestrations.read().unwrap();
        orch.get(name).cloned()
    }

    /// List all orchestrations
    pub fn list(&self) -> Vec<String> {
        let orch = self.orchestrations.read().unwrap();
        orch.keys().cloned().collect()
    }

    /// Delete an orchestration
    pub fn delete(&self, name: &str) -> bool {
        let mut orch = self.orchestrations.write().unwrap();
        orch.remove(name).is_some()
    }

    /// Import from directory
    pub fn import_from_directory(&self, _path: &str) -> Result<usize, OrchestrationError> {
        // In production, this would scan a directory and import files
        // For now, just return 0
        Ok(0)
    }

    /// Export to directory
    pub fn export_to_directory(&self, _path: &str) -> Result<usize, OrchestrationError> {
        // In production, this would export all orchestrations to files
        // For now, just return count
        let orch = self.orchestrations.read().unwrap();
        Ok(orch.len())
    }
}

impl Default for OrchestrationLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_equals() {
        let mut context = ExecutionContext::new();
        context.set_variable("test".to_string(), JsonValue::String("value".to_string()));

        let condition = Condition::Equals {
            variable: "test".to_string(),
            value: JsonValue::String("value".to_string()),
        };

        assert!(condition.evaluate(&context).unwrap());
    }

    #[test]
    fn test_condition_and() {
        let mut context = ExecutionContext::new();
        context.set_variable("a".to_string(), JsonValue::Number(5.into()));
        context.set_variable("b".to_string(), JsonValue::Number(10.into()));

        let condition = Condition::And {
            conditions: vec![
                Condition::GreaterThan {
                    variable: "a".to_string(),
                    value: 3.0,
                },
                Condition::LessThan {
                    variable: "b".to_string(),
                    value: 15.0,
                },
            ],
        };

        assert!(condition.evaluate(&context).unwrap());
    }

    #[test]
    fn test_execution_context() {
        let mut context = ExecutionContext::new();
        context.set_variable("test".to_string(), JsonValue::String("value".to_string()));
        context.record_metric("latency".to_string(), 100.0);

        assert_eq!(
            context.get_variable("test").unwrap(),
            &JsonValue::String("value".to_string())
        );
        assert_eq!(*context.metrics.get("latency").unwrap(), 100.0);
    }

    #[test]
    fn test_orchestration_library() {
        let library = OrchestrationLibrary::new();

        let orch = AdvancedOrchestratedScenario::from_base(
            OrchestratedScenario::new("test")
        );

        library.store("test".to_string(), orch.clone());

        let retrieved = library.retrieve("test");
        assert!(retrieved.is_some());

        let list = library.list();
        assert_eq!(list.len(), 1);

        let deleted = library.delete("test");
        assert!(deleted);

        let list = library.list();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_execution_report() {
        let report = ExecutionReport::new("test".to_string(), Utc::now());
        let context = ExecutionContext::new();

        let final_report = report.finalize(&context);
        assert_eq!(final_report.orchestration_name, "test");
        assert!(final_report.total_duration_seconds >= 0.0);
    }
}
