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

/// Rule type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    /// CRUD operation rule
    Crud,
    /// Validation rule
    Validation,
    /// Pagination rule
    Pagination,
    /// Consistency rule
    Consistency,
    /// State transition rule
    StateTransition,
    /// Unknown/other rule type
    Other,
}

/// Pattern match information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    /// Pattern that was matched
    pub pattern: String,
    /// Number of examples that matched this pattern
    pub match_count: usize,
    /// Example IDs that matched
    pub example_ids: Vec<String>,
}

/// Rule explanation metadata
///
/// Provides information about why and how a rule was generated,
/// including source examples, confidence scores, and reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExplanation {
    /// Unique identifier for the rule
    pub rule_id: String,
    /// Type of rule
    pub rule_type: RuleType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Source example IDs that triggered rule generation
    pub source_examples: Vec<String>,
    /// Human-readable reasoning explanation
    pub reasoning: String,
    /// Pattern matches that contributed to this rule
    pub pattern_matches: Vec<PatternMatch>,
    /// Timestamp when rule was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl RuleExplanation {
    /// Create a new rule explanation
    pub fn new(rule_id: String, rule_type: RuleType, confidence: f64, reasoning: String) -> Self {
        Self {
            rule_id,
            rule_type,
            confidence,
            source_examples: Vec::new(),
            reasoning,
            pattern_matches: Vec::new(),
            generated_at: chrono::Utc::now(),
        }
    }

    /// Add a source example
    pub fn with_source_example(mut self, example_id: String) -> Self {
        self.source_examples.push(example_id);
        self
    }

    /// Add a pattern match
    pub fn with_pattern_match(mut self, pattern_match: PatternMatch) -> Self {
        self.pattern_matches.push(pattern_match);
        self
    }
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

    /// Generate behavioral rules with explanations from example pairs
    ///
    /// Similar to `generate_rules_from_examples`, but also returns
    /// detailed explanations for each generated rule.
    pub async fn generate_rules_with_explanations(
        &self,
        examples: Vec<ExamplePair>,
    ) -> Result<(BehaviorRules, Vec<RuleExplanation>)> {
        if examples.is_empty() {
            return Ok((BehaviorRules::default(), Vec::new()));
        }

        // Generate rules first
        let rules = self.generate_rules_from_examples(examples.clone()).await?;

        // Generate explanations for each rule
        let mut explanations = Vec::new();

        // Explain consistency rules
        for (idx, rule) in rules.consistency_rules.iter().enumerate() {
            let rule_id = format!("consistency_rule_{}", idx);
            let explanation = RuleExplanation::new(
                rule_id,
                RuleType::Consistency,
                0.8, // Default confidence for consistency rules
                format!(
                    "Inferred from {} examples matching pattern: {}",
                    examples.len(),
                    rule.condition
                ),
            )
            .with_source_example(format!("example_{}", idx));
            explanations.push(explanation);
        }

        // Explain state machines
        for (resource_type, state_machine) in &rules.state_transitions {
            let rule_id = format!("state_machine_{}", resource_type);
            let explanation = RuleExplanation::new(
                rule_id,
                RuleType::StateTransition,
                0.85, // Higher confidence for state machines
                format!(
                    "State machine for {} with {} states and {} transitions inferred from CRUD patterns",
                    resource_type,
                    state_machine.states.len(),
                    state_machine.transitions.len()
                ),
            );
            explanations.push(explanation);
        }

        // Explain schemas
        for resource_name in rules.schemas.keys() {
            let rule_id = format!("schema_{}", resource_name);
            let explanation = RuleExplanation::new(
                rule_id,
                RuleType::Other,
                0.75, // Moderate confidence for inferred schemas
                format!("Schema for {} resource inferred from response examples", resource_name),
            );
            explanations.push(explanation);
        }

        Ok((rules, explanations))
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
                field_errors.entry(field.clone()).or_default().push(error);
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
                        parameters.insert(validation_type.clone(), Value::Number(length));
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
            for key in example.query_params.keys() {
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
            resource_groups.entry(example.resource_type.clone()).or_default().push(example);
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
            groups.entry(base_path).or_default().push(example);
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
        // Extract last meaningful segment, skipping numeric IDs
        let segments: Vec<&str> =
            path.split('/').filter(|s| !s.is_empty() && !s.starts_with('{')).collect();

        // Find the last non-numeric segment (resource name, not ID)
        for segment in segments.iter().rev() {
            if !segment.chars().all(|c| c.is_ascii_digit()) {
                return segment.to_string();
            }
        }

        // Fallback to last segment if all are numeric
        segments.last().map(|s| s.to_string()).unwrap_or_else(|| "Resource".to_string())
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

    #[test]
    fn test_example_pair_creation() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let pair = ExamplePair {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            request: None,
            status: 200,
            response: Some(json!({"users": []})),
            query_params,
            headers,
            metadata: HashMap::new(),
        };

        assert_eq!(pair.method, "GET");
        assert_eq!(pair.path, "/api/users");
        assert_eq!(pair.status, 200);
    }

    #[test]
    fn test_example_pair_serialization() {
        let pair = ExamplePair {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            request: Some(json!({"name": "Alice"})),
            status: 201,
            response: Some(json!({"id": 1, "name": "Alice"})),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&pair).unwrap();
        assert!(json.contains("POST"));
        assert!(json.contains("/api/users"));
    }

    #[test]
    fn test_error_example_creation() {
        let error = ErrorExample {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            request: Some(json!({"email": "invalid"})),
            status: 400,
            error_response: json!({"error": "Invalid email"}),
            field: Some("email".to_string()),
        };

        assert_eq!(error.method, "POST");
        assert_eq!(error.status, 400);
        assert_eq!(error.field, Some("email".to_string()));
    }

    #[test]
    fn test_error_example_serialization() {
        let error = ErrorExample {
            method: "PUT".to_string(),
            path: "/api/users/1".to_string(),
            request: None,
            status: 404,
            error_response: json!({"error": "Not found"}),
            field: None,
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("404"));
    }

    #[test]
    fn test_paginated_response_creation() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let response = PaginatedResponse {
            path: "/api/users".to_string(),
            query_params,
            response: json!({"data": [], "page": 1, "total": 100}),
            page: Some(1),
            page_size: Some(10),
            total: Some(100),
        };

        assert_eq!(response.path, "/api/users");
        assert_eq!(response.page, Some(1));
        assert_eq!(response.total, Some(100));
    }

    #[test]
    fn test_crud_example_creation() {
        let crud = CrudExample {
            operation: "create".to_string(),
            resource_type: "user".to_string(),
            path: "/api/users".to_string(),
            request: Some(json!({"name": "Alice"})),
            status: 201,
            response: Some(json!({"id": 1, "name": "Alice"})),
            resource_state: Some("active".to_string()),
        };

        assert_eq!(crud.operation, "create");
        assert_eq!(crud.resource_type, "user");
        assert_eq!(crud.status, 201);
    }

    #[test]
    fn test_validation_rule_creation() {
        let mut parameters = HashMap::new();
        parameters.insert("min_length".to_string(), json!(3));
        parameters.insert("max_length".to_string(), json!(50));

        let rule = ValidationRule {
            field: "username".to_string(),
            validation_type: "length".to_string(),
            parameters,
            error_message: "Username must be between 3 and 50 characters".to_string(),
            status_code: 400,
        };

        assert_eq!(rule.field, "username");
        assert_eq!(rule.validation_type, "length");
        assert_eq!(rule.status_code, 400);
    }

    #[test]
    fn test_pagination_rule_creation() {
        let mut parameter_names = HashMap::new();
        parameter_names.insert("page".to_string(), "page".to_string());
        parameter_names.insert("limit".to_string(), "limit".to_string());

        let rule = PaginationRule {
            default_page_size: 20,
            max_page_size: 100,
            min_page_size: 1,
            parameter_names,
            format: "page-based".to_string(),
        };

        assert_eq!(rule.default_page_size, 20);
        assert_eq!(rule.max_page_size, 100);
        assert_eq!(rule.format, "page-based");
    }

    #[test]
    fn test_rule_type_serialization() {
        let rule_types = vec![
            RuleType::Crud,
            RuleType::Validation,
            RuleType::Pagination,
            RuleType::Consistency,
            RuleType::StateTransition,
            RuleType::Other,
        ];

        for rule_type in rule_types {
            let json = serde_json::to_string(&rule_type).unwrap();
            assert!(!json.is_empty());
            let deserialized: RuleType = serde_json::from_str(&json).unwrap();
            assert_eq!(rule_type, deserialized);
        }
    }

    #[test]
    fn test_pattern_match_creation() {
        let pattern = PatternMatch {
            pattern: "/api/users/*".to_string(),
            match_count: 5,
            example_ids: vec!["ex1".to_string(), "ex2".to_string()],
        };

        assert_eq!(pattern.pattern, "/api/users/*");
        assert_eq!(pattern.match_count, 5);
        assert_eq!(pattern.example_ids.len(), 2);
    }

    #[test]
    fn test_rule_explanation_new() {
        let explanation = RuleExplanation::new(
            "rule-1".to_string(),
            RuleType::Consistency,
            0.85,
            "Inferred from examples".to_string(),
        );

        assert_eq!(explanation.rule_id, "rule-1");
        assert_eq!(explanation.rule_type, RuleType::Consistency);
        assert_eq!(explanation.confidence, 0.85);
        assert!(explanation.source_examples.is_empty());
    }

    #[test]
    fn test_rule_explanation_with_source_example() {
        let explanation = RuleExplanation::new(
            "rule-1".to_string(),
            RuleType::Validation,
            0.9,
            "Test reasoning".to_string(),
        )
        .with_source_example("example-1".to_string())
        .with_source_example("example-2".to_string());

        assert_eq!(explanation.source_examples.len(), 2);
        assert_eq!(explanation.source_examples[0], "example-1");
    }

    #[test]
    fn test_rule_explanation_with_pattern_match() {
        let pattern_match = PatternMatch {
            pattern: "/api/*".to_string(),
            match_count: 3,
            example_ids: vec!["ex1".to_string()],
        };

        let explanation = RuleExplanation::new(
            "rule-1".to_string(),
            RuleType::Pagination,
            0.75,
            "Test".to_string(),
        )
        .with_pattern_match(pattern_match.clone());

        assert_eq!(explanation.pattern_matches.len(), 1);
        assert_eq!(explanation.pattern_matches[0].pattern, "/api/*");
    }

    #[test]
    fn test_rule_generator_new() {
        let config = BehaviorModelConfig::default();
        let generator = RuleGenerator::new(config);
        // Just verify it can be created
        let _ = generator;
    }

    #[test]
    fn test_rule_generator_new_with_disabled_llm() {
        let mut config = BehaviorModelConfig::default();
        config.llm_provider = "disabled".to_string();
        let generator = RuleGenerator::new(config);
        // Just verify it can be created
        let _ = generator;
    }

    #[test]
    fn test_paginated_response_serialization() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "2".to_string());
        let response = PaginatedResponse {
            path: "/api/items".to_string(),
            query_params: query_params.clone(),
            response: json!({"items": []}),
            page: Some(2),
            page_size: Some(20),
            total: Some(50),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("/api/items"));
        assert!(json.contains("2"));
    }

    #[test]
    fn test_crud_example_serialization() {
        let crud = CrudExample {
            operation: "update".to_string(),
            resource_type: "order".to_string(),
            path: "/api/orders/123".to_string(),
            request: Some(json!({"status": "shipped"})),
            status: 200,
            response: Some(json!({"id": 123, "status": "shipped"})),
            resource_state: Some("shipped".to_string()),
        };

        let json = serde_json::to_string(&crud).unwrap();
        assert!(json.contains("update"));
        assert!(json.contains("order"));
    }

    #[test]
    fn test_validation_rule_serialization() {
        let mut parameters = HashMap::new();
        parameters.insert("pattern".to_string(), json!("^[a-z]+$"));
        let rule = ValidationRule {
            field: "username".to_string(),
            validation_type: "pattern".to_string(),
            parameters: parameters.clone(),
            error_message: "Invalid format".to_string(),
            status_code: 422,
        };

        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("username"));
        assert!(json.contains("pattern"));
    }

    #[test]
    fn test_pagination_rule_serialization() {
        let mut parameter_names = HashMap::new();
        parameter_names.insert("offset".to_string(), "offset".to_string());
        parameter_names.insert("limit".to_string(), "limit".to_string());
        let rule = PaginationRule {
            default_page_size: 25,
            max_page_size: 200,
            min_page_size: 5,
            parameter_names: parameter_names.clone(),
            format: "offset-based".to_string(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("offset-based"));
        assert!(json.contains("25"));
    }

    #[test]
    fn test_rule_type_variants() {
        assert_eq!(RuleType::Crud, RuleType::Crud);
        assert_eq!(RuleType::Validation, RuleType::Validation);
        assert_eq!(RuleType::Pagination, RuleType::Pagination);
        assert_eq!(RuleType::Consistency, RuleType::Consistency);
        assert_eq!(RuleType::StateTransition, RuleType::StateTransition);
        assert_eq!(RuleType::Other, RuleType::Other);
    }

    #[test]
    fn test_pattern_match_serialization() {
        let pattern = PatternMatch {
            pattern: "/api/v1/*".to_string(),
            match_count: 10,
            example_ids: vec!["ex1".to_string(), "ex2".to_string(), "ex3".to_string()],
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("/api/v1/*"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_rule_explanation_serialization() {
        let explanation = RuleExplanation::new(
            "rule-123".to_string(),
            RuleType::Consistency,
            0.92,
            "High confidence rule".to_string(),
        )
        .with_source_example("ex1".to_string())
        .with_pattern_match(PatternMatch {
            pattern: "/api/*".to_string(),
            match_count: 5,
            example_ids: vec!["ex1".to_string()],
        });

        let json = serde_json::to_string(&explanation).unwrap();
        assert!(json.contains("rule-123"));
        assert!(json.contains("0.92"));
        assert!(json.contains("High confidence"));
    }

    #[test]
    fn test_error_example_with_field() {
        let error = ErrorExample {
            method: "PATCH".to_string(),
            path: "/api/users/1".to_string(),
            request: Some(json!({"email": "invalid-email"})),
            status: 422,
            error_response: json!({"field": "email", "message": "Invalid email format"}),
            field: Some("email".to_string()),
        };

        assert_eq!(error.field, Some("email".to_string()));
        assert_eq!(error.status, 422);
    }

    #[test]
    fn test_error_example_without_field() {
        let error = ErrorExample {
            method: "DELETE".to_string(),
            path: "/api/users/999".to_string(),
            request: None,
            status: 404,
            error_response: json!({"error": "Resource not found"}),
            field: None,
        };

        assert!(error.field.is_none());
        assert_eq!(error.status, 404);
    }

    #[test]
    fn test_paginated_response_without_pagination_info() {
        let response = PaginatedResponse {
            path: "/api/data".to_string(),
            query_params: HashMap::new(),
            response: json!({"data": []}),
            page: None,
            page_size: None,
            total: None,
        };

        assert!(response.page.is_none());
        assert!(response.page_size.is_none());
        assert!(response.total.is_none());
    }

    #[test]
    fn test_crud_example_without_state() {
        let crud = CrudExample {
            operation: "read".to_string(),
            resource_type: "product".to_string(),
            path: "/api/products/1".to_string(),
            request: None,
            status: 200,
            response: Some(json!({"id": 1, "name": "Product"})),
            resource_state: None,
        };

        assert!(crud.resource_state.is_none());
        assert_eq!(crud.operation, "read");
    }

    #[test]
    fn test_validation_rule_without_parameters() {
        let rule = ValidationRule {
            field: "required_field".to_string(),
            validation_type: "required".to_string(),
            parameters: HashMap::new(),
            error_message: "Field is required".to_string(),
            status_code: 400,
        };

        assert!(rule.parameters.is_empty());
        assert_eq!(rule.validation_type, "required");
    }

    #[test]
    fn test_rule_explanation_with_multiple_pattern_matches() {
        let explanation = RuleExplanation::new(
            "rule-456".to_string(),
            RuleType::StateTransition,
            0.88,
            "Complex rule".to_string(),
        )
        .with_pattern_match(PatternMatch {
            pattern: "/api/v1/*".to_string(),
            match_count: 3,
            example_ids: vec![],
        })
        .with_pattern_match(PatternMatch {
            pattern: "/api/v2/*".to_string(),
            match_count: 2,
            example_ids: vec![],
        });

        assert_eq!(explanation.pattern_matches.len(), 2);
    }

    #[test]
    fn test_example_pair_clone() {
        let pair1 = ExamplePair {
            method: "GET".to_string(),
            path: "/test".to_string(),
            request: None,
            status: 200,
            response: Some(json!({})),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        };
        let pair2 = pair1.clone();
        assert_eq!(pair1.method, pair2.method);
    }

    #[test]
    fn test_example_pair_debug() {
        let pair = ExamplePair {
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            request: Some(json!({"data": "test"})),
            status: 201,
            response: Some(json!({"id": 1})),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        };
        let debug_str = format!("{:?}", pair);
        assert!(debug_str.contains("ExamplePair"));
    }

    #[test]
    fn test_error_example_clone() {
        let error1 = ErrorExample {
            method: "PATCH".to_string(),
            path: "/test".to_string(),
            request: None,
            status: 400,
            error_response: json!({"error": "Bad request"}),
            field: None,
        };
        let error2 = error1.clone();
        assert_eq!(error1.status, error2.status);
    }

    #[test]
    fn test_error_example_debug() {
        let error = ErrorExample {
            method: "PUT".to_string(),
            path: "/api/users/1".to_string(),
            request: Some(json!({"email": "invalid"})),
            status: 422,
            error_response: json!({"field": "email", "message": "Invalid"}),
            field: Some("email".to_string()),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ErrorExample"));
    }

    #[test]
    fn test_paginated_response_clone() {
        let response1 = PaginatedResponse {
            path: "/api/data".to_string(),
            query_params: HashMap::new(),
            response: json!({}),
            page: Some(1),
            page_size: Some(10),
            total: Some(100),
        };
        let response2 = response1.clone();
        assert_eq!(response1.page, response2.page);
    }

    #[test]
    fn test_paginated_response_debug() {
        let response = PaginatedResponse {
            path: "/api/users".to_string(),
            query_params: HashMap::from([("page".to_string(), "1".to_string())]),
            response: json!({"data": []}),
            page: Some(1),
            page_size: Some(20),
            total: Some(50),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("PaginatedResponse"));
    }

    #[test]
    fn test_crud_example_clone() {
        let crud1 = CrudExample {
            operation: "create".to_string(),
            resource_type: "user".to_string(),
            path: "/api/users".to_string(),
            request: None,
            status: 201,
            response: None,
            resource_state: None,
        };
        let crud2 = crud1.clone();
        assert_eq!(crud1.operation, crud2.operation);
    }

    #[test]
    fn test_crud_example_debug() {
        let crud = CrudExample {
            operation: "update".to_string(),
            resource_type: "product".to_string(),
            path: "/api/products/1".to_string(),
            request: Some(json!({"name": "New Name"})),
            status: 200,
            response: Some(json!({"id": 1, "name": "New Name"})),
            resource_state: Some("updated".to_string()),
        };
        let debug_str = format!("{:?}", crud);
        assert!(debug_str.contains("CrudExample"));
    }

    #[test]
    fn test_validation_rule_clone() {
        let rule1 = ValidationRule {
            field: "email".to_string(),
            validation_type: "format".to_string(),
            parameters: HashMap::new(),
            error_message: "Invalid format".to_string(),
            status_code: 400,
        };
        let rule2 = rule1.clone();
        assert_eq!(rule1.field, rule2.field);
    }

    #[test]
    fn test_validation_rule_debug() {
        let mut parameters = HashMap::new();
        parameters.insert("pattern".to_string(), json!(r"^[a-z]+$"));
        let rule = ValidationRule {
            field: "username".to_string(),
            validation_type: "pattern".to_string(),
            parameters,
            error_message: "Invalid pattern".to_string(),
            status_code: 422,
        };
        let debug_str = format!("{:?}", rule);
        assert!(debug_str.contains("ValidationRule"));
    }

    #[test]
    fn test_pagination_rule_clone() {
        let rule1 = PaginationRule {
            default_page_size: 20,
            max_page_size: 100,
            min_page_size: 1,
            parameter_names: HashMap::new(),
            format: "page-based".to_string(),
        };
        let rule2 = rule1.clone();
        assert_eq!(rule1.default_page_size, rule2.default_page_size);
    }

    #[test]
    fn test_pagination_rule_debug() {
        let mut parameter_names = HashMap::new();
        parameter_names.insert("page".to_string(), "page".to_string());
        parameter_names.insert("size".to_string(), "limit".to_string());
        let rule = PaginationRule {
            default_page_size: 25,
            max_page_size: 200,
            min_page_size: 5,
            parameter_names,
            format: "offset-based".to_string(),
        };
        let debug_str = format!("{:?}", rule);
        assert!(debug_str.contains("PaginationRule"));
    }

    #[test]
    fn test_rule_type_clone() {
        let rule_type1 = RuleType::Validation;
        let rule_type2 = rule_type1.clone();
        assert_eq!(rule_type1, rule_type2);
    }

    #[test]
    fn test_rule_type_debug() {
        let rule_type = RuleType::StateTransition;
        let debug_str = format!("{:?}", rule_type);
        assert!(debug_str.contains("StateTransition") || debug_str.contains("RuleType"));
    }

    #[test]
    fn test_pattern_match_clone() {
        let pattern1 = PatternMatch {
            pattern: "/api/*".to_string(),
            match_count: 10,
            example_ids: vec!["ex1".to_string()],
        };
        let pattern2 = pattern1.clone();
        assert_eq!(pattern1.pattern, pattern2.pattern);
    }

    #[test]
    fn test_pattern_match_debug() {
        let pattern = PatternMatch {
            pattern: "/api/v1/users/*".to_string(),
            match_count: 15,
            example_ids: vec!["ex1".to_string(), "ex2".to_string(), "ex3".to_string()],
        };
        let debug_str = format!("{:?}", pattern);
        assert!(debug_str.contains("PatternMatch"));
    }

    #[test]
    fn test_rule_explanation_clone() {
        let explanation1 = RuleExplanation::new(
            "rule-1".to_string(),
            RuleType::Consistency,
            0.95,
            "Test rule".to_string(),
        );
        let explanation2 = explanation1.clone();
        assert_eq!(explanation1.rule_id, explanation2.rule_id);
    }

    #[test]
    fn test_rule_explanation_debug() {
        let explanation = RuleExplanation::new(
            "rule-123".to_string(),
            RuleType::Validation,
            0.88,
            "Validation rule".to_string(),
        )
        .with_source_example("ex-1".to_string())
        .with_pattern_match(PatternMatch {
            pattern: "/api/*".to_string(),
            match_count: 5,
            example_ids: vec![],
        });
        let debug_str = format!("{:?}", explanation);
        assert!(debug_str.contains("RuleExplanation"));
    }
}
