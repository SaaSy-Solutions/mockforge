//! Intelligent Mock Behavior System
//!
//! This module provides LLM-powered stateful mock behavior that maintains
//! consistency across multiple API requests, simulating a real, thinking backend.
//!
//! # Features
//!
//! - **Stateful Context Management**: Tracks state across requests using sessions
//! - **LLM-Powered Decision Making**: Uses AI to generate intelligent, context-aware responses
//! - **Vector Memory**: Semantic search over past interactions for long-term memory
//! - **Consistency Rules**: Enforces logical behavior patterns (e.g., auth requirements)
//! - **State Machines**: Resources follow realistic lifecycle transitions
//!
//! # Architecture
//!
//! ```text
//! Request → Context Manager → Behavior Model → LLM + Vector Store → Response
//!              ↓                    ↓                    ↓
//!         Session State      Consistency Rules    Past Interactions
//! ```
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::intelligent_behavior::{
//!     StatefulAiContext, BehaviorModel, IntelligentBehaviorConfig,
//! };
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! // Create a stateful context
//! let config = IntelligentBehaviorConfig::default();
//! let mut context = StatefulAiContext::new("session_123", config);
//!
//! // Record an interaction
//! context.record_interaction(
//!     "POST",
//!     "/api/users",
//!     Some(serde_json::json!({"name": "Alice"})),
//!     Some(serde_json::json!({"id": "user_1", "name": "Alice"})),
//! ).await?;
//!
//! // Get current state
//! let state = context.get_state();
//! # Ok(())
//! # }
//! ```

pub mod behavior;
pub mod cache;
pub mod condition_evaluator;
pub mod config;
pub mod context;
pub mod embedding_client;
pub mod history;
pub mod llm_client;
pub mod memory;
pub mod mockai;
pub mod mutation_analyzer;
pub mod openapi_generator;
pub mod pagination_intelligence;
pub mod relationship_inference;
pub mod rule_generator;
pub mod rules;
pub mod session;
pub mod spec_suggestion;
pub mod sub_scenario;
pub mod types;
pub mod validation_generator;
pub mod visual_layout;

#[cfg(test)]
mod persona_integration_test;

// Re-export main types
pub use behavior::BehaviorModel;
pub use condition_evaluator::{ConditionError, ConditionEvaluator, ConditionResult};
pub use config::{IntelligentBehaviorConfig, Persona, PersonasConfig};
pub use context::StatefulAiContext;
pub use history::HistoryManager;
pub use memory::VectorMemoryStore;
pub use mockai::{MockAI, Request, Response};
pub use mutation_analyzer::{
    ChangeType, FieldChange, MutationAnalysis, MutationAnalyzer, MutationType, ResponseType,
    ValidationIssue, ValidationIssueType, ValidationSeverity,
};
pub use openapi_generator::{
    ConfidenceScore, HttpExchange, OpenApiGenerationConfig, OpenApiGenerationMetadata,
    OpenApiGenerationResult, OpenApiSpecGenerator,
};
pub use pagination_intelligence::{
    PaginationFormat, PaginationIntelligence, PaginationMetadata, PaginationRequest, PaginationRule,
};
pub use relationship_inference::{Relationship, RelationshipInference};
pub use rule_generator::{
    CrudExample, ErrorExample, ExamplePair, PaginatedResponse, PatternMatch, RuleExplanation,
    RuleGenerator, RuleType, ValidationRule,
};
pub use rules::{ConsistencyRule, RuleAction, StateMachine, StateTransition};
pub use session::{SessionManager, SessionTracking};
pub use spec_suggestion::{
    EndpointSuggestion, OutputFormat, ParameterInfo, SpecSuggestionEngine, SuggestionConfig,
    SuggestionInput, SuggestionMetadata, SuggestionResult,
};
pub use sub_scenario::SubScenario;
pub use types::{BehaviorRules, InteractionRecord};
pub use validation_generator::{
    ErrorFormat, FieldError, RequestContext, ValidationErrorExample, ValidationErrorResponse,
    ValidationGenerator,
};
pub use visual_layout::{Viewport, VisualEdge, VisualLayout, VisualNode};
