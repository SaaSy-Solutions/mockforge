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
//!
//! Still in `mockforge-core` and worth migrating in follow-ups:
//! - `ai_studio`: 3 sub-files still pull `reality` (debug_context,
//!   debug_context_integrator) and `voice` (nl_mock_generator). Reality is
//!   moveable next; voice is the wall and a multi-day campaign.

pub mod ai_contract_diff;
pub mod ai_response;
pub mod behavioral_cloning;
pub mod contract_validation;
pub mod failure_analysis;
pub mod intelligent_behavior;
pub mod pr_generation;
pub mod reality;
pub mod threat_modeling;
