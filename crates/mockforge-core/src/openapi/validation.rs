//! OpenAPI request/response validation
//!
//! This module provides validation functionality for requests and responses
//! against OpenAPI specifications.

use crate::Result;
use openapiv3::{Operation, Parameter, ReferenceOr, Response, Responses, Header, MediaType, ParameterSchemaOrContent, ParameterData};
use serde_json::Value;
use std::collections::HashMap;
use indexmap::IndexMap;
use jsonschema::{self, Draft};

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
        path_params: &std::collections::HashMap<String, String>,
        query_params: &std::collections::HashMap<String, String>,
        headers: &std::collections::HashMap<String, String>,
        body: Option<&Value>,
    ) -> Result<RequestValidationResult> {
        let mut errors = Vec::new();

        // Validate parameters
        for param_ref in &operation.parameters {
            if let Some(param) = param_ref.as_item() {
                match param {
                    Parameter::Path { parameter_data, .. } => {
                        validate_parameter_data(parameter_data, path_params, "path", spec, &mut errors);
                    }
                    Parameter::Query { parameter_data, .. } => {
                        validate_parameter_data(parameter_data, query_params, "query", spec, &mut errors);
                    }
                    Parameter::Header { parameter_data, .. } => {
                        validate_parameter_data(parameter_data, headers, "header", spec, &mut errors);
                    }
                    Parameter::Cookie { .. } => {
                        // Cookie parameter validation not implemented
                    }
                }
            }
        }

        // Validate request body
        if let Some(request_body_ref) = &operation.request_body {
            match request_body_ref {
                openapiv3::ReferenceOr::Reference { reference } => {
                    if let Some(request_body) = spec.get_request_body(reference) {
                        if let Some(body_errors) = validate_request_body(body, &request_body.content, spec) {
                            errors.extend(body_errors);
                        }
                    }
                }
                openapiv3::ReferenceOr::Item(request_body) => {
                    if let Some(body_errors) = validate_request_body(body, &request_body.content, spec) {
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

/// Response validator
pub struct ResponseValidator;

impl ResponseValidator {
    /// Validate a response against an OpenAPI operation
    pub fn validate_response(
        spec: &crate::openapi::OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        headers: &std::collections::HashMap<String, String>,
        body: Option<&Value>,
    ) -> Result<ResponseValidationResult> {
        let mut errors = Vec::new();

        // Find the response definition for the status code
        let response = find_response_for_status(&operation.responses, status_code);

        if let Some(response_ref) = response {
            if let Some(response_item) = response_ref.as_item() {
                // Validate headers
                if let Some(header_errors) = validate_response_headers(headers, &response_item.headers, spec) {
                    errors.extend(header_errors);
                }

                // Validate body
                if let Some(body_errors) = validate_response_body(body, &response_item.content, spec) {
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
fn find_response_for_status(responses: &Responses, status_code: u16) -> Option<&ReferenceOr<Response>> {
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
                                                errors.push(format!("Header '{}' validation failed: {}", header_name, schema_errors.join(", ")));
                                            }
                                        }
                                        Err(e) => {
                                            errors.push(format!("Failed to create schema validator for header '{}': {}", header_name, e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to convert schema for header '{}' to JSON: {}", header_name, e));
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
                                                    errors.push(format!("Header '{}' validation failed: {}", header_name, schema_errors.join(", ")));
                                                }
                                            }
                                            Err(e) => {
                                                errors.push(format!("Failed to create schema validator for header '{}': {}", header_name, e));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("Failed to convert schema for header '{}' to JSON: {}", header_name, e));
                                    }
                                }
                            } else {
                                errors.push(format!("Failed to resolve schema reference for header '{}': {}", header_name, reference));
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
                                        Err(e) => {
                                            Some(vec![format!("Failed to create schema validator: {}", e)])
                                        }
                                    }
                                }
                                Err(e) => {
                                    Some(vec![format!("Failed to convert OpenAPI schema to JSON: {}", e)])
                                }
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
                                            Err(e) => {
                                                Some(vec![format!("Failed to create schema validator: {}", e)])
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        Some(vec![format!("Failed to convert OpenAPI schema to JSON: {}", e)])
                                    }
                                }
                            } else {
                                Some(vec![format!("Failed to resolve schema reference: {}", reference)])
                            }
                        }
                    }
                }
                None => {
                    Some(vec!["Response body is required but not provided".to_string()])
                }
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
                                        Err(e) => {
                                            Some(vec![format!("Failed to create schema validator: {}", e)])
                                        }
                                    }
                                }
                                Err(e) => {
                                    Some(vec![format!("Failed to convert OpenAPI schema to JSON: {}", e)])
                                }
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
                                            Err(e) => {
                                                Some(vec![format!("Failed to create schema validator: {}", e)])
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        Some(vec![format!("Failed to convert OpenAPI schema to JSON: {}", e)])
                                    }
                                }
                            } else {
                                Some(vec![format!("Failed to resolve schema reference: {}", reference)])
                            }
                        }
                    }
                }
                None => {
                    Some(vec!["Request body is required but not provided".to_string()])
                }
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
                ReferenceOr::Item(schema) => {
                    match serde_json::to_value(schema) {
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
                                        errors.push(format!("Parameter '{}' {} validation failed: {}", parameter_data.name, location, schema_errors.join(", ")));
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to create schema validator for parameter '{}': {}", parameter_data.name, e));
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Failed to convert schema for parameter '{}' to JSON: {}", parameter_data.name, e));
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
                                        for error in validator.iter_errors(&param_value) {
                                            schema_errors.push(error.to_string());
                                        }
                                        if !schema_errors.is_empty() {
                                            errors.push(format!("Parameter '{}' {} validation failed: {}", parameter_data.name, location, schema_errors.join(", ")));
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("Failed to create schema validator for parameter '{}': {}", parameter_data.name, e));
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Failed to convert schema for parameter '{}' to JSON: {}", parameter_data.name, e));
                            }
                        }
                    } else {
                        errors.push(format!("Failed to resolve schema reference for parameter '{}': {}", parameter_data.name, reference));
                    }
                }
            }
        }
    }
}