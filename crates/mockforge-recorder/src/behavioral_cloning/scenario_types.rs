//! Behavioral scenario types for flow recording and replay
//!
//! This module defines types for named behavioral scenarios that can be
//! compiled from recorded flows and replayed deterministically.

use crate::models::{RecordedRequest, RecordedResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A behavioral scenario that can be replayed deterministically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralScenario {
    /// Unique identifier for this scenario
    pub id: String,
    /// Human-readable name (e.g., "checkout_success")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Ordered list of steps in this scenario
    pub steps: Vec<BehavioralScenarioStep>,
    /// State variables extracted from responses (user_id, cart_id, etc.)
    pub state_variables: HashMap<String, StateVariable>,
    /// Whether to use strict mode (exact sequence) or flex mode (minor variations allowed)
    pub strict_mode: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// A single step in a behavioral scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralScenarioStep {
    /// Step identifier (unique within scenario)
    pub step_id: String,
    /// Optional step label (e.g., "login", "checkout")
    pub label: Option<String>,
    /// The recorded request for this step
    pub request: RecordedRequest,
    /// The recorded response for this step
    pub response: RecordedResponse,
    /// Timing delay from previous step in milliseconds
    pub timing_ms: Option<u64>,
    /// Variables to extract from response (variable_name -> json_path)
    pub extracts: HashMap<String, String>,
    /// Step IDs that this step depends on
    pub depends_on: Vec<String>,
}

/// A state variable extracted from a scenario step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVariable {
    /// Variable name (e.g., "user_id", "cart_id")
    pub name: String,
    /// JSONPath expression to extract the value
    pub json_path: String,
    /// The step ID where this variable is extracted
    pub extracted_from_step: String,
    /// Optional default value if extraction fails
    pub default_value: Option<serde_json::Value>,
}

impl BehavioralScenario {
    /// Create a new behavioral scenario
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            steps: Vec::new(),
            state_variables: HashMap::new(),
            strict_mode: true,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add a step to the scenario
    pub fn add_step(mut self, step: BehavioralScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add a state variable
    pub fn add_state_variable(mut self, variable: StateVariable) -> Self {
        self.state_variables.insert(variable.name.clone(), variable);
        self
    }

    /// Set strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl BehavioralScenarioStep {
    /// Create a new scenario step
    pub fn new(
        step_id: impl Into<String>,
        request: RecordedRequest,
        response: RecordedResponse,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            label: None,
            request,
            response,
            timing_ms: None,
            extracts: HashMap::new(),
            depends_on: Vec::new(),
        }
    }

    /// Set step label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set timing delay
    pub fn with_timing(mut self, timing_ms: u64) -> Self {
        self.timing_ms = Some(timing_ms);
        self
    }

    /// Add a variable extraction
    pub fn add_extract(
        mut self,
        variable_name: impl Into<String>,
        json_path: impl Into<String>,
    ) -> Self {
        self.extracts.insert(variable_name.into(), json_path.into());
        self
    }

    /// Add a dependency on another step
    pub fn add_dependency(mut self, step_id: impl Into<String>) -> Self {
        self.depends_on.push(step_id.into());
        self
    }
}

impl StateVariable {
    /// Create a new state variable
    pub fn new(
        name: impl Into<String>,
        json_path: impl Into<String>,
        extracted_from_step: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            json_path: json_path.into(),
            extracted_from_step: extracted_from_step.into(),
            default_value: None,
        }
    }

    /// Set default value
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default_value = Some(default);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RecordedRequest, RecordedResponse};

    fn create_test_request() -> RecordedRequest {
        RecordedRequest {
            id: "req_001".to_string(),
            protocol: crate::models::Protocol::Http,
            timestamp: chrono::Utc::now(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            query_params: None,
            headers: "{}".to_string(),
            body: None,
            body_encoding: "utf8".to_string(),
            client_ip: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            status_code: None,
            tags: None,
        }
    }

    fn create_test_response() -> RecordedResponse {
        RecordedResponse {
            request_id: "req_001".to_string(),
            status_code: 200,
            headers: "{}".to_string(),
            body: Some(r#"{"id": 1, "name": "Test"}"#.to_string()),
            body_encoding: "utf8".to_string(),
            size_bytes: 26,
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_behavioral_scenario_new() {
        let scenario = BehavioralScenario::new("scenario_1", "Test Scenario");

        assert_eq!(scenario.id, "scenario_1");
        assert_eq!(scenario.name, "Test Scenario");
        assert!(scenario.description.is_none());
        assert!(scenario.steps.is_empty());
        assert!(scenario.state_variables.is_empty());
        assert!(scenario.strict_mode); // default true
        assert!(scenario.metadata.is_empty());
        assert!(scenario.tags.is_empty());
    }

    #[test]
    fn test_behavioral_scenario_with_description() {
        let scenario = BehavioralScenario::new("scenario_1", "Test Scenario")
            .with_description("A test scenario description");

        assert_eq!(scenario.description, Some("A test scenario description".to_string()));
    }

    #[test]
    fn test_behavioral_scenario_with_strict_mode() {
        let scenario =
            BehavioralScenario::new("scenario_1", "Test Scenario").with_strict_mode(false);

        assert!(!scenario.strict_mode);
    }

    #[test]
    fn test_behavioral_scenario_with_tags() {
        let tags = vec!["checkout".to_string(), "critical".to_string()];
        let scenario =
            BehavioralScenario::new("scenario_1", "Test Scenario").with_tags(tags.clone());

        assert_eq!(scenario.tags, tags);
    }

    #[test]
    fn test_behavioral_scenario_add_step() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response());

        let scenario = BehavioralScenario::new("scenario_1", "Test Scenario").add_step(step);

        assert_eq!(scenario.steps.len(), 1);
        assert_eq!(scenario.steps[0].step_id, "step_1");
    }

    #[test]
    fn test_behavioral_scenario_add_state_variable() {
        let variable = StateVariable::new("user_id", "$.id", "step_1");

        let scenario =
            BehavioralScenario::new("scenario_1", "Test Scenario").add_state_variable(variable);

        assert_eq!(scenario.state_variables.len(), 1);
        assert!(scenario.state_variables.contains_key("user_id"));
    }

    #[test]
    fn test_behavioral_scenario_step_new() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response());

        assert_eq!(step.step_id, "step_1");
        assert!(step.label.is_none());
        assert!(step.timing_ms.is_none());
        assert!(step.extracts.is_empty());
        assert!(step.depends_on.is_empty());
    }

    #[test]
    fn test_behavioral_scenario_step_with_label() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response())
                .with_label("login");

        assert_eq!(step.label, Some("login".to_string()));
    }

    #[test]
    fn test_behavioral_scenario_step_with_timing() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response())
                .with_timing(500);

        assert_eq!(step.timing_ms, Some(500));
    }

    #[test]
    fn test_behavioral_scenario_step_add_extract() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response())
                .add_extract("user_id", "$.data.user.id");

        assert_eq!(step.extracts.len(), 1);
        assert_eq!(step.extracts.get("user_id"), Some(&"$.data.user.id".to_string()));
    }

    #[test]
    fn test_behavioral_scenario_step_add_dependency() {
        let step =
            BehavioralScenarioStep::new("step_2", create_test_request(), create_test_response())
                .add_dependency("step_1");

        assert_eq!(step.depends_on.len(), 1);
        assert_eq!(step.depends_on[0], "step_1");
    }

    #[test]
    fn test_behavioral_scenario_step_builder_chain() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response())
                .with_label("checkout")
                .with_timing(100)
                .add_extract("cart_id", "$.cart.id")
                .add_dependency("step_0");

        assert_eq!(step.label, Some("checkout".to_string()));
        assert_eq!(step.timing_ms, Some(100));
        assert_eq!(step.extracts.len(), 1);
        assert_eq!(step.depends_on.len(), 1);
    }

    #[test]
    fn test_state_variable_new() {
        let variable = StateVariable::new("user_id", "$.data.id", "step_1");

        assert_eq!(variable.name, "user_id");
        assert_eq!(variable.json_path, "$.data.id");
        assert_eq!(variable.extracted_from_step, "step_1");
        assert!(variable.default_value.is_none());
    }

    #[test]
    fn test_state_variable_with_default() {
        let variable = StateVariable::new("user_id", "$.data.id", "step_1")
            .with_default(serde_json::json!("default_user"));

        assert_eq!(variable.default_value, Some(serde_json::json!("default_user")));
    }

    #[test]
    fn test_state_variable_with_default_number() {
        let variable = StateVariable::new("count", "$.data.count", "step_1")
            .with_default(serde_json::json!(0));

        assert_eq!(variable.default_value, Some(serde_json::json!(0)));
    }

    #[test]
    fn test_behavioral_scenario_serialization() {
        let scenario = BehavioralScenario::new("scenario_1", "Test Scenario")
            .with_description("A test")
            .with_strict_mode(true)
            .with_tags(vec!["test".to_string()]);

        let json = serde_json::to_string(&scenario).unwrap();

        assert!(json.contains("\"id\":\"scenario_1\""));
        assert!(json.contains("\"name\":\"Test Scenario\""));
        assert!(json.contains("\"strict_mode\":true"));
    }

    #[test]
    fn test_behavioral_scenario_deserialization() {
        let json = r#"{
            "id": "scenario_1",
            "name": "Test Scenario",
            "description": "A test",
            "steps": [],
            "state_variables": {},
            "strict_mode": false,
            "metadata": {},
            "tags": ["checkout"]
        }"#;

        let scenario: BehavioralScenario = serde_json::from_str(json).unwrap();

        assert_eq!(scenario.id, "scenario_1");
        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.description, Some("A test".to_string()));
        assert!(!scenario.strict_mode);
        assert_eq!(scenario.tags, vec!["checkout".to_string()]);
    }

    #[test]
    fn test_state_variable_serialization() {
        let variable = StateVariable::new("user_id", "$.id", "step_1")
            .with_default(serde_json::json!("anonymous"));

        let json = serde_json::to_string(&variable).unwrap();

        assert!(json.contains("\"name\":\"user_id\""));
        assert!(json.contains("\"json_path\":\"$.id\""));
        assert!(json.contains("\"extracted_from_step\":\"step_1\""));
        assert!(json.contains("\"default_value\":\"anonymous\""));
    }

    #[test]
    fn test_state_variable_deserialization() {
        let json = r#"{
            "name": "token",
            "json_path": "$.access_token",
            "extracted_from_step": "login_step",
            "default_value": null
        }"#;

        let variable: StateVariable = serde_json::from_str(json).unwrap();

        assert_eq!(variable.name, "token");
        assert_eq!(variable.json_path, "$.access_token");
        assert_eq!(variable.extracted_from_step, "login_step");
        assert!(variable.default_value.is_none());
    }

    #[test]
    fn test_behavioral_scenario_clone() {
        let scenario = BehavioralScenario::new("scenario_1", "Test Scenario")
            .with_description("A test")
            .with_tags(vec!["tag1".to_string()]);

        let cloned = scenario.clone();

        assert_eq!(scenario.id, cloned.id);
        assert_eq!(scenario.name, cloned.name);
        assert_eq!(scenario.description, cloned.description);
        assert_eq!(scenario.tags, cloned.tags);
    }

    #[test]
    fn test_behavioral_scenario_step_clone() {
        let step =
            BehavioralScenarioStep::new("step_1", create_test_request(), create_test_response())
                .with_label("test");

        let cloned = step.clone();

        assert_eq!(step.step_id, cloned.step_id);
        assert_eq!(step.label, cloned.label);
    }

    #[test]
    fn test_state_variable_clone() {
        let variable = StateVariable::new("user_id", "$.id", "step_1");
        let cloned = variable.clone();

        assert_eq!(variable.name, cloned.name);
        assert_eq!(variable.json_path, cloned.json_path);
    }

    #[test]
    fn test_full_scenario_with_steps() {
        let request1 = create_test_request();
        let response1 = create_test_response();
        let request2 = create_test_request();
        let response2 = create_test_response();

        let step1 = BehavioralScenarioStep::new("step_1", request1, response1)
            .with_label("get_user")
            .add_extract("user_id", "$.id");

        let step2 = BehavioralScenarioStep::new("step_2", request2, response2)
            .with_label("update_user")
            .with_timing(50)
            .add_dependency("step_1");

        let variable = StateVariable::new("user_id", "$.id", "step_1");

        let scenario = BehavioralScenario::new("user_flow", "User Flow")
            .with_description("Test user CRUD operations")
            .add_step(step1)
            .add_step(step2)
            .add_state_variable(variable)
            .with_tags(vec!["user".to_string(), "crud".to_string()]);

        assert_eq!(scenario.steps.len(), 2);
        assert_eq!(scenario.state_variables.len(), 1);
        assert_eq!(scenario.tags.len(), 2);

        // Verify serialization round-trip
        let json = serde_json::to_string(&scenario).unwrap();
        let parsed: BehavioralScenario = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.steps.len(), 2);
        assert_eq!(parsed.state_variables.len(), 1);
    }
}
