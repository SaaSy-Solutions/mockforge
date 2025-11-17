//! Flow execution engine
//!
//! This module provides the execution engine for business flows defined in the Scenario Studio.

use crate::error::{Error, Result};
use crate::scenario_studio::types::{
    FlowCondition, FlowConnection, FlowDefinition, FlowStep, StepType,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

/// Result of executing a flow step
#[derive(Debug, Clone)]
pub struct FlowStepResult {
    /// ID of the step that was executed
    pub step_id: String,
    /// Whether the step executed successfully
    pub success: bool,
    /// Response data (if applicable)
    pub response: Option<Value>,
    /// Error message (if execution failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Variables extracted from the response
    pub extracted_variables: HashMap<String, Value>,
}

/// Result of executing an entire flow
#[derive(Debug, Clone)]
pub struct FlowExecutionResult {
    /// ID of the flow that was executed
    pub flow_id: String,
    /// Whether the flow completed successfully
    pub success: bool,
    /// Results for each step that was executed
    pub step_results: Vec<FlowStepResult>,
    /// Final variables after flow execution
    pub final_variables: HashMap<String, Value>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Error message (if execution failed)
    pub error: Option<String>,
}

/// Flow execution engine
///
/// Executes business flows defined in the Scenario Studio, handling
/// step sequencing, conditions, and variable extraction.
pub struct FlowExecutor {
    /// Variables available during execution
    variables: HashMap<String, Value>,
}

impl FlowExecutor {
    /// Create a new flow executor
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Create a new flow executor with initial variables
    pub fn with_variables(variables: HashMap<String, Value>) -> Self {
        Self { variables }
    }

    /// Execute a flow definition
    ///
    /// This method executes the flow steps in order, following connections
    /// and evaluating conditions to determine the execution path.
    pub async fn execute(&mut self, flow: &FlowDefinition) -> Result<FlowExecutionResult> {
        let start_time = Utc::now();
        let mut step_results = Vec::new();
        let mut executed_step_ids = std::collections::HashSet::new();
        let mut current_step_ids = self.find_start_steps(flow);

        // Initialize variables from flow
        for (key, value) in &flow.variables {
            self.variables.insert(key.clone(), value.clone());
        }

        // Execute flow until no more steps to execute
        while !current_step_ids.is_empty() {
            let mut next_step_ids = Vec::new();

            for step_id in current_step_ids {
                if executed_step_ids.contains(&step_id) {
                    continue; // Skip already executed steps to prevent infinite loops
                }

                let step = flow
                    .steps
                    .iter()
                    .find(|s| s.id == step_id)
                    .ok_or_else(|| Error::validation(format!("Step {} not found", step_id)))?;

                // Check if step condition is met
                if let Some(ref condition) = step.condition {
                    if !self.evaluate_condition(condition)? {
                        continue; // Skip this step if condition is not met
                    }
                }

                // Execute the step
                let step_result = self.execute_step(step).await?;
                step_results.push(step_result.clone());
                executed_step_ids.insert(step_id.clone());

                // Find next steps based on connections
                let connections = flow.connections.iter().filter(|c| c.from_step_id == step_id);

                for connection in connections {
                    // Check connection condition if present
                    if let Some(ref condition) = connection.condition {
                        if !self.evaluate_condition(condition)? {
                            continue; // Skip this connection if condition is not met
                        }
                    }

                    if !executed_step_ids.contains(&connection.to_step_id) {
                        next_step_ids.push(connection.to_step_id.clone());
                    }
                }
            }

            current_step_ids = next_step_ids;
        }

        let end_time = Utc::now();
        let total_duration_ms = (end_time - start_time).num_milliseconds() as u64;

        let success = step_results.iter().all(|r| r.success);

        // Extract error before moving step_results
        let error = if success {
            None
        } else {
            step_results.iter().find_map(|r| r.error.as_ref()).map(|e| e.clone())
        };

        Ok(FlowExecutionResult {
            flow_id: flow.id.clone(),
            success,
            step_results,
            final_variables: self.variables.clone(),
            total_duration_ms,
            error,
        })
    }

    /// Find the starting steps in a flow (steps with no incoming connections)
    fn find_start_steps(&self, flow: &FlowDefinition) -> Vec<String> {
        let mut has_incoming: std::collections::HashSet<String> =
            flow.connections.iter().map(|c| c.to_step_id.clone()).collect();

        flow.steps
            .iter()
            .filter(|s| !has_incoming.contains(&s.id))
            .map(|s| s.id.clone())
            .collect()
    }

    /// Execute a single flow step
    async fn execute_step(&mut self, step: &FlowStep) -> Result<FlowStepResult> {
        let start_time = Utc::now();
        let mut extracted_variables = HashMap::new();

        // Apply delay if specified
        if let Some(delay_ms) = step.delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        let (success, response, error) = match step.step_type {
            StepType::ApiCall => {
                // TODO: Implement actual API call execution
                // For now, return a placeholder response
                (
                    true,
                    Some(serde_json::json!({"status": "ok", "message": "API call executed"})),
                    None,
                )
            }
            StepType::Condition => {
                // Conditions are evaluated before step execution
                (true, None, None)
            }
            StepType::Delay => {
                // Delay is already applied above
                (true, None, None)
            }
            StepType::Loop => {
                // TODO: Implement loop execution
                (true, None, None)
            }
            StepType::Parallel => {
                // TODO: Implement parallel execution
                (true, None, None)
            }
        };

        // Extract variables from response
        if let Some(ref resp) = response {
            for (key, path) in &step.extract {
                if let Some(value) = self.extract_value(resp, path) {
                    extracted_variables.insert(key.clone(), value.clone());
                    self.variables.insert(key.clone(), value);
                }
            }
        }

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;

        Ok(FlowStepResult {
            step_id: step.id.clone(),
            success,
            response,
            error,
            duration_ms,
            extracted_variables,
        })
    }

    /// Evaluate a flow condition
    fn evaluate_condition(&self, condition: &FlowCondition) -> Result<bool> {
        // TODO: Implement proper condition evaluation
        // For now, return true for all conditions
        // This should evaluate the expression against the current variables
        Ok(true)
    }

    /// Extract a value from a JSON object using a path expression
    fn extract_value(&self, json: &Value, path: &str) -> Option<Value> {
        // Simple path extraction (e.g., "body.id" or "status")
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }
}

impl Default for FlowExecutor {
    fn default() -> Self {
        Self::new()
    }
}
