//! Schema design analysis
//!
//! This module analyzes schema design for security and consistency issues:
//! - Too many optional fields
//! - Inconsistent patterns
//! - Missing validation

use super::types::{ThreatCategory, ThreatFinding, ThreatLevel};
use crate::openapi::OpenApiSpec;
use openapiv3::ReferenceOr;
use std::collections::HashMap;

/// Schema analyzer for design issues
pub struct SchemaAnalyzer {
    /// Maximum optional fields threshold
    max_optional_fields: usize,
}

impl SchemaAnalyzer {
    /// Create a new schema analyzer
    pub fn new(max_optional_fields: usize) -> Self {
        Self {
            max_optional_fields,
        }
    }

    /// Analyze schemas for design issues
    pub fn analyze_schemas(&self, spec: &OpenApiSpec) -> Vec<ThreatFinding> {
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
                                    findings.extend(
                                        self.analyze_schema_design(schema, &base_path, "request"),
                                    );
                                }
                            }
                        }
                    }

                    // Analyze responses
                    for (status_code, response) in &operation.responses.responses {
                        if let ReferenceOr::Item(resp) = response {
                            for media_type in resp.content.values() {
                                if let Some(schema) = &media_type.schema {
                                    findings.extend(self.analyze_schema_design(
                                        schema,
                                        &base_path,
                                        &format!("response.{}", status_code),
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

    /// Analyze schema design
    fn analyze_schema_design(
        &self,
        schema_ref: &ReferenceOr<openapiv3::Schema>,
        base_path: &str,
        context: &str,
    ) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        if let ReferenceOr::Item(schema) = schema_ref {
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) =
                &schema.schema_kind
            {
                let required = obj_type.required.len();
                let total_fields = obj_type.properties.len();
                let optional_fields = total_fields.saturating_sub(required);

                if optional_fields > self.max_optional_fields {
                    findings.push(ThreatFinding {
                        finding_type: ThreatCategory::ExcessiveOptionalFields,
                        severity: ThreatLevel::Medium,
                        description: format!(
                            "Schema has {} optional fields (threshold: {}), which may indicate inconsistent backend behavior",
                            optional_fields,
                            self.max_optional_fields
                        ),
                        field_path: Some(format!("{}.{}", base_path, context)),
                        context: HashMap::new(),
                        confidence: 0.7,
                    });
                }

                // Check for missing validation constraints
                for (prop_name, prop_schema) in &obj_type.properties {
                    if let ReferenceOr::Item(prop_schema_item) = prop_schema {
                        // Check if it's a string type and has validation constraints
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::String(string_type)) =
                            &prop_schema_item.as_ref().schema_kind
                        {
                            // Check if string has format, pattern, or maxLength
                            let has_format = !matches!(
                                string_type.format,
                                openapiv3::VariantOrUnknownOrEmpty::Empty
                            );
                            let has_pattern = string_type.pattern.is_some();
                            let has_max_length = string_type.max_length.is_some();

                            if !has_format && !has_pattern && !has_max_length {
                                findings.push(ThreatFinding {
                                    finding_type: ThreatCategory::MissingValidation,
                                    severity: ThreatLevel::Low,
                                    description: format!(
                                        "String field '{}' lacks validation constraints (format, pattern, or maxLength)",
                                        prop_name
                                    ),
                                    field_path: Some(format!("{}.{}.{}", base_path, context, prop_name)),
                                    context: HashMap::new(),
                                    confidence: 0.6,
                                });
                            }
                        }
                    }
                }
            }
        }

        findings
    }
}

impl Default for SchemaAnalyzer {
    fn default() -> Self {
        Self::new(10)
    }
}
