//! Behavioral Economics Engine — unified re-export shim.
//!
//! Issue #562 phase 8 moved this module to `mockforge_intelligence::behavioral_economics`.
//! It only ever depended on `crate::Result` (now `mockforge_foundation::Result`),
//! making the migration mechanical.
//!
//! This shim re-exports the new home so existing
//! `mockforge_core::behavioral_economics::{BehavioralEconomicsEngine, BehaviorRule, ...}`
//! call sites keep working unchanged.

pub use mockforge_intelligence::behavioral_economics::*;

// Re-export the sub-modules so existing
// `mockforge_core::behavioral_economics::config::*` and similar paths keep
// resolving (used internally by `priority_handler` and `config::contracts`).
pub use mockforge_intelligence::behavioral_economics::{
    actions, conditions, config, engine, rules,
};
