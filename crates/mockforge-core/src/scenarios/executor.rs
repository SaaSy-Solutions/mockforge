//! Scenario executor that converts scenarios to chains and executes them

use crate::chain_execution::{ChainExecutionEngine, ChainExecutionResult, ChainExecutionStatus};
use crate::request_chaining::{
    ChainConfig, ChainDefinition, ChainLink, ChainRequest, RequestChainRegistry, RequestBody,
};
use crate::scenarios::registry::ScenarioRegistry;
use crate::scenarios::types::{ScenarioDefinition, ScenarioResult, ScenarioStep, StepResult};
use crate::{Error, Result};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Executor for running scenario definitions
#[derive(Debug, Clone)]
pub struct ScenarioExecutor {
    /// Scenario registry
    registry: Arc<ScenarioRegistry>,
    /// HTTP client for making requests
    http_client: Client,
    /// Base URL for API requests
    base_url: String,
}

impl ScenarioExecutor {
    /// Create a new scenario executor
    pub fn new(registry: Arc<ScenarioRegistry>, base_url: impl Into<String>) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::generic(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            registry,
            http_client,
            base_url: base_url.into(),
        })
    }

    /// Execute a scenario by ID
    pub async fn execute_scenario(
        &self,
        scenario_id: &str,
        parameters: Option<HashMap<String, Value>>,
    ) -> Result<ScenarioResult> {
        let scenario = self
            .registry
            .get(scenario_id)
            .await
            .ok_or_else(|| Error::generic(format!("Scenario not found: {}", scenario_id)))?;

        self.execute_scenario_definition(&scenario, parameters).await
    }

    /// Execute a scenario definition directly
    pub async fn execute_scenario_definition(
        &self,
        scenario: &ScenarioDefinition,
        parameters: Option<HashMap<String, Value>>,
    ) -> Result<ScenarioResult> {
        let start_time = Instant::now();
        let mut step_results = Vec::new();
        let mut state = scenario.variables.clone();

        // Merge parameters into state
        if let Some(params) = parameters {
            for (key, value) in params {
                state.insert(key, value);
            }
        }

        // Execute steps in order (respecting dependencies)
        let mut executed_steps = std::collections::HashSet::new();
        let mut remaining_steps: Vec<&ScenarioStep> = scenario.steps.iter().collect();

        while !remaining_steps.is_empty() {
            let mut progress_made = false;

            for step in remaining_steps.iter() {
                // Check if dependencies are satisfied
                let deps_satisfied = step
                    .depends_on
                    .iter()
                    .all(|dep_id| executed_steps.contains(dep_id));

                if !deps_satisfied {
                    continue;
                }

                // Execute step
                let step_result = self.execute_step(step, &state).await;
                let success = step_result.success;

                // Update state with extracted variables
                for (var_name, var_value) in &step_result.extracted_variables {
                    state.insert(var_name.clone(), var_value.clone());
                }

                step_results.push(step_result);
                executed_steps.insert(step.id.clone());
                progress_made = true;

                // If step failed and we shouldn't continue, break
                if !success && !step.continue_on_failure {
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(ScenarioResult {
                        scenario_id: scenario.id.clone(),
                        success: false,
                        step_results,
                        duration_ms,
                        error: Some(format!("Step '{}' failed", step.id)),
                        final_state: state,
                    });
                }
            }

            // Remove executed steps
            remaining_steps.retain(|step| !executed_steps.contains(&step.id));

            if !progress_made && !remaining_steps.is_empty() {
                // Circular dependency or unsatisfiable dependencies
                let duration_ms = start_time.elapsed().as_millis() as u64;
                return Ok(ScenarioResult {
                    scenario_id: scenario.id.clone(),
                    success: false,
                    step_results,
                    duration_ms,
                    error: Some("Circular or unsatisfiable dependencies detected".to_string()),
                    final_state: state,
                });
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let all_successful = step_results.iter().all(|r| r.success);

        Ok(ScenarioResult {
            scenario_id: scenario.id.clone(),
            success: all_successful,
            step_results,
            duration_ms,
            error: if all_successful {
                None
            } else {
                Some("One or more steps failed".to_string())
            },
            final_state: state,
        })
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &ScenarioStep,
        state: &HashMap<String, Value>,
    ) -> StepResult {
        let step_start = Instant::now();

        // Apply delay if specified
        if let Some(delay_ms) = step.delay_ms {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }

        // Build URL with path parameters
        let mut url = format!("{}{}", self.base_url, step.path);
        for (param, value) in &step.path_params {
            // Simple template substitution (in production, use proper templating)
            let value_str = if let Some(state_value) = state.get(value) {
                state_value.as_str().unwrap_or(value).to_string()
            } else {
                value.clone()
            };
            url = url.replace(&format!("{{{}}}", param), &value_str);
        }

        // Build query string
        let mut query_parts = Vec::new();
        for (key, value) in &step.query_params {
            let value_str = if let Some(state_value) = state.get(value) {
                state_value.as_str().unwrap_or(value).to_string()
            } else {
                value.clone()
            };
            query_parts.push(format!("{}={}", key, urlencoding::encode(&value_str)));
        }
        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }

        // Prepare request body (apply template substitution)
        let body = step.body.as_ref().map(|b| {
            // Simple JSON template substitution
            // In production, use proper templating engine
            let body_str = serde_json::to_string(b).unwrap_or_default();
            let mut body_value = serde_json::from_str::<Value>(&body_str).unwrap_or(b.clone());
            Self::substitute_templates(&mut body_value, state);
            body_value
        });

        // Build request
        let mut request = match step.method.as_str() {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "PATCH" => self.http_client.patch(&url),
            "DELETE" => self.http_client.delete(&url),
            _ => {
                return StepResult {
                    step_id: step.id.clone(),
                    success: false,
                    status_code: None,
                    response_body: None,
                    extracted_variables: HashMap::new(),
                    error: Some(format!("Unsupported HTTP method: {}", step.method)),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                };
            }
        };

        // Add headers
        for (key, value) in &step.headers {
            request = request.header(key, value);
        }

        // Add body
        if let Some(body_value) = body {
            request = request.json(&body_value);
        }

        // Execute request
        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let response_body: Option<Value> = response.json().await.ok();

                // Check expected status
                let success = step
                    .expected_status
                    .map(|expected| status == expected)
                    .unwrap_or(status >= 200 && status < 300);

                // Extract variables from response
                let mut extracted = HashMap::new();
                if let Some(ref body) = response_body {
                    for (var_name, json_path) in &step.extract {
                        if let Some(value) = Self::extract_json_path(body, json_path) {
                            extracted.insert(var_name.clone(), value);
                        }
                    }
                }

                StepResult {
                    step_id: step.id.clone(),
                    success,
                    status_code: Some(status),
                    response_body,
                    extracted_variables: extracted,
                    error: if success {
                        None
                    } else {
                        Some(format!(
                            "Expected status {}, got {}",
                            step.expected_status.unwrap_or(200),
                            status
                        ))
                    },
                    duration_ms: step_start.elapsed().as_millis() as u64,
                }
            }
            Err(e) => StepResult {
                step_id: step.id.clone(),
                success: false,
                status_code: None,
                response_body: None,
                extracted_variables: HashMap::new(),
                error: Some(format!("Request failed: {}", e)),
                duration_ms: step_start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Substitute template variables in a JSON value
    fn substitute_templates(value: &mut Value, state: &HashMap<String, Value>) {
        match value {
            Value::String(s) => {
                // Simple template substitution: {{variable_name}}
                if s.starts_with("{{") && s.ends_with("}}") {
                    let var_name = s.trim_start_matches("{{").trim_end_matches("}}").trim();
                    if let Some(var_value) = state.get(var_name) {
                        *value = var_value.clone();
                    }
                }
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    Self::substitute_templates(v, state);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    Self::substitute_templates(v, state);
                }
            }
            _ => {}
        }
    }

    /// Extract a value from JSON using a simple path (e.g., "body.user.id")
    fn extract_json_path(value: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

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
