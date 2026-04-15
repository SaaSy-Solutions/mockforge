//! Data types used by the rule generator
//!
//! Extracted from `mockforge-core::intelligent_behavior::rule_generator` so
//! consumers (http, ui) can reference rule metadata without depending on
//! deprecated core modules. The `RuleGenerator` impl stays in core because it
//! depends on `LlmClient`.

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
