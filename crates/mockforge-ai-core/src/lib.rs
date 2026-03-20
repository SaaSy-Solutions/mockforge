//! Core AI/LLM Infrastructure for MockForge
//!
//! This crate provides the foundational AI and LLM infrastructure for MockForge's
//! intelligent mock behavior system. It includes:
//!
//! - **LLM Client**: Provider abstraction for OpenAI, Anthropic, Ollama, and compatible APIs
//! - **Stateful Context**: Session state management for context-aware responses
//! - **Vector Memory**: Semantic search over past interactions using embeddings
//! - **Consistency Rules**: Rule evaluation engine and state machines
//! - **Behavior Model**: LLM-powered decision making for mock responses
//! - **Mutation Analysis**: Schema mutation detection and validation
//! - **Pagination Intelligence**: Context-aware pagination generation
//! - **Rule Generation**: Auto-inference of rules from examples
//! - **Validation Generation**: AI-driven validation error generation
//!
//! # Architecture
//!
//! ```text
//! Request -> Context Manager -> Behavior Model -> LLM + Vector Store -> Response
//!              |                    |                    |
//!         Session State      Consistency Rules    Past Interactions
//! ```

pub mod behavior;
pub mod cache;
pub mod condition_evaluator;
pub mod config;
pub mod context;
pub mod embedding_client;
pub mod error;
pub mod history;
pub mod llm_client;
pub mod memory;
pub mod mutation_analyzer;
pub mod pagination_intelligence;
pub mod rule_generator;
pub mod rules;
pub mod session;
pub mod sub_scenario;
pub mod types;
pub mod validation_generator;
pub mod visual_layout;

// Re-export error types
pub use error::{Error, Result};

// Re-export main types
pub use behavior::BehaviorModel;
pub use cache::{generate_cache_key, ResponseCache};
pub use condition_evaluator::{ConditionError, ConditionEvaluator, ConditionResult};
pub use config::{
    BehaviorModelConfig, IntelligentBehaviorConfig, PerformanceConfig, Persona, PersonasConfig,
    VectorStoreConfig,
};
pub use context::StatefulAiContext;
pub use embedding_client::{cosine_similarity, EmbeddingClient};
pub use history::HistoryManager;
pub use llm_client::{LlmClient, LlmUsage};
pub use memory::VectorMemoryStore;
pub use mutation_analyzer::{
    ChangeType, FieldChange, MutationAnalysis, MutationAnalyzer, MutationType, ResponseType,
    ValidationIssue, ValidationIssueType, ValidationSeverity,
};
pub use pagination_intelligence::{
    PaginationFormat, PaginationIntelligence, PaginationMetadata, PaginationRequest, PaginationRule,
};
pub use rule_generator::{
    CrudExample, ErrorExample, ExamplePair, PaginatedResponse, PatternMatch, RuleExplanation,
    RuleGenerator, RuleType, ValidationRule,
};
pub use rules::{ConsistencyRule, EvaluationContext, RuleAction, StateMachine, StateTransition};
pub use session::{SessionManager, SessionTracking};
pub use sub_scenario::SubScenario;
pub use types::{BehaviorRules, InteractionRecord, LlmGenerationRequest, SessionState};
pub use validation_generator::{
    ErrorFormat, FieldError, RequestContext, ValidationErrorExample, ValidationErrorResponse,
    ValidationGenerator,
};
pub use visual_layout::{Viewport, VisualEdge, VisualLayout, VisualNode};
