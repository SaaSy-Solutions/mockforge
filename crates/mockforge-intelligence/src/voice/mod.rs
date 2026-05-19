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
//! Issue #562 phase 7 moved the 6 leaf voice files into this crate. The 7th file,
//! `workspace_builder.rs`, stays in `mockforge_core::voice_workspace` because it
//! depends on `multi_tenant`, `scenarios`, `workspace`, `contract_drift`, and
//! `reality_continuum` — all still core-only. The `mockforge_core::voice::*`
//! shim consolidates both halves so callers see one unified public API.
//!
//! ## Do not extract `voice_workspace` into this crate
//!
//! See the matching rationale in `mockforge_core::voice` (the shim) for the
//! full cycle diagram. Short version: `voice_workspace` calls the live
//! `MultiTenantWorkspaceRegistry` engine, and the 5 modules it depends on are
//! domain primitives that belong in core. Importing them here would invert
//! the established `core → intelligence` dep direction
//! (already in use via `contract_drift` re-exporting `threat_modeling`) and
//! reintroduce the cycle that Issue #562 phase 1 broke. Issue #562 is
//! complete; the location of `voice_workspace` is a feature, not debt.

pub mod command_parser;
pub mod conversation;
pub mod hook_transpiler;
pub mod spec_generator;
pub mod workspace_scenario_generator;

pub use command_parser::{
    ApiRequirement, EntityRequirement, ParsedCommand, ParsedContinuumRule, ParsedDriftBudget,
    ParsedRealityContinuum, ParsedServiceBudget, ParsedWorkspaceCreation, ParsedWorkspaceScenario,
    PersonaRequirement, ScenarioRequirement, VoiceCommandParser,
};
pub use conversation::{ConversationContext, ConversationManager, ConversationState};
pub use hook_transpiler::HookTranspiler;
pub use spec_generator::VoiceSpecGenerator;
pub use workspace_scenario_generator::{
    GeneratedWorkspaceScenario, WorkspaceConfigSummary, WorkspaceScenarioGenerator,
};
