//! Body transformation middleware for proxy requests and responses
//!
//! This module provides functionality to transform request and response bodies
//! using JSONPath expressions and template expansion. Useful for browser proxy
//! mode where you want to inspect and replace values in intercepted traffic.

use crate::config::{BodyTransform, BodyTransformRule, TransformOperation};
use mockforge_core::templating::TemplateEngine;
use mockforge_core::Result;
use serde_json::Value;
use tracing::{debug, error, warn};

/// Body transformation middleware that applies JSONPath-based transformations
pub struct BodyTransformationMiddleware {
    /// Request transformation rules
    request_rules: Vec<BodyTransformRule>,
    /// Response transformation rules
    response_rules: Vec<BodyTransformRule>,
    /// Template engine for expanding template tokens
    template_engine: TemplateEngine,
}

impl BodyTransformationMiddleware {
    /// Create a new body transformation middleware
    pub fn new(
        request_rules: Vec<BodyTransformRule>,
        response_rules: Vec<BodyTransformRule>,
    ) -> Self {
        Self {
            request_rules,
            response_rules,
            template_engine: TemplateEngine::new(),
        }
    }

    /// Transform a request body based on configured rules
    pub fn transform_request_body(&self, url: &str, body: &mut Option<Vec<u8>>) -> Result<()> {
        if body.is_none() || self.request_rules.is_empty() {
            return Ok(());
        }

        // Find matching rules for this URL
        let matching_rules: Vec<&BodyTransformRule> =
            self.request_rules.iter().filter(|rule| rule.matches_url(url)).collect();

        if matching_rules.is_empty() {
            return Ok(());
        }

        // Try to parse as JSON
        let body_str = match String::from_utf8(body.as_ref().unwrap().clone()) {
            Ok(s) => s,
            Err(_) => {
                // Not UTF-8, skip transformation
                return Ok(());
            }
        };

        // Try to parse as JSON
        let mut json: Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => {
                // Not JSON, skip transformation
                debug!("Request body is not JSON, skipping transformation");
                return Ok(());
            }
        };

        // Apply all matching rules
        for rule in matching_rules {
            if let Err(e) = self.apply_transform_rule(&mut json, rule) {
                warn!("Failed to apply request transformation rule: {}", e);
            }
        }

        // Serialize back to bytes
        let new_body = serde_json::to_vec(&json)?;
        *body = Some(new_body);

        Ok(())
    }

    /// Transform a response body based on configured rules
    pub fn transform_response_body(
        &self,
        url: &str,
        status_code: u16,
        body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        if body.is_none() || self.response_rules.is_empty() {
            return Ok(());
        }

        // Find matching rules for this URL
        let matching_rules: Vec<&BodyTransformRule> = self
            .response_rules
            .iter()
            .filter(|rule| rule.matches_url(url) && rule.matches_status_code(status_code))
            .collect();

        if matching_rules.is_empty() {
            return Ok(());
        }

        // Try to parse as JSON
        let body_str = match String::from_utf8(body.as_ref().unwrap().clone()) {
            Ok(s) => s,
            Err(_) => {
                // Not UTF-8, skip transformation
                return Ok(());
            }
        };

        // Try to parse as JSON
        let mut json: Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => {
                // Not JSON, skip transformation
                debug!("Response body is not JSON, skipping transformation");
                return Ok(());
            }
        };

        // Apply all matching rules
        for rule in matching_rules {
            if let Err(e) = self.apply_transform_rule(&mut json, rule) {
                warn!("Failed to apply response transformation rule: {}", e);
            }
        }

        // Serialize back to bytes
        let new_body = serde_json::to_vec(&json)?;
        *body = Some(new_body);

        Ok(())
    }

    /// Apply a single transformation rule to JSON
    fn apply_transform_rule(&self, json: &mut Value, rule: &BodyTransformRule) -> Result<()> {
        for transform in &rule.body_transforms {
            match self.apply_single_transform(json, transform) {
                Ok(_) => {
                    debug!("Applied transformation: {} -> {}", transform.path, transform.replace);
                }
                Err(e) => {
                    error!("Failed to apply transformation {}: {}", transform.path, e);
                    // Continue with other transforms even if one fails
                }
            }
        }
        Ok(())
    }

    /// Apply a single transform to JSON using JSONPath
    /// Uses a simplified path-based approach for common JSONPath expressions
    fn apply_single_transform(&self, json: &mut Value, transform: &BodyTransform) -> Result<()> {
        // For now, use the simplified path-based approach
        // Full JSONPath support can be added later if needed
        self.apply_single_transform_simple(json, transform)
    }
}

// Simplified implementation that works with direct path access
impl BodyTransformationMiddleware {
    /// Apply a single transform using a simplified path-based approach
    /// This works for simple paths like "$.field" or "$.field.subfield"
    fn apply_single_transform_simple(
        &self,
        json: &mut Value,
        transform: &BodyTransform,
    ) -> Result<()> {
        // Expand template in replacement value
        let replacement_value = self.template_engine.expand_str(&transform.replace);

        // Parse replacement value as JSON if possible, otherwise use as string
        let replacement_json: Value = match serde_json::from_str(&replacement_value) {
            Ok(v) => v,
            Err(_) => Value::String(replacement_value.clone()),
        };

        // Extract path components (simplified - supports $.field.subfield)
        let path = transform.path.trim_start_matches("$.");
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(mockforge_core::Error::internal("Empty JSONPath".to_string()));
        }

        // Navigate to the target location
        let mut current = json;
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            if is_last {
                // Apply the transformation
                // Check type first to avoid multiple mutable borrows
                match transform.operation {
                    TransformOperation::Replace => {
                        match current {
                            Value::Object(ref mut obj) => {
                                obj.insert(part.to_string(), replacement_json.clone());
                            }
                            Value::Array(ref mut arr) => {
                                if let Ok(idx) = part.parse::<usize>() {
                                    if idx < arr.len() {
                                        arr[idx] = replacement_json.clone();
                                    }
                                }
                            }
                            _ => {}
                        }
                        // Break early since we're done
                        break;
                    }
                    TransformOperation::Add => {
                        if let Value::Object(ref mut obj) = current {
                            obj.insert(part.to_string(), replacement_json.clone());
                        }
                        // Break early since we're done
                        break;
                    }
                    TransformOperation::Remove => {
                        match current {
                            Value::Object(ref mut obj) => {
                                // part is &str, which String can Borrow from
                                obj.remove(*part);
                            }
                            Value::Array(ref mut arr) => {
                                if let Ok(idx) = part.parse::<usize>() {
                                    if idx < arr.len() {
                                        arr.remove(idx);
                                    }
                                }
                            }
                            _ => {}
                        }
                        // Break early since we're done
                        break;
                    }
                }
            } else {
                // Navigate deeper - use match to avoid borrow conflicts
                match current {
                    Value::Object(ref mut obj) => {
                        current = obj
                            .entry(part.to_string())
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    }
                    Value::Array(ref mut arr) => {
                        if let Ok(idx) = part.parse::<usize>() {
                            if idx < arr.len() {
                                current = &mut arr[idx];
                            } else {
                                return Err(mockforge_core::Error::internal(format!(
                                    "Array index {} out of bounds",
                                    idx
                                )));
                            }
                        } else {
                            return Err(mockforge_core::Error::internal(format!(
                                "Invalid array index: {}",
                                part
                            )));
                        }
                    }
                    _ => {
                        // Create intermediate objects as needed
                        *current = Value::Object(serde_json::Map::new());
                        if let Value::Object(ref mut obj) = current {
                            current = obj
                                .entry(part.to_string())
                                .or_insert_with(|| Value::Object(serde_json::Map::new()));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod body_transform_tests {
    use super::*;
    use crate::config::{BodyTransform, BodyTransformRule, TransformOperation};

    fn transform(path: &str, replace: &str, operation: TransformOperation) -> BodyTransform {
        BodyTransform {
            path: path.to_string(),
            replace: replace.to_string(),
            operation,
        }
    }

    fn rule(pattern: &str, transforms: Vec<BodyTransform>) -> BodyTransformRule {
        BodyTransformRule {
            pattern: pattern.to_string(),
            status_codes: Vec::new(),
            body_transforms: transforms,
            enabled: true,
        }
    }

    fn body_bytes(value: serde_json::Value) -> Option<Vec<u8>> {
        Some(serde_json::to_vec(&value).expect("serialize fixture body"))
    }

    fn parsed(body: &Option<Vec<u8>>) -> serde_json::Value {
        serde_json::from_slice(body.as_ref().expect("body present")).expect("body is JSON")
    }

    // ─── Request transformation ─────────────────────────────────────────────

    #[test]
    fn replaces_nested_fields_in_request_bodies() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform(
                    "$.user.role",
                    "admin",
                    TransformOperation::Replace,
                )],
            )],
            vec![],
        );
        let mut body = body_bytes(serde_json::json!({"user": {"role": "guest", "name": "sam"}}));

        mw.transform_request_body("/api/users", &mut body).expect("transform succeeds");
        let value = parsed(&body);
        assert_eq!(value["user"]["role"], "admin");
        // Sibling fields survive the rewrite.
        assert_eq!(value["user"]["name"], "sam");
    }

    #[test]
    fn replacement_values_parse_as_json_when_valid() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![
                    transform("$.count", "42", TransformOperation::Replace),
                    transform("$.flag", "true", TransformOperation::Replace),
                    transform("$.label", "hello", TransformOperation::Replace),
                ],
            )],
            vec![],
        );
        let mut body = body_bytes(serde_json::json!({}));

        mw.transform_request_body("/api/things", &mut body).expect("transform succeeds");
        let value = parsed(&body);
        assert_eq!(value["count"], 42);
        assert_eq!(value["flag"], true);
        // Non-JSON replacement text stays a string.
        assert_eq!(value["label"], "hello");
    }

    #[test]
    fn add_creates_intermediate_objects() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform("$.meta.source", "proxy", TransformOperation::Add)],
            )],
            vec![],
        );
        let mut body = body_bytes(serde_json::json!({"user": {}}));

        mw.transform_request_body("/api/users", &mut body).expect("transform succeeds");
        assert_eq!(parsed(&body), serde_json::json!({"user": {}, "meta": {"source": "proxy"}}));
    }

    #[test]
    fn remove_deletes_fields_and_array_items() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![
                    transform("$.user.email", "", TransformOperation::Remove),
                    transform("$.tags.1", "", TransformOperation::Remove),
                ],
            )],
            vec![],
        );
        let mut body = body_bytes(serde_json::json!({
            "user": {"email": "a@example.com", "id": 1},
            "tags": ["a", "b", "c"]
        }));

        mw.transform_request_body("/api/users", &mut body).expect("transform succeeds");
        let value = parsed(&body);
        assert!(value["user"].get("email").is_none());
        assert_eq!(value["user"]["id"], 1);
        assert_eq!(value["tags"], serde_json::json!(["a", "c"]));
    }

    #[test]
    fn array_replace_ignores_out_of_bounds_indexes() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform("$.items.7", "9", TransformOperation::Replace)],
            )],
            vec![],
        );
        let original = serde_json::json!({"items": [1, 2, 3]});
        let mut body = body_bytes(original.clone());

        mw.transform_request_body("/api/items", &mut body).expect("transform succeeds");
        assert_eq!(parsed(&body), original);
    }

    #[test]
    fn failed_transforms_do_not_block_the_rest() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![
                    // Navigating past the end of the array fails, but the
                    // remaining transforms in the rule must still apply.
                    transform("$.items.9.name", "x", TransformOperation::Replace),
                    transform("$.ok", "yes", TransformOperation::Replace),
                ],
            )],
            vec![],
        );
        let mut body = body_bytes(serde_json::json!({"items": [1]}));

        mw.transform_request_body("/api/items", &mut body).expect("transform succeeds");
        let value = parsed(&body);
        assert_eq!(value["items"], serde_json::json!([1]));
        assert_eq!(value["ok"], "yes");
    }

    // ─── Bodies that must not be touched ────────────────────────────────────

    #[test]
    fn non_json_bodies_pass_through() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform("$.x", "1", TransformOperation::Replace)],
            )],
            vec![],
        );
        let original = b"this is not json{".to_vec();
        let mut body = Some(original.clone());

        mw.transform_request_body("/api/users", &mut body).expect("transform succeeds");
        assert_eq!(body.as_deref(), Some(&original[..]));
    }

    #[test]
    fn non_utf8_bodies_pass_through() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform("$.x", "1", TransformOperation::Replace)],
            )],
            vec![],
        );
        let original = vec![0xff, 0xfe, 0x00, 0x01];
        let mut body = Some(original.clone());

        mw.transform_request_body("/api/upload", &mut body).expect("transform succeeds");
        assert_eq!(body.as_deref(), Some(&original[..]));
    }

    #[test]
    fn absent_bodies_and_unmatched_urls_are_noops() {
        let mw = BodyTransformationMiddleware::new(
            vec![rule(
                "/api/*",
                vec![transform("$.x", "1", TransformOperation::Replace)],
            )],
            vec![],
        );

        let mut no_body = None;
        mw.transform_request_body("/api/users", &mut no_body).expect("no body is fine");
        assert!(no_body.is_none());

        let mut body = body_bytes(serde_json::json!({"x": 0}));
        mw.transform_request_body("/other/path", &mut body).expect("no match is fine");
        assert_eq!(parsed(&body)["x"], 0);
    }

    #[test]
    fn disabled_rules_are_skipped() {
        let mut disabled = rule("/api/*", vec![transform("$.x", "1", TransformOperation::Replace)]);
        disabled.enabled = false;
        let mw = BodyTransformationMiddleware::new(vec![disabled], vec![]);
        let original = serde_json::json!({"x": 0});
        let mut body = body_bytes(original.clone());

        mw.transform_request_body("/api/users", &mut body).expect("transform succeeds");
        assert_eq!(parsed(&body), original);
    }

    // ─── Response transformation ────────────────────────────────────────────

    #[test]
    fn response_transforms_filter_by_status_code() {
        let mut response_rule =
            rule("/api/*", vec![transform("$.error.code", "X1", TransformOperation::Replace)]);
        response_rule.status_codes = vec![500];
        let mw = BodyTransformationMiddleware::new(vec![], vec![response_rule]);

        let mut ok_body = body_bytes(serde_json::json!({"error": {"code": "A"}}));
        mw.transform_response_body("/api/users", 200, &mut ok_body)
            .expect("transform succeeds");
        assert_eq!(parsed(&ok_body)["error"]["code"], "A");

        let mut error_body = body_bytes(serde_json::json!({"error": {"code": "A"}}));
        mw.transform_response_body("/api/users", 500, &mut error_body)
            .expect("transform succeeds");
        assert_eq!(parsed(&error_body)["error"]["code"], "X1");
    }

    #[test]
    fn empty_status_filter_matches_all_responses() {
        let mw = BodyTransformationMiddleware::new(
            vec![],
            vec![rule(
                "/api/*",
                vec![transform("$.note", "seen", TransformOperation::Replace)],
            )],
        );
        let mut body = body_bytes(serde_json::json!({}));

        mw.transform_response_body("/api/users", 404, &mut body)
            .expect("transform succeeds");
        assert_eq!(parsed(&body)["note"], "seen");
    }

    // ─── URL and status matching ────────────────────────────────────────────

    #[test]
    fn url_patterns_match_prefixes_and_wildcards() {
        let wildcard = rule("/api/users/*", vec![]);
        assert!(wildcard.matches_url("/api/users/1"));
        assert!(!wildcard.matches_url("/api/orders/1"));

        // Non-wildcard patterns match the exact URL and any path prefixed
        // by them (so "/health" also matches "/healthcheck").
        let exact = rule("/health", vec![]);
        assert!(exact.matches_url("/health"));
        assert!(exact.matches_url("/healthcheck"));
    }

    #[test]
    fn status_filter_matches_all_when_empty() {
        let unfiltered = rule("/api/*", vec![]);
        assert!(unfiltered.matches_status_code(200));
        assert!(unfiltered.matches_status_code(404));

        let mut filtered = rule("/api/*", vec![]);
        filtered.status_codes = vec![500];
        assert!(!filtered.matches_status_code(200));
        assert!(filtered.matches_status_code(500));
    }
}
