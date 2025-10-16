//! Override loading functionality
//!
//! This module handles loading override rules from YAML files
//! using glob patterns and environment variable configuration.

use globwalk::GlobWalkerBuilder;
use std::collections::HashMap;

use super::models::{OverrideRule, Overrides, PatchOp};
use crate::templating::expand_tokens as core_expand_tokens;

impl Overrides {
    /// Load overrides from glob patterns, with support for MOCKFORGE_HTTP_OVERRIDES_GLOB
    pub async fn load_from_globs(patterns: &[&str]) -> anyhow::Result<Self> {
        // Check for environment variable override
        let patterns: Vec<String> =
            if let Ok(env_patterns) = std::env::var("MOCKFORGE_HTTP_OVERRIDES_GLOB") {
                env_patterns.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                patterns.iter().map(|s| s.to_string()).collect()
            };

        let mut rules = Vec::new();
        let mut regex_cache = HashMap::new();

        for pat in patterns {
            // Check if the pattern is an absolute path to a specific file
            if std::path::Path::new(&pat).is_absolute() && std::path::Path::new(&pat).is_file() {
                // Handle absolute file path
                let path = std::path::Path::new(&pat).to_path_buf();
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
                                let pattern = target
                                    .strip_prefix("regex:")
                                    .or_else(|| target.strip_prefix("path:"))
                                    .unwrap();
                                if !regex_cache.contains_key(pattern) {
                                    let regex = regex::Regex::new(pattern)?;
                                    regex_cache.insert(pattern.to_string(), regex);
                                }
                            }
                        }
                    }

                    rules.extend(file_rules);
                }
            } else {
                // Handle glob patterns
                for entry in GlobWalkerBuilder::from_patterns(".", &[pat]).build()? {
                    let entry = entry?;
                    let path = entry.path().to_path_buf();
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
                                    let pattern = target
                                        .strip_prefix("regex:")
                                        .or_else(|| target.strip_prefix("path:"))
                                        .unwrap();
                                    if !regex_cache.contains_key(pattern) {
                                        let regex = regex::Regex::new(pattern)?;
                                        regex_cache.insert(pattern.to_string(), regex);
                                    }
                                }
                            }
                        }

                        rules.extend(file_rules);
                    }
                }
            }
        }

        Ok(Self { rules, regex_cache })
    }
}
