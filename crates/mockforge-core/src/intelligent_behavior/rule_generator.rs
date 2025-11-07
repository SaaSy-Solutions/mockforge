//! Rule auto-generation engine for MockAI
//!
//! This module analyzes example request/response pairs and OpenAPI specifications
//! to automatically generate behavioral rules, validation rules, pagination patterns,
//! and state machines.

use super::config::BehaviorModelConfig;
use super::llm_client::LlmClient;
use super::rules::{ConsistencyRule, RuleAction, StateMachine, StateTransition};
use super::types::{BehaviorRules, LlmGenerationRequest};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Example request/response pair for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplePair {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body (optional)
    pub request: Option<Value>,
    /// Response status code
    pub status: u16,
    /// Response body (optional)
    pub response: Option<Value>,
    /// Query parameters (optional)
    #[serde(default)]
    pub query_params: HashMap<String, String>,
    /// Headers (optional)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Metadata about this example
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Error example for learning validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExample {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body that caused the error
    pub request: Option<Value>,
    /// Error status code
    pub status: u16,
    /// Error response body
    pub error_response: Value,
    /// Field that caused the error (if applicable)
    pub field: Option<String>,
}

/// Paginated response example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    /// Request path
    pub path: String,
    /// Query parameters including pagination params
    pub query_params: HashMap<String, String>,
    /// Response body with pagination metadata
    pub response: Value,
    /// Page number (if applicable)
    pub page: Option<usize>,
    /// Page size (if applicable)
    pub page_size: Option<usize>,
    /// Total count (if available)
    pub total: Option<usize>,
}

/// CRUD example for state machine generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrudExample {
    /// Operation type (create, read, update, delete)
    pub operation: String,
    /// Resource type
    pub resource_type: String,
    /// Request path
    pub path: String,
    /// Request body
    pub request: Option<Value>,
    /// Response status
    pub status: u16,
    /// Response body
    pub response: Option<Value>,
    /// Resource state after operation (if applicable)
    pub resource_state: Option<String>,
}

/// Validation rule inferred from examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Field name this rule applies to
    pub field: String,
    /// Validation type (required, format, min_length, max_length, pattern, etc.)
    pub validation_type: String,
    /// Validation parameters
    pub parameters: HashMap<String, Value>,
    /// Error message template
    pub error_message: String,
    /// HTTP status code for this validation error
    pub status_code: u16,
}

/// Pagination rule inferred from examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRule {
    /// Default page size
    pub default_page_size: usize,
    /// Maximum page size
    pub max_page_size: usize,
    /// Minimum page size
    pub min_page_size: usize,
    /// Pagination parameter names (page, limit, offset, cursor, etc.)
    pub parameter_names: HashMap<String, String>,
    /// Response format (page-based, offset-based, cursor-based)
    pub format: String,
}

/// Rule generator that learns from examples
pub struct RuleGenerator {
    /// LLM client for intelligent rule generation
    llm_client: Option<LlmClient>,
    /// Configuration
    config: BehaviorModelConfig,
}

impl RuleGenerator {
    /// Create a new rule generator
    pub fn new(config: BehaviorModelConfig) -> Self {
        let llm_client = if config.llm_provider != "disabled" {
            Some(LlmClient::new(config.clone()))
        } else {
            None
        };

        Self { llm_client, config }
    }

    /// Generate behavioral rules from example pairs
    ///
    /// Analyzes request/response examples to infer:
    /// - Consistency rules
    /// - Resource schemas
    /// - State machines
    /// - System prompts
    pub async fn generate_rules_from_examples(
        &self,
        examples: Vec<ExamplePair>,
    ) -> Result<BehaviorRules> {
        if examples.is_empty() {
            return Ok(BehaviorRules::default());
        }

        // Group examples by path pattern
        let path_groups = self.group_by_path_pattern(&examples);

        // Generate consistency rules from patterns
        let consistency_rules = self.infer_consistency_rules(&examples, &path_groups).await?;

        // Extract schemas from responses
        let schemas = self.extract_schemas_from_examples(&examples).await?;

        // Generate state machines from CRUD patterns
        let state_machines = self.infer_state_machines(&examples).await?;

        // Generate system prompt
        let system_prompt = self.generate_system_prompt(&examples).await?;

        Ok(BehaviorRules {
            system_prompt,
            schemas,
            consistency_rules,
            state_transitions: state_machines,
            max_context_interactions: 10,
            enable_semantic_search: true,
        })
    }

    /// Infer validation rules from error examples
    pub async fn infer_validation_rules(
        &self,
        error_examples: Vec<ErrorExample>,
    ) -> Result<Vec<ValidationRule>> {
        if error_examples.is_empty() {
            return Ok(Vec::new());
        }

        let mut rules = Vec::new();

        // Group errors by field and type
        let mut field_errors: HashMap<String, Vec<&ErrorExample>> = HashMap::new();
        for error in &error_examples {
            if let Some(ref field) = error.field {
                field_errors.entry(field.clone()).or_insert_with(Vec::new).push(error);
            }
        }

        // Analyze each field's error patterns
        for (field, errors) in field_errors {
            // Determine validation type from error patterns
            let validation_type = self.determine_validation_type(&errors)?;
            let error_message = self.extract_error_message_template(&errors)?;
            let status_code = errors[0].status;

            let mut parameters = HashMap::new();
            match validation_type.as_str() {
                "required" => {
                    parameters.insert("required".to_string(), Value::Bool(true));
                }
                "format" => {
                    // Try to infer format from error message
                    if let Some(format) = self.infer_format_from_errors(&errors) {
                        parameters.insert("format".to_string(), Value::String(format));
                    }
                }
                "min_length" | "max_length" => {
                    // Try to infer length constraints
                    if let Some(length) = self.infer_length_constraint(&errors, &validation_type) {
                        parameters.insert(validation_type.clone(), Value::Number(length.into()));
                    }
                }
                _ => {}
            }

            rules.push(ValidationRule {
                field,
                validation_type,
                parameters,
                error_message,
                status_code,
            });
        }

        Ok(rules)
    }

    /// Extract pagination pattern from examples
    pub async fn extract_pagination_pattern(
        &self,
        examples: Vec<PaginatedResponse>,
    ) -> Result<PaginationRule> {
        if examples.is_empty() {
            return Ok(PaginationRule {
                default_page_size: 20,
                max_page_size: 100,
                min_page_size: 1,
                parameter_names: HashMap::new(),
                format: "page-based".to_string(),
            });
        }

        // Analyze pagination parameters
        let mut parameter_names = HashMap::new();
        let mut page_sizes = Vec::new();
        let mut formats = Vec::new();

        for example in &examples {
            // Detect pagination parameters
            for (key, _value) in &example.query_params {
                match key.to_lowercase().as_str() {
                    "page" | "p" => {
                        parameter_names.insert("page".to_string(), key.clone());
                    }
                    "limit" | "per_page" | "size" => {
                        parameter_names.insert("limit".to_string(), key.clone());
                    }
                    "offset" => {
                        parameter_names.insert("offset".to_string(), key.clone());
                        formats.push("offset-based".to_string());
                    }
                    "cursor" => {
                        parameter_names.insert("cursor".to_string(), key.clone());
                        formats.push("cursor-based".to_string());
                    }
                    _ => {}
                }
            }

            if let Some(size) = example.page_size {
                page_sizes.push(size);
            }
        }

        // Determine format (default to page-based if not detected)
        let format = formats.first().cloned().unwrap_or_else(|| "page-based".to_string());

        // Calculate page size statistics
        let default_page_size = page_sizes.iter().copied().min().unwrap_or(20);
        let max_page_size = page_sizes.iter().copied().max().unwrap_or(100);
        let min_page_size = 1;

        Ok(PaginationRule {
            default_page_size,
            max_page_size,
            min_page_size,
            parameter_names,
            format,
        })
    }

    /// Analyze CRUD patterns to generate state machines
    pub async fn analyze_crud_pattern(
        &self,
        examples: Vec<CrudExample>,
    ) -> Result<HashMap<String, StateMachine>> {
        let mut machines: HashMap<String, StateMachine> = HashMap::new();

        // Group by resource type
        let mut resource_groups: HashMap<String, Vec<&CrudExample>> = HashMap::new();
        for example in &examples {
            resource_groups
                .entry(example.resource_type.clone())
                .or_insert_with(Vec::new)
                .push(example);
        }

        // Generate state machine for each resource type
        for (resource_type, resource_examples) in resource_groups {
            let states = self.infer_states_from_crud(&resource_examples)?;
            let initial_state = states.first().cloned().unwrap_or_else(|| "created".to_string());
            let transitions = self.infer_transitions_from_crud(&resource_examples, &states)?;

            let machine = StateMachine::new(resource_type.clone(), states, initial_state)
                .add_transitions(transitions);

            machines.insert(resource_type, machine);
        }

        Ok(machines)
    }

    // ===== Private helper methods =====

    /// Group examples by path pattern
    fn group_by_path_pattern<'a>(
        &self,
        examples: &'a [ExamplePair],
    ) -> HashMap<String, Vec<&'a ExamplePair>> {
        let mut groups: HashMap<String, Vec<&'a ExamplePair>> = HashMap::new();

        for example in examples {
            // Extract base path (remove IDs)
            let base_path = self.normalize_path(&example.path);
            groups.entry(base_path).or_insert_with(Vec::new).push(example);
        }

        groups
    }

    /// Normalize path by replacing IDs with placeholders
    fn normalize_path(&self, path: &str) -> String {
        // Simple heuristic: replace UUIDs and numeric IDs with placeholders
        path.split('/')
            .map(|segment| {
                if segment.parse::<u64>().is_ok() || segment.len() == 36 {
                    // Likely an ID
                    "{id}"
                } else {
                    segment
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Infer consistency rules from examples
    async fn infer_consistency_rules<'a>(
        &self,
        examples: &'a [ExamplePair],
        _path_groups: &HashMap<String, Vec<&'a ExamplePair>>,
    ) -> Result<Vec<ConsistencyRule>> {
        let mut rules = Vec::new();

        // Rule 1: POST creates resources (status 201)
        for example in examples {
            if example.method == "POST" && example.status == 201 {
                let path_pattern = self.normalize_path(&example.path);
                rules.push(ConsistencyRule::new(
                    format!("create_{}", path_pattern.replace('/', "_")),
                    format!("method == 'POST' AND path starts_with '{}'", path_pattern),
                    RuleAction::Transform {
                        description: format!("Create new resource at {}", path_pattern),
                    },
                ));
            }
        }

        // Rule 2: GET retrieves resources (status 200)
        for example in examples {
            if example.method == "GET" && example.status == 200 {
                let path_pattern = self.normalize_path(&example.path);
                rules.push(ConsistencyRule::new(
                    format!("get_{}", path_pattern.replace('/', "_")),
                    format!("method == 'GET' AND path starts_with '{}'", path_pattern),
                    RuleAction::Transform {
                        description: format!("Retrieve resource from {}", path_pattern),
                    },
                ));
            }
        }

        // Rule 3: PUT/PATCH updates resources (status 200)
        for example in examples {
            if (example.method == "PUT" || example.method == "PATCH") && example.status == 200 {
                let path_pattern = self.normalize_path(&example.path);
                rules.push(ConsistencyRule::new(
                    format!("update_{}", path_pattern.replace('/', "_")),
                    format!("method IN ['PUT', 'PATCH'] AND path starts_with '{}'", path_pattern),
                    RuleAction::Transform {
                        description: format!("Update resource at {}", path_pattern),
                    },
                ));
            }
        }

        // Rule 4: DELETE removes resources (status 204 or 200)
        for example in examples {
            if example.method == "DELETE" && (example.status == 204 || example.status == 200) {
                let path_pattern = self.normalize_path(&example.path);
                rules.push(ConsistencyRule::new(
                    format!("delete_{}", path_pattern.replace('/', "_")),
                    format!("method == 'DELETE' AND path starts_with '{}'", path_pattern),
                    RuleAction::Transform {
                        description: format!("Delete resource from {}", path_pattern),
                    },
                ));
            }
        }

        // Use LLM to generate additional rules if available
        if let Some(ref llm_client) = self.llm_client {
            let additional_rules = self.generate_rules_with_llm(examples).await?;
            rules.extend(additional_rules);
        }

        Ok(rules)
    }

    /// Extract schemas from example responses
    async fn extract_schemas_from_examples(
        &self,
        examples: &[ExamplePair],
    ) -> Result<HashMap<String, Value>> {
        let mut schemas: HashMap<String, Value> = HashMap::new();

        for example in examples {
            if let Some(ref response) = example.response {
                // Extract resource name from path
                let resource_name = self.extract_resource_name(&example.path);

                // Generate JSON Schema from response
                if let Some(schema) = self.infer_schema_from_value(response) {
                    schemas.insert(resource_name, schema);
                }
            }
        }

        Ok(schemas)
    }

    /// Infer JSON Schema from a JSON value
    fn infer_schema_from_value(&self, value: &Value) -> Option<Value> {
        match value {
            Value::Object(obj) => {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for (key, val) in obj {
                    if let Some(prop_schema) = self.infer_schema_from_value(val) {
                        properties.insert(key.clone(), prop_schema);
                        required.push(key.clone());
                    }
                }

                Some(serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                }))
            }
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    if let Some(item_schema) = self.infer_schema_from_value(first) {
                        Some(serde_json::json!({
                            "type": "array",
                            "items": item_schema
                        }))
                    } else {
                        Some(serde_json::json!({"type": "array"}))
                    }
                } else {
                    Some(serde_json::json!({"type": "array"}))
                }
            }
            Value::String(_) => Some(serde_json::json!({"type": "string"})),
            Value::Number(n) => {
                if n.is_i64() {
                    Some(serde_json::json!({"type": "integer"}))
                } else {
                    Some(serde_json::json!({"type": "number"}))
                }
            }
            Value::Bool(_) => Some(serde_json::json!({"type": "boolean"})),
            Value::Null => None,
        }
    }

    /// Extract resource name from path
    fn extract_resource_name(&self, path: &str) -> String {
        // Extract last meaningful segment
        path.split('/')
            .filter(|s| !s.is_empty() && !s.starts_with('{'))
            .last()
            .unwrap_or("Resource")
            .to_string()
    }

    /// Infer state machines from examples
    async fn infer_state_machines(
        &self,
        examples: &[ExamplePair],
    ) -> Result<HashMap<String, StateMachine>> {
        // Convert examples to CRUD examples
        let crud_examples: Vec<CrudExample> = examples
            .iter()
            .filter_map(|ex| {
                let operation = match ex.method.as_str() {
                    "POST" => Some("create"),
                    "GET" => Some("read"),
                    "PUT" | "PATCH" => Some("update"),
                    "DELETE" => Some("delete"),
                    _ => None,
                }?;

                let resource_type = self.extract_resource_name(&ex.path);

                Some(CrudExample {
                    operation: operation.to_string(),
                    resource_type,
                    path: ex.path.clone(),
                    request: ex.request.clone(),
                    status: ex.status,
                    response: ex.response.clone(),
                    resource_state: None,
                })
            })
            .collect();

        self.analyze_crud_pattern(crud_examples).await
    }

    /// Infer states from CRUD examples
    fn infer_states_from_crud(&self, examples: &[&CrudExample]) -> Result<Vec<String>> {
        // Default states for CRUD operations
        let mut states = vec!["created".to_string(), "active".to_string()];

        // Check for delete operations (add deleted state)
        if examples.iter().any(|e| e.operation == "delete") {
            states.push("deleted".to_string());
        }

        // Check for update operations (add updated state)
        if examples.iter().any(|e| e.operation == "update") {
            states.push("updated".to_string());
        }

        Ok(states)
    }

    /// Infer transitions from CRUD examples
    fn infer_transitions_from_crud(
        &self,
        _examples: &[&CrudExample],
        states: &[String],
    ) -> Result<Vec<StateTransition>> {
        let mut transitions = Vec::new();

        // Create -> Active
        if states.contains(&"created".to_string()) && states.contains(&"active".to_string()) {
            transitions.push(StateTransition::new("created", "active").with_probability(1.0));
        }

        // Active -> Updated
        if states.contains(&"active".to_string()) && states.contains(&"updated".to_string()) {
            transitions.push(StateTransition::new("active", "updated").with_probability(0.8));
        }

        // Updated -> Active (can revert)
        if states.contains(&"updated".to_string()) && states.contains(&"active".to_string()) {
            transitions.push(StateTransition::new("updated", "active").with_probability(0.5));
        }

        // Active -> Deleted
        if states.contains(&"active".to_string()) && states.contains(&"deleted".to_string()) {
            transitions.push(StateTransition::new("active", "deleted").with_probability(0.3));
        }

        Ok(transitions)
    }

    /// Generate system prompt from examples
    async fn generate_system_prompt(&self, examples: &[ExamplePair]) -> Result<String> {
        // Analyze examples to understand API domain
        let mut methods = std::collections::HashSet::new();
        let mut paths = std::collections::HashSet::new();

        for example in examples {
            methods.insert(example.method.clone());
            paths.insert(self.normalize_path(&example.path));
        }

        let mut prompt = String::from("You are simulating a realistic REST API. ");

        // Add method information
        if !methods.is_empty() {
            let methods_vec: Vec<&str> = methods.iter().map(|s| s.as_str()).collect();
            prompt.push_str(&format!("Supported methods: {}. ", methods_vec.join(", ")));
        }

        // Add path information
        if !paths.is_empty() {
            let paths_vec: Vec<&str> = paths.iter().take(5).map(|s| s.as_str()).collect();
            prompt.push_str(&format!("Available endpoints: {}. ", paths_vec.join(", ")));
        }

        prompt.push_str("Maintain consistency across requests and follow REST conventions.");

        // Use LLM to enhance prompt if available
        if let Some(ref llm_client) = self.llm_client {
            let enhanced = self.enhance_prompt_with_llm(&prompt, examples).await?;
            return Ok(enhanced);
        }

        Ok(prompt)
    }

    /// Generate additional rules using LLM
    async fn generate_rules_with_llm(
        &self,
        examples: &[ExamplePair],
    ) -> Result<Vec<ConsistencyRule>> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not available"))?;

        // Build prompt with examples
        let examples_json = serde_json::to_string(examples)?;
        let system_prompt = "You are a rule generation system. Analyze API examples and generate consistency rules.";
        let user_prompt = format!(
            "Analyze these API examples and suggest additional consistency rules:\n\n{}",
            examples_json
        );

        let request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3, // Lower temperature for more consistent rules
            max_tokens: 2000,
            schema: None,
        };

        let response = llm_client.generate(&request).await?;

        // Parse rules from LLM response (simplified - in production, use structured output)
        // For now, return empty vector as LLM rule parsing is complex
        Ok(Vec::new())
    }

    /// Enhance system prompt using LLM
    async fn enhance_prompt_with_llm(
        &self,
        base_prompt: &str,
        examples: &[ExamplePair],
    ) -> Result<String> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not available"))?;

        let examples_summary: Vec<String> = examples
            .iter()
            .take(10)
            .map(|e| format!("{} {} -> {}", e.method, e.path, e.status))
            .collect();

        let user_prompt = format!(
            "Based on this base prompt and API examples, generate an enhanced system prompt:\n\nBase: {}\n\nExamples:\n{}\n\nGenerate a comprehensive system prompt that describes the API behavior.",
            base_prompt,
            examples_summary.join("\n")
        );

        let request = LlmGenerationRequest {
            system_prompt: "You are a system prompt generator for API simulation.".to_string(),
            user_prompt,
            temperature: 0.7,
            max_tokens: 500,
            schema: None,
        };

        let response = llm_client.generate(&request).await?;

        // Extract text from response
        if let Some(text) = response.as_str() {
            Ok(text.to_string())
        } else {
            Ok(base_prompt.to_string())
        }
    }

    /// Determine validation type from error examples
    fn determine_validation_type(&self, errors: &[&ErrorExample]) -> Result<String> {
        // Analyze error messages and status codes
        for error in errors {
            let error_str =
                serde_json::to_string(&error.error_response).unwrap_or_default().to_lowercase();

            if error_str.contains("required") || error_str.contains("missing") {
                return Ok("required".to_string());
            }
            if error_str.contains("format") || error_str.contains("invalid format") {
                return Ok("format".to_string());
            }
            if error_str.contains("too short") || error_str.contains("minimum") {
                return Ok("min_length".to_string());
            }
            if error_str.contains("too long") || error_str.contains("maximum") {
                return Ok("max_length".to_string());
            }
            if error_str.contains("pattern") || error_str.contains("regex") {
                return Ok("pattern".to_string());
            }
        }

        // Default to required if status is 400
        if errors[0].status == 400 {
            Ok("required".to_string())
        } else {
            Ok("validation_error".to_string())
        }
    }

    /// Extract error message template
    fn extract_error_message_template(&self, errors: &[&ErrorExample]) -> Result<String> {
        // Use first error's message as template
        if let Some(error) = errors.first() {
            if let Some(message) = error.error_response.get("message").and_then(|m| m.as_str()) {
                return Ok(message.to_string());
            }
            if let Some(error_field) = error.error_response.get("error").and_then(|e| e.as_str()) {
                return Ok(error_field.to_string());
            }
        }

        Ok("Validation error".to_string())
    }

    /// Infer format from error messages
    fn infer_format_from_errors(&self, errors: &[&ErrorExample]) -> Option<String> {
        for error in errors {
            let error_str =
                serde_json::to_string(&error.error_response).unwrap_or_default().to_lowercase();

            if error_str.contains("email") {
                return Some("email".to_string());
            }
            if error_str.contains("url") {
                return Some("uri".to_string());
            }
            if error_str.contains("date") {
                return Some("date-time".to_string());
            }
            if error_str.contains("uuid") {
                return Some("uuid".to_string());
            }
        }

        None
    }

    /// Infer length constraint from errors
    fn infer_length_constraint(
        &self,
        errors: &[&ErrorExample],
        _validation_type: &str,
    ) -> Option<serde_json::Number> {
        for error in errors {
            let error_str =
                serde_json::to_string(&error.error_response).unwrap_or_default().to_lowercase();

            // Try to extract number from error message
            if let Some(num_str) =
                error_str.split_whitespace().find_map(|word| word.parse::<u64>().ok())
            {
                return Some(serde_json::Number::from(num_str));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_normalize_path() {
        let config = BehaviorModelConfig::default();
        let generator = RuleGenerator::new(config);

        assert_eq!(generator.normalize_path("/api/users/123"), "/api/users/{id}");
        assert_eq!(generator.normalize_path("/api/users"), "/api/users");
    }

    #[tokio::test]
    async fn test_infer_schema_from_value() {
        let config = BehaviorModelConfig::default();
        let generator = RuleGenerator::new(config);

        let value = json!({
            "id": "123",
            "name": "Alice",
            "age": 30,
            "active": true
        });

        let schema = generator.infer_schema_from_value(&value).unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].is_object());
    }

    #[tokio::test]
    async fn test_extract_resource_name() {
        let config = BehaviorModelConfig::default();
        let generator = RuleGenerator::new(config);

        assert_eq!(generator.extract_resource_name("/api/users"), "users");
        assert_eq!(generator.extract_resource_name("/api/users/123"), "users");
    }

    #[tokio::test]
    async fn test_determine_validation_type() {
        let config = BehaviorModelConfig::default();
        let generator = RuleGenerator::new(config);

        let errors = vec![ErrorExample {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            request: Some(json!({"name": ""})),
            status: 400,
            error_response: json!({"message": "Field is required"}),
            field: Some("email".to_string()),
        }];

        let validation_type =
            generator.determine_validation_type(&errors.iter().collect::<Vec<_>>()).unwrap();
        assert_eq!(validation_type, "required");
    }
}
