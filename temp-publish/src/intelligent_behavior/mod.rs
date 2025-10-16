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
pub mod config;
pub mod context;
pub mod embedding_client;
pub mod llm_client;
pub mod memory;
pub mod rules;
pub mod session;
pub mod spec_suggestion;
pub mod types;

// Re-export main types
pub use behavior::BehaviorModel;
pub use config::IntelligentBehaviorConfig;
pub use context::StatefulAiContext;
pub use memory::VectorMemoryStore;
pub use rules::{ConsistencyRule, RuleAction, StateMachine, StateTransition};
pub use session::{SessionManager, SessionTracking};
pub use spec_suggestion::{
    EndpointSuggestion, OutputFormat, ParameterInfo, SpecSuggestionEngine, SuggestionConfig,
    SuggestionInput, SuggestionMetadata, SuggestionResult,
};
pub use types::{BehaviorRules, InteractionRecord};
