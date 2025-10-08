//! Integration testing workflow engine
//!
//! Supports multi-endpoint test flows with state management, variable extraction,
//! and conditional logic for comprehensive integration testing.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Integration test workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationWorkflow {
    /// Workflow ID
    pub id: String,
    /// Workflow name
    pub name: String,
    /// Description
    pub description: String,
    /// Steps in the workflow
    pub steps: Vec<WorkflowStep>,
    /// Global setup (variables, config)
    pub setup: WorkflowSetup,
    /// Cleanup steps
    pub cleanup: Vec<WorkflowStep>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Workflow setup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSetup {
    /// Initial variables
    pub variables: HashMap<String, String>,
    /// Base URL
    pub base_url: String,
    /// Global headers
    pub headers: HashMap<String, String>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for WorkflowSetup {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            base_url: "http://localhost:3000".to_string(),
            headers: HashMap::new(),
            timeout_ms: 30000,
        }
    }
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Description
    pub description: String,
    /// HTTP request to execute
    pub request: StepRequest,
    /// Expected response validation
    pub validation: StepValidation,
    /// Variables to extract from response
    pub extract: Vec<VariableExtraction>,
    /// Conditional execution
    pub condition: Option<StepCondition>,
    /// Delay after step (ms)
    pub delay_ms: Option<u64>,
}

/// Step HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRequest {
    /// HTTP method
    pub method: String,
    /// Endpoint path (supports variable substitution)
    pub path: String,
    /// Headers (supports variable substitution)
    pub headers: HashMap<String, String>,
    /// Request body (supports variable substitution)
    pub body: Option<String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
}

/// Step validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepValidation {
    /// Expected status code
    pub status_code: Option<u16>,
    /// Expected response body assertions
    pub body_assertions: Vec<BodyAssertion>,
    /// Header assertions
    pub header_assertions: Vec<HeaderAssertion>,
    /// Response time assertion (max ms)
    pub max_response_time_ms: Option<u64>,
}

/// Body assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyAssertion {
    /// JSON path or regex
    pub path: String,
    /// Assertion type
    pub assertion_type: AssertionType,
    /// Expected value
    pub expected: Value,
}

/// Assertion type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    /// Equals
    Equals,
    /// Not equals
    NotEquals,
    /// Contains
    Contains,
    /// Matches regex
    Matches,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Exists (field is present)
    Exists,
    /// Not null
    NotNull,
}

/// Header assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderAssertion {
    /// Header name
    pub name: String,
    /// Expected value or pattern
    pub expected: String,
    /// Use regex matching
    pub regex: bool,
}

/// Variable extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableExtraction {
    /// Variable name
    pub name: String,
    /// Extraction source
    pub source: ExtractionSource,
    /// JSONPath or regex pattern
    pub pattern: String,
    /// Default value if extraction fails
    pub default: Option<String>,
}

/// Extraction source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionSource {
    /// Response body
    Body,
    /// Response header
    Header,
    /// Status code
    StatusCode,
}

/// Step condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    /// Variable to check
    pub variable: String,
    /// Condition operator
    pub operator: ConditionOperator,
    /// Value to compare
    pub value: String,
}

/// Condition operator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equals
    Equals,
    /// Not equals
    NotEquals,
    /// Contains
    Contains,
    /// Exists (variable is set)
    Exists,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
}

/// Workflow execution state
#[derive(Debug, Clone)]
pub struct WorkflowState {
    /// Current variables
    pub variables: HashMap<String, String>,
    /// Step execution history
    pub history: Vec<StepExecution>,
    /// Current step index
    pub current_step: usize,
}

/// Step execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    /// Step ID
    pub step_id: String,
    /// Step name
    pub step_name: String,
    /// Executed timestamp
    pub executed_at: DateTime<Utc>,
    /// Request sent
    pub request: ExecutedRequest,
    /// Response received
    pub response: ExecutedResponse,
    /// Validation result
    pub validation_result: ValidationResult,
    /// Variables extracted
    pub extracted_variables: HashMap<String, String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Executed request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedRequest {
    /// Method
    pub method: String,
    /// Full URL
    pub url: String,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    pub body: Option<String>,
}

/// Executed response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedResponse {
    /// Status code
    pub status_code: u16,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    pub body: String,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall success
    pub success: bool,
    /// Individual assertion results
    pub assertions: Vec<AssertionResult>,
    /// Error messages
    pub errors: Vec<String>,
}

/// Assertion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Assertion description
    pub description: String,
    /// Success
    pub success: bool,
    /// Expected value
    pub expected: String,
    /// Actual value
    pub actual: String,
}

/// Workflow generator - generates integration test code
pub struct IntegrationTestGenerator {
    /// Workflow to generate tests from
    workflow: IntegrationWorkflow,
}

impl IntegrationTestGenerator {
    /// Create new generator
    pub fn new(workflow: IntegrationWorkflow) -> Self {
        Self { workflow }
    }

    /// Generate Rust integration test
    pub fn generate_rust_test(&self) -> String {
        let mut code = String::new();

        // Imports
        code.push_str("use reqwest;\n");
        code.push_str("use serde_json::{json, Value};\n");
        code.push_str("use std::collections::HashMap;\n\n");

        // Test function
        code.push_str(&format!("#[tokio::test]\n"));
        code.push_str(&format!("async fn test_{}() {{\n", self.sanitize_name(&self.workflow.name)));

        // Setup
        code.push_str("    let client = reqwest::Client::new();\n");
        code.push_str(&format!("    let base_url = \"{}\";\n", self.workflow.setup.base_url));

        // Variables
        code.push_str("    let mut variables: HashMap<String, String> = HashMap::new();\n");
        for (key, value) in &self.workflow.setup.variables {
            code.push_str(&format!("    variables.insert(\"{}\".to_string(), \"{}\".to_string());\n", key, value));
        }
        code.push_str("\n");

        // Steps
        for (idx, step) in self.workflow.steps.iter().enumerate() {
            code.push_str(&format!("    // Step {}: {}\n", idx + 1, step.name));

            // Condition check
            if let Some(condition) = &step.condition {
                code.push_str(&self.generate_condition_check(condition));
            }

            // Build URL
            code.push_str(&format!("    let url_{} = format!(\"{{}}{}\"", idx, self.replace_vars(&step.request.path)));
            code.push_str(", base_url");
            // Add variable substitutions
            for var in self.extract_variables(&step.request.path) {
                code.push_str(&format!(", variables.get(\"{}\").unwrap_or(&String::new())", var));
            }
            code.push_str(");\n");

            // Build request
            code.push_str(&format!("    let mut request_{} = client.{}(&url_{})",
                idx, step.request.method.to_lowercase(), idx));

            // Add headers
            if !step.request.headers.is_empty() {
                code.push_str("\n");
                for (key, value) in &step.request.headers {
                    let value_with_vars = self.replace_vars(value);
                    code.push_str(&format!("        .header(\"{}\", {})\n", key, value_with_vars));
                }
            }

            // Add body
            if let Some(body) = &step.request.body {
                let body_with_vars = self.replace_vars(body);
                code.push_str(&format!("        .body({})\n", body_with_vars));
            }

            code.push_str(";\n\n");

            // Send request
            code.push_str(&format!("    let response_{} = request_{}.send().await.expect(\"Request failed\");\n", idx, idx));

            // Validation
            if let Some(status) = step.validation.status_code {
                code.push_str(&format!("    assert_eq!(response_{}.status().as_u16(), {});\n", idx, status));
            }

            // Extract variables
            if !step.extract.is_empty() {
                code.push_str(&format!("    let body_{} = response_{}.text().await.expect(\"Failed to read body\");\n", idx, idx));
                code.push_str(&format!("    let json_{}: Value = serde_json::from_str(&body_{}).expect(\"Invalid JSON\");\n", idx, idx));

                for extraction in &step.extract {
                    if extraction.source == ExtractionSource::Body {
                        code.push_str(&format!("    variables.insert(\"{}\".to_string(), json_{}[\"{}\"].as_str().unwrap_or(\"{}\").to_string());\n",
                            extraction.name, idx, extraction.pattern, extraction.default.as_deref().unwrap_or("")));
                    }
                }
            }

            // Delay
            if let Some(delay) = step.delay_ms {
                code.push_str(&format!("    tokio::time::sleep(tokio::time::Duration::from_millis({})).await;\n", delay));
            }

            code.push_str("\n");
        }

        code.push_str("}\n");
        code
    }

    /// Generate Python integration test
    pub fn generate_python_test(&self) -> String {
        let mut code = String::new();

        // Imports
        code.push_str("import requests\n");
        code.push_str("import time\n");
        code.push_str("import pytest\n\n");

        // Test function
        code.push_str(&format!("def test_{}():\n", self.sanitize_name(&self.workflow.name)));

        // Setup
        code.push_str(&format!("    base_url = '{}'\n", self.workflow.setup.base_url));
        code.push_str("    variables = {}\n");
        for (key, value) in &self.workflow.setup.variables {
            code.push_str(&format!("    variables['{}'] = '{}'\n", key, value));
        }
        code.push_str("\n");

        // Steps
        for (idx, step) in self.workflow.steps.iter().enumerate() {
            code.push_str(&format!("    # Step {}: {}\n", idx + 1, step.name));

            // Build URL
            let path = self.replace_vars_python(&step.request.path);
            code.push_str(&format!("    url = f'{{base_url}}{}'\n", path));

            // Build request
            let method = step.request.method.to_lowercase();
            code.push_str(&format!("    response = requests.{}(url", method));

            // Add headers
            if !step.request.headers.is_empty() {
                code.push_str(", headers={");
                let headers: Vec<String> = step.request.headers.iter()
                    .map(|(k, v)| format!("'{}': '{}'", k, self.replace_vars_python(v)))
                    .collect();
                code.push_str(&headers.join(", "));
                code.push_str("}");
            }

            // Add body
            if let Some(body) = &step.request.body {
                code.push_str(&format!(", json={}", self.replace_vars_python(body)));
            }

            code.push_str(")\n");

            // Validation
            if let Some(status) = step.validation.status_code {
                code.push_str(&format!("    assert response.status_code == {}\n", status));
            }

            // Extract variables
            for extraction in &step.extract {
                if extraction.source == ExtractionSource::Body {
                    code.push_str(&format!("    variables['{}'] = response.json().get('{}', '{}')\n",
                        extraction.name, extraction.pattern, extraction.default.as_deref().unwrap_or("")));
                }
            }

            // Delay
            if let Some(delay) = step.delay_ms {
                code.push_str(&format!("    time.sleep({:.2})\n", delay as f64 / 1000.0));
            }

            code.push_str("\n");
        }

        code
    }

    /// Generate JavaScript integration test
    pub fn generate_javascript_test(&self) -> String {
        let mut code = String::new();

        // Test describe block
        code.push_str(&format!("describe('{}', () => {{\n", self.workflow.name));
        code.push_str(&format!("  it('{}', async () => {{\n", self.workflow.description));

        // Setup
        code.push_str(&format!("    const baseUrl = '{}';\n", self.workflow.setup.base_url));
        code.push_str("    const variables = {};\n");
        for (key, value) in &self.workflow.setup.variables {
            code.push_str(&format!("    variables['{}'] = '{}';\n", key, value));
        }
        code.push_str("\n");

        // Steps
        for (idx, step) in self.workflow.steps.iter().enumerate() {
            code.push_str(&format!("    // Step {}: {}\n", idx + 1, step.name));

            // Build URL
            let path = self.replace_vars_js(&step.request.path);
            code.push_str(&format!("    const url{} = `${{baseUrl}}{}`;\n", idx, path));

            // Build request
            code.push_str(&format!("    const response{} = await fetch(url{}, {{\n", idx, idx));
            code.push_str(&format!("      method: '{}',\n", step.request.method.to_uppercase()));

            // Add headers
            if !step.request.headers.is_empty() {
                code.push_str("      headers: {\n");
                for (key, value) in &step.request.headers {
                    code.push_str(&format!("        '{}': '{}',\n", key, self.replace_vars_js(value)));
                }
                code.push_str("      },\n");
            }

            // Add body
            if let Some(body) = &step.request.body {
                code.push_str(&format!("      body: JSON.stringify({}),\n", self.replace_vars_js(body)));
            }

            code.push_str("    });\n");

            // Validation
            if let Some(status) = step.validation.status_code {
                code.push_str(&format!("    expect(response{}.status).toBe({});\n", idx, status));
            }

            // Extract variables
            if !step.extract.is_empty() {
                code.push_str(&format!("    const data{} = await response{}.json();\n", idx, idx));
                for extraction in &step.extract {
                    if extraction.source == ExtractionSource::Body {
                        code.push_str(&format!("    variables['{}'] = data{}.{} || '{}';\n",
                            extraction.name, idx, extraction.pattern, extraction.default.as_deref().unwrap_or("")));
                    }
                }
            }

            // Delay
            if let Some(delay) = step.delay_ms {
                code.push_str(&format!("    await new Promise(resolve => setTimeout(resolve, {}));\n", delay));
            }

            code.push_str("\n");
        }

        code.push_str("  });\n");
        code.push_str("});\n");
        code
    }

    // Helper methods
    fn sanitize_name(&self, name: &str) -> String {
        name.to_lowercase()
            .replace(' ', "_")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    fn extract_variables(&self, text: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut in_var = false;
        let mut current_var = String::new();

        for c in text.chars() {
            if c == '{' {
                in_var = true;
            } else if c == '}' && in_var {
                if !current_var.is_empty() {
                    vars.push(current_var.clone());
                    current_var.clear();
                }
                in_var = false;
            } else if in_var {
                current_var.push(c);
            }
        }

        vars
    }

    fn replace_vars(&self, text: &str) -> String {
        let vars = self.extract_variables(text);
        if vars.is_empty() {
            return format!("\"{}\"", text);
        }

        let mut result = text.to_string();
        for var in vars {
            result = result.replace(&format!("{{{}}}", var), "{}");
        }
        format!("\"{}\"", result)
    }

    fn replace_vars_python(&self, text: &str) -> String {
        let mut result = text.to_string();
        for var in self.extract_variables(text) {
            result = result.replace(&format!("{{{}}}", var), &format!("{{variables['{}']}}",  var));
        }
        result
    }

    fn replace_vars_js(&self, text: &str) -> String {
        let mut result = text.to_string();
        for var in self.extract_variables(text) {
            result = result.replace(&format!("{{{}}}", var), &format!("${{variables['{}']}}",  var));
        }
        result
    }

    fn generate_condition_check(&self, condition: &StepCondition) -> String {
        let mut code = String::new();
        code.push_str(&format!("    if let Some(val) = variables.get(\"{}\") {{\n", condition.variable));

        let check = match condition.operator {
            ConditionOperator::Equals => format!("val == \"{}\"", condition.value),
            ConditionOperator::NotEquals => format!("val != \"{}\"", condition.value),
            ConditionOperator::Contains => format!("val.contains(\"{}\")", condition.value),
            ConditionOperator::Exists => "true".to_string(),
            ConditionOperator::GreaterThan => format!("val.parse::<f64>().unwrap_or(0.0) > {}", condition.value),
            ConditionOperator::LessThan => format!("val.parse::<f64>().unwrap_or(0.0) < {}", condition.value),
        };

        code.push_str(&format!("        if !({}) {{\n", check));
        code.push_str("            return; // Skip this step\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let workflow = IntegrationWorkflow {
            id: "test-1".to_string(),
            name: "User Registration Flow".to_string(),
            description: "Test user registration and login".to_string(),
            steps: vec![],
            setup: WorkflowSetup::default(),
            cleanup: vec![],
            created_at: Utc::now(),
        };

        assert_eq!(workflow.name, "User Registration Flow");
    }

    #[test]
    fn test_variable_extraction() {
        let gen = IntegrationTestGenerator::new(IntegrationWorkflow {
            id: "test".to_string(),
            name: "test".to_string(),
            description: "".to_string(),
            steps: vec![],
            setup: WorkflowSetup::default(),
            cleanup: vec![],
            created_at: Utc::now(),
        });

        let vars = gen.extract_variables("/api/users/{user_id}/posts/{post_id}");
        assert_eq!(vars, vec!["user_id", "post_id"]);
    }
}
