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
//!
//! The remaining AI cluster (`ai_contract_diff`, `ai_studio`,
//! `threat_modeling::*`) can now follow the same `pub use mockforge_intelligence::*;`
//! pattern from `mockforge-core` — phase 2 already removed the transitive
//! coupling that blocked them.

pub mod ai_response;
pub mod behavioral_cloning;
pub mod intelligent_behavior;
pub mod pr_generation;
