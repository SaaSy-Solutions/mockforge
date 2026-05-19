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
//! ## Why `voice_workspace` is not extracted (do not attempt)
//!
//! It is tempting to "finish the job" and move `voice_workspace` into
//! `mockforge-intelligence` too. **Do not.** It would introduce a real
//! dependency cycle:
//!
//! ```text
//! mockforge_intelligence::voice::voice_workspace
//!   → mockforge_core::multi_tenant::MultiTenantWorkspaceRegistry
//!     → mockforge_core::workspace
//!     → mockforge_core::contract_drift
//!       → mockforge_intelligence::threat_modeling   (re-exported from core)
//! ```
//!
//! The five blocker modules (`multi_tenant`, `workspace`, `contract_drift`,
//! `reality_continuum::engine`, `scenarios`) are **domain primitives**, not
//! AI features — they belong with the rest of core. Moving the live
//! `MultiTenantWorkspaceRegistry` engine (not just its data types) is what
//! `voice_workspace` actually needs, so promoting POD types to
//! `mockforge-foundation` does not help either.
//!
//! `mockforge-intelligence` does **not** consume `voice_workspace` — only
//! `mockforge-cli/voice_commands.rs` and `mockforge-ui/handlers/voice.rs` do.
//! The cycle is already broken; this file's location is a feature, not a
//! debt. Issue #562 is complete.
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

// `voice_workspace` was previously a sub-module of `voice` (path:
// `mockforge_core::voice::workspace_builder`). Phase 7 promoted it to a
// top-level core module, but the integration test
// `tests/tests/voice_workspace_creation.rs` still imports via the old
// path. Alias for caller backwards compat.
pub use crate::voice_workspace as workspace_builder;
