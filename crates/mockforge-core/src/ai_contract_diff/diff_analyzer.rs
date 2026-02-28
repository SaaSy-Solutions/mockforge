//! Core diff analysis engine for contract comparison
//!
//! This module performs structural comparison between captured requests and contract
//! specifications, detecting mismatches and preparing data for AI-powered recommendations.

use super::types::{
    CapturedRequest, ContractDiffResult, DiffMetadata, Mismatch, MismatchSeverity, MismatchType,
};
use crate::openapi::OpenApiSpec;
use crate::schema_diff::validation_diff;
use crate::Result;
use serde_json::Value;
use std::collections::HashMap;

/// Check if a path matches a pattern with path parameters
///
/// Examples:
/// - `/users/{id}` matches `/users/123`
/// - `/users/{userId}/posts/{postId}` matches `/users/123/posts/456`
fn path_matches_with_params(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        return false;
    }

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        // Check for path parameters {param} or {param:type}
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            // Matches any value (could add type validation here)
            continue;
        }

        if pattern_part != path_part {
            return false;
        }
    }

    true
}

/// Contract diff analyzer
pub struct DiffAnalyzer {
    /// Configuration for analysis
    config: super::types::ContractDiffConfig,
}

impl DiffAnalyzer {
    /// Create a new diff analyzer
    pub fn new(config: super::types::ContractDiffConfig) -> Self {
        Self { config }
    }

    /// Analyze a captured request against an OpenAPI specification
    pub async fn analyze_request(
        &self,
        request: &CapturedRequest,
        spec: &OpenApiSpec,
    ) -> Result<ContractDiffResult> {
        let mut mismatches = Vec::new();

        // Find matching endpoint in spec
        let endpoint_match = self.find_endpoint_in_spec(&request.path, &request.method, spec);

        // Analyze endpoint existence
        if endpoint_match.is_none() {
            mismatches.push(Mismatch {
                mismatch_type: MismatchType::EndpointNotFound,
                path: request.path.clone(),
                method: Some(request.method.clone()),
                expected: Some("Endpoint defined in OpenAPI spec".to_string()),
                actual: Some("Endpoint not found in spec".to_string()),
                description: format!(
                    "Endpoint {} {} not found in contract specification",
                    request.method, request.path
                ),
                severity: MismatchSeverity::Critical,
                confidence: 1.0, // Structural mismatch is always certain
                context: HashMap::new(),
            });
        }

        // Analyze request body against schema
        if let Some(body) = &request.body {
            if let Some(endpoint) = &endpoint_match {
                let body_mismatches =
                    self.analyze_request_body(body, endpoint, &request.path, spec)?;
                mismatches.extend(body_mismatches);
            }
        }

        // Analyze headers
        let header_mismatches = self.analyze_headers(&request.headers, endpoint_match.as_ref());
        mismatches.extend(header_mismatches);

        // Analyze query parameters
        let query_mismatches = self.analyze_query_params(
            &request.query_params,
            endpoint_match.as_ref(),
            &request.path,
        );
        mismatches.extend(query_mismatches);

        // Calculate overall confidence
        let overall_confidence =
            super::confidence_scorer::ConfidenceScorer::calculate_overall_confidence(&mismatches);

        // Create metadata
        let metadata = DiffMetadata {
            analyzed_at: chrono::Utc::now(),
            request_source: request.source.clone(),
            contract_version: spec.spec.info.version.clone().into(),
            contract_format: "openapi-3.0".to_string(), // Could detect version
            endpoint_path: request.path.clone(),
            http_method: request.method.clone(),
            request_count: 1,
            llm_provider: Some(self.config.llm_provider.clone()),
            llm_model: Some(self.config.llm_model.clone()),
        };

        Ok(ContractDiffResult {
            matches: mismatches.is_empty(),
            confidence: overall_confidence,
            mismatches,
            recommendations: Vec::new(), // Will be populated by recommendation engine
            corrections: Vec::new(),     // Will be populated by correction proposer
            metadata,
        })
    }

    /// Find matching endpoint in OpenAPI spec
    fn find_endpoint_in_spec(
        &self,
        path: &str,
        method: &str,
        spec: &OpenApiSpec,
    ) -> Option<openapiv3::Operation> {
        // Normalize path (remove query params, trailing slashes)
        let normalized_path = path.split('?').next().unwrap_or(path).trim_end_matches('/');

        // Try exact match first
        for (spec_path, path_item_ref) in &spec.spec.paths.paths {
            let spec_path_normalized = spec_path.trim_end_matches('/');

            if spec_path_normalized == normalized_path {
                if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                    return match method.to_uppercase().as_str() {
                        "GET" => path_item.get.clone(),
                        "POST" => path_item.post.clone(),
                        "PUT" => path_item.put.clone(),
                        "DELETE" => path_item.delete.clone(),
                        "PATCH" => path_item.patch.clone(),
                        _ => None,
                    };
                }
            }
        }

        // Path parameter matching (e.g., /users/{id} matches /users/123)
        for (spec_path, path_item_ref) in &spec.spec.paths.paths {
            let spec_path_normalized = spec_path.trim_end_matches('/');

            if path_matches_with_params(spec_path_normalized, normalized_path) {
                if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                    return match method.to_uppercase().as_str() {
                        "GET" => path_item.get.clone(),
                        "POST" => path_item.post.clone(),
                        "PUT" => path_item.put.clone(),
                        "DELETE" => path_item.delete.clone(),
                        "PATCH" => path_item.patch.clone(),
                        _ => None,
                    };
                }
            }
        }

        None
    }

    /// Analyze request body against schema
    fn analyze_request_body(
        &self,
        body: &Value,
        operation: &openapiv3::Operation,
        path: &str,
        spec: &OpenApiSpec,
    ) -> Result<Vec<Mismatch>> {
        let mut mismatches = Vec::new();

        // Get request body schema
        if let Some(openapiv3::ReferenceOr::Item(request_body)) = &operation.request_body {
            // Get JSON schema from content
            if let Some(content) = request_body.content.get("application/json") {
                if let Some(schema_ref) = &content.schema {
                    // Convert OpenAPI schema to JSON Schema for validation
                    let schema_value = self.openapi_schema_to_json(schema_ref, spec)?;

                    // Use existing validation_diff function
                    let validation_errors = validation_diff(&schema_value, body);

                    // Convert validation errors to mismatches
                    for error in &validation_errors {
                        let mismatch_type = match error.error_type.as_str() {
                            "missing_required" => MismatchType::MissingRequiredField,
                            "type_mismatch" => MismatchType::TypeMismatch,
                            "additional_property" => MismatchType::UnexpectedField,
                            "length_mismatch" => MismatchType::ConstraintViolation,
                            _ => MismatchType::SchemaMismatch,
                        };

                        let severity = match mismatch_type {
                            MismatchType::MissingRequiredField => MismatchSeverity::Critical,
                            MismatchType::TypeMismatch => MismatchSeverity::High,
                            MismatchType::UnexpectedField => MismatchSeverity::Low,
                            _ => MismatchSeverity::Medium,
                        };

                        mismatches.push(Mismatch {
                            mismatch_type,
                            path: format!("{}{}", path, error.path),
                            method: None,
                            expected: Some(error.expected.clone()),
                            actual: Some(error.found.clone()),
                            description: error.message.clone().unwrap_or_else(|| {
                                format!("Validation error: {}", error.error_type)
                            }),
                            severity,
                            confidence: 0.9, // Structural validation is high confidence
                            context: error
                                .schema_info
                                .as_ref()
                                .map(|info| {
                                    let mut ctx = HashMap::new();
                                    ctx.insert(
                                        "data_type".to_string(),
                                        Value::String(info.data_type.clone()),
                                    );
                                    if let Some(required) = info.required {
                                        ctx.insert("required".to_string(), Value::Bool(required));
                                    }
                                    if let Some(format) = &info.format {
                                        ctx.insert(
                                            "format".to_string(),
                                            Value::String(format.clone()),
                                        );
                                    }
                                    ctx
                                })
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }

        Ok(mismatches)
    }

    /// Analyze headers against spec requirements
    fn analyze_headers(
        &self,
        headers: &HashMap<String, String>,
        operation: Option<&openapiv3::Operation>,
    ) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        if let Some(op) = operation {
            // Check security requirements
            if let Some(security) = &op.security {
                for sec_req in security {
                    // Check if required headers are present
                    // This is simplified - real implementation would check OAuth, API keys, etc.
                    for (name, _) in sec_req {
                        let header_name_lower = name.to_lowercase();
                        let found =
                            headers.iter().any(|(k, _)| k.to_lowercase() == header_name_lower);

                        if !found {
                            mismatches.push(Mismatch {
                                mismatch_type: MismatchType::HeaderMismatch,
                                path: "headers".to_string(),
                                method: None,
                                expected: Some(format!("Header: {}", name)),
                                actual: Some("Header missing".to_string()),
                                description: format!(
                                    "Required security header '{}' is missing",
                                    name
                                ),
                                severity: MismatchSeverity::High,
                                confidence: 1.0,
                                context: HashMap::new(),
                            });
                        }
                    }
                }
            }
        }

        mismatches
    }

    /// Analyze query parameters against spec
    fn analyze_query_params(
        &self,
        query_params: &HashMap<String, String>,
        operation: Option<&openapiv3::Operation>,
        path: &str,
    ) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        if let Some(op) = operation {
            // Check parameters
            for param in &op.parameters {
                if let openapiv3::ReferenceOr::Item(openapiv3::Parameter::Query {
                    parameter_data,
                    ..
                }) = param
                {
                    let param_name = &parameter_data.name;
                    let required = parameter_data.required;

                    let found = query_params.contains_key(param_name);

                    if required && !found {
                        mismatches.push(Mismatch {
                            mismatch_type: MismatchType::QueryParamMismatch,
                            path: format!("{}?{}", path, param_name),
                            method: None,
                            expected: Some(format!("Required query parameter: {}", param_name)),
                            actual: Some("Parameter missing".to_string()),
                            description: format!(
                                "Required query parameter '{}' is missing",
                                param_name
                            ),
                            severity: MismatchSeverity::High,
                            confidence: 1.0,
                            context: HashMap::new(),
                        });
                    }
                }
            }
        }

        mismatches
    }

    /// Convert OpenAPI schema to JSON Schema value for validation
    ///
    /// This method resolves `$ref` references using the provided OpenAPI spec.
    fn openapi_schema_to_json(
        &self,
        schema: &openapiv3::ReferenceOr<openapiv3::Schema>,
        spec: &OpenApiSpec,
    ) -> Result<Value> {
        match schema {
            openapiv3::ReferenceOr::Item(schema) => {
                self.openapi_schema_to_json_from_schema(schema, spec)
            }
            openapiv3::ReferenceOr::Reference { reference } => {
                // Resolve the reference using the spec
                if let Some(resolved_schema) = spec.resolve_schema_ref(reference) {
                    self.openapi_schema_to_json_from_schema(&resolved_schema, spec)
                } else {
                    // Reference couldn't be resolved, return empty schema with warning
                    tracing::warn!("Could not resolve schema reference: {}", reference);
                    Ok(Value::Object(serde_json::Map::new()))
                }
            }
        }
    }

    /// Convert a Schema directly (helper for Box<Schema> case)
    ///
    /// This method resolves `$ref` references for nested properties using the spec.
    #[allow(clippy::only_used_in_recursion)]
    fn openapi_schema_to_json_from_schema(
        &self,
        schema: &openapiv3::Schema,
        spec: &OpenApiSpec,
    ) -> Result<Value> {
        let mut json_schema = serde_json::Map::new();

        // Add type
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
                json_schema.insert("type".to_string(), Value::String("string".to_string()));
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
                json_schema.insert("type".to_string(), Value::String("number".to_string()));
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => {
                json_schema.insert("type".to_string(), Value::String("integer".to_string()));
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => {
                json_schema.insert("type".to_string(), Value::String("boolean".to_string()));
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Array(array_type)) => {
                json_schema.insert("type".to_string(), Value::String("array".to_string()));
                // Handle array items
                if let Some(items) = &array_type.items {
                    let items_json = match items {
                        openapiv3::ReferenceOr::Item(item_schema) => {
                            self.openapi_schema_to_json_from_schema(item_schema, spec)?
                        }
                        openapiv3::ReferenceOr::Reference { reference } => {
                            // Resolve array item reference
                            if let Some(resolved) = spec.resolve_schema_ref(reference.as_str()) {
                                self.openapi_schema_to_json_from_schema(&resolved, spec)?
                            } else {
                                tracing::warn!(
                                    "Could not resolve array item reference: {}",
                                    reference
                                );
                                Value::Object(serde_json::Map::new())
                            }
                        }
                    };
                    json_schema.insert("items".to_string(), items_json);
                }
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(_)) => {
                json_schema.insert("type".to_string(), Value::String("object".to_string()));
            }
            _ => {}
        }

        // Add properties if object
        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj_type)) = &schema.schema_kind
        {
            let mut props = serde_json::Map::new();
            for (name, prop_schema_ref) in &obj_type.properties {
                // Handle ReferenceOr<Box<Schema>> with proper reference resolution
                let prop_json = match prop_schema_ref {
                    openapiv3::ReferenceOr::Item(boxed_schema) => {
                        // Process the boxed schema directly
                        self.openapi_schema_to_json_from_schema(boxed_schema.as_ref(), spec)
                    }
                    openapiv3::ReferenceOr::Reference { reference } => {
                        // Resolve the property reference using the spec
                        if let Some(resolved_schema) = spec.resolve_schema_ref(reference) {
                            self.openapi_schema_to_json_from_schema(&resolved_schema, spec)
                        } else {
                            tracing::warn!(
                                "Could not resolve property reference for '{}': {}",
                                name,
                                reference
                            );
                            Ok(Value::Object(serde_json::Map::new()))
                        }
                    }
                };
                if let Ok(prop_json) = prop_json {
                    props.insert(name.clone(), prop_json);
                }
            }
            if !props.is_empty() {
                json_schema.insert("properties".to_string(), Value::Object(props));
            }

            // Add required fields
            if !obj_type.required.is_empty() {
                let required_array: Vec<Value> =
                    obj_type.required.iter().map(|s| Value::String(s.clone())).collect();
                json_schema.insert("required".to_string(), Value::Array(required_array));
            }
        }

        Ok(Value::Object(json_schema))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_analyzer_creation() {
        let config = crate::ai_contract_diff::ContractDiffConfig::default();
        let _analyzer = DiffAnalyzer::new(config);
        // DiffAnalyzer was successfully created with default config
    }
}
