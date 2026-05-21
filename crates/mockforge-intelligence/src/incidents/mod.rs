//! Drift-incident management — semantic side.
//!
//! Issue #562 phase 9: the AI-coupled pieces of `mockforge_core::incidents` moved
//! here. The structural incident manager (`IncidentManager`), in-memory store
//! (`IncidentStore`), and shared types (`IncidentSeverity`, `IncidentStatus`,
//! etc., which live in `mockforge_foundation::incidents_types`) stay in core /
//! foundation respectively — they don't depend on AI primitives.
//!
//! What's here:
//!
//! - [`semantic_manager`]: tracks **semantic** drift incidents (cross-linked
//!   with structural incidents in core but built on top of the LLM-driven
//!   semantic-change taxonomy from `crate::ai_contract_diff::semantic_analyzer`).
//! - [`integrations`]: Jira / Linear / generic-webhook configuration types
//!   used to push incident notifications to external systems.
//! - [`slack_formatter`] (`integrations/slack.rs`) and [`jira_formatter`]
//!   (`integrations/jira.rs`): payload-formatting helpers for the two
//!   built-in integrations. Aliased under the same names as in the legacy
//!   `mockforge_core::incidents` layout so existing call sites that go
//!   through the core re-export shim resolve unchanged.

pub mod integrations;
pub mod semantic_manager;

// Match the legacy core layout exactly — the two formatter aliases share
// the same files as their parent `integrations` submodules. Existing
// `mockforge_core::incidents::{slack_formatter, jira_formatter}` callers
// pick these up via the core re-export shim.
#[path = "integrations/slack.rs"]
pub mod slack_formatter;

#[path = "integrations/jira.rs"]
pub mod jira_formatter;

pub use semantic_manager::{SemanticIncident, SemanticIncidentManager};
