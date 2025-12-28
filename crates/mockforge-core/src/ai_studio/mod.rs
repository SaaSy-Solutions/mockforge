//! Pillars: [AI]
//!
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
//! ```rust,ignore
//! use mockforge_core::ai_studio::{ChatOrchestrator, ChatRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! async fn example() -> mockforge_core::Result<()> {
//!     let config = IntelligentBehaviorConfig::default();
//!     let orchestrator = ChatOrchestrator::new(config);
//!
//!     // Process a natural language command
//!     let request = ChatRequest {
//!         message: "Create a user API with CRUD operations".to_string(),
//!         context: None,
//!     };
//!     let response = orchestrator.process(&request).await?;
//!     Ok(())
//! }
//! ```

pub mod api_critique;
pub mod artifact_freezer;
pub mod behavioral_simulator;
pub mod budget_manager;
pub mod chat_orchestrator;
pub mod config;
pub mod contract_diff_handler;
pub mod conversation_store;
pub mod debug_analyzer;
pub mod debug_context;
pub mod debug_context_integrator;
pub mod nl_mock_generator;
pub mod org_controls;
#[cfg(feature = "database")]
pub mod org_controls_db;
pub mod persona_generator;
pub mod system_generator;

pub use api_critique::{
    AntiPattern, ApiCritique, ApiCritiqueEngine, ConsolidationOpportunity, CritiqueRequest,
    HierarchyImprovement, NamingIssue, Redundancy, ResourceModelingSuggestion,
    RestructuringRecommendations, ToneAnalysis, ToneIssue,
};
pub use artifact_freezer::{ArtifactFreezer, FreezeMetadata, FreezeRequest, FrozenArtifact};
pub use behavioral_simulator::{
    AppState, BehaviorPolicy, BehavioralSimulator, BehavioralTraits, CartState, CreateAgentRequest,
    Intention, NarrativeAgent, NextAction, PolicyRule, SimulateBehaviorRequest,
    SimulateBehaviorResponse,
};
pub use budget_manager::{AiFeature, BudgetConfig, BudgetManager, FeatureUsage, UsageStats};
pub use chat_orchestrator::{
    ChatContext, ChatIntent, ChatMessage, ChatOrchestrator, ChatRequest, ChatResponse,
};
pub use config::{AiStudioConfig, FreezeMode};
pub use contract_diff_handler::{
    BreakingChange, ContractDiffFilters, ContractDiffHandler, ContractDiffIntent,
    ContractDiffQueryResult,
};
pub use conversation_store::{
    get_conversation_store, initialize_conversation_store, ConversationStore,
};
pub use debug_analyzer::{
    DebugAnalyzer, DebugRequest, DebugResponse, DebugSuggestion, LinkedArtifact,
};
pub use debug_context::{
    ChaosContext, ContractContext, DebugContext, PersonaContext, RealityContext, ScenarioContext,
};
pub use debug_context_integrator::{
    ChaosAccessor, ContractAccessor, DebugContextIntegrator, PersonaAccessor, RealityAccessor,
    ScenarioAccessor,
};
pub use nl_mock_generator::{MockGenerationResult, MockGenerator};
pub use org_controls::{
    OrgAiControlsConfig, OrgBudgetConfig, OrgControls, OrgControlsAccessor, OrgRateLimitConfig,
};
#[cfg(feature = "database")]
pub use org_controls_db::DbOrgControls;
pub use persona_generator::{
    PersonaGenerationRequest, PersonaGenerationResponse, PersonaGenerator,
};
pub use system_generator::{
    AppliedSystem, GeneratedSystem, SystemArtifact, SystemGenerationRequest, SystemGenerator,
    SystemMetadata,
};
