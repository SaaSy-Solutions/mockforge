//! Error response analysis
//!
//! This module analyzes error responses for security issues like:
//! - Stack trace leakage
//! - Database error messages
//! - Internal paths/URLs
//! - Sensitive configuration

use super::types::{ThreatCategory, ThreatFinding, ThreatLevel};
use crate::openapi::OpenApiSpec;
use openapiv3::ReferenceOr;
use std::collections::HashMap;

/// Error analyzer for detecting error message leakage
pub struct ErrorAnalyzer {
    /// Whether error leakage detection is enabled
    enabled: bool,
}

impl ErrorAnalyzer {
    /// Create a new error analyzer
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Analyze error responses for leakage
    pub fn analyze_errors(&self, spec: &OpenApiSpec) -> Vec<ThreatFinding> {
        if !self.enabled {
            return Vec::new();
        }

        let mut findings = Vec::new();

        for (path, path_item) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(path_item) = path_item {
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

                    // Check error responses (4xx, 5xx)
                    for (status_code, response) in &operation.responses.responses {
                        let status_num = match status_code {
                            openapiv3::StatusCode::Code(code) => *code,
                            openapiv3::StatusCode::Range(range) => {
                                // Range is a u16: 2 = 2XX, 3 = 3XX, 4 = 4XX, 5 = 5XX
                                match *range {
                                    4 => 400,
                                    5 => 500,
                                    _ => continue,
                                }
                            }
                        };

                        // Focus on error status codes
                        if status_num >= 400 {
                            if let openapiv3::ReferenceOr::Item(resp) = response {
                                for (content_type, media_type) in &resp.content {
                                    if let Some(schema) = &media_type.schema {
                                        findings.extend(
                                            self.analyze_error_schema(
                                                schema, &base_path, status_num,
                                            ),
                                        );
                                    }

                                    // Check examples
                                    for example in media_type.examples.values() {
                                        if let openapiv3::ReferenceOr::Item(example_item) = example
                                        {
                                            if let Some(example_value) = &example_item.value {
                                                findings.extend(self.analyze_error_example(
                                                    example_value,
                                                    &base_path,
                                                    status_num,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    /// Analyze error schema for leakage patterns
    fn analyze_error_schema(
        &self,
        schema_ref: &ReferenceOr<openapiv3::Schema>,
        base_path: &str,
        status_code: u16,
    ) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        if let ReferenceOr::Item(schema) = schema_ref {
            // Check description for stack trace keywords
            if let Some(description) = &schema.schema_data.description {
                if self.contains_stack_trace_keywords(description) {
                    findings.push(ThreatFinding {
                        finding_type: ThreatCategory::StackTraceLeakage,
                        severity: ThreatLevel::High,
                        description:
                            "Error response schema description suggests stack trace exposure"
                                .to_string(),
                        field_path: Some(format!("{}.error.{}", base_path, status_code)),
                        context: HashMap::new(),
                        confidence: 0.8,
                    });
                }
            }

            // Check properties for sensitive fields
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) =
                &schema.schema_kind
            {
                for (prop_name, _) in &obj_type.properties {
                    let prop_lower = prop_name.to_lowercase();
                    if prop_lower.contains("stack")
                        || prop_lower.contains("trace")
                        || prop_lower.contains("exception")
                        || prop_lower.contains("error_detail")
                    {
                        findings.push(ThreatFinding {
                            finding_type: ThreatCategory::StackTraceLeakage,
                            severity: ThreatLevel::Critical,
                            description: format!(
                                "Error response contains '{}' field which may leak stack traces",
                                prop_name
                            ),
                            field_path: Some(format!(
                                "{}.error.{}.{}",
                                base_path, status_code, prop_name
                            )),
                            context: HashMap::new(),
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        findings
    }

    /// Analyze error example for leakage
    fn analyze_error_example(
        &self,
        example: &serde_json::Value,
        base_path: &str,
        status_code: u16,
    ) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        if let Some(obj) = example.as_object() {
            for (key, value) in obj {
                // Check for stack traces in values
                if let Some(str_value) = value.as_str() {
                    if self.contains_stack_trace_patterns(str_value) {
                        findings.push(ThreatFinding {
                            finding_type: ThreatCategory::StackTraceLeakage,
                            severity: ThreatLevel::Critical,
                            description:
                                "Error example contains stack trace or sensitive error details"
                                    .to_string(),
                            field_path: Some(format!(
                                "{}.error.{}.{}",
                                base_path, status_code, key
                            )),
                            context: HashMap::new(),
                            confidence: 1.0,
                        });
                    }

                    // Check for file paths
                    if str_value.contains("/")
                        && (str_value.contains(".py")
                            || str_value.contains(".java")
                            || str_value.contains(".rs"))
                    {
                        findings.push(ThreatFinding {
                            finding_type: ThreatCategory::ErrorLeakage,
                            severity: ThreatLevel::Medium,
                            description:
                                "Error message contains file path which may leak internal structure"
                                    .to_string(),
                            field_path: Some(format!(
                                "{}.error.{}.{}",
                                base_path, status_code, key
                            )),
                            context: HashMap::new(),
                            confidence: 0.7,
                        });
                    }
                }
            }
        }

        findings
    }

    /// Check if text contains stack trace keywords
    fn contains_stack_trace_keywords(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        text_lower.contains("stack trace")
            || text_lower.contains("stacktrace")
            || text_lower.contains("exception")
            || text_lower.contains("traceback")
    }

    /// Check if text contains stack trace patterns
    fn contains_stack_trace_patterns(&self, text: &str) -> bool {
        // Look for common stack trace patterns
        text.contains("at ") && (text.contains("(") || text.contains("line"))
            || text.contains("Traceback")
            || text.contains("Exception in thread")
    }
}

impl Default for ErrorAnalyzer {
    fn default() -> Self {
        Self::new(true)
    }
}
