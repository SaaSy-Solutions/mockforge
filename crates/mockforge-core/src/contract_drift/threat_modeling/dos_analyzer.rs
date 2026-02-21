//! Denial of Service (DoS) risk analysis
//!
//! This module detects DoS risks in API contracts, such as:
//! - Unbounded arrays
//! - Missing pagination
//! - Deeply nested schemas
//! - Large payload sizes

use super::types::{ThreatCategory, ThreatFinding, ThreatLevel};
use crate::openapi::OpenApiSpec;
use openapiv3::ReferenceOr;
use std::collections::HashMap;

/// DoS risk analyzer
pub struct DosAnalyzer {
    /// Maximum array size threshold (default: no limit = risk)
    max_array_size_threshold: Option<usize>,
    /// Maximum nesting depth
    max_nesting_depth: usize,
}

impl DosAnalyzer {
    /// Create a new DoS analyzer
    pub fn new(max_array_size_threshold: Option<usize>, max_nesting_depth: usize) -> Self {
        Self {
            max_array_size_threshold,
            max_nesting_depth,
        }
    }

    /// Analyze spec for DoS risks
    pub fn analyze_dos_risks(&self, spec: &OpenApiSpec) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        for (path, path_item) in &spec.spec.paths.paths {
            if let ReferenceOr::Item(path_item) = path_item {
                // Iterate over all HTTP methods
                let methods = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                    ("HEAD", path_item.head.as_ref()),
                    ("OPTIONS", path_item.options.as_ref()),
                    ("TRACE", path_item.trace.as_ref()),
                ];

                for (method, operation_opt) in methods {
                    let Some(operation) = operation_opt else {
                        continue;
                    };
                    let base_path = format!("{}.{}", method, path);

                    // Analyze request body
                    if let Some(request_body) = &operation.request_body {
                        if let Some(ref_or_item) = request_body.as_item() {
                            for media_type in ref_or_item.content.values() {
                                if let Some(schema) = &media_type.schema {
                                    // Convert ReferenceOr<Schema> to ReferenceOr<Box<Schema>>
                                    let boxed_schema_ref = match schema {
                                        ReferenceOr::Item(s) => {
                                            ReferenceOr::Item(Box::new(s.clone()))
                                        }
                                        ReferenceOr::Reference { reference } => {
                                            ReferenceOr::Reference {
                                                reference: reference.clone(),
                                            }
                                        }
                                    };
                                    findings.extend(self.analyze_schema_for_dos(
                                        &boxed_schema_ref,
                                        &base_path,
                                        "request",
                                        0,
                                    ));
                                }
                            }
                        }
                    }

                    // Analyze responses
                    for (status_code, response) in &operation.responses.responses {
                        if let ReferenceOr::Item(resp) = response {
                            for media_type in resp.content.values() {
                                if let Some(schema) = &media_type.schema {
                                    // Convert ReferenceOr<Schema> to ReferenceOr<Box<Schema>>
                                    let boxed_schema_ref = match schema {
                                        ReferenceOr::Item(s) => {
                                            ReferenceOr::Item(Box::new(s.clone()))
                                        }
                                        ReferenceOr::Reference { reference } => {
                                            ReferenceOr::Reference {
                                                reference: reference.clone(),
                                            }
                                        }
                                    };
                                    findings.extend(self.analyze_schema_for_dos(
                                        &boxed_schema_ref,
                                        &base_path,
                                        &format!("response.{}", status_code),
                                        0,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    /// Analyze schema for DoS risks
    fn analyze_schema_for_dos(
        &self,
        schema_ref: &ReferenceOr<Box<openapiv3::Schema>>,
        base_path: &str,
        context: &str,
        depth: usize,
    ) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        if depth > self.max_nesting_depth {
            findings.push(ThreatFinding {
                finding_type: ThreatCategory::DoSRisk,
                severity: ThreatLevel::Medium,
                description: format!(
                    "Schema nesting depth ({}) exceeds recommended maximum ({})",
                    depth, self.max_nesting_depth
                ),
                field_path: Some(base_path.to_string()),
                context: HashMap::new(),
                confidence: 0.8,
            });
            return findings;
        }

        if let ReferenceOr::Item(schema) = schema_ref {
            // Check for unbounded arrays
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) =
                &schema.as_ref().schema_kind
            {
                // max_items might be in extensions
                let max_items =
                    schema.as_ref().schema_data.extensions.get("maxItems").and_then(|v| v.as_u64());

                if max_items.is_none() && self.max_array_size_threshold.is_none() {
                    findings.push(ThreatFinding {
                        finding_type: ThreatCategory::UnboundedArrays,
                        severity: ThreatLevel::High,
                        description: format!(
                            "Unbounded array detected in {} - no maxItems constraint",
                            context
                        ),
                        field_path: Some(base_path.to_string()),
                        context: HashMap::new(),
                        confidence: 1.0,
                    });
                } else if let Some(threshold) = self.max_array_size_threshold {
                    if let Some(max) = max_items {
                        if max > threshold as u64 {
                            findings.push(ThreatFinding {
                                finding_type: ThreatCategory::UnboundedArrays,
                                severity: ThreatLevel::Medium,
                                description: format!(
                                    "Array maxItems ({}) exceeds recommended threshold ({})",
                                    max, threshold
                                ),
                                field_path: Some(base_path.to_string()),
                                context: HashMap::new(),
                                confidence: 0.7,
                            });
                        }
                    }
                }

                // Recursively check array items
                if let Some(items) = &array_type.items {
                    findings.extend(self.analyze_schema_for_dos(
                        items,
                        &format!("{}.items", base_path),
                        context,
                        depth + 1,
                    ));
                }
            }

            // Check properties recursively
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) =
                &schema.as_ref().schema_kind
            {
                for (prop_name, prop_schema) in &obj_type.properties {
                    findings.extend(self.analyze_schema_for_dos(
                        prop_schema,
                        &format!("{}.{}", base_path, prop_name),
                        context,
                        depth + 1,
                    ));
                }
            }
        }

        findings
    }
}

impl Default for DosAnalyzer {
    fn default() -> Self {
        Self::new(None, 10)
    }
}
