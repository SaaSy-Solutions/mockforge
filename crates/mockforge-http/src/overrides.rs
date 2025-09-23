//! Overrides engine with templating helpers.
use globwalk::GlobWalkerBuilder;
use json_patch::{patch, AddOperation, PatchOperation, RemoveOperation, ReplaceOperation};
use mockforge_core::conditions::{evaluate_condition, ConditionContext, ConditionError};
use mockforge_core::templating::expand_tokens as core_expand_tokens;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct OverrideRule {
    pub targets: Vec<String>, // "operation:opId", "tag:Tag", "regex:pattern", or "path:pattern"
    pub patch: Vec<PatchOp>,
    pub when: Option<String>,
    /// Override mode: "replace" (default) or "merge"
    #[serde(default = "default_mode")]
    pub mode: OverrideMode,
    /// Whether to apply post-templating expansion after patching
    #[serde(default = "default_post_templating")]
    pub post_templating: bool,
}

/// Override mode for applying patches
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum OverrideMode {
    /// Replace values (default JSON patch behavior)
    #[serde(rename = "replace")]
    Replace,
    /// Merge objects and arrays instead of replacing
    #[serde(rename = "merge")]
    Merge,
}

fn default_mode() -> OverrideMode {
    OverrideMode::Replace
}

fn default_post_templating() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op")]
pub enum PatchOp {
    #[serde(rename = "add")]
    Add { path: String, value: Value },
    #[serde(rename = "replace")]
    Replace { path: String, value: Value },
    #[serde(rename = "remove")]
    Remove { path: String },
}

#[derive(Debug, Default, Clone)]
pub struct Overrides {
    rules: Vec<OverrideRule>,
    /// Compiled regex patterns for performance
    regex_cache: HashMap<String, Regex>,
}

impl Overrides {
    /// Load overrides from glob patterns, with support for MOCKFORGE_HTTP_OVERRIDES_GLOB
    pub async fn load_from_globs(patterns: &[&str]) -> anyhow::Result<Self> {
        // Check for environment variable override
        let patterns = if let Ok(env_patterns) = std::env::var("MOCKFORGE_HTTP_OVERRIDES_GLOB") {
            env_patterns.split(',').map(|s| s.trim()).collect::<Vec<_>>()
        } else {
            patterns.iter().map(|s| *s).collect::<Vec<_>>()
        };

        let mut rules = Vec::new();
        let mut regex_cache = HashMap::new();

        for pat in patterns {
            for entry in GlobWalkerBuilder::from_patterns(".", &[pat]).build()? {
                let path = entry?.path().to_path_buf();
                if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                    let text = tokio::fs::read_to_string(&path).await?;
                    let mut file_rules: Vec<OverrideRule> = serde_yaml::from_str(&text)?;

                    for r in file_rules.iter_mut() {
                        // Pre-expand templating tokens in patch values
                        for op in r.patch.iter_mut() {
                            match op {
                                PatchOp::Add { value, .. } | PatchOp::Replace { value, .. } => {
                                    *value = core_expand_tokens(value);
                                }
                                _ => {}
                            }
                        }

                        // Compile regex patterns for performance
                        for target in &r.targets {
                            if target.starts_with("regex:") || target.starts_with("path:") {
                                let pattern = target.strip_prefix("regex:").or_else(|| target.strip_prefix("path:")).unwrap();
                                if !regex_cache.contains_key(pattern) {
                                    if let Ok(regex) = Regex::new(pattern) {
                                        regex_cache.insert(pattern.to_string(), regex);
                                    }
                                }
                            }
                        }
                    }
                    rules.extend(file_rules);
                }
            }
        }
        Ok(Overrides { rules, regex_cache })
    }

    pub fn apply(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value) {
        self.apply_with_context(operation_id, tags, path, body, &ConditionContext::new())
    }

    /// Apply overrides with condition evaluation
    pub fn apply_with_context(&self, operation_id: &str, tags: &[String], path: &str, body: &mut Value, context: &ConditionContext) {
        for r in &self.rules {
            if !matches_target(&r.targets, operation_id, tags, path, &self.regex_cache) {
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
                        apply_patch(body, op);
                    }
                }
                OverrideMode::Merge => {
                    for op in &r.patch {
                        apply_merge_patch(body, op);
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

fn matches_target(
    targets: &[String],
    op_id: &str,
    tags: &[String],
    path: &str,
    regex_cache: &HashMap<String, Regex>
) -> bool {
    targets.iter().any(|t| {
        if let Some(rest) = t.strip_prefix("operation:") {
            rest == op_id
        } else if let Some(rest) = t.strip_prefix("tag:") {
            tags.iter().any(|g| g == rest)
        } else if let Some(pattern) = t.strip_prefix("regex:") {
            // Match against operation ID
            regex_cache.get(pattern).map_or(false, |re| re.is_match(op_id))
        } else if let Some(pattern) = t.strip_prefix("path:") {
            // Match against request path
            regex_cache.get(pattern).map_or(false, |re| re.is_match(path))
        } else {
            false
        }
    })
}

fn apply_patch(doc: &mut Value, op: &PatchOp) {
    let ops = match op {
        PatchOp::Add { path, value } => vec![PatchOperation::Add(AddOperation {
            path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
            value: value.clone(),
        })],
        PatchOp::Replace { path, value } => vec![PatchOperation::Replace(ReplaceOperation {
            path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
            value: value.clone(),
        })],
        PatchOp::Remove { path } => vec![PatchOperation::Remove(RemoveOperation {
            path: path.parse().unwrap_or_else(|_| json_patch::jsonptr::PointerBuf::new()),
        })],
    };

    // `Patch` is just a Vec<PatchOperation>
    let _ = patch(doc, &ops);
}

/// Apply merge patch operation (deep merge for objects, append for arrays)
fn apply_merge_patch(doc: &mut Value, op: &PatchOp) {
    match op {
        PatchOp::Add { path, value } => {
            if let Ok(pointer) = path.parse::<json_patch::jsonptr::PointerBuf>() {
                if let Some(target) = pointer.get_mut(doc) {
                    match (target, value) {
                        (Value::Object(target_obj), Value::Object(value_obj)) => {
                            // Deep merge objects
                            for (key, val) in value_obj {
                                target_obj.insert(key.clone(), val.clone());
                            }
                        }
                        (Value::Array(target_arr), Value::Array(value_arr)) => {
                            // Append to arrays
                            target_arr.extend(value_arr.iter().cloned());
                        }
                        (target, value) => {
                            // Replace for other types
                            *target = value.clone();
                        }
                    }
                } else {
                    // Path doesn't exist, create it
                    let _ = pointer.set(doc, value.clone());
                }
            }
        }
        PatchOp::Replace { path, value } => {
            if let Ok(pointer) = path.parse::<json_patch::jsonptr::PointerBuf>() {
                if let Some(target) = pointer.get_mut(doc) {
                    match (target, value) {
                        (Value::Object(target_obj), Value::Object(value_obj)) => {
                            // Deep merge objects
                            for (key, val) in value_obj {
                                target_obj.insert(key.clone(), val.clone());
                            }
                        }
                        (Value::Array(target_arr), Value::Array(value_arr)) => {
                            // Replace array contents
                            target_arr.clear();
                            target_arr.extend(value_arr.iter().cloned());
                        }
                        (target, value) => {
                            // Replace for other types
                            *target = value.clone();
                        }
                    }
                } else {
                    // Path doesn't exist, create it
                    let _ = pointer.set(doc, value.clone());
                }
            }
        }
        PatchOp::Remove { path } => {
            if let Ok(pointer) = path.parse::<json_patch::jsonptr::PointerBuf>() {
                let _ = pointer.remove(doc);
            }
        }
    }
}

// templating moved to mockforge-core::templating
