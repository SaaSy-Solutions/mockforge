//! MockForge AI Studio - Unified AI Copilot
//!
//! This module provides a unified interface for all AI-powered features in MockForge,
//! including natural language mock generation, AI-guided debugging, persona generation,
//! and artifact freezing for deterministic testing.
//!
//! # Features
//!
//! - **Natural Language Mock Generation**: Generate mocks from conversational descriptions
//! - **AI-Guided Debugging**: Analyze test failures and suggest fixes
//! - **Persona Generation**: Create and tweak personas using AI
//! - **Artifact Freezing**: Convert AI outputs to deterministic YAML/JSON
//! - **Cost & Budget Management**: Track tokens and enforce budgets
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::ai_studio::{ChatOrchestrator, ChatRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! let config = IntelligentBehaviorConfig::default();
//! let orchestrator = ChatOrchestrator::new(config);
//!
//! // Process a natural language command
//! let request = ChatRequest {
//!     message: "Create a user API with CRUD operations".to_string(),
//!     context: None,
//! };
//! let response = orchestrator.process(&request).await?;
//! # Ok(())
//! # }
//! ```

pub mod artifact_freezer;
pub mod budget_manager;
pub mod chat_orchestrator;
pub mod config;
pub mod conversation_store;
pub mod debug_analyzer;
pub mod nl_mock_generator;
pub mod persona_generator;

pub use artifact_freezer::{ArtifactFreezer, FreezeRequest, FrozenArtifact};
pub use budget_manager::{BudgetConfig, BudgetManager, UsageStats};
pub use chat_orchestrator::{
    ChatContext, ChatIntent, ChatMessage, ChatOrchestrator, ChatRequest, ChatResponse,
};
pub use config::AiStudioConfig;
pub use conversation_store::{
    get_conversation_store, initialize_conversation_store, ConversationStore,
};
pub use debug_analyzer::{DebugAnalyzer, DebugRequest, DebugResponse, DebugSuggestion};
pub use nl_mock_generator::{MockGenerationResult, MockGenerator};
pub use persona_generator::{
    PersonaGenerationRequest, PersonaGenerationResponse, PersonaGenerator,
};
