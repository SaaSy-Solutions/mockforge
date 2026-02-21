//! Behavioral scenario replay engine
//!
//! This module provides deterministic replay of behavioral scenarios with
//! state coherence tracking and strict/flex mode support.

use crate::behavioral_cloning::{BehavioralScenario, BehavioralScenarioStep};
use anyhow::Result;
use axum::http::{HeaderMap, Method, Uri};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Active scenario instance state
#[derive(Debug, Clone)]
struct ScenarioInstance {
    /// Scenario being replayed
    scenario_id: String,
    /// Current step index
    current_step: usize,
    /// State variables extracted so far
    state: HashMap<String, Value>,
    /// When this instance was created
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Replay engine for behavioral scenarios
pub struct BehavioralScenarioReplayEngine {
    /// Active scenario instances (session_id -> instance)
    active_instances: Arc<RwLock<HashMap<String, ScenarioInstance>>>,
    /// Active scenarios by ID
    active_scenarios: Arc<RwLock<HashMap<String, BehavioralScenario>>>,
}

impl Default for BehavioralScenarioReplayEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BehavioralScenarioReplayEngine {
    /// Create a new replay engine
    pub fn new() -> Self {
        Self {
            active_instances: Arc::new(RwLock::new(HashMap::new())),
            active_scenarios: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Activate a scenario for replay
    pub async fn activate_scenario(&self, scenario: BehavioralScenario) -> Result<()> {
        let mut scenarios = self.active_scenarios.write().await;
        scenarios.insert(scenario.id.clone(), scenario);
        debug!("Activated scenario for replay");
        Ok(())
    }

    /// Deactivate a scenario
    pub async fn deactivate_scenario(&self, scenario_id: &str) -> Result<()> {
        let mut scenarios = self.active_scenarios.write().await;
        scenarios.remove(scenario_id);
        debug!("Deactivated scenario: {}", scenario_id);
        Ok(())
    }

    /// Try to match a request to a scenario step and return the response
    ///
    /// Returns Some(response) if a match is found, None otherwise
    pub async fn try_replay(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
        session_id: Option<&str>,
    ) -> Result<Option<ReplayResponse>> {
        let scenarios = self.active_scenarios.read().await;

        // Try to find matching scenario
        for scenario in scenarios.values() {
            // Check if we have an active instance for this session
            let instance_key = session_id.unwrap_or("default");
            let mut instances = self.active_instances.write().await;

            // Get or create instance
            let instance = if let Some(inst) = instances.get_mut(instance_key) {
                // Check if this instance matches the scenario
                if inst.scenario_id == scenario.id {
                    inst.clone()
                } else {
                    // Different scenario, create new instance
                    ScenarioInstance {
                        scenario_id: scenario.id.clone(),
                        current_step: 0,
                        state: HashMap::new(),
                        created_at: chrono::Utc::now(),
                    }
                }
            } else {
                // No instance, check if request matches first step
                if let Some(step) = scenario.steps.first() {
                    if Self::matches_step(step, method, uri, headers, body, scenario.strict_mode)? {
                        ScenarioInstance {
                            scenario_id: scenario.id.clone(),
                            current_step: 0,
                            state: HashMap::new(),
                            created_at: chrono::Utc::now(),
                        }
                    } else {
                        continue; // Doesn't match this scenario
                    }
                } else {
                    continue; // Empty scenario
                }
            };

            // Check if current step matches
            if instance.current_step < scenario.steps.len() {
                let step = &scenario.steps[instance.current_step];
                if Self::matches_step(step, method, uri, headers, body, scenario.strict_mode)? {
                    // Match! Return the response
                    let response = Self::build_response(step, &instance.state)?;

                    // Extract state variables from response
                    let mut new_state = instance.state.clone();
                    Self::extract_state_variables(&response.body, &step.extracts, &mut new_state);

                    // Update instance
                    instances.insert(
                        instance_key.to_string(),
                        ScenarioInstance {
                            scenario_id: scenario.id.clone(),
                            current_step: instance.current_step + 1,
                            state: new_state,
                            created_at: instance.created_at,
                        },
                    );

                    debug!("Replayed step {} of scenario {}", instance.current_step, scenario.id);

                    return Ok(Some(response));
                }
            }

            // In flex mode, try to find any matching step
            if !scenario.strict_mode {
                for (idx, step) in scenario.steps.iter().enumerate() {
                    if Self::matches_step(step, method, uri, headers, body, false)? {
                        let response = Self::build_response(step, &instance.state)?;

                        // Extract state variables
                        let mut new_state = instance.state.clone();
                        Self::extract_state_variables(
                            &response.body,
                            &step.extracts,
                            &mut new_state,
                        );

                        // Update instance
                        instances.insert(
                            instance_key.to_string(),
                            ScenarioInstance {
                                scenario_id: scenario.id.clone(),
                                current_step: idx + 1,
                                state: new_state,
                                created_at: instance.created_at,
                            },
                        );

                        return Ok(Some(response));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if a request matches a scenario step
    fn matches_step(
        step: &BehavioralScenarioStep,
        method: &Method,
        uri: &Uri,
        _headers: &HeaderMap,
        _body: Option<&[u8]>,
        strict: bool,
    ) -> Result<bool> {
        // Check method
        if step.request.method.to_uppercase() != method.as_str().to_uppercase() {
            return Ok(false);
        }

        // Check path
        let request_path = uri.path();
        let step_path = &step.request.path;

        if strict {
            // Strict mode: exact match or path parameter match
            Ok(Self::paths_match_strict(step_path, request_path))
        } else {
            // Flex mode: allow minor variations
            Ok(Self::paths_match_flex(step_path, request_path))
        }
    }

    /// Check if paths match in strict mode
    ///
    /// Supports path parameters like /users/{id} matching /users/123
    fn paths_match_strict(pattern: &str, actual: &str) -> bool {
        // Exact match
        if pattern == actual {
            return true;
        }

        // Check for path parameters: {param_name}
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let actual_parts: Vec<&str> = actual.split('/').collect();

        if pattern_parts.len() != actual_parts.len() {
            return false;
        }

        // Compare each segment
        for (pattern_part, actual_part) in pattern_parts.iter().zip(actual_parts.iter()) {
            if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
                // This is a path parameter, accept any non-empty value
                if actual_part.is_empty() {
                    return false;
                }
            } else if pattern_part != actual_part {
                // Not a parameter and doesn't match
                return false;
            }
        }

        true
    }

    /// Check if paths match in flex mode (allows minor variations)
    ///
    /// In flex mode, we:
    /// - Allow different IDs in path parameters
    /// - Compare path structure (number of segments)
    /// - Use edit distance for minor variations
    fn paths_match_flex(pattern: &str, actual: &str) -> bool {
        // Exact match
        if pattern == actual {
            return true;
        }

        // Check for path parameters
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let actual_parts: Vec<&str> = actual.split('/').collect();

        // Must have same number of segments
        if pattern_parts.len() != actual_parts.len() {
            // Try prefix/suffix matching for minor variations
            return actual.contains(pattern) || pattern.contains(actual);
        }

        // Compare structure - allow different IDs but same structure
        let mut matches = 0;
        let mut total = 0;

        for (pattern_part, actual_part) in pattern_parts.iter().zip(actual_parts.iter()) {
            total += 1;
            if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
                // Path parameter - accept any non-empty value
                if !actual_part.is_empty() {
                    matches += 1;
                }
            } else if pattern_part == actual_part {
                // Exact match
                matches += 1;
            } else {
                // Check if similar (edit distance <= 2 for minor typos)
                if Self::edit_distance(pattern_part, actual_part) <= 2 {
                    matches += 1;
                }
            }
        }

        // Require at least 80% match
        (matches as f64 / total as f64) >= 0.8
    }

    /// Calculate edit distance (Levenshtein distance) between two strings
    fn edit_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let n = s1_chars.len();
        let m = s2_chars.len();

        if n == 0 {
            return m;
        }
        if m == 0 {
            return n;
        }

        let mut dp = vec![vec![0; m + 1]; n + 1];

        // Initialize first row and column
        for i in 0..=n {
            dp[i][0] = i;
        }
        for j in 0..=m {
            dp[0][j] = j;
        }

        // Fill the dp table
        for i in 1..=n {
            for j in 1..=m {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[n][m]
    }

    /// Build response from scenario step with state variable substitution
    fn build_response(
        step: &BehavioralScenarioStep,
        state: &HashMap<String, Value>,
    ) -> Result<ReplayResponse> {
        let mut body = step.response.body.clone().unwrap_or_default();

        // Substitute state variables in response body
        if let Ok(mut json) = serde_json::from_str::<Value>(&body) {
            Self::substitute_state_variables(&mut json, state);
            body = serde_json::to_string(&json)
                .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;
        } else {
            // Not JSON, do simple string substitution
            for (var_name, var_value) in state {
                let placeholder = format!("{{{{scenario.{}}}}}", var_name);
                let value_str = match var_value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => serde_json::to_string(var_value).unwrap_or_default(),
                };
                body = body.replace(&placeholder, &value_str);
            }
        }

        // Parse response headers
        let headers: HashMap<String, String> =
            serde_json::from_str(&step.response.headers).unwrap_or_default();

        Ok(ReplayResponse {
            status_code: step.response.status_code as u16,
            headers,
            body,
            timing_ms: step.timing_ms,
        })
    }

    /// Substitute state variables in JSON value
    fn substitute_state_variables(value: &mut Value, state: &HashMap<String, Value>) {
        match value {
            Value::String(s) => {
                // Check for template variables like {{scenario.user_id}}
                for (var_name, var_value) in state {
                    let placeholder = format!("{{{{scenario.{}}}}}", var_name);
                    if s.contains(&placeholder) {
                        let value_str = match var_value {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => serde_json::to_string(var_value).unwrap_or_default(),
                        };
                        *s = s.replace(&placeholder, &value_str);
                    }
                }
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    Self::substitute_state_variables(v, state);
                }
            }
            Value::Array(arr) => {
                for v in arr.iter_mut() {
                    Self::substitute_state_variables(v, state);
                }
            }
            _ => {}
        }
    }

    /// Extract state variables from response body
    fn extract_state_variables(
        body: &str,
        extracts: &HashMap<String, String>,
        state: &mut HashMap<String, Value>,
    ) {
        if let Ok(json) = serde_json::from_str::<Value>(body) {
            for (var_name, json_path) in extracts {
                if let Some(value) = Self::extract_json_path(&json, json_path) {
                    state.insert(var_name.clone(), value);
                }
            }
        }
    }

    /// Extract value from JSON using simple path
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
}

// Implement the trait for BehavioralScenarioReplayEngine
#[async_trait::async_trait]
impl mockforge_core::priority_handler::BehavioralScenarioReplay for BehavioralScenarioReplayEngine {
    async fn try_replay(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Option<&[u8]>,
        session_id: Option<&str>,
    ) -> mockforge_core::Result<Option<mockforge_core::priority_handler::BehavioralReplayResponse>>
    {
        match self.try_replay(method, uri, headers, body, session_id).await {
            Ok(Some(response)) => {
                let content_type = response
                    .headers
                    .get("content-type")
                    .unwrap_or(&"application/json".to_string())
                    .clone();
                Ok(Some(mockforge_core::priority_handler::BehavioralReplayResponse {
                    status_code: response.status_code,
                    headers: response.headers,
                    body: response.body.into_bytes(),
                    timing_ms: response.timing_ms,
                    content_type,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(mockforge_core::Error::generic(format!("Replay error: {}", e))),
        }
    }
}

/// Response from scenario replay
#[derive(Debug, Clone)]
pub struct ReplayResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Timing delay in milliseconds
    pub timing_ms: Option<u64>,
}
