//! Overrides engine with templating helpers.
//!
//! This module provides a comprehensive override system for modifying
//! API responses based on operation IDs, tags, paths, and conditions.

use crate::conditions::{evaluate_condition, ConditionContext};
use crate::templating::expand_tokens as core_expand_tokens;
use serde_json::Value;

pub mod models;
pub mod loader;
pub mod matcher;
pub mod patcher;

// Re-export main types and functions for convenience
pub use models::{OverrideRule, OverrideMode, PatchOp, Overrides};
pub use matcher::*;
pub use patcher::*;

impl Overrides {

    pub fn apply(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value) {
        self.apply_with_context(operation_id, tags, path, body, &ConditionContext::new())
    }

    /// Apply overrides with condition evaluation
    pub fn apply_with_context(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value, context: &ConditionContext) {
        for r in &self.rules {
            if !matcher::matches_target(r, operation_id, tags, path, &self.regex_cache) {
                continue;
            }

            // Evaluate condition if present
            if let Some(ref condition) = r.when {
                match evaluate_condition(condition, context) {
                    Ok(true) => {
                        // Condition passed, continue with patch application
                    }
                    Ok(false) => {
                        // Condition failed, skip this rule
                        continue;
                    }
                    Err(e) => {
                        // Log condition evaluation error but don't fail the entire override process
                        tracing::warn!("Failed to evaluate condition '{}': {}", condition, e);
                        continue;
                    }
                }
            }

            // Apply patches based on mode
            match r.mode {
                OverrideMode::Replace => {
                    for op in &r.patch {
                        let _ = patcher::apply_patch(body, op);
                    }
                }
                OverrideMode::Merge => {
                    for op in &r.patch {
                        let _ = patcher::apply_merge_patch(body, op);
                    }
                }
            }

            // Apply post-templating expansion if enabled
            if r.post_templating {
                *body = core_expand_tokens(body);
            }
        }
    }
}
