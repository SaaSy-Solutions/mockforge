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
//!
//! Still in `mockforge-core` and worth migrating in follow-ups:
//! - `ai_contract_diff`: pulls `crate::pillar_tracking::record_ai_usage`
//!   (analytics global). Needs the tracking hook re-homed or injected before
//!   it can move cleanly.
//! - `ai_studio`: pulls `reality`, `voice`, `failure_analysis`, and
//!   `contract_validation` — all still core-only. Blocked until those move.

pub mod ai_response;
pub mod behavioral_cloning;
pub mod intelligent_behavior;
pub mod pr_generation;
pub mod threat_modeling;
