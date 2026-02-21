//! Flow-to-scenario compiler
//!
//! This module converts recorded flows into behavioral scenarios that can be replayed.

use super::{
    flow_recorder::Flow,
    scenario_types::{BehavioralScenario, BehavioralScenarioStep, StateVariable},
};
use crate::database::RecorderDatabase;
use crate::models::RecordedResponse;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Compiler that converts flows to behavioral scenarios
pub struct FlowCompiler {
    db: RecorderDatabase,
}

impl FlowCompiler {
    /// Create a new flow compiler
    pub fn new(db: RecorderDatabase) -> Self {
        Self { db }
    }

    /// Compile a flow into a behavioral scenario
    pub async fn compile_flow(
        &self,
        flow: &Flow,
        scenario_name: String,
        strict_mode: bool,
    ) -> Result<BehavioralScenario> {
        let scenario_id = uuid::Uuid::new_v4().to_string();
        let mut scenario =
            BehavioralScenario::new(&scenario_id, &scenario_name).with_strict_mode(strict_mode);
        if let Some(ref desc) = flow.description {
            scenario = scenario.with_description(desc.clone());
        }

        // Fetch all request/response pairs for the flow steps
        let mut steps = Vec::new();
        let mut state_variables = HashMap::new();

        for (idx, flow_step) in flow.steps.iter().enumerate() {
            // Fetch request and response from database
            let request = self
                .db
                .get_request(&flow_step.request_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get request: {}", e))?
                .ok_or_else(|| anyhow::anyhow!("Request not found: {}", flow_step.request_id))?;

            let response = self
                .db
                .get_response(&flow_step.request_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get response: {}", e))?
                .ok_or_else(|| anyhow::anyhow!("Response not found: {}", flow_step.request_id))?;

            // Create scenario step
            let step_id = format!("step_{}", idx);
            let mut scenario_step = BehavioralScenarioStep::new(step_id.clone(), request, response);

            // Apply step label (use flow step label or generate heuristically)
            let label = flow_step.step_label.clone().or_else(|| {
                Self::heuristic_step_label(
                    &scenario_step.request.method,
                    &scenario_step.request.path,
                )
            });
            if let Some(label) = label {
                scenario_step = scenario_step.with_label(label);
            }

            // Apply timing
            if let Some(timing) = flow_step.timing_ms {
                scenario_step = scenario_step.with_timing(timing);
            }

            // Extract state variables from response
            if let Some(extracted_vars) =
                Self::extract_state_variables(&scenario_step.response, &step_id)
            {
                for (var_name, (json_path, _var_value)) in extracted_vars {
                    // Add extraction to step
                    scenario_step = scenario_step.add_extract(var_name.clone(), json_path.clone());

                    // Add state variable to scenario
                    let state_var =
                        StateVariable::new(var_name.clone(), json_path.clone(), step_id.clone());
                    state_variables.insert(var_name, state_var);
                }
            }

            // Add dependencies (steps that this step depends on)
            // For now, each step depends on the previous step
            if idx > 0 {
                scenario_step = scenario_step.add_dependency(format!("step_{}", idx - 1));
            }

            steps.push(scenario_step);
        }

        // Add all steps to scenario
        for step in steps {
            scenario = scenario.add_step(step);
        }

        // Add all state variables to scenario
        for (_, state_var) in state_variables {
            scenario = scenario.add_state_variable(state_var);
        }

        Ok(scenario)
    }

    /// Extract state variables from a response heuristically
    ///
    /// Looks for common patterns like id, user_id, cart_id, order_id, etc.
    fn extract_state_variables(
        response: &RecordedResponse,
        _step_id: &str,
    ) -> Option<HashMap<String, (String, Value)>> {
        // Try to parse response body as JSON
        let body_str = response.body.as_ref()?;
        let json: Value = serde_json::from_str(body_str).ok()?;

        let mut variables = HashMap::new();

        // Common state variable patterns to look for
        let patterns = vec![
            ("id", vec!["id", "ID", "Id"]),
            ("user_id", vec!["user_id", "userId", "user.id", "user.ID"]),
            ("cart_id", vec!["cart_id", "cartId", "cart.id"]),
            ("order_id", vec!["order_id", "orderId", "order.id"]),
            ("session_id", vec!["session_id", "sessionId", "session.id"]),
            ("token", vec!["token", "access_token", "accessToken"]),
        ];

        for (var_name, paths) in patterns {
            for path in paths {
                if let Some(value) = Self::extract_json_path(&json, path) {
                    // Only extract if it's a string or number (not an object/array)
                    if matches!(value, Value::String(_) | Value::Number(_)) {
                        variables.insert(var_name.to_string(), (path.to_string(), value));
                        break; // Found this variable, move to next
                    }
                }
            }
        }

        if variables.is_empty() {
            None
        } else {
            Some(variables)
        }
    }

    /// Extract value from JSON using simple path (supports dot notation)
    fn extract_json_path(json: &Value, path: &str) -> Option<Value> {
        let path = path.trim_start_matches('$').trim_start_matches('.');
        let parts: Vec<&str> = path.split('.').collect();

        let mut current = json;
        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    let idx: usize = part.parse().ok()?;
                    current = arr.get(idx)?;
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }

    /// Generate a heuristic step label from method and path
    fn heuristic_step_label(method: &str, path: &str) -> Option<String> {
        let path_lower = path.to_lowercase();

        // Check for common patterns
        if path_lower.contains("/login") || path_lower.contains("/auth") {
            return Some("login".to_string());
        }
        if path_lower.contains("/logout") {
            return Some("logout".to_string());
        }
        if path_lower.contains("/checkout") {
            return Some("checkout".to_string());
        }
        if path_lower.contains("/payment") {
            return Some("payment".to_string());
        }
        if path_lower.contains("/cart") {
            return Some("cart".to_string());
        }
        if path_lower.contains("/order") {
            return Some("order".to_string());
        }
        if path_lower.contains("/user") && method == "GET" {
            return Some("get_user".to_string());
        }
        if path_lower.contains("/list") || (method == "GET" && !path.contains('{')) {
            return Some("list".to_string());
        }
        if path_lower.contains("/detail") || (method == "GET" && path.contains('{')) {
            return Some("detail".to_string());
        }
        if method == "POST" {
            return Some("create".to_string());
        }
        if method == "PUT" || method == "PATCH" {
            return Some("update".to_string());
        }
        if method == "DELETE" {
            return Some("delete".to_string());
        }

        None
    }
}
