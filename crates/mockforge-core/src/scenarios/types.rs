//! Scenario type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scenario definition for high-level business workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    /// Unique scenario identifier (e.g., "checkout-success")
    pub id: String,
    /// Human-readable name (e.g., "CheckoutSuccess")
    pub name: String,
    /// Scenario description
    pub description: Option<String>,
    /// Ordered list of API calls to execute
    pub steps: Vec<ScenarioStep>,
    /// Default variables for the scenario
    pub variables: HashMap<String, serde_json::Value>,
    /// Input parameters for the scenario
    pub parameters: Vec<ScenarioParameter>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// A single step in a scenario (represents one API call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    /// Step identifier
    pub id: String,
    /// Step name/description
    pub name: String,
    /// HTTP method
    pub method: String,
    /// API endpoint path
    pub path: String,
    /// Request body (can use template variables)
    pub body: Option<serde_json::Value>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Path parameters (for dynamic paths)
    pub path_params: HashMap<String, String>,
    /// Variables to extract from response (for use in subsequent steps)
    pub extract: HashMap<String, String>, // variable_name -> json_path
    /// Expected status code
    pub expected_status: Option<u16>,
    /// Whether to continue on failure
    pub continue_on_failure: bool,
    /// Delay before executing this step (in milliseconds)
    pub delay_ms: Option<u64>,
    /// Dependencies on other steps (step IDs that must complete first)
    pub depends_on: Vec<String>,
}

/// Input parameter for a scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: Option<String>,
    /// Parameter type (e.g., "string", "number", "object")
    pub parameter_type: String,
    /// Whether parameter is required
    pub required: bool,
    /// Default value (if optional)
    pub default: Option<serde_json::Value>,
}

/// Result of scenario execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Scenario ID that was executed
    pub scenario_id: String,
    /// Whether scenario completed successfully
    pub success: bool,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Error message (if scenario failed)
    pub error: Option<String>,
    /// Final state (all variables after execution)
    pub final_state: HashMap<String, serde_json::Value>,
}

/// Result of a single step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step ID
    pub step_id: String,
    /// Whether step succeeded
    pub success: bool,
    /// HTTP status code
    pub status_code: Option<u16>,
    /// Response body
    pub response_body: Option<serde_json::Value>,
    /// Extracted variables from this step
    pub extracted_variables: HashMap<String, serde_json::Value>,
    /// Error message (if step failed)
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

impl ScenarioDefinition {
    /// Create a new scenario definition
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            steps: Vec::new(),
            variables: HashMap::new(),
            parameters: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Add a step to the scenario
    pub fn add_step(mut self, step: ScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add a parameter to the scenario
    pub fn add_parameter(mut self, param: ScenarioParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Set default variables
    pub fn with_variables(mut self, variables: HashMap<String, serde_json::Value>) -> Self {
        self.variables = variables;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl ScenarioStep {
    /// Create a new scenario step
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        method: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            method: method.into(),
            path: path.into(),
            body: None,
            headers: HashMap::new(),
            query_params: HashMap::new(),
            path_params: HashMap::new(),
            extract: HashMap::new(),
            expected_status: None,
            continue_on_failure: false,
            delay_ms: None,
            depends_on: Vec::new(),
        }
    }

    /// Set request body
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Add a variable extraction rule
    pub fn extract_variable(mut self, var_name: impl Into<String>, json_path: impl Into<String>) -> Self {
        self.extract.insert(var_name.into(), json_path.into());
        self
    }

    /// Set expected status code
    pub fn expect_status(mut self, status: u16) -> Self {
        self.expected_status = Some(status);
        self
    }

    /// Add a dependency on another step
    pub fn depends_on(mut self, step_id: impl Into<String>) -> Self {
        self.depends_on.push(step_id.into());
        self
    }
}
