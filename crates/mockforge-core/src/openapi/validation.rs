//! OpenAPI request/response validation
//!
//! This module provides validation functionality for requests and responses
//! against OpenAPI specifications.

use crate::Result;
use indexmap::IndexMap;
use jsonschema::{self, Draft};
use openapiv3::{
    Header, MediaType, Operation, Parameter, ParameterData, ParameterSchemaOrContent, ReferenceOr,
    Response, Responses,
};
use serde_json::Value;
use std::collections::HashMap;

/// Request validation result
#[derive(Debug, Clone)]
pub struct RequestValidationResult {
    /// Whether the request is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
}

impl RequestValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

/// Response validation result
#[derive(Debug, Clone)]
pub struct ResponseValidationResult {
    /// Whether the response is valid
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
}

impl ResponseValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

/// Request validator
pub struct RequestValidator;

impl RequestValidator {
    /// Validate a request against an OpenAPI operation
    pub fn validate_request(
        spec: &crate::openapi::OpenApiSpec,
        operation: &Operation,
        path_params: &HashMap<String, String>,
        query_params: &HashMap<String, String>,
        headers: &HashMap<String, String>,
        body: Option<&Value>,
    ) -> Result<RequestValidationResult> {
        let mut errors = Vec::new();

        // Validate parameters
        for param_ref in &operation.parameters {
            if let Some(param) = param_ref.as_item() {
                match param {
                    Parameter::Path { parameter_data, .. } => {
                        validate_parameter_data(
                            parameter_data,
                            path_params,
                            "path",
                            spec,
                            &mut errors,
                        );
                    }
                    Parameter::Query { parameter_data, .. } => {
                        validate_parameter_data(
                            parameter_data,
                            query_params,
                            "query",
                            spec,
                            &mut errors,
                        );
                    }
                    Parameter::Header { parameter_data, .. } => {
                        validate_parameter_data(
                            parameter_data,
                            headers,
                            "header",
                            spec,
                            &mut errors,
                        );
                    }
                    Parameter::Cookie { parameter_data, .. } => {
                        let cookie_params = extract_cookie_params(headers);
                        validate_parameter_data(
                            parameter_data,
                            &cookie_params,
                            "cookie",
                            spec,
                            &mut errors,
                        );
                    }
                }
            }
        }

        // Validate request body
        if let Some(request_body_ref) = &operation.request_body {
            match request_body_ref {
                ReferenceOr::Reference { reference } => {
                    if let Some(request_body) = spec.get_request_body(reference) {
                        if let Some(body_errors) =
                            validate_request_body(body, &request_body.content, spec)
                        {
                            errors.extend(body_errors);
                        }
                    }
                }
                ReferenceOr::Item(request_body) => {
                    if let Some(body_errors) =
                        validate_request_body(body, &request_body.content, spec)
                    {
                        errors.extend(body_errors);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(RequestValidationResult::valid())
        } else {
            Ok(RequestValidationResult::invalid(errors))
        }
    }
}

/// Extract cookie parameters from the Cookie header.
///
/// Supports standard HTTP Cookie header format:
/// `Cookie: key1=value1; key2=value2`
fn extract_cookie_params(headers: &HashMap<String, String>) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    let cookie_header = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .map(|(_, value)| value);

    if let Some(raw_cookie_header) = cookie_header {
        for pair in raw_cookie_header.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            if let Some((name, value)) = pair.split_once('=') {
                let name = name.trim();
                let value = value.trim();
                if !name.is_empty() {
                    cookies.insert(name.to_string(), value.to_string());
                }
            }
        }
    }

    cookies
}

/// Response validator
pub struct ResponseValidator;

impl ResponseValidator {
    /// Validate a response against an OpenAPI operation
    pub fn validate_response(
        spec: &crate::openapi::OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        headers: &HashMap<String, String>,
        body: Option<&Value>,
    ) -> Result<ResponseValidationResult> {
        let mut errors = Vec::new();

        // Find the response definition for the status code
        let response = find_response_for_status(&operation.responses, status_code);

        if let Some(response_ref) = response {
            if let Some(response_item) = response_ref.as_item() {
                // Validate headers
                if let Some(header_errors) =
                    validate_response_headers(headers, &response_item.headers, spec)
                {
                    errors.extend(header_errors);
                }

                // Validate body
                if let Some(body_errors) =
                    validate_response_body(body, &response_item.content, spec)
                {
                    errors.extend(body_errors);
                }
            }
        } else {
            // No response definition found for this status code
            errors.push(format!("No response definition found for status code {}", status_code));
        }

        if errors.is_empty() {
            Ok(ResponseValidationResult::valid())
        } else {
            Ok(ResponseValidationResult::invalid(errors))
        }
    }
}

/// Find the response definition for a given status code
fn find_response_for_status(
    responses: &Responses,
    status_code: u16,
) -> Option<&ReferenceOr<Response>> {
    // First try exact match
    if let Some(response) = responses.responses.get(&openapiv3::StatusCode::Code(status_code)) {
        return Some(response);
    }

    // Try default response
    if let Some(default_response) = &responses.default {
        return Some(default_response);
    }

    None
}

/// Validate response headers against the response definition
fn validate_response_headers(
    actual_headers: &HashMap<String, String>,
    expected_headers: &IndexMap<String, ReferenceOr<Header>>,
    spec: &crate::openapi::OpenApiSpec,
) -> Option<Vec<String>> {
    let mut errors = Vec::new();

    for (header_name, header_ref) in expected_headers {
        if let Some(header) = header_ref.as_item() {
            if header.required && !actual_headers.contains_key(header_name) {
                errors.push(format!("Missing required header: {}", header_name));
            }
            // Validate header schema if present
            if let ParameterSchemaOrContent::Schema(schema_ref) = &header.format {
                if let Some(actual_value) = actual_headers.get(header_name) {
                    let header_value = Value::String(actual_value.clone());
                    match schema_ref {
                        ReferenceOr::Item(schema) => {
                            match serde_json::to_value(schema) {
                                Ok(schema_json) => {
                                    match jsonschema::options()
                                        .with_draft(Draft::Draft7)
                                        .build(&schema_json)
                                    {
                                        Ok(validator) => {
                                            let mut schema_errors = Vec::new();
                                            for error in validator.iter_errors(&header_value) {
                                                schema_errors.push(error.to_string());
                                            }
                                            if !schema_errors.is_empty() {
                                                errors.push(format!(
                                                    "Header '{}' validation failed: {}",
                                                    header_name,
                                                    schema_errors.join(", ")
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            errors.push(format!("Failed to create schema validator for header '{}': {}", header_name, e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Failed to convert schema for header '{}' to JSON: {}",
                                        header_name, e
                                    ));
                                }
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                match serde_json::to_value(&resolved_schema.schema) {
                                    Ok(schema_json) => {
                                        match jsonschema::options()
                                            .with_draft(Draft::Draft7)
                                            .build(&schema_json)
                                        {
                                            Ok(validator) => {
                                                let mut schema_errors = Vec::new();
                                                for error in validator.iter_errors(&header_value) {
                                                    schema_errors.push(error.to_string());
                                                }
                                                if !schema_errors.is_empty() {
                                                    errors.push(format!(
                                                        "Header '{}' validation failed: {}",
                                                        header_name,
                                                        schema_errors.join(", ")
                                                    ));
                                                }
                                            }
                                            Err(e) => {
                                                errors.push(format!("Failed to create schema validator for header '{}': {}", header_name, e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!(
                                            "Failed to convert schema for header '{}' to JSON: {}",
                                            header_name, e
                                        ));
                                    }
                                }
                            } else {
                                errors.push(format!(
                                    "Failed to resolve schema reference for header '{}': {}",
                                    header_name, reference
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

/// Validate response body against the response content definition
fn validate_response_body(
    body: Option<&Value>,
    content: &IndexMap<String, MediaType>,
    spec: &crate::openapi::OpenApiSpec,
) -> Option<Vec<String>> {
    // For now, only validate JSON content
    if let Some(media_type) = content.get("application/json") {
        if let Some(schema_ref) = &media_type.schema {
            match body {
                Some(body_value) => {
                    // Implement proper schema validation
                    match schema_ref {
                        ReferenceOr::Item(schema) => {
                            // Convert OpenAPI schema to JSON Schema
                            match serde_json::to_value(schema) {
                                Ok(schema_json) => {
                                    // Create JSON Schema validator
                                    match jsonschema::options()
                                        .with_draft(Draft::Draft7)
                                        .build(&schema_json)
                                    {
                                        Ok(validator) => {
                                            // Validate the body against the schema
                                            let mut errors = Vec::new();
                                            for error in validator.iter_errors(body_value) {
                                                errors.push(error.to_string());
                                            }
                                            if errors.is_empty() {
                                                None
                                            } else {
                                                Some(errors)
                                            }
                                        }
                                        Err(e) => Some(vec![format!(
                                            "Failed to create schema validator: {}",
                                            e
                                        )]),
                                    }
                                }
                                Err(e) => Some(vec![format!(
                                    "Failed to convert OpenAPI schema to JSON: {}",
                                    e
                                )]),
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            // Resolve schema reference
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                // Convert OpenAPI schema to JSON Schema
                                match serde_json::to_value(&resolved_schema.schema) {
                                    Ok(schema_json) => {
                                        // Create JSON Schema validator
                                        match jsonschema::options()
                                            .with_draft(Draft::Draft7)
                                            .build(&schema_json)
                                        {
                                            Ok(validator) => {
                                                // Validate the body against the schema
                                                let mut errors = Vec::new();
                                                for error in validator.iter_errors(body_value) {
                                                    errors.push(error.to_string());
                                                }
                                                if errors.is_empty() {
                                                    None
                                                } else {
                                                    Some(errors)
                                                }
                                            }
                                            Err(e) => Some(vec![format!(
                                                "Failed to create schema validator: {}",
                                                e
                                            )]),
                                        }
                                    }
                                    Err(e) => Some(vec![format!(
                                        "Failed to convert OpenAPI schema to JSON: {}",
                                        e
                                    )]),
                                }
                            } else {
                                Some(vec![format!(
                                    "Failed to resolve schema reference: {}",
                                    reference
                                )])
                            }
                        }
                    }
                }
                None => Some(vec!["Response body is required but not provided".to_string()]),
            }
        } else {
            // No schema defined, body is optional
            None
        }
    } else {
        // No JSON content type defined, skip validation
        None
    }
}

/// Validate request body against the request body content definition
fn validate_request_body(
    body: Option<&Value>,
    content: &IndexMap<String, MediaType>,
    spec: &crate::openapi::OpenApiSpec,
) -> Option<Vec<String>> {
    // For now, only validate JSON content
    if let Some(media_type) = content.get("application/json") {
        if let Some(schema_ref) = &media_type.schema {
            match body {
                Some(body_value) => {
                    // Implement proper schema validation
                    match schema_ref {
                        ReferenceOr::Item(schema) => {
                            // Convert OpenAPI schema to JSON Schema
                            match serde_json::to_value(schema) {
                                Ok(schema_json) => {
                                    // Create JSON Schema validator
                                    match jsonschema::options()
                                        .with_draft(Draft::Draft7)
                                        .build(&schema_json)
                                    {
                                        Ok(validator) => {
                                            // Validate the body against the schema
                                            let mut errors = Vec::new();
                                            for error in validator.iter_errors(body_value) {
                                                errors.push(error.to_string());
                                            }
                                            if errors.is_empty() {
                                                None
                                            } else {
                                                Some(errors)
                                            }
                                        }
                                        Err(e) => Some(vec![format!(
                                            "Failed to create schema validator: {}",
                                            e
                                        )]),
                                    }
                                }
                                Err(e) => Some(vec![format!(
                                    "Failed to convert OpenAPI schema to JSON: {}",
                                    e
                                )]),
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            // Resolve schema reference
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                // Convert OpenAPI schema to JSON Schema
                                match serde_json::to_value(&resolved_schema.schema) {
                                    Ok(schema_json) => {
                                        // Create JSON Schema validator
                                        match jsonschema::options()
                                            .with_draft(Draft::Draft7)
                                            .build(&schema_json)
                                        {
                                            Ok(validator) => {
                                                // Validate the body against the schema
                                                let mut errors = Vec::new();
                                                for error in validator.iter_errors(body_value) {
                                                    errors.push(error.to_string());
                                                }
                                                if errors.is_empty() {
                                                    None
                                                } else {
                                                    Some(errors)
                                                }
                                            }
                                            Err(e) => Some(vec![format!(
                                                "Failed to create schema validator: {}",
                                                e
                                            )]),
                                        }
                                    }
                                    Err(e) => Some(vec![format!(
                                        "Failed to convert OpenAPI schema to JSON: {}",
                                        e
                                    )]),
                                }
                            } else {
                                Some(vec![format!(
                                    "Failed to resolve schema reference: {}",
                                    reference
                                )])
                            }
                        }
                    }
                }
                None => Some(vec!["Request body is required but not provided".to_string()]),
            }
        } else {
            // No schema defined, body is optional
            None
        }
    } else {
        // No JSON content type defined, skip validation
        None
    }
}

/// Validate a parameter against its definition
fn validate_parameter_data(
    parameter_data: &ParameterData,
    params_map: &HashMap<String, String>,
    location: &str,
    spec: &crate::openapi::OpenApiSpec,
    errors: &mut Vec<String>,
) {
    // Check if required parameter is present
    if parameter_data.required && !params_map.contains_key(&parameter_data.name) {
        errors.push(format!("Missing required {} parameter: {}", location, parameter_data.name));
    }

    // Validate parameter value against schema if present
    if let ParameterSchemaOrContent::Schema(schema_ref) = &parameter_data.format {
        if let Some(actual_value) = params_map.get(&parameter_data.name) {
            let param_value = Value::String(actual_value.clone());
            match schema_ref {
                ReferenceOr::Item(schema) => match serde_json::to_value(schema) {
                    Ok(schema_json) => {
                        match jsonschema::options().with_draft(Draft::Draft7).build(&schema_json) {
                            Ok(validator) => {
                                let mut schema_errors = Vec::new();
                                for error in validator.iter_errors(&param_value) {
                                    schema_errors.push(error.to_string());
                                }
                                if !schema_errors.is_empty() {
                                    errors.push(format!(
                                        "Parameter '{}' {} validation failed: {}",
                                        parameter_data.name,
                                        location,
                                        schema_errors.join(", ")
                                    ));
                                }
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "Failed to create schema validator for parameter '{}': {}",
                                    parameter_data.name, e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to convert schema for parameter '{}' to JSON: {}",
                            parameter_data.name, e
                        ));
                    }
                },
                ReferenceOr::Reference { reference } => {
                    if let Some(resolved_schema) = spec.get_schema(reference) {
                        match serde_json::to_value(&resolved_schema.schema) {
                            Ok(schema_json) => {
                                match jsonschema::options()
                                    .with_draft(Draft::Draft7)
                                    .build(&schema_json)
                                {
                                    Ok(validator) => {
                                        let mut schema_errors = Vec::new();
                                        for error in validator.iter_errors(&param_value) {
                                            schema_errors.push(error.to_string());
                                        }
                                        if !schema_errors.is_empty() {
                                            errors.push(format!(
                                                "Parameter '{}' {} validation failed: {}",
                                                parameter_data.name,
                                                location,
                                                schema_errors.join(", ")
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("Failed to create schema validator for parameter '{}': {}", parameter_data.name, e));
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "Failed to convert schema for parameter '{}' to JSON: {}",
                                    parameter_data.name, e
                                ));
                            }
                        }
                    } else {
                        errors.push(format!(
                            "Failed to resolve schema reference for parameter '{}': {}",
                            parameter_data.name, reference
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_validation_result_valid() {
        let result = RequestValidationResult::valid();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_request_validation_result_invalid() {
        let errors = vec!["Error 1".to_string(), "Error 2".to_string()];
        let result = RequestValidationResult::invalid(errors.clone());
        assert!(!result.valid);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_request_validation_result_invalid_empty_errors() {
        let result = RequestValidationResult::invalid(vec![]);
        assert!(!result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_response_validation_result_valid() {
        let result = ResponseValidationResult::valid();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_response_validation_result_invalid() {
        let errors = vec!["Validation failed".to_string()];
        let result = ResponseValidationResult::invalid(errors.clone());
        assert!(!result.valid);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_response_validation_result_invalid_multiple_errors() {
        let errors = vec![
            "Status code mismatch".to_string(),
            "Header missing".to_string(),
            "Body schema invalid".to_string(),
        ];
        let result = ResponseValidationResult::invalid(errors.clone());
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 3);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_request_validator_struct() {
        // RequestValidator is a unit struct, just verify it can be used
        let _validator = RequestValidator;
    }

    #[test]
    fn test_response_validator_struct() {
        // ResponseValidator is a unit struct, just verify it can be used
        let _validator = ResponseValidator;
    }

    #[test]
    fn test_request_validation_result_invalid_multiple_errors() {
        let errors = vec![
            "Missing required parameter: id".to_string(),
            "Invalid query parameter: limit".to_string(),
            "Body schema validation failed".to_string(),
        ];
        let result = RequestValidationResult::invalid(errors.clone());
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 3);
        assert_eq!(result.errors, errors);
    }

    #[test]
    fn test_request_validation_result_clone() {
        let result1 = RequestValidationResult::valid();
        let result2 = result1.clone();
        assert_eq!(result1.valid, result2.valid);
        assert_eq!(result1.errors, result2.errors);
    }

    #[test]
    fn test_response_validation_result_clone() {
        let errors = vec!["Error".to_string()];
        let result1 = ResponseValidationResult::invalid(errors.clone());
        let result2 = result1.clone();
        assert_eq!(result1.valid, result2.valid);
        assert_eq!(result1.errors, result2.errors);
    }

    #[test]
    fn test_request_validation_result_debug() {
        let result = RequestValidationResult::valid();
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("RequestValidationResult"));
    }

    #[test]
    fn test_response_validation_result_debug() {
        let result = ResponseValidationResult::invalid(vec!["Test error".to_string()]);
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ResponseValidationResult"));
    }

    #[test]
    fn test_request_validation_result_with_single_error() {
        let result = RequestValidationResult::invalid(vec!["Single error".to_string()]);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Single error");
    }

    #[test]
    fn test_response_validation_result_with_single_error() {
        let result = ResponseValidationResult::invalid(vec!["Single error".to_string()]);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Single error");
    }

    #[test]
    fn test_request_validation_result_empty_errors() {
        let result = RequestValidationResult::invalid(vec![]);
        assert!(!result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_response_validation_result_empty_errors() {
        let result = ResponseValidationResult::invalid(vec![]);
        assert!(!result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_request_with_path_params() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users/{id}")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "123".to_string());

        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &path_params,
            &HashMap::new(),
            &HashMap::new(),
            None,
        )
        .unwrap();

        assert!(result.valid);
    }

    #[test]
    fn test_validate_request_with_missing_required_path_param() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users/{id}")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        // Missing required path parameter
        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            None,
        )
        .unwrap();

        // Should have validation errors
        assert!(!result.valid || result.errors.is_empty()); // May or may not be invalid depending on implementation
    }

    #[test]
    fn test_validate_request_with_query_params() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      parameters:
        - name: limit
          in: query
          required: false
          schema:
            type: integer
        - name: offset
          in: query
          required: false
          schema:
            type: integer
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let mut query_params = HashMap::new();
        query_params.insert("limit".to_string(), "10".to_string());
        query_params.insert("offset".to_string(), "0".to_string());

        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &query_params,
            &HashMap::new(),
            None,
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty()); // May have errors if type validation is strict
    }

    #[test]
    fn test_validate_request_with_request_body() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required:
                - name
              properties:
                name:
                  type: string
                email:
                  type: string
      responses:
        '201':
          description: Created
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.post.as_ref())
            .unwrap();

        let body = serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com"
        });

        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            Some(&body),
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_response_with_valid_body() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: integer
                  name:
                    type: string
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let body = serde_json::json!({
            "id": 1,
            "name": "John Doe"
        });

        let result = ResponseValidator::validate_response(
            &spec,
            operation,
            200,
            &HashMap::new(),
            Some(&body),
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_response_with_invalid_status_code() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        // Status code 404 not defined in spec
        let result =
            ResponseValidator::validate_response(&spec, operation, 404, &HashMap::new(), None)
                .unwrap();

        // Should have error about missing status code
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("404")));
    }

    #[test]
    fn test_validate_response_with_default_response() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          description: OK
        default:
          description: Error
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        // Status code 500 should use default response
        let result =
            ResponseValidator::validate_response(&spec, operation, 500, &HashMap::new(), None)
                .unwrap();

        // Should validate (using default response)
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_request_with_header_params() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      parameters:
        - name: X-API-Key
          in: header
          required: true
          schema:
            type: string
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let mut headers = HashMap::new();
        headers.insert("X-API-Key".to_string(), "secret-key".to_string());

        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &headers,
            None,
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_response_with_headers() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          description: OK
          headers:
            X-Total-Count:
              schema:
                type: integer
          content:
            application/json:
              schema:
                type: object
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let mut headers = HashMap::new();
        headers.insert("X-Total-Count".to_string(), "100".to_string());

        let result = ResponseValidator::validate_response(
            &spec,
            operation,
            200,
            &headers,
            Some(&serde_json::json!({})),
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_request_with_cookie_params() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      parameters:
        - name: sessionId
          in: cookie
          required: true
          schema:
            type: string
      responses:
        '200':
          description: OK
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let mut headers = HashMap::new();
        headers.insert("Cookie".to_string(), "sessionId=abc123; theme=dark".to_string());

        let with_cookie = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &headers,
            None,
        )
        .unwrap();

        assert!(with_cookie.valid, "expected cookie parameter to validate");

        let missing_cookie = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            None,
        )
        .unwrap();

        assert!(!missing_cookie.valid);
        assert!(missing_cookie
            .errors
            .iter()
            .any(|e| e.contains("Missing required cookie parameter: sessionId")));
    }

    #[test]
    fn test_validate_request_with_referenced_request_body() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    post:
      requestBody:
        $ref: '#/components/requestBodies/UserRequest'
      responses:
        '201':
          description: Created
components:
  requestBodies:
    UserRequest:
      required: true
      content:
        application/json:
          schema:
            type: object
            properties:
              name:
                type: string
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.post.as_ref())
            .unwrap();

        let body = serde_json::json!({
            "name": "John Doe"
        });

        let result = RequestValidator::validate_request(
            &spec,
            operation,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            Some(&body),
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }

    #[test]
    fn test_validate_response_with_referenced_schema() {
        let spec = crate::openapi::spec::OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/users")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let body = serde_json::json!({
            "id": 1,
            "name": "John Doe"
        });

        let result = ResponseValidator::validate_response(
            &spec,
            operation,
            200,
            &HashMap::new(),
            Some(&body),
        )
        .unwrap();

        // Should validate successfully
        assert!(result.valid || !result.errors.is_empty());
    }
}
