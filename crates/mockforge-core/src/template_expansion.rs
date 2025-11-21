//! Template expansion utilities for request context variables
//!
//! This module provides Send-safe template expansion that does NOT use rng().
//! It is completely isolated from the templating module to avoid Send issues
//! in async contexts.

use crate::ai_response::{expand_prompt_template, RequestContext};
use serde_json::Value;

/// Expand template variables in a JSON value recursively using request context
///
/// This function is Send-safe and does not use rng() or any non-Send types.
/// It only uses expand_prompt_template which performs simple string replacements.
///
/// # Arguments
/// * `value` - JSON value to process
/// * `context` - Request context for template variable expansion
///
/// # Returns
/// New JSON value with all template tokens expanded
pub fn expand_templates_in_json(
    value: Value,
    context: &RequestContext,
) -> Value {
    match value {
        Value::String(s) => {
            // Normalize {{request.query.name}} to {{query.name}} format for compatibility
            let normalized = s
                .replace("{{request.query.", "{{query.")
                .replace("{{request.path.", "{{path.")
                .replace("{{request.headers.", "{{headers.")
                .replace("{{request.body.", "{{body.")
                .replace("{{request.method}}", "{{method}}")
                .replace("{{request.path}}", "{{path}}");
            // Use expand_prompt_template which is Send-safe and doesn't use rng()
            Value::String(expand_prompt_template(&normalized, context))
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| expand_templates_in_json(v, context))
                .collect(),
        ),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, expand_templates_in_json(v, context)))
                .collect(),
        ),
        _ => value,
    }
}

