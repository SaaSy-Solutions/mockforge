//! Pillars: [AI][DevX]
//!
//! Voice + LLM Interface — unified re-export shim.
//!
//! Issue #562 phase 7 split this module: 6 leaf files (command_parser,
//! conversation, hook_transpiler, spec_generator, workspace_scenario_generator,
//! mod) moved to `mockforge_intelligence::voice`. The 7th, `workspace_builder.rs`,
//! stays in `mockforge_core::voice_workspace` because it depends on
//! multi_tenant, scenarios, workspace, contract_drift, and reality_continuum —
//! all still core-only.
//!
//! This shim re-exports both halves so existing
//! `mockforge_core::voice::{VoiceCommandParser, WorkspaceBuilder, ...}` call
//! sites keep working unchanged.

pub use crate::voice_workspace::{BuiltWorkspace, WorkspaceBuilder};
pub use mockforge_intelligence::voice::*;

// Re-export the sub-modules too so existing
// `mockforge_core::voice::command_parser::*` and
// `mockforge_core::voice::spec_generator::*` paths keep resolving (used by
// `voice_workspace` and any external callers that imported via the
// sub-module path rather than the top-level re-exports).
pub use mockforge_intelligence::voice::{
    command_parser, conversation, hook_transpiler, spec_generator, workspace_scenario_generator,
};
