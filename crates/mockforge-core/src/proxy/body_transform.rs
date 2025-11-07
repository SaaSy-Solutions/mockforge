//! Body transformation middleware for proxy requests and responses
//!
//! This module provides functionality to transform request and response bodies
//! using JSONPath expressions and template expansion. Useful for browser proxy
//! mode where you want to inspect and replace values in intercepted traffic.

use crate::proxy::config::{BodyTransform, BodyTransformRule, TransformOperation};
use crate::templating::TemplateEngine;
use crate::Result;
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
    pub fn transform_request_body(
        &self,
        url: &str,
        body: &mut Option<Vec<u8>>,
    ) -> Result<()> {
        if body.is_none() || self.request_rules.is_empty() {
            return Ok(());
        }

        // Find matching rules for this URL
        let matching_rules: Vec<&BodyTransformRule> = self
            .request_rules
            .iter()
            .filter(|rule| rule.matches_url(url))
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
    fn apply_transform_rule(
        &self,
        json: &mut Value,
        rule: &BodyTransformRule,
    ) -> Result<()> {
        for transform in &rule.body_transforms {
            match self.apply_single_transform(json, transform) {
                Ok(_) => {
                    debug!(
                        "Applied transformation: {} -> {}",
                        transform.path, transform.replace
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to apply transformation {}: {}",
                        transform.path, e
                    );
                    // Continue with other transforms even if one fails
                }
            }
        }
        Ok(())
    }

    /// Apply a single transform to JSON using JSONPath
    /// Uses a simplified path-based approach for common JSONPath expressions
    fn apply_single_transform(
        &self,
        json: &mut Value,
        transform: &BodyTransform,
    ) -> Result<()> {
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
            return Err(crate::Error::generic("Empty JSONPath".to_string()));
        }

        // Navigate to the target location
        let mut current = json;
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            if is_last {
                // Apply the transformation
                match transform.operation {
                    TransformOperation::Replace => {
                        if let Some(obj) = current.as_object_mut() {
                            obj.insert(part.to_string(), replacement_json.clone());
                        } else if let Some(arr) = current.as_array_mut() {
                            if let Ok(idx) = part.parse::<usize>() {
                                if idx < arr.len() {
                                    arr[idx] = replacement_json.clone();
                                }
                            }
                        }
                    }
                    TransformOperation::Add => {
                        if let Some(obj) = current.as_object_mut() {
                            obj.insert(part.to_string(), replacement_json.clone());
                        }
                    }
                    TransformOperation::Remove => {
                        if let Some(obj) = current.as_object_mut() {
                            obj.remove(part);
                        } else if let Some(arr) = current.as_array_mut() {
                            if let Ok(idx) = part.parse::<usize>() {
                                if idx < arr.len() {
                                    arr.remove(idx);
                                }
                            }
                        }
                    }
                }
            } else {
                // Navigate deeper
                if let Some(obj) = current.as_object_mut() {
                    current = obj
                        .entry(part.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                } else if let Some(arr) = current.as_array_mut() {
                    if let Ok(idx) = part.parse::<usize>() {
                        if idx < arr.len() {
                            current = &mut arr[idx];
                        } else {
                            return Err(crate::Error::generic(format!(
                                "Array index {} out of bounds",
                                idx
                            )));
                        }
                    } else {
                        return Err(crate::Error::generic(format!(
                            "Invalid array index: {}",
                            part
                        )));
                    }
                } else {
                    // Create intermediate objects as needed
                    *current = Value::Object(serde_json::Map::new());
                    if let Some(obj) = current.as_object_mut() {
                        current = obj
                            .entry(part.to_string())
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    }
                }
            }
        }

        Ok(())
    }
}
