//! Flow execution engine
//!
//! This module provides the execution engine for business flows defined in the Scenario Studio.

use crate::error::{Error, Result};
use crate::scenario_studio::types::{
    ConditionOperator, FlowCondition, FlowDefinition, FlowStep, StepType,
};
use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

/// Regex for variable substitution (e.g., "{{variable_name}}")
static VARIABLE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{([^}]+)\}\}").expect("Invalid regex pattern"));

/// Result of executing a flow step
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// HTTP client for API calls
    http_client: Client,
}

impl FlowExecutor {
    /// Create a new flow executor
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            http_client: Client::new(),
        }
    }

    /// Create a new flow executor with initial variables
    pub fn with_variables(variables: HashMap<String, Value>) -> Self {
        Self {
            variables,
            http_client: Client::new(),
        }
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

                // Handle special step types
                match step.step_type {
                    StepType::Loop => {
                        // Execute loop: get child steps and iterate
                        let loop_results = self.execute_loop(step, flow).await?;
                        step_results.extend(loop_results);
                        executed_step_ids.insert(step_id.clone());
                    }
                    StepType::Parallel => {
                        // Execute parallel: get child steps and run in parallel
                        let parallel_results = self.execute_parallel(step, flow).await?;
                        step_results.extend(parallel_results);
                        executed_step_ids.insert(step_id.clone());
                    }
                    _ => {
                        // Execute the step normally
                        let step_result = self.execute_step(step).await?;
                        step_results.push(step_result.clone());
                        executed_step_ids.insert(step_id.clone());
                    }
                }

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
            step_results.iter().find_map(|r| r.error.as_ref()).cloned()
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
        let has_incoming: std::collections::HashSet<String> =
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
            StepType::ApiCall => self.execute_api_call(step).await,
            StepType::Condition => {
                // Conditions are evaluated before step execution
                (true, None, None)
            }
            StepType::Delay => {
                // Delay is already applied above
                (true, None, None)
            }
            StepType::Loop => {
                // Loop execution is handled at the flow level, not step level
                // This should not be reached in normal execution
                (false, None, Some("Loop steps must be handled at flow level".to_string()))
            }
            StepType::Parallel => {
                // Parallel execution is handled at the flow level, not step level
                // This should not be reached in normal execution
                (false, None, Some("Parallel steps must be handled at flow level".to_string()))
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

    /// Execute an API call step
    async fn execute_api_call(&self, step: &FlowStep) -> (bool, Option<Value>, Option<String>) {
        // Get method and endpoint
        let method = match step.method.as_ref() {
            Some(m) => m,
            None => {
                return (false, None, Some("API call step missing method".to_string()));
            }
        };

        let endpoint = match step.endpoint.as_ref() {
            Some(e) => e,
            None => {
                return (false, None, Some("API call step missing endpoint".to_string()));
            }
        };

        // Substitute variables in endpoint and body
        let endpoint = self.substitute_variables(endpoint);
        let body = step.body.as_ref().map(|b| self.substitute_variables_in_value(b));

        // Build request
        let method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => {
                return (false, None, Some(format!("Unsupported HTTP method: {}", method)));
            }
        };

        let mut request = self.http_client.request(method, &endpoint);

        // Add headers
        for (key, value) in &step.headers {
            let header_value = self.substitute_variables(value);
            request = request.header(key, &header_value);
        }

        // Add body if present
        if let Some(ref body_value) = body {
            if let Ok(json_body) = serde_json::to_string(body_value) {
                request = request.header("Content-Type", "application/json").body(json_body);
            }
        }

        // Execute request
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();

                // Check expected status if specified
                if let Some(expected) = step.expected_status {
                    if status_code != expected {
                        return (
                            false,
                            Some(serde_json::json!({
                                "status": status_code,
                                "error": format!("Expected status {}, got {}", expected, status_code)
                            })),
                            Some(format!(
                                "Status code mismatch: expected {}, got {}",
                                expected, status_code
                            )),
                        );
                    }
                }

                // Parse response body
                let response_body = match response.text().await {
                    Ok(text) => {
                        // Try to parse as JSON, fallback to string
                        serde_json::from_str(&text).unwrap_or_else(|_| {
                            serde_json::json!({
                                "body": text,
                                "status": status_code
                            })
                        })
                    }
                    Err(e) => {
                        return (false, None, Some(format!("Failed to read response body: {}", e)));
                    }
                };

                // Build full response object
                let full_response = serde_json::json!({
                    "status": status_code,
                    "headers": {}, // Could extract headers if needed
                    "body": response_body
                });

                (true, Some(full_response), None)
            }
            Err(e) => (false, None, Some(format!("API call failed: {}", e))),
        }
    }

    /// Substitute variables in a string (e.g., "{{variable_name}}")
    fn substitute_variables(&self, text: &str) -> String {
        VARIABLE_REGEX
            .replace_all(text, |caps: &regex::Captures| {
                let var_name = caps.get(1).unwrap().as_str().trim();
                self.variables
                    .get(var_name)
                    .map(|v| {
                        // Convert value to string
                        if let Some(s) = v.as_str() {
                            s.to_string()
                        } else {
                            v.to_string()
                        }
                    })
                    .unwrap_or_else(|| format!("{{{{{}}}}}", var_name)) // Keep original if not found
            })
            .to_string()
    }

    /// Substitute variables in a JSON value
    fn substitute_variables_in_value(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.substitute_variables(s)),
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.substitute_variables_in_value(v));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.substitute_variables_in_value(v)).collect())
            }
            _ => value.clone(),
        }
    }

    /// Evaluate a flow condition
    fn evaluate_condition(&self, condition: &FlowCondition) -> Result<bool> {
        // Substitute variables in expression
        let expression = self.substitute_variables(&condition.expression);

        // Get the value to compare (from variables or literal)
        let left_value = if expression.starts_with("{{") && expression.ends_with("}}") {
            // Extract variable name
            let var_name = expression
                .strip_prefix("{{")
                .and_then(|s| s.strip_suffix("}}"))
                .map(|s| s.trim());
            var_name
                .and_then(|name| self.variables.get(name))
                .cloned()
                .unwrap_or(Value::Null)
        } else {
            // Try to parse as JSON value
            serde_json::from_str(&expression).unwrap_or(Value::String(expression))
        };

        let right_value = &condition.value;

        // Apply operator
        let result = match condition.operator {
            ConditionOperator::Eq => left_value == *right_value,
            ConditionOperator::Ne => left_value != *right_value,
            ConditionOperator::Gt => self.compare_values(&left_value, right_value, |a, b| a > b),
            ConditionOperator::Gte => self.compare_values(&left_value, right_value, |a, b| a >= b),
            ConditionOperator::Lt => self.compare_values(&left_value, right_value, |a, b| a < b),
            ConditionOperator::Lte => self.compare_values(&left_value, right_value, |a, b| a <= b),
            ConditionOperator::Contains => {
                if let (Some(left_str), Some(right_str)) =
                    (left_value.as_str(), right_value.as_str())
                {
                    left_str.contains(right_str)
                } else {
                    false
                }
            }
            ConditionOperator::NotContains => {
                if let (Some(left_str), Some(right_str)) =
                    (left_value.as_str(), right_value.as_str())
                {
                    !left_str.contains(right_str)
                } else {
                    true
                }
            }
            ConditionOperator::Matches => {
                if let (Some(left_str), Some(right_str)) =
                    (left_value.as_str(), right_value.as_str())
                {
                    Regex::new(right_str).map(|re| re.is_match(left_str)).unwrap_or(false)
                } else {
                    false
                }
            }
            ConditionOperator::Exists => left_value != Value::Null,
        };

        Ok(result)
    }

    /// Compare two values numerically
    fn compare_values<F>(&self, left: &Value, right: &Value, cmp: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left.as_f64(), right.as_f64()) {
            (Some(l), Some(r)) => cmp(l, r),
            _ => false,
        }
    }

    /// Execute a loop step
    ///
    /// Loops execute their child steps multiple times based on loop configuration
    /// stored in step metadata (e.g., "loop_count" or "loop_condition").
    async fn execute_loop(
        &mut self,
        loop_step: &FlowStep,
        flow: &FlowDefinition,
    ) -> Result<Vec<FlowStepResult>> {
        let mut all_results = Vec::new();

        // Get loop configuration from metadata
        let loop_count = loop_step.metadata.get("loop_count").and_then(|v| v.as_u64()).unwrap_or(1);

        let loop_condition = loop_step.metadata.get("loop_condition");

        // Find child steps (steps connected from this loop step)
        let child_step_ids: Vec<String> = flow
            .connections
            .iter()
            .filter(|c| c.from_step_id == loop_step.id)
            .map(|c| c.to_step_id.clone())
            .collect();

        if child_step_ids.is_empty() {
            return Ok(all_results);
        }

        // Execute loop iterations
        for iteration in 0..loop_count {
            // Set loop iteration variable
            self.variables
                .insert("loop_iteration".to_string(), serde_json::json!(iteration));
            self.variables.insert("loop_index".to_string(), serde_json::json!(iteration));

            // Check loop condition if specified
            if let Some(condition_value) = loop_condition {
                if let Some(condition_str) = condition_value.as_str() {
                    // Evaluate condition (simplified - could be enhanced)
                    let condition_result = self
                        .evaluate_condition(&FlowCondition {
                            expression: condition_str.to_string(),
                            operator: ConditionOperator::Eq,
                            value: Value::Bool(true),
                        })
                        .unwrap_or(false);

                    if !condition_result {
                        break; // Exit loop if condition fails
                    }
                }
            }

            // Execute child steps for this iteration
            for child_step_id in &child_step_ids {
                if let Some(child_step) = flow.steps.iter().find(|s| s.id == *child_step_id) {
                    // Check if step condition is met
                    if let Some(ref condition) = child_step.condition {
                        if !self.evaluate_condition(condition)? {
                            continue;
                        }
                    }

                    let step_result = self.execute_step(child_step).await?;
                    all_results.push(step_result);
                }
            }
        }

        Ok(all_results)
    }

    /// Execute a parallel step
    ///
    /// Parallel steps execute their child steps concurrently.
    async fn execute_parallel(
        &mut self,
        parallel_step: &FlowStep,
        flow: &FlowDefinition,
    ) -> Result<Vec<FlowStepResult>> {
        // Find child steps (steps connected from this parallel step)
        let child_step_ids: Vec<String> = flow
            .connections
            .iter()
            .filter(|c| c.from_step_id == parallel_step.id)
            .map(|c| c.to_step_id.clone())
            .collect();

        if child_step_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Collect child steps
        let child_steps: Vec<&FlowStep> = child_step_ids
            .iter()
            .filter_map(|step_id| flow.steps.iter().find(|s| s.id == *step_id))
            .collect();

        // Execute all child steps in parallel using tokio::spawn
        // Note: We need to clone variables for each parallel execution
        let mut tasks = Vec::new();

        for child_step in child_steps {
            // Clone variables for this parallel branch
            let variables_clone = self.variables.clone();
            let step_clone = child_step.clone();
            let http_client = self.http_client.clone();

            // Create a task for this parallel step
            let task = tokio::spawn(async move {
                // Create a temporary executor for this parallel branch
                let mut branch_executor = FlowExecutor {
                    variables: variables_clone,
                    http_client,
                };

                // Check condition if present
                if let Some(ref condition) = step_clone.condition {
                    match branch_executor.evaluate_condition(condition) {
                        Ok(true) => {}
                        Ok(false) => {
                            return FlowStepResult {
                                step_id: step_clone.id.clone(),
                                success: false,
                                response: None,
                                error: Some("Condition not met".to_string()),
                                duration_ms: 0,
                                extracted_variables: HashMap::new(),
                            };
                        }
                        Err(e) => {
                            return FlowStepResult {
                                step_id: step_clone.id.clone(),
                                success: false,
                                response: None,
                                error: Some(format!("Condition evaluation error: {}", e)),
                                duration_ms: 0,
                                extracted_variables: HashMap::new(),
                            };
                        }
                    }
                }

                // Execute the step
                branch_executor
                    .execute_step(&step_clone)
                    .await
                    .unwrap_or_else(|e| FlowStepResult {
                        step_id: step_clone.id.clone(),
                        success: false,
                        response: None,
                        error: Some(format!("Execution error: {}", e)),
                        duration_ms: 0,
                        extracted_variables: HashMap::new(),
                    })
            });

            tasks.push(task);
        }

        // Wait for all parallel tasks to complete
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    results.push(FlowStepResult {
                        step_id: "unknown".to_string(),
                        success: false,
                        response: None,
                        error: Some(format!("Parallel task error: {}", e)),
                        duration_ms: 0,
                        extracted_variables: HashMap::new(),
                    });
                }
            }
        }

        // Merge variables from all parallel branches
        // (Last write wins for conflicts)
        for result in &results {
            for (key, value) in &result.extracted_variables {
                self.variables.insert(key.clone(), value.clone());
            }
        }

        Ok(results)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_flow_step_result_creation() {
        let mut extracted = HashMap::new();
        extracted.insert("user_id".to_string(), json!("123"));

        let result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"status": "ok"})),
            error: None,
            duration_ms: 150,
            extracted_variables: extracted.clone(),
        };

        assert_eq!(result.step_id, "step-1");
        assert!(result.success);
        assert!(result.response.is_some());
        assert!(result.error.is_none());
        assert_eq!(result.duration_ms, 150);
        assert_eq!(result.extracted_variables.len(), 1);
    }

    #[test]
    fn test_flow_step_result_with_error() {
        let result = FlowStepResult {
            step_id: "step-2".to_string(),
            success: false,
            response: None,
            error: Some("Request failed".to_string()),
            duration_ms: 50,
            extracted_variables: HashMap::new(),
        };

        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Request failed");
    }

    #[test]
    fn test_flow_execution_result_creation() {
        let step_result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: None,
            error: None,
            duration_ms: 100,
            extracted_variables: HashMap::new(),
        };

        let mut final_vars = HashMap::new();
        final_vars.insert("result".to_string(), json!("success"));

        let result = FlowExecutionResult {
            flow_id: "flow-123".to_string(),
            success: true,
            step_results: vec![step_result],
            final_variables: final_vars.clone(),
            total_duration_ms: 200,
            error: None,
        };

        assert_eq!(result.flow_id, "flow-123");
        assert!(result.success);
        assert_eq!(result.step_results.len(), 1);
        assert_eq!(result.final_variables.len(), 1);
        assert_eq!(result.total_duration_ms, 200);
    }

    #[test]
    fn test_flow_execution_result_with_error() {
        let step_result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: false,
            response: None,
            error: Some("Step failed".to_string()),
            duration_ms: 50,
            extracted_variables: HashMap::new(),
        };

        let result = FlowExecutionResult {
            flow_id: "flow-456".to_string(),
            success: false,
            step_results: vec![step_result],
            final_variables: HashMap::new(),
            total_duration_ms: 100,
            error: Some("Flow execution failed".to_string()),
        };

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_flow_executor_new() {
        let executor = FlowExecutor::new();
        // Just verify it can be created
        let _ = executor;
    }

    #[test]
    fn test_flow_executor_default() {
        let executor = FlowExecutor::default();
        // Just verify it can be created
        let _ = executor;
    }

    #[test]
    fn test_flow_executor_with_variables() {
        let mut variables = HashMap::new();
        variables.insert("api_key".to_string(), json!("secret123"));
        variables.insert("base_url".to_string(), json!("https://api.example.com"));

        let executor = FlowExecutor::with_variables(variables);
        // Just verify it can be created
        let _ = executor;
    }

    #[test]
    fn test_flow_step_result_clone() {
        let result1 = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"status": "ok"})),
            error: None,
            duration_ms: 100,
            extracted_variables: HashMap::new(),
        };
        let result2 = result1.clone();
        assert_eq!(result1.step_id, result2.step_id);
        assert_eq!(result1.success, result2.success);
    }

    #[test]
    fn test_flow_step_result_debug() {
        let result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: None,
            error: None,
            duration_ms: 150,
            extracted_variables: HashMap::new(),
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("FlowStepResult"));
    }

    #[test]
    fn test_flow_step_result_serialization() {
        let result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"data": "test"})),
            error: None,
            duration_ms: 200,
            extracted_variables: HashMap::from([("var1".to_string(), json!("value1"))]),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("step-1"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_flow_execution_result_clone() {
        let result1 = FlowExecutionResult {
            flow_id: "flow-1".to_string(),
            success: true,
            step_results: vec![],
            final_variables: HashMap::new(),
            total_duration_ms: 100,
            error: None,
        };
        let result2 = result1.clone();
        assert_eq!(result1.flow_id, result2.flow_id);
        assert_eq!(result1.success, result2.success);
    }

    #[test]
    fn test_flow_execution_result_debug() {
        let result = FlowExecutionResult {
            flow_id: "flow-123".to_string(),
            success: false,
            step_results: vec![],
            final_variables: HashMap::new(),
            total_duration_ms: 50,
            error: Some("Error".to_string()),
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("FlowExecutionResult"));
    }

    #[test]
    fn test_flow_execution_result_serialization() {
        let step_result = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"id": 1})),
            error: None,
            duration_ms: 100,
            extracted_variables: HashMap::new(),
        };
        let result = FlowExecutionResult {
            flow_id: "flow-456".to_string(),
            success: true,
            step_results: vec![step_result],
            final_variables: HashMap::from([("result".to_string(), json!("success"))]),
            total_duration_ms: 200,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("flow-456"));
        assert!(json.contains("step-1"));
    }

    #[test]
    fn test_flow_step_result_with_all_fields() {
        let mut extracted = HashMap::new();
        extracted.insert("user_id".to_string(), json!("123"));
        extracted.insert("token".to_string(), json!("abc123"));
        extracted.insert("expires_at".to_string(), json!("2024-01-01"));

        let result = FlowStepResult {
            step_id: "step-auth".to_string(),
            success: true,
            response: Some(json!({
                "user": {"id": 123, "name": "Alice"},
                "token": "abc123",
                "expires_at": "2024-01-01"
            })),
            error: None,
            duration_ms: 250,
            extracted_variables: extracted.clone(),
        };

        assert_eq!(result.extracted_variables.len(), 3);
        assert!(result.response.is_some());
        assert_eq!(result.duration_ms, 250);
    }

    #[test]
    fn test_flow_execution_result_with_multiple_steps() {
        let step1 = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"id": 1})),
            error: None,
            duration_ms: 100,
            extracted_variables: HashMap::from([("id".to_string(), json!(1))]),
        };
        let step2 = FlowStepResult {
            step_id: "step-2".to_string(),
            success: true,
            response: Some(json!({"status": "ok"})),
            error: None,
            duration_ms: 150,
            extracted_variables: HashMap::new(),
        };
        let step3 = FlowStepResult {
            step_id: "step-3".to_string(),
            success: true,
            response: None,
            error: None,
            duration_ms: 50,
            extracted_variables: HashMap::new(),
        };

        let result = FlowExecutionResult {
            flow_id: "flow-multi".to_string(),
            success: true,
            step_results: vec![step1, step2, step3],
            final_variables: HashMap::from([
                ("id".to_string(), json!(1)),
                ("status".to_string(), json!("ok")),
            ]),
            total_duration_ms: 300,
            error: None,
        };

        assert_eq!(result.step_results.len(), 3);
        assert_eq!(result.final_variables.len(), 2);
        assert_eq!(result.total_duration_ms, 300);
    }

    #[test]
    fn test_flow_step_result_with_extracted_variables() {
        let mut extracted = HashMap::new();
        extracted.insert("order_id".to_string(), json!("order-123"));
        extracted.insert("total".to_string(), json!(99.99));
        extracted.insert("currency".to_string(), json!("USD"));

        let result = FlowStepResult {
            step_id: "step-checkout".to_string(),
            success: true,
            response: Some(json!({
                "order": {"id": "order-123", "total": 99.99, "currency": "USD"}
            })),
            error: None,
            duration_ms: 300,
            extracted_variables: extracted.clone(),
        };

        assert_eq!(result.extracted_variables.len(), 3);
        assert_eq!(result.extracted_variables.get("order_id"), Some(&json!("order-123")));
    }

    #[test]
    fn test_flow_execution_result_with_error_and_steps() {
        let step1 = FlowStepResult {
            step_id: "step-1".to_string(),
            success: true,
            response: Some(json!({"id": 1})),
            error: None,
            duration_ms: 100,
            extracted_variables: HashMap::new(),
        };
        let step2 = FlowStepResult {
            step_id: "step-2".to_string(),
            success: false,
            response: None,
            error: Some("Connection timeout".to_string()),
            duration_ms: 5000,
            extracted_variables: HashMap::new(),
        };

        let result = FlowExecutionResult {
            flow_id: "flow-error".to_string(),
            success: false,
            step_results: vec![step1, step2],
            final_variables: HashMap::new(),
            total_duration_ms: 5100,
            error: Some("Flow failed at step-2: Connection timeout".to_string()),
        };

        assert!(!result.success);
        assert_eq!(result.step_results.len(), 2);
        assert!(result.error.is_some());
        assert_eq!(result.total_duration_ms, 5100);
    }
}
