//! Override data models and types
//!
//! This module defines the core data structures for the override system:
//! - OverrideRule: Configuration for applying overrides
//! - OverrideMode: How patches are applied
//! - PatchOp: Individual patch operations
//! - Overrides: Container for multiple rules

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Override rule configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverrideRule {
    /// Target selectors: "operation:opId", "tag:Tag", "regex:pattern", or "path:pattern"
    pub targets: Vec<String>,
    /// JSON patch operations to apply
    pub patch: Vec<PatchOp>,
    /// Optional condition expression
    pub when: Option<String>,
    /// Override mode: "replace" (default) or "merge"
    #[serde(default = "default_mode")]
    pub mode: OverrideMode,
    /// Whether to apply post-templating expansion after patching
    #[serde(default = "default_post_templating")]
    pub post_templating: bool,
}

/// Override mode for applying patches
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum OverrideMode {
    /// Replace values (default JSON patch behavior)
    #[serde(rename = "replace")]
    Replace,
    /// Merge objects and arrays instead of replacing
    #[serde(rename = "merge")]
    Merge,
}

/// JSON patch operation
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

/// Container for override rules with caching
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Overrides {
    /// Loaded override rules
    pub rules: Vec<OverrideRule>,
    /// Compiled regex patterns for performance
    #[serde(skip)]
    pub regex_cache: HashMap<String, regex::Regex>,
}

fn default_mode() -> OverrideMode {
    OverrideMode::Replace
}

fn default_post_templating() -> bool {
    false
}
