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
    /// Get the loaded override rules
    pub fn rules(&self) -> &[OverrideRule] {
        &self.rules
    }

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
                            if let Some(pattern) = target.strip_prefix("regex:").or_else(|| target.strip_prefix("path:")) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::fs;

    #[test]
    fn test_override_mode_default() {
        let mode = default_mode();
        assert_eq!(mode, OverrideMode::Replace);
    }

    #[test]
    fn test_post_templating_default() {
        assert!(!default_post_templating());
    }

    #[test]
    fn test_patch_op_serialization() {
        let add_op = PatchOp::Add {
            path: "/name".to_string(),
            value: json!("John"),
        };

        let serialized = serde_json::to_string(&add_op).unwrap();
        assert!(serialized.contains("\"op\":\"add\""));
        assert!(serialized.contains("\"path\":\"/name\""));
    }

    #[tokio::test]
    async fn test_overrides_default() {
        let overrides = Overrides::default();
        assert_eq!(overrides.rules.len(), 0);
        assert_eq!(overrides.regex_cache.len(), 0);
    }

    #[tokio::test]
    async fn test_load_from_globs_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());

        let result = Overrides::load_from_globs(&[&pattern]).await;
        assert!(result.is_ok());
        let overrides = result.unwrap();
        assert_eq!(overrides.rules.len(), 0);
    }

    #[tokio::test]
    async fn test_load_from_globs_with_yaml_file() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("overrides.yaml");

        let yaml_content = r#"
- targets:
    - "operation:getUser"
  patch:
    - op: replace
      path: "/name"
      value: "Jane Doe"
"#;

        fs::write(&yaml_path, yaml_content).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let result = Overrides::load_from_globs(&[&pattern]).await;

        assert!(result.is_ok());
        let overrides = result.unwrap();
        assert_eq!(overrides.rules.len(), 1);
        assert_eq!(overrides.rules[0].targets[0], "operation:getUser");
    }

    #[tokio::test]
    async fn test_load_from_globs_with_regex_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("overrides.yaml");

        let yaml_content = r#"
- targets:
    - "regex:get.*"
  patch:
    - op: add
      path: "/timestamp"
      value: "2024-01-01"
"#;

        fs::write(&yaml_path, yaml_content).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let result = Overrides::load_from_globs(&[&pattern]).await;

        assert!(result.is_ok());
        let overrides = result.unwrap();
        assert_eq!(overrides.rules.len(), 1);
        // Regex should be cached
        assert!(overrides.regex_cache.contains_key("get.*"));
    }

    #[test]
    fn test_matches_target_operation() {
        let targets = vec!["operation:getUser".to_string()];
        let regex_cache = HashMap::new();

        assert!(matches_target(&targets, "getUser", &[], "/users", &regex_cache));
        assert!(!matches_target(&targets, "createUser", &[], "/users", &regex_cache));
    }

    #[test]
    fn test_matches_target_tag() {
        let targets = vec!["tag:admin".to_string()];
        let regex_cache = HashMap::new();
        let tags = vec!["admin".to_string(), "users".to_string()];

        assert!(matches_target(&targets, "getUser", &tags, "/users", &regex_cache));

        let tags_no_match = vec!["users".to_string()];
        assert!(!matches_target(&targets, "getUser", &tags_no_match, "/users", &regex_cache));
    }

    #[test]
    fn test_matches_target_regex() {
        let targets = vec!["regex:get.*".to_string()];
        let mut regex_cache = HashMap::new();
        regex_cache.insert("get.*".to_string(), Regex::new("get.*").unwrap());

        assert!(matches_target(&targets, "getUser", &[], "/users", &regex_cache));
        assert!(matches_target(&targets, "getUserById", &[], "/users", &regex_cache));
        assert!(!matches_target(&targets, "createUser", &[], "/users", &regex_cache));
    }

    #[test]
    fn test_matches_target_path() {
        let targets = vec!["path:/users/.*".to_string()];
        let mut regex_cache = HashMap::new();
        regex_cache.insert("/users/.*".to_string(), Regex::new("/users/.*").unwrap());

        assert!(matches_target(&targets, "getUser", &[], "/users/123", &regex_cache));
        assert!(!matches_target(&targets, "getUser", &[], "/posts/456", &regex_cache));
    }

    #[test]
    fn test_apply_patch_add() {
        let mut doc = json!({"name": "John"});
        let op = PatchOp::Add {
            path: "/age".to_string(),
            value: json!(30),
        };

        apply_patch(&mut doc, &op);
        assert_eq!(doc["age"], 30);
    }

    #[test]
    fn test_apply_patch_replace() {
        let mut doc = json!({"name": "John", "age": 25});
        let op = PatchOp::Replace {
            path: "/age".to_string(),
            value: json!(30),
        };

        apply_patch(&mut doc, &op);
        assert_eq!(doc["age"], 30);
    }

    #[test]
    fn test_apply_patch_remove() {
        let mut doc = json!({"name": "John", "age": 30});
        let op = PatchOp::Remove {
            path: "/age".to_string(),
        };

        apply_patch(&mut doc, &op);
        assert!(doc.get("age").is_none());
    }

    #[test]
    fn test_apply_merge_patch_add_object() {
        let mut doc = json!({"user": {"name": "John"}});
        let op = PatchOp::Add {
            path: "/user".to_string(),
            value: json!({"age": 30}),
        };

        apply_merge_patch(&mut doc, &op);

        assert_eq!(doc["user"]["name"], "John");
        assert_eq!(doc["user"]["age"], 30);
    }

    #[test]
    fn test_apply_merge_patch_add_array() {
        let mut doc = json!({"items": [1, 2]});
        let op = PatchOp::Add {
            path: "/items".to_string(),
            value: json!([3, 4]),
        };

        apply_merge_patch(&mut doc, &op);

        assert_eq!(doc["items"], json!([1, 2, 3, 4]));
    }

    #[test]
    fn test_apply_merge_patch_replace_array() {
        let mut doc = json!({"items": [1, 2]});
        let op = PatchOp::Replace {
            path: "/items".to_string(),
            value: json!([3, 4]),
        };

        apply_merge_patch(&mut doc, &op);

        assert_eq!(doc["items"], json!([3, 4]));
    }

    #[test]
    fn test_apply_merge_patch_remove() {
        let mut doc = json!({"name": "John", "age": 30});
        let op = PatchOp::Remove {
            path: "/age".to_string(),
        };

        apply_merge_patch(&mut doc, &op);
        assert!(doc.get("age").is_none());
    }

    #[test]
    fn test_overrides_apply_no_match() {
        let overrides = Overrides::default();
        let mut body = json!({"name": "John"});
        let original = body.clone();

        overrides.apply("getUser", &[], "/users", &mut body);

        assert_eq!(body, original);
    }

    #[tokio::test]
    async fn test_overrides_apply_with_operation_match() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("overrides.yaml");

        let yaml_content = r#"
- targets:
    - "operation:getUser"
  patch:
    - op: replace
      path: "/name"
      value: "Jane Doe"
"#;

        fs::write(&yaml_path, yaml_content).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let overrides = Overrides::load_from_globs(&[&pattern]).await.unwrap();

        let mut body = json!({"name": "John"});
        overrides.apply("getUser", &[], "/users", &mut body);

        assert_eq!(body["name"], "Jane Doe");
    }

    #[tokio::test]
    async fn test_overrides_apply_with_tag_match() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("overrides.yaml");

        let yaml_content = r#"
- targets:
    - "tag:admin"
  patch:
    - op: add
      path: "/role"
      value: "administrator"
"#;

        fs::write(&yaml_path, yaml_content).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let overrides = Overrides::load_from_globs(&[&pattern]).await.unwrap();

        let mut body = json!({"name": "John"});
        let tags = vec!["admin".to_string()];
        overrides.apply("getUser", &tags, "/users", &mut body);

        assert_eq!(body["role"], "administrator");
    }

    #[tokio::test]
    async fn test_overrides_with_merge_mode() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("overrides.yaml");

        let yaml_content = r#"
- targets:
    - "operation:getUser"
  mode: merge
  patch:
    - op: add
      path: "/user"
      value:
        age: 30
"#;

        fs::write(&yaml_path, yaml_content).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let overrides = Overrides::load_from_globs(&[&pattern]).await.unwrap();

        let mut body = json!({"user": {"name": "John"}});
        overrides.apply("getUser", &[], "/users", &mut body);

        assert_eq!(body["user"]["name"], "John");
        assert_eq!(body["user"]["age"], 30);
    }

    #[tokio::test]
    async fn test_load_from_globs_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path1 = temp_dir.path().join("override1.yaml");
        let yaml_path2 = temp_dir.path().join("override2.yaml");

        let yaml_content1 = r#"
- targets:
    - "operation:getUser"
  patch:
    - op: add
      path: "/field1"
      value: "value1"
"#;

        let yaml_content2 = r#"
- targets:
    - "operation:createUser"
  patch:
    - op: add
      path: "/field2"
      value: "value2"
"#;

        fs::write(&yaml_path1, yaml_content1).await.unwrap();
        fs::write(&yaml_path2, yaml_content2).await.unwrap();

        let pattern = format!("{}/**/*.yaml", temp_dir.path().display());
        let result = Overrides::load_from_globs(&[&pattern]).await;

        assert!(result.is_ok());
        let overrides = result.unwrap();
        assert_eq!(overrides.rules.len(), 2);
    }

    #[test]
    fn test_override_rule_deserialize() {
        let yaml = r#"
targets:
  - "operation:getUser"
patch:
  - op: replace
    path: "/name"
    value: "Jane"
when: "env == 'test'"
mode: merge
post_templating: true
"#;

        let rule: OverrideRule = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(rule.targets.len(), 1);
        assert_eq!(rule.patch.len(), 1);
        assert_eq!(rule.when, Some("env == 'test'".to_string()));
        assert_eq!(rule.mode, OverrideMode::Merge);
        assert!(rule.post_templating);
    }
}
