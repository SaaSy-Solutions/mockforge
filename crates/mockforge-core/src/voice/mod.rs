//! Pillars: [AI][DevX]
//!
//! Voice + LLM Interface for MockForge
//!
//! This module provides voice input capability that allows users to build mocks
//! conversationally using natural language commands. It leverages MockForge's
//! existing LLM infrastructure to interpret voice commands and generate mock APIs.
//!
//! # Features
//!
//! - **Natural Language Command Parsing**: Interpret voice commands using LLM
//! - **OpenAPI Spec Generation**: Generate OpenAPI 3.0 specs from voice commands
//! - **Conversational Mode**: Support multi-turn conversations for iterative refinement
//! - **Single-Shot Mode**: Process complete commands in one go
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::voice::{VoiceCommandParser, VoiceCommand};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! let config = IntelligentBehaviorConfig::default();
//! let parser = VoiceCommandParser::new(config);
//!
//! // Parse a voice command
//! let command = "Create a fake e-commerce API with 20 products and a checkout flow";
//! let parsed = parser.parse_command(command).await?;
//!
//! // Generate OpenAPI spec
//! let spec_generator = VoiceSpecGenerator::new();
//! let spec = spec_generator.generate_spec(&parsed).await?;
//! # Ok(())
//! # }
//! ```

pub mod command_parser;
pub mod conversation;
pub mod hook_transpiler;
pub mod spec_generator;
pub mod workspace_builder;
pub mod workspace_scenario_generator;

pub use command_parser::{
    ApiRequirement, EntityRequirement, ParsedCommand, ParsedContinuumRule, ParsedDriftBudget,
    ParsedRealityContinuum, ParsedServiceBudget, ParsedWorkspaceCreation, ParsedWorkspaceScenario,
    PersonaRequirement, ScenarioRequirement, VoiceCommandParser,
};
pub use conversation::{ConversationContext, ConversationManager, ConversationState};
pub use hook_transpiler::HookTranspiler;
pub use spec_generator::VoiceSpecGenerator;
pub use workspace_builder::{BuiltWorkspace, WorkspaceBuilder};
pub use workspace_scenario_generator::{
    GeneratedWorkspaceScenario, WorkspaceConfigSummary, WorkspaceScenarioGenerator,
};
