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
//!
//! Full migration of `intelligent_behavior`, `ai_contract_diff`, `ai_studio`,
//! and `behavioral_economics` is blocked by circular dependencies with
//! non-deprecated core code (openapi, reality, priority_handler, etc.) that
//! require a foundation-types crate to untangle.

pub mod ai_response;
pub mod behavioral_cloning;
