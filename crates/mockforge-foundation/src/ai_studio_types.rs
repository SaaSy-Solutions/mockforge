//! AI Studio data types
//!
//! Pure data types extracted from `mockforge-core::ai_studio::*`. The engines
//! (`SystemGenerator`, `ArtifactFreezer`, `ApiCritiqueEngine`,
//! `BehavioralSimulator`) stay in core because they hold `LlmClient` and
//! perform I/O; only the request/response/result data types live here so
//! consumers (http handlers, ui) can reference them without depending on
//! deprecated core modules.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// System generator types (from system_generator.rs)
// ============================================================================

/// Request for system generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemGenerationRequest {
    /// Natural language description of the system to generate
    pub description: String,
    /// Output formats to generate
    /// Valid values: "openapi", "graphql", "personas", "lifecycles", "websocket", "chaos", "ci"
    #[serde(default)]
    pub output_formats: Vec<String>,
    /// Optional workspace ID
    pub workspace_id: Option<String>,
    /// Optional system ID (for versioning - if provided, creates new version)
    pub system_id: Option<String>,
}

/// Generated system with all artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSystem {
    /// System ID
    pub system_id: String,
    /// Version (v1, v2, etc.)
    pub version: String,
    /// Generated artifacts by type
    pub artifacts: HashMap<String, SystemArtifact>,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Status: "draft" or "frozen"
    pub status: String,
    /// Token usage for this generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,
    /// Estimated cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Generation metadata
    pub metadata: SystemMetadata,
}

/// Result of applying a system design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedSystem {
    /// System ID
    pub system_id: String,
    /// Version
    pub version: String,
    /// Artifact IDs that were applied
    pub applied_artifacts: Vec<String>,
    /// Whether artifacts were frozen
    pub frozen: bool,
}

/// System artifact (OpenAPI spec, persona, lifecycle, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemArtifact {
    /// Artifact type: "openapi", "persona", "lifecycle", "websocket", "chaos", "ci", "graphql", "typings"
    pub artifact_type: String,
    /// Artifact content (JSON or YAML string)
    pub content: Value,
    /// Artifact format: "json" or "yaml"
    pub format: String,
    /// Artifact ID
    pub artifact_id: String,
}

/// System generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetadata {
    /// Original description
    pub description: String,
    /// Detected entities from description
    pub entities: Vec<String>,
    /// Detected relationships
    pub relationships: Vec<String>,
    /// Detected operations
    pub operations: Vec<String>,
    /// Generated at timestamp
    pub generated_at: String,
}

// ============================================================================
// Artifact freezer types (from artifact_freezer.rs)
// ============================================================================

/// Request to freeze an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRequest {
    /// Type of artifact (mock, persona, scenario, etc.)
    pub artifact_type: String,
    /// Artifact content
    pub content: Value,
    /// Output format (yaml or json)
    pub format: String,
    /// Output path
    pub path: Option<String>,
    /// Optional metadata for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FreezeMetadata>,
}

/// Metadata for frozen artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeMetadata {
    /// LLM provider used (e.g., "openai", "anthropic", "ollama")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,
    /// LLM model used (e.g., "gpt-4", "claude-3-opus")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    /// LLM version (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_version: Option<String>,
    /// Hash of the input prompt/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
    /// Hash of the output content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
    /// Original prompt/description (optional, for reference)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_prompt: Option<String>,
}

/// Frozen artifact result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenArtifact {
    /// Type of artifact
    pub artifact_type: String,
    /// Frozen content
    pub content: Value,
    /// Output format
    pub format: String,
    /// File path where artifact was saved
    pub path: String,
    /// Metadata used for freezing (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FreezeMetadata>,
    /// Output hash for integrity verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
}

// ============================================================================
// API critique types (from api_critique.rs)
// ============================================================================

/// Request for API critique analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueRequest {
    /// API schema (OpenAPI JSON, GraphQL SDL, or Protobuf)
    pub schema: Value,
    /// Schema type: "openapi", "graphql", or "protobuf"
    pub schema_type: String,
    /// Optional focus areas for analysis
    /// Valid values: "anti-patterns", "redundancy", "naming", "tone", "restructuring"
    #[serde(default)]
    pub focus_areas: Vec<String>,
    /// Optional workspace ID for context
    pub workspace_id: Option<String>,
}

/// API critique result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCritique {
    /// Detected anti-patterns
    pub anti_patterns: Vec<AntiPattern>,
    /// Detected redundancies
    pub redundancies: Vec<Redundancy>,
    /// Naming quality issues
    pub naming_issues: Vec<NamingIssue>,
    /// Emotional tone analysis
    pub tone_analysis: ToneAnalysis,
    /// Restructuring recommendations
    pub restructuring: RestructuringRecommendations,
    /// Overall score (0-100, higher is better)
    pub overall_score: f64,
    /// Summary of findings
    pub summary: String,
    /// Token usage for this critique
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,
    /// Estimated cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Detected anti-pattern in API design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    /// Type of anti-pattern (e.g., "rest_violation", "inconsistent_naming", "poor_resource_modeling")
    pub pattern_type: String,
    /// Severity: "low", "medium", "high", "critical"
    pub severity: String,
    /// Location in schema (path, endpoint, etc.)
    pub location: String,
    /// Description of the issue
    pub description: String,
    /// Suggested fix
    pub suggestion: String,
    /// Example of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// Detected redundancy in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redundancy {
    /// Type of redundancy (e.g., "duplicate_endpoint", "overlapping_functionality")
    pub redundancy_type: String,
    /// Severity: "low", "medium", "high"
    pub severity: String,
    /// Affected endpoints/resources
    pub affected_items: Vec<String>,
    /// Description of the redundancy
    pub description: String,
    /// Suggested consolidation
    pub suggestion: String,
}

/// Naming quality issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingIssue {
    /// Type of naming issue (e.g., "inconsistent_convention", "unclear_name", "abbreviation")
    pub issue_type: String,
    /// Severity: "low", "medium", "high"
    pub severity: String,
    /// Location (field name, endpoint name, etc.)
    pub location: String,
    /// Current name
    pub current_name: String,
    /// Description of the issue
    pub description: String,
    /// Suggested improvement
    pub suggestion: String,
}

/// Emotional tone analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneAnalysis {
    /// Overall tone assessment
    pub overall_tone: String,
    /// Issues found in error messages
    pub error_message_issues: Vec<ToneIssue>,
    /// Issues found in user-facing text
    pub user_facing_issues: Vec<ToneIssue>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Tone issue in API text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneIssue {
    /// Type of tone issue (e.g., "too_vague", "too_technical", "unfriendly")
    pub issue_type: String,
    /// Severity: "low", "medium", "high"
    pub severity: String,
    /// Location (error message, description, etc.)
    pub location: String,
    /// Current text
    pub current_text: String,
    /// Description of the issue
    pub description: String,
    /// Suggested improvement
    pub suggestion: String,
}

/// Restructuring recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestructuringRecommendations {
    /// Recommended resource hierarchy improvements
    pub hierarchy_improvements: Vec<HierarchyImprovement>,
    /// Consolidation opportunities
    pub consolidation_opportunities: Vec<ConsolidationOpportunity>,
    /// Resource modeling suggestions
    pub resource_modeling: Vec<ResourceModelingSuggestion>,
    /// Overall restructuring priority: "low", "medium", "high"
    pub priority: String,
}

/// Hierarchy improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyImprovement {
    /// Current structure
    pub current: String,
    /// Suggested structure
    pub suggested: String,
    /// Rationale
    pub rationale: String,
    /// Impact: "low", "medium", "high"
    pub impact: String,
}

/// Consolidation opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationOpportunity {
    /// Items that can be consolidated
    pub items: Vec<String>,
    /// Description of the opportunity
    pub description: String,
    /// Suggested consolidation approach
    pub suggestion: String,
    /// Benefits of consolidation
    pub benefits: Vec<String>,
}

/// Resource modeling suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceModelingSuggestion {
    /// Current modeling approach
    pub current: String,
    /// Suggested modeling approach
    pub suggested: String,
    /// Rationale
    pub rationale: String,
}

// ============================================================================
// Behavioral simulator types (from behavioral_simulator.rs)
// ============================================================================

/// Request to create a narrative agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    /// Optional: Attach to existing persona ID
    pub persona_id: Option<String>,
    /// Optional: Behavior policy type (e.g., "bargain-hunter", "power-user", "churn-risk")
    pub behavior_policy: Option<String>,
    /// If true, generate new persona if persona_id is not provided or doesn't exist
    pub generate_persona: bool,
    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Narrative agent that models user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeAgent {
    /// Agent ID
    pub agent_id: String,
    /// Persona ID (links to existing persona or new generated one)
    pub persona_id: String,
    /// Current intention
    pub current_intention: Intention,
    /// Session history of interactions
    pub session_history: Vec<Interaction>,
    /// Behavioral traits
    pub behavioral_traits: BehavioralTraits,
    /// Current app state awareness
    pub state_awareness: AppState,
    /// Behavior policy attached to persona
    pub behavior_policy: BehaviorPolicy,
    /// Created at timestamp
    pub created_at: String,
}

/// User intention types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intention {
    /// Explore products/content
    Browse,
    /// Actively looking to purchase
    Shop,
    /// Ready to complete purchase
    Buy,
    /// Leave due to frustration/error
    Abandon,
    /// Retry after error
    Retry,
    /// Move to different section
    Navigate,
    /// Search for something
    Search,
    /// Compare options
    Compare,
    /// Review/read content
    Review,
}

/// Behavioral traits for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralTraits {
    /// Patience level (0.0-1.0, higher = more patient)
    pub patience: f64,
    /// Price sensitivity (0.0-1.0, higher = more price-sensitive)
    pub price_sensitivity: f64,
    /// Risk tolerance (0.0-1.0, higher = more risk-tolerant)
    pub risk_tolerance: f64,
    /// Technical proficiency (0.0-1.0, higher = more technical)
    pub technical_proficiency: f64,
    /// Engagement level (0.0-1.0, higher = more engaged)
    pub engagement_level: f64,
}

/// Behavior policy attached to persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPolicy {
    /// Policy type (e.g., "bargain-hunter", "power-user", "churn-risk")
    pub policy_type: String,
    /// Policy description
    pub description: String,
    /// Policy rules/behaviors
    pub rules: Vec<PolicyRule>,
}

/// Policy rule for behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Condition that triggers this rule
    pub condition: String,
    /// Action to take
    pub action: String,
    /// Priority (higher = more important)
    pub priority: i32,
}

/// App state awareness
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    /// Current page/section
    pub current_page: Option<String>,
    /// Cart state
    pub cart: CartState,
    /// Authentication state
    pub authenticated: bool,
    /// Recent errors encountered
    pub recent_errors: Vec<ErrorEncounter>,
    /// Current context
    pub context: HashMap<String, Value>,
}

/// Cart state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CartState {
    /// Whether cart is empty
    pub is_empty: bool,
    /// Number of items
    pub item_count: usize,
    /// Total value
    pub total_value: f64,
    /// Items in cart
    pub items: Vec<CartItem>,
}

/// Cart item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    /// Item ID
    pub item_id: String,
    /// Item name
    pub name: String,
    /// Price
    pub price: f64,
    /// Quantity
    pub quantity: usize,
}

/// Error encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEncounter {
    /// Error type (e.g., "500", "timeout", "validation_error")
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Timestamp
    pub timestamp: String,
    /// Number of times encountered
    pub count: usize,
}

/// Interaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// Interaction timestamp
    pub timestamp: String,
    /// Action taken
    pub action: String,
    /// Intention at time of action
    pub intention: Intention,
    /// Request details
    pub request: Option<Value>,
    /// Response details
    pub response: Option<Value>,
    /// Result (success, error, etc.)
    pub result: String,
}

/// Request to simulate behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateBehaviorRequest {
    /// Optional: Use existing agent ID
    pub agent_id: Option<String>,
    /// Optional: Attach to existing persona
    pub persona_id: Option<String>,
    /// Current app state
    pub current_state: AppState,
    /// Trigger event (e.g., "error_500", "cart_empty", "payment_failed")
    pub trigger_event: Option<String>,
    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Response from behavior simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateBehaviorResponse {
    /// Next action to take
    pub next_action: NextAction,
    /// New intention
    pub intention: Intention,
    /// Reasoning for the action
    pub reasoning: String,
    /// Updated agent state
    pub agent: Option<NarrativeAgent>,
    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,
    /// Estimated cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Next action to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    /// Action type (e.g., "GET", "POST", "navigate", "abandon")
    pub action_type: String,
    /// Target endpoint or page
    pub target: String,
    /// Optional request body
    pub body: Option<Value>,
    /// Optional query parameters
    pub query_params: Option<HashMap<String, String>>,
    /// Delay before action (ms)
    pub delay_ms: Option<u64>,
}
