//! Template expansion utilities for request context variables
//!
//! This module provides backward-compatible template expansion functions.
//! The primary implementation has moved to `mockforge-template-expansion` crate
//! to avoid Send issues with rng() in async contexts.
//!
//! For new code, import directly from mockforge-template-expansion crate:
//! ```rust
//! use mockforge_template_expansion::expand_templates_in_json;
//! ```

/// Expand template variables in a JSON value recursively using request context
///
/// **Note**: This function has been moved to `mockforge-template-expansion` crate.
/// Use `mockforge_template_expansion::expand_templates_in_json` for the full implementation.
///
/// This backward-compatible version performs basic template expansion for common patterns.
/// It handles `{{method}}`, `{{path}}`, `{{query.*}}`, `{{path.*}}`, `{{headers.*}}`, and `{{body.*}}`.
///
/// # Arguments
/// * `value` - JSON value to process
/// * `context` - Request context for template variable expansion
///
/// # Returns
/// JSON value with template variables expanded
#[deprecated(note = "Use mockforge_template_expansion::expand_templates_in_json instead")]
pub fn expand_templates_in_json(
    value: serde_json::Value,
    context: &crate::ai_response::RequestContext,
) -> serde_json::Value {
    use serde_json::Value;

    match value {
        Value::String(s) => {
            let mut result = s;

            // Normalize {{request.*}} prefix to standard format
            result = result
                .replace("{{request.query.", "{{query.")
                .replace("{{request.path.", "{{path.")
                .replace("{{request.headers.", "{{headers.")
                .replace("{{request.body.", "{{body.")
                .replace("{{request.method}}", "{{method}}")
                .replace("{{request.path}}", "{{path}}");

            // Replace {{method}}
            result = result.replace("{{method}}", &context.method);

            // Replace {{path}}
            result = result.replace("{{path}}", &context.path);

            // Replace {{path.*}} variables
            for (key, val) in &context.path_params {
                let placeholder = format!("{{{{path.{key}}}}}");
                let replacement = value_to_string(val);
                result = result.replace(&placeholder, &replacement);
            }

            // Replace {{query.*}} variables
            for (key, val) in &context.query_params {
                let placeholder = format!("{{{{query.{key}}}}}");
                let replacement = value_to_string(val);
                result = result.replace(&placeholder, &replacement);
            }

            // Replace {{headers.*}} variables
            for (key, val) in &context.headers {
                let placeholder = format!("{{{{headers.{key}}}}}");
                let replacement = value_to_string(val);
                result = result.replace(&placeholder, &replacement);
            }

            // Replace {{body.*}} variables
            if let Some(body) = &context.body {
                if let Some(obj) = body.as_object() {
                    for (key, val) in obj {
                        let placeholder = format!("{{{{body.{key}}}}}");
                        let replacement = value_to_string(val);
                        result = result.replace(&placeholder, &replacement);
                    }
                }
            }

            Value::String(result)
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|v| expand_templates_in_json(v, context)).collect())
        }
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, expand_templates_in_json(v, context)))
                .collect(),
        ),
        _ => value,
    }
}

/// Helper to convert a JSON value to a string representation
fn value_to_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => serde_json::to_string(val).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_response::RequestContext;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    #[allow(deprecated)]
    fn test_expand_templates_basic() {
        let context = RequestContext {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            ..Default::default()
        };

        let value = json!({"message": "Request to {{path}} with {{method}}"});
        let expanded = expand_templates_in_json(value, &context);

        assert_eq!(expanded["message"], "Request to /api/users with GET");
    }

    #[test]
    #[allow(deprecated)]
    fn test_expand_templates_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("search".to_string(), json!("test"));

        let context = RequestContext {
            method: "GET".to_string(),
            path: "/search".to_string(),
            query_params,
            ..Default::default()
        };

        let value = json!({"query": "{{query.search}}"});
        let expanded = expand_templates_in_json(value, &context);

        assert_eq!(expanded["query"], "test");
    }

    #[test]
    #[allow(deprecated)]
    fn test_expand_templates_request_prefix() {
        let context = RequestContext {
            method: "POST".to_string(),
            path: "/data".to_string(),
            ..Default::default()
        };

        let value = json!({"msg": "{{request.method}} to {{request.path}}"});
        let expanded = expand_templates_in_json(value, &context);

        assert_eq!(expanded["msg"], "POST to /data");
    }

    #[test]
    #[allow(deprecated)]
    fn test_expand_templates_nested() {
        let context = RequestContext {
            method: "GET".to_string(),
            path: "/nested".to_string(),
            ..Default::default()
        };

        let value = json!({
            "outer": {
                "inner": "{{method}}"
            },
            "array": ["{{path}}", "static"]
        });
        let expanded = expand_templates_in_json(value, &context);

        assert_eq!(expanded["outer"]["inner"], "GET");
        assert_eq!(expanded["array"][0], "/nested");
        assert_eq!(expanded["array"][1], "static");
    }
}
