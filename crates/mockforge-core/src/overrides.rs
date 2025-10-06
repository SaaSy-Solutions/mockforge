//! Overrides engine with templating helpers.
//!
//! This module provides a comprehensive override system for modifying
//! API responses based on operation IDs, tags, paths, and conditions.

use crate::conditions::{evaluate_condition, ConditionContext};
use crate::templating::expand_tokens as core_expand_tokens;
use serde_json::Value;

pub mod loader;
pub mod matcher;
pub mod models;
pub mod patcher;

// Re-export main types and functions for convenience
pub use matcher::*;
pub use models::{OverrideMode, OverrideRule, Overrides, PatchOp};
pub use patcher::*;

impl Overrides {
    pub fn apply(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value) {
        self.apply_with_context(operation_id, tags, path, body, &ConditionContext::new())
    }

    /// Apply overrides with condition evaluation
    pub fn apply_with_context(
        &self,
        operation_id: &str,
        tags: &[String],
        path: &str,
        body: &mut Value,
        context: &ConditionContext,
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_overrides_apply_basic() {
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/value".to_string(),
                    value: json!("replaced"),
                }],
                when: None,
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "replaced");
    }

    #[test]
    fn test_overrides_apply_no_match() {
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["operation:other_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/value".to_string(),
                    value: json!("replaced"),
                }],
                when: None,
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "original");
    }

    #[test]
    fn test_overrides_apply_with_tag() {
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["tag:test_tag".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/status".to_string(),
                    value: json!("tagged"),
                }],
                when: None,
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"status": "normal"});
        overrides.apply("any_op", &vec!["test_tag".to_string()], "/test", &mut body);

        assert_eq!(body["status"], "tagged");
    }

    #[test]
    fn test_overrides_apply_merge_mode() {
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Merge,
                patch: vec![PatchOp::Add {
                    path: "/extra".to_string(),
                    value: json!("added"),
                }],
                when: None,
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "original");
        assert_eq!(body["extra"], "added");
    }

    #[test]
    fn test_overrides_apply_with_context() {
        // Test that the with_context method is callable
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Replace {
                    path: "/value".to_string(),
                    value: json!("replaced"),
                }],
                when: None, // No condition for simplicity
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"value": "original"});
        let context = ConditionContext::new();
        overrides.apply_with_context("test_op", &[], "/test", &mut body, &context);

        assert_eq!(body["value"], "replaced");
    }

    #[test]
    fn test_overrides_apply_multiple_patches() {
        let overrides = Overrides {
            rules: vec![OverrideRule {
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![
                    PatchOp::Add {
                        path: "/field1".to_string(),
                        value: json!("value1"),
                    },
                    PatchOp::Add {
                        path: "/field2".to_string(),
                        value: json!("value2"),
                    },
                ],
                when: None,
                post_templating: false,
            }],
            regex_cache: Default::default(),
        };

        let mut body = json!({"existing": "value"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["field1"], "value1");
        assert_eq!(body["field2"], "value2");
    }

    #[test]
    fn test_overrides_apply_multiple_rules() {
        let overrides = Overrides {
            rules: vec![
                OverrideRule {
                    targets: vec!["operation:test_op".to_string()],
                    mode: OverrideMode::Replace,
                    patch: vec![PatchOp::Add {
                        path: "/first".to_string(),
                        value: json!("first_rule"),
                    }],
                    when: None,
                    post_templating: false,
                },
                OverrideRule {
                    targets: vec!["operation:test_op".to_string()],
                    mode: OverrideMode::Replace,
                    patch: vec![PatchOp::Add {
                        path: "/second".to_string(),
                        value: json!("second_rule"),
                    }],
                    when: None,
                    post_templating: false,
                },
            ],
            regex_cache: Default::default(),
        };

        let mut body = json!({"existing": "value"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["first"], "first_rule");
        assert_eq!(body["second"], "second_rule");
    }
}
