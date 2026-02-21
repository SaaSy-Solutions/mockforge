//! Response generation trace for debugging and observability
//!
//! This module provides structures to track how responses are generated,
//! including template selection, persona graph usage, rules/hooks execution,
//! and template expansion details.

use crate::openapi::response_selection::ResponseSelectionMode;
use crate::schema_diff::ValidationError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Response generation trace
///
/// Captures detailed information about how a response was generated,
/// enabling users to understand "why did I get this response?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseGenerationTrace {
    /// Selected template or fixture path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_path: Option<String>,

    /// Selected fixture path (if using fixtures)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture_path: Option<String>,

    /// Response selection mode used
    pub response_selection_mode: ResponseSelectionMode,

    /// Selected example/scenario name (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_example: Option<String>,

    /// Persona graph nodes used in response generation
    #[serde(default)]
    pub persona_graph_nodes: Vec<PersonaGraphNodeUsage>,

    /// Rules/hook scripts that fired during generation
    #[serde(default)]
    pub rules_executed: Vec<RuleExecution>,

    /// Template expansion steps
    #[serde(default)]
    pub template_expansions: Vec<TemplateExpansion>,

    /// Reality blending decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blending_decision: Option<BlendingDecision>,

    /// Final resolved response payload (after all transformations)
    ///
    /// This is the complete response body that was sent to the client,
    /// after all template expansions, persona graph enrichments, and
    /// rule/hook modifications have been applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_payload: Option<Value>,

    /// Schema validation diff results
    ///
    /// Contains validation errors if the final payload doesn't match
    /// the expected contract schema. Empty vector means the payload
    /// is valid according to the schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_validation_diff: Option<Vec<ValidationError>>,

    /// Additional metadata about the generation process
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl Default for ResponseGenerationTrace {
    fn default() -> Self {
        Self {
            template_path: None,
            fixture_path: None,
            response_selection_mode: ResponseSelectionMode::First,
            selected_example: None,
            persona_graph_nodes: Vec::new(),
            rules_executed: Vec::new(),
            template_expansions: Vec::new(),
            blending_decision: None,
            final_payload: None,
            schema_validation_diff: None,
            metadata: HashMap::new(),
        }
    }
}

/// Persona graph node usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaGraphNodeUsage {
    /// Persona ID
    pub persona_id: String,

    /// Entity type (e.g., "user", "order", "payment")
    pub entity_type: String,

    /// How this node was used (e.g., "data_source", "relationship_traversal")
    pub usage_type: String,

    /// Relationship path traversed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_path: Option<Vec<String>>,
}

/// Rule or hook script execution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExecution {
    /// Rule or hook name
    pub name: String,

    /// Rule type (e.g., "hook", "consistency_rule", "mutation_rule")
    pub rule_type: String,

    /// Whether the rule condition matched
    pub condition_matched: bool,

    /// Actions executed by the rule
    #[serde(default)]
    pub actions_executed: Vec<String>,

    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,

    /// Error message (if execution failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Template expansion step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateExpansion {
    /// Template expression that was expanded (e.g., "{{user.name}}")
    pub template: String,

    /// Expanded value
    pub value: Value,

    /// Source of the value (e.g., "persona", "faker", "context")
    pub source: String,

    /// Step number in the expansion sequence
    pub step: usize,
}

/// Reality blending decision information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendingDecision {
    /// Blend ratio used (0.0 = mock, 1.0 = real)
    pub blend_ratio: f64,

    /// Source of the blend ratio (e.g., "global", "route_rule", "time_schedule")
    pub ratio_source: String,

    /// Whether blending was actually performed
    pub blended: bool,

    /// Merge strategy used (if blended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_strategy: Option<String>,

    /// Field-level blending decisions (if applicable)
    #[serde(default)]
    pub field_decisions: Vec<FieldBlendingDecision>,
}

/// Field-level blending decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldBlendingDecision {
    /// JSON path to the field
    pub field_path: String,

    /// Blend ratio used for this field
    pub field_ratio: f64,

    /// Source of the field value (e.g., "mock", "real", "blended")
    pub value_source: String,
}

impl ResponseGenerationTrace {
    /// Create a new empty trace
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a persona graph node usage
    pub fn add_persona_node(&mut self, usage: PersonaGraphNodeUsage) {
        self.persona_graph_nodes.push(usage);
    }

    /// Add a rule execution
    pub fn add_rule_execution(&mut self, execution: RuleExecution) {
        self.rules_executed.push(execution);
    }

    /// Add a template expansion step
    pub fn add_template_expansion(&mut self, expansion: TemplateExpansion) {
        self.template_expansions.push(expansion);
    }

    /// Set the blending decision
    pub fn set_blending_decision(&mut self, decision: BlendingDecision) {
        self.blending_decision = Some(decision);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }

    /// Set the final resolved payload
    pub fn set_final_payload(&mut self, payload: Value) {
        self.final_payload = Some(payload);
    }

    /// Set the schema validation diff results
    pub fn set_schema_validation_diff(&mut self, diff: Vec<ValidationError>) {
        self.schema_validation_diff = Some(diff);
    }
}
