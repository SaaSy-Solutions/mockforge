//! Target matching logic for overrides
//!
//! This module contains functions for determining whether an override rule
//! should be applied to a given operation based on its targets.

use super::models::OverrideRule;

/// Check if an override rule matches the given operation
pub fn matches_target(
    rule: &OverrideRule,
    operation_id: &str,
    tags: &[String],
    path: &str,
    regex_cache: &std::collections::HashMap<String, regex::Regex>,
) -> bool {
    for target in &rule.targets {
        if target.starts_with("operation:") {
            let op_id = target.strip_prefix("operation:").unwrap();
            if op_id == operation_id {
                return true;
            }
        } else if target.starts_with("tag:") {
            let tag = target.strip_prefix("tag:").unwrap();
            if tags.contains(&tag.to_string()) {
                return true;
            }
        } else if target.starts_with("regex:") {
            let pattern = target.strip_prefix("regex:").unwrap();
            if let Some(regex) = regex_cache.get(pattern) {
                if regex.is_match(operation_id) {
                    return true;
                }
            }
        } else if target.starts_with("path:") {
            let pattern = target.strip_prefix("path:").unwrap();
            if let Some(regex) = regex_cache.get(pattern) {
                if regex.is_match(path) {
                    return true;
                }
            }
        } else if target == "*" {
            // Wildcard matches everything
            return true;
        }
    }
    false
}
