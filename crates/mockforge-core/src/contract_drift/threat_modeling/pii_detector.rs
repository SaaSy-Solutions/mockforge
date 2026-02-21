//! PII (Personally Identifiable Information) detection
//!
//! This module detects potential PII exposure in API contracts
//! by analyzing field names, descriptions, and schema patterns.

use super::types::{ThreatCategory, ThreatFinding, ThreatLevel};
use crate::openapi::OpenApiSpec;
use regex::Regex;
use std::collections::HashMap;

/// PII detector for API contracts
pub struct PiiDetector {
    /// PII field name patterns
    pii_patterns: Vec<Regex>,
    /// Common PII field names
    pii_field_names: Vec<String>,
}

impl PiiDetector {
    /// Create a new PII detector
    pub fn new(pii_patterns: Vec<String>) -> Self {
        let regex_patterns: Vec<Regex> = pii_patterns
            .iter()
            .filter_map(|p| Regex::new(&format!(r"(?i){}", p)).ok())
            .collect();

        let field_names = vec![
            "email".to_string(),
            "ssn".to_string(),
            "social_security_number".to_string(),
            "credit_card".to_string(),
            "card_number".to_string(),
            "password".to_string(),
            "token".to_string(),
            "secret".to_string(),
            "api_key".to_string(),
            "access_token".to_string(),
            "refresh_token".to_string(),
            "phone".to_string(),
            "phone_number".to_string(),
            "address".to_string(),
            "date_of_birth".to_string(),
            "dob".to_string(),
        ];

        Self {
            pii_patterns: regex_patterns,
            pii_field_names: field_names,
        }
    }

    /// Detect PII in an OpenAPI spec
    pub fn detect_pii(&self, spec: &OpenApiSpec) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        // Analyze all paths and schemas
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
                    // Analyze request body
                    if let Some(request_body) = &operation.request_body {
                        if let Some(ref_or_item) = request_body.as_item() {
                            for media_type in ref_or_item.content.values() {
                                if let Some(schema) = &media_type.schema {
                                    findings.extend(self.analyze_schema(
                                        schema,
                                        &format!("{}.{}", method, path),
                                        "request",
                                    ));
                                }
                            }
                        }
                    }

                    // Analyze responses
                    for (status_code, response) in &operation.responses.responses {
                        if let openapiv3::ReferenceOr::Item(resp) = response {
                            for media_type in resp.content.values() {
                                if let Some(schema) = &media_type.schema {
                                    findings.extend(self.analyze_schema(
                                        schema,
                                        &format!("{}.{}", method, path),
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

    /// Analyze a schema for PII
    fn analyze_schema(
        &self,
        schema_ref: &openapiv3::ReferenceOr<openapiv3::Schema>,
        base_path: &str,
        _context: &str,
    ) -> Vec<ThreatFinding> {
        let mut findings = Vec::new();

        if let openapiv3::ReferenceOr::Item(schema) = schema_ref {
            // Check schema description
            if let Some(description) = &schema.schema_data.description {
                if self.contains_pii_keywords(description) {
                    findings.push(ThreatFinding {
                        finding_type: ThreatCategory::PiiExposure,
                        severity: ThreatLevel::Medium,
                        description: format!(
                            "Schema description contains PII keywords: {}",
                            description
                        ),
                        field_path: Some(format!("{}.description", base_path)),
                        context: HashMap::new(),
                        confidence: 0.7,
                    });
                }
            }

            // Check properties
            if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) =
                &schema.schema_kind
            {
                for (prop_name, prop_schema) in &obj_type.properties {
                    let field_path = format!("{}.{}", base_path, prop_name);

                    // Check field name
                    if self.is_pii_field_name(prop_name) {
                        findings.push(ThreatFinding {
                            finding_type: ThreatCategory::PiiExposure,
                            severity: ThreatLevel::High,
                            description: format!(
                                "Field '{}' appears to contain PII based on name",
                                prop_name
                            ),
                            field_path: Some(field_path.clone()),
                            context: HashMap::new(),
                            confidence: 0.9,
                        });
                    }

                    // Recursively check nested schemas
                    if let openapiv3::ReferenceOr::Item(prop_schema_item) = prop_schema {
                        if let Some(prop_desc) = &prop_schema_item.as_ref().schema_data.description
                        {
                            if self.contains_pii_keywords(prop_desc) {
                                findings.push(ThreatFinding {
                                    finding_type: ThreatCategory::PiiExposure,
                                    severity: ThreatLevel::Medium,
                                    description: format!(
                                        "Field '{}' description contains PII keywords",
                                        prop_name
                                    ),
                                    field_path: Some(field_path),
                                    context: HashMap::new(),
                                    confidence: 0.7,
                                });
                            }
                        }
                    }
                }
            }
        }

        findings
    }

    /// Check if a string contains PII keywords
    fn contains_pii_keywords(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        for pattern in &self.pii_patterns {
            if pattern.is_match(&text_lower) {
                return true;
            }
        }
        false
    }

    /// Check if a field name indicates PII
    fn is_pii_field_name(&self, field_name: &str) -> bool {
        let field_lower = field_name.to_lowercase();
        self.pii_field_names.iter().any(|pii_name| field_lower.contains(pii_name))
    }
}

impl Default for PiiDetector {
    fn default() -> Self {
        Self::new(vec![
            "email".to_string(),
            "ssn".to_string(),
            "credit.*card".to_string(),
            "password".to_string(),
            "token".to_string(),
            "secret".to_string(),
        ])
    }
}
