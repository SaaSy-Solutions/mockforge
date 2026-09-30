//! Overrides engine with templating helpers.
//!
//! This module provides a comprehensive override system for modifying
//! API responses based on operation IDs, tags, paths, and conditions.

use crate::conditions::{evaluate_condition, ConditionContext};
use crate::templating::expand_tokens as core_expand_tokens;
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub mod loader;
pub mod models;
pub mod patcher;

// Re-export main types and functions for convenience
pub use loader::{OVERRIDES_GLOB_ENV, OVERRIDES_INLINE_ENV};
pub use models::{OverrideMode, OverrideRule, Overrides, OverridesError, PatchOp, Target};
pub use patcher::*;

impl Overrides {
    /// Apply overrides to a response body using default condition context
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
        for (r, targets) in self.compiled() {
            if !r.enabled || !targets.iter().any(|t| t.matches(operation_id, tags, path)) {
                continue;
            }

            if let Some(ref condition) = r.when {
                match evaluate_condition(condition, context) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        tracing::warn!("Failed to evaluate condition '{}': {}", condition, e);
                        continue;
                    }
                }
            }

            match r.mode {
                OverrideMode::Replace => {
                    for op in &r.patch {
                        let _ = apply_patch(body, op);
                    }
                }
                OverrideMode::Merge => {
                    for op in &r.patch {
                        let _ = apply_merge_patch(body, op);
                    }
                }
            }

            if r.post_templating {
                *body = core_expand_tokens(body);
            }
        }
    }
}

/// A rule set that a running server reads on every response and that the
/// admin API can replace without a restart. Cheap to clone.
#[derive(Debug, Clone, Default)]
pub struct SharedOverrides {
    inner: Arc<RwLock<Overrides>>,
}

impl SharedOverrides {
    /// Wrap an initial rule set
    pub fn new(initial: Overrides) -> Self {
        Self {
            inner: Arc::new(RwLock::new(initial)),
        }
    }

    /// Start from `config_rules` plus the rules in
    /// `MOCKFORGE_HTTP_OVERRIDES_GLOB` and `MOCKFORGE_HTTP_OVERRIDES`. Rules
    /// that fail to load are logged and the server starts with none, so a bad
    /// rule file never blocks startup.
    pub async fn load(config_rules: Vec<OverrideRule>) -> Self {
        match Overrides::load(config_rules).await {
            Ok(overrides) => {
                if !overrides.rules().is_empty() {
                    tracing::info!("Loaded {} override rules", overrides.rules().len());
                }
                Self::new(overrides)
            }
            Err(e) => {
                tracing::error!("Failed to load override rules, starting with none: {e}");
                Self::default()
            }
        }
    }

    /// A copy of the current rules
    pub fn rules(&self) -> Vec<OverrideRule> {
        self.read().rules().to_vec()
    }

    /// Whether any enabled rule has a `when` condition, which is the only
    /// case that needs the request and response in a `ConditionContext`
    pub fn needs_context(&self) -> bool {
        self.read().rules().iter().any(|r| r.enabled && r.when.is_some())
    }

    /// Whether the set has no rules at all
    pub fn is_empty(&self) -> bool {
        self.read().rules().is_empty()
    }

    /// Replace the whole rule set
    pub fn replace(&self, overrides: Overrides) {
        *self.inner.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = overrides;
    }

    /// Apply the current rules to a response body
    pub fn apply_with_context(
        &self,
        operation_id: &str,
        tags: &[String],
        path: &str,
        body: &mut Value,
        context: &ConditionContext,
    ) {
        let overrides = self.read();
        if !overrides.rules().is_empty() {
            overrides.apply_with_context(operation_id, tags, path, body, context);
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Overrides> {
        self.inner.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_overrides_apply_basic() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!("replaced"),
            }],
            when: None,
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "replaced");
    }

    #[test]
    fn test_overrides_apply_no_match() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["operation:other_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!("replaced"),
            }],
            when: None,
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "original");
    }

    #[test]
    fn test_overrides_apply_with_tag() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["tag:test_tag".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/status".to_string(),
                value: json!("tagged"),
            }],
            when: None,
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"status": "normal"});
        overrides.apply("any_op", &["test_tag".to_string()], "/test", &mut body);

        assert_eq!(body["status"], "tagged");
    }

    #[test]
    fn test_overrides_apply_merge_mode() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Merge,
            patch: vec![PatchOp::Add {
                path: "/extra".to_string(),
                value: json!("added"),
            }],
            when: None,
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"value": "original"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["value"], "original");
        assert_eq!(body["extra"], "added");
    }

    #[test]
    fn test_overrides_apply_with_context() {
        // Test that the with_context method is callable
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["operation:test_op".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "/value".to_string(),
                value: json!("replaced"),
            }],
            when: None, // No condition for simplicity
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"value": "original"});
        let context = ConditionContext::new();
        overrides.apply_with_context("test_op", &[], "/test", &mut body, &context);

        assert_eq!(body["value"], "replaced");
    }

    #[test]
    fn test_overrides_apply_multiple_patches() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
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
        }])
        .unwrap();

        let mut body = json!({"existing": "value"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["field1"], "value1");
        assert_eq!(body["field2"], "value2");
    }

    #[test]
    fn test_overrides_apply_multiple_rules() {
        let overrides = Overrides::new(vec![
            OverrideRule {
                name: None,
                enabled: true,
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
                name: None,
                enabled: true,
                targets: vec!["operation:test_op".to_string()],
                mode: OverrideMode::Replace,
                patch: vec![PatchOp::Add {
                    path: "/second".to_string(),
                    value: json!("second_rule"),
                }],
                when: None,
                post_templating: false,
            },
        ])
        .unwrap();

        let mut body = json!({"existing": "value"});
        overrides.apply("test_op", &[], "/test", &mut body);

        assert_eq!(body["first"], "first_rule");
        assert_eq!(body["second"], "second_rule");
    }

    #[test]
    fn test_overrides_replace_root_object() {
        let overrides = Overrides::new(vec![OverrideRule {
            name: None,
            enabled: true,
            targets: vec!["operation:testReplace".to_string()],
            mode: OverrideMode::Replace,
            patch: vec![PatchOp::Replace {
                path: "".to_string(),
                value: json!({"id": "1234", "kind": "replaced"}),
            }],
            when: None,
            post_templating: false,
        }])
        .unwrap();

        let mut body = json!({"existing": true});
        overrides.apply("testReplace", &[], "/test-replace", &mut body);

        assert_eq!(body["kind"], "replaced");
        assert_eq!(body["id"], "1234");
        assert!(body.get("existing").is_none());
    }

    fn rule(targets: &[&str], patch: Vec<PatchOp>) -> OverrideRule {
        OverrideRule {
            name: None,
            enabled: true,
            targets: targets.iter().map(|t| t.to_string()).collect(),
            patch,
            when: None,
            mode: OverrideMode::Replace,
            post_templating: false,
        }
    }

    fn set_status(value: &str) -> Vec<PatchOp> {
        vec![PatchOp::Replace {
            path: "/status".to_string(),
            value: json!(value),
        }]
    }

    #[test]
    fn new_rejects_rules_that_could_never_apply_correctly() {
        assert_eq!(
            Overrides::new(vec![rule(&[], set_status("x"))]).unwrap_err(),
            OverridesError::NoTargets { rule: 0 }
        );
        assert!(matches!(
            Overrides::new(vec![rule(&["*"], vec![]), rule(&["path:(unclosed"], vec![])]),
            Err(OverridesError::InvalidTarget { rule: 1, .. })
        ));
        assert!(matches!(
            Overrides::new(vec![rule(&["users"], vec![])]),
            Err(OverridesError::InvalidTarget { rule: 0, .. })
        ));
        assert_eq!(
            Overrides::new(vec![rule(
                &["*"],
                vec![PatchOp::Remove {
                    path: "status".to_string()
                }]
            )])
            .unwrap_err(),
            OverridesError::InvalidPointer {
                rule: 0,
                path: "status".to_string()
            }
        );
    }

    #[test]
    fn disabled_rules_are_kept_but_not_applied() {
        let mut off = rule(&["*"], set_status("patched"));
        off.enabled = false;
        let overrides = Overrides::new(vec![off]).unwrap();
        let mut body = json!({"status": "original"});
        overrides.apply("op", &[], "/x", &mut body);
        assert_eq!(body["status"], "original");
        assert_eq!(overrides.rules().len(), 1);
    }

    #[test]
    fn deserialized_rule_sets_compile_their_regex_targets() {
        let overrides: Overrides = serde_json::from_value(json!({
            "rules": [{
                "targets": ["path:^/users/"],
                "patch": [{"op": "replace", "path": "/status", "value": "patched"}]
            }]
        }))
        .unwrap();
        let mut body = json!({"status": "original"});
        overrides.apply("getUser", &[], "/users/{id}", &mut body);
        assert_eq!(body["status"], "patched");
        assert!(overrides.rules()[0].enabled);

        let invalid = serde_json::from_value::<Overrides>(json!({
            "rules": [{"targets": ["regex:("], "patch": []}]
        }));
        assert!(invalid.is_err());
    }

    #[test]
    fn shared_overrides_apply_the_latest_replacement() {
        let shared = SharedOverrides::default();
        let mut body = json!({"status": "original"});
        shared.apply_with_context("op", &[], "/x", &mut body, &ConditionContext::new());
        assert_eq!(body["status"], "original");

        shared.replace(Overrides::new(vec![rule(&["operation:op"], set_status("live"))]).unwrap());
        shared.apply_with_context("op", &[], "/x", &mut body, &ConditionContext::new());
        assert_eq!(body["status"], "live");
        assert_eq!(shared.rules().len(), 1);
    }

    #[test]
    fn when_conditions_see_request_headers() {
        let mut vip = rule(&["*"], set_status("vip"));
        vip.when = Some("header[x-tier]=gold".to_string());
        let overrides = Overrides::new(vec![vip]).unwrap();

        let mut body = json!({"status": "standard"});
        overrides.apply_with_context("op", &[], "/x", &mut body, &ConditionContext::new());
        assert_eq!(body["status"], "standard");

        let ctx = ConditionContext::new()
            .with_headers([("x-tier".to_string(), "gold".to_string())].into_iter().collect());
        overrides.apply_with_context("op", &[], "/x", &mut body, &ctx);
        assert_eq!(body["status"], "vip");
    }

    #[tokio::test]
    async fn load_from_env_reads_inline_json_rules() {
        std::env::remove_var(OVERRIDES_GLOB_ENV);
        std::env::set_var(
            OVERRIDES_INLINE_ENV,
            r#"[{"name":"hosted","targets":["*"],"patch":[{"op":"add","path":"/src","value":"env"}]}]"#,
        );
        let overrides = Overrides::load(Vec::new()).await.unwrap();
        std::env::remove_var(OVERRIDES_INLINE_ENV);
        assert_eq!(overrides.rules()[0].name.as_deref(), Some("hosted"));

        let mut body = json!({});
        overrides.apply("op", &[], "/x", &mut body);
        assert_eq!(body["src"], "env");
    }
}
