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

/// Configuration for a single override rule
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverrideRule {
    /// Target selectors for matching operations:
    /// - "operation:opId" - match by operation ID
    /// - "tag:Tag" - match by OpenAPI tag
    /// - "regex:pattern" - match path by regex pattern
    /// - "path:pattern" - match path by literal pattern
    pub targets: Vec<String>,
    /// JSON patch operations to apply when this rule matches
    pub patch: Vec<PatchOp>,
    /// Optional condition expression (JSONPath/XPath) that must evaluate to true
    pub when: Option<String>,
    /// Override mode for applying patches: "replace" (default) or "merge"
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

/// JSON patch operation (RFC 6902 format)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op")]
pub enum PatchOp {
    /// Add a new value at the specified path
    #[serde(rename = "add")]
    Add {
        /// JSON pointer path to add the value
        path: String,
        /// Value to add
        value: Value,
    },
    /// Replace the value at the specified path
    #[serde(rename = "replace")]
    Replace {
        /// JSON pointer path to replace
        path: String,
        /// New value
        value: Value,
    },
    /// Remove the value at the specified path
    #[serde(rename = "remove")]
    Remove {
        /// JSON pointer path to remove
        path: String,
    },
}

/// Container for override rules with performance optimizations
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Overrides {
    /// Loaded override rules to apply to responses
    pub rules: Vec<OverrideRule>,
    /// Compiled regex patterns for performance (cached compilation)
    #[serde(skip)]
    pub regex_cache: HashMap<String, regex::Regex>,
}

fn default_mode() -> OverrideMode {
    OverrideMode::Replace
}

fn default_post_templating() -> bool {
    false
}
