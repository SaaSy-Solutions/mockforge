// Some ai_response helpers are themselves deprecated in favor of
// mockforge-template-expansion; internal tests still reference them.
#![allow(deprecated)]

//! AI-powered intelligence for MockForge
//!
//! This crate contains modules extracted from `mockforge-core` related to
//! intelligent behavior, AI response generation, and behavioral cloning.
//!
//! Currently migrated:
//! - `ai_response`: Typed AI response generation helpers
//! - `behavioral_cloning`: Probability models, sequence learning, edge amplification
//! - `pr_generation`: GitHub/GitLab PR generation client (Issue #562 phase 1 —
//!   moved out of `mockforge-core` because it only depends on
//!   `mockforge_foundation::Error`, no other core internals)
//! - `intelligent_behavior`: LLM-driven behavior model, persona-aware response
//!   generation, OpenAPI-backed example/rule generation (Issue #562 phase 2 —
//!   the AI cluster's leaf module, depends only on `mockforge-openapi` and
//!   `mockforge-foundation`)
//! - `threat_modeling`: LLM-driven security analyzer (DoS / PII / schema /
//!   error / threat / remediation generators) for the contract-drift pillar
//!   (Issue #562 phase 3). Depends only on sibling `intelligent_behavior` +
//!   `mockforge-openapi` + `mockforge-foundation`.
//! - `ai_contract_diff`: LLM-assisted OpenAPI diff with semantic analysis,
//!   confidence scoring, recommendations, and correction proposals (Issue #562
//!   phase 4). Records `ai_generation` pillar usage via the now-foundation
//!   `pillar_tracking` global, so the analytics dashboard keeps reflecting
//!   contract-diff activity unchanged.
//! - `contract_validation`: OpenAPI-spec-to-response contract validator (Issue
//!   #562 phase 5). Single file; depends only on `serde` + `mockforge-openapi`
//!   + `mockforge-foundation::pillar_tracking`.
//! - `failure_analysis`: LLM-driven failure context + narrative generator
//!   (Issue #562 phase 5). Depends only on sibling `intelligent_behavior`.
//! - `behavioral_economics`: Declarative + scriptable rule engine that makes
//!   mocks react to pressure, load, pricing, fraud, and customer segments
//!   (Issue #562 phase 8). Self-contained — the only core-side dep was
//!   `crate::Result`, now sourced from `mockforge-foundation` directly.
//! - `incidents`: AI-coupled pieces of drift-incident management — semantic
//!   incident manager (built on `ai_contract_diff::semantic_analyzer`) and
//!   Jira/Slack/webhook integrations (Issue #562 phase 9). The structural
//!   `IncidentManager` and in-memory `IncidentStore` stay in `mockforge-core`;
//!   the shared types (`DriftIncident`, `IncidentSeverity`, ...) live in
//!   `mockforge_foundation::incidents_types`. Core re-exports this module so
//!   the legacy `mockforge_core::incidents::{semantic_manager, integrations,
//!   slack_formatter, jira_formatter}` paths keep resolving.

pub mod ai_contract_diff;
pub mod ai_response;
pub mod ai_studio;
pub mod behavioral_cloning;
pub mod behavioral_economics;
pub mod contract_validation;
/// Postgres pool wrapper used by HTTP handlers that persist drift
/// budgets / incidents / consumer contracts. Moved here from
/// `mockforge_http::database` under #555 (prereq for handler moves —
/// once handlers leave `mockforge-http`, they pick up this dep without
/// re-introducing a cycle). Gated by the `database` feature.
#[cfg(feature = "database")]
pub mod database;
pub mod failure_analysis;
/// HTTP handlers for AI-coupled features. New in #555 phase 2 — see
/// `handlers/mod.rs` for migration progress.
pub mod handlers;
pub mod incidents;
pub mod intelligent_behavior;
pub mod pr_generation;
pub mod reality;
pub mod threat_modeling;
pub mod voice;
