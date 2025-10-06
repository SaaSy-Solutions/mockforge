//! OpenAPI response generation and mocking
//!
//! This module provides functionality for generating mock responses
//! based on OpenAPI specifications.

use crate::{OpenApiSpec, Result};
use chrono;
use openapiv3::{Operation, ReferenceOr, Response, Responses, Schema};
use rand::{rng, Rng};
use serde_json::Value;
use std::collections::HashMap;
use uuid;

/// Response generator for creating mock responses
pub struct ResponseGenerator;

impl ResponseGenerator {
    /// Generate a mock response for an operation and status code
    pub fn generate_response(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
    ) -> Result<Value> {
        Self::generate_response_with_expansion(spec, operation, status_code, content_type, true)
    }

    /// Generate a mock response for an operation and status code with token expansion control
    pub fn generate_response_with_expansion(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
    ) -> Result<Value> {
        // Find the response for the status code
        let response = Self::find_response_for_status(&operation.responses, status_code);

        match response {
            Some(response_ref) => {
                match response_ref {
                    ReferenceOr::Item(response) => {
                        Self::generate_from_response(spec, response, content_type, expand_tokens)
                    }
                    ReferenceOr::Reference { reference } => {
                        // Resolve the reference
                        if let Some(resolved_response) = spec.get_response(reference) {
                            Self::generate_from_response(spec, resolved_response, content_type, expand_tokens)
                        } else {
                            // Reference not found, return empty object
                            Ok(Value::Object(serde_json::Map::new()))
                        }
                    }
                }
            }
            None => {
                // No response found for this status code
                Ok(Value::Object(serde_json::Map::new()))
            }
        }
    }

    /// Find response for a given status code
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

    /// Generate response from a Response object
    fn generate_from_response(
        spec: &OpenApiSpec,
        response: &Response,
        content_type: Option<&str>,
        expand_tokens: bool,
    ) -> Result<Value> {
        // If content_type is specified, look for that media type
        if let Some(content_type) = content_type {
            if let Some(media_type) = response.content.get(content_type) {
                return Self::generate_from_media_type(spec, media_type, expand_tokens);
            }
        }

        // If no content_type specified or not found, try common content types
        let preferred_types = ["application/json", "application/xml", "text/plain"];

        for content_type in &preferred_types {
            if let Some(media_type) = response.content.get(*content_type) {
                return Self::generate_from_media_type(spec, media_type, expand_tokens);
            }
        }

        // If no suitable content type found, return the first available
        if let Some((_, media_type)) = response.content.iter().next() {
            return Self::generate_from_media_type(spec, media_type, expand_tokens);
        }

        // No content found, return empty object
        Ok(Value::Object(serde_json::Map::new()))
    }

    /// Generate response from a MediaType
    fn generate_from_media_type(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
    ) -> Result<Value> {
        // First, check if there's an explicit example
        if let Some(example) = &media_type.example {
            tracing::debug!("Using explicit example from media type: {:?}", example);
            // Expand templates in the example if enabled
            if expand_tokens {
                let expanded_example = Self::expand_templates(example);
                return Ok(expanded_example);
            } else {
                return Ok(example.clone());
            }
        }

        // Then check examples map
        if !media_type.examples.is_empty() {
            if let Some((_, example_ref)) = media_type.examples.iter().next() {
                match example_ref {
                    ReferenceOr::Item(example) => {
                        if let Some(value) = &example.value {
                            tracing::debug!("Using example from examples map: {:?}", value);
                            if expand_tokens {
                                return Ok(Self::expand_templates(value));
                            } else {
                                return Ok(value.clone());
                            }
                        }
                    }
                    ReferenceOr::Reference { reference } => {
                        // Resolve the example reference
                        if let Some(example) = spec.get_example(reference) {
                            if let Some(value) = &example.value {
                                tracing::debug!("Using resolved example reference: {:?}", value);
                                if expand_tokens {
                                    return Ok(Self::expand_templates(value));
                                } else {
                                    return Ok(value.clone());
                                }
                            }
                        } else {
                            tracing::warn!("Example reference '{}' not found", reference);
                        }
                    }
                }
            }
        }

        // Fall back to schema-based generation
        match &media_type.schema {
            Some(schema_ref) => {
                match schema_ref {
                    ReferenceOr::Item(schema) => Ok(Self::generate_example_from_schema(schema)),
                    ReferenceOr::Reference { reference } => {
                        // Resolve the schema reference
                        if let Some(schema) = spec.get_schema(reference) {
                            Ok(Self::generate_example_from_schema(&schema.schema))
                        } else {
                            // Reference not found, return empty object
                            Ok(Value::Object(serde_json::Map::new()))
                        }
                    }
                }
            }
            None => {
                // No schema, return empty object
                Ok(Value::Object(serde_json::Map::new()))
            }
        }
    }

    /// Generate example data from an OpenAPI schema
    fn generate_example_from_schema(schema: &Schema) -> Value {
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
                // Use faker for string fields based on field name hints
                Value::String("example string".to_string())
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => Value::Number(42.into()),
            openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
                Value::Number(serde_json::Number::from_f64(3.14).unwrap())
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => Value::Bool(true),
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                let mut map = serde_json::Map::new();
                for (prop_name, _) in &obj.properties {
                    let value = Self::generate_example_for_property(prop_name);
                    map.insert(prop_name.clone(), value);
                }
                Value::Object(map)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => match &arr.items {
                Some(ReferenceOr::Item(item_schema)) => {
                    let example_item = Self::generate_example_from_schema(item_schema);
                    Value::Array(vec![example_item])
                }
                _ => Value::Array(vec![Value::String("item".to_string())]),
            },
            _ => Value::Object(serde_json::Map::new()),
        }
    }

    /// Generate example value for a property based on its name
    fn generate_example_for_property(prop_name: &str) -> Value {
        let prop_lower = prop_name.to_lowercase();

        // Generate realistic data based on property name patterns
        if prop_lower.contains("id") || prop_lower.contains("uuid") {
            Value::String(uuid::Uuid::new_v4().to_string())
        } else if prop_lower.contains("email") {
            Value::String(format!("user{}@example.com", rng().random_range(1000..=9999)))
        } else if prop_lower.contains("name") || prop_lower.contains("title") {
            let names = ["John Doe", "Jane Smith", "Bob Johnson", "Alice Brown"];
            Value::String(names[rng().random_range(0..names.len())].to_string())
        } else if prop_lower.contains("phone") || prop_lower.contains("mobile") {
            Value::String(format!("+1-555-{:04}", rng().random_range(1000..=9999)))
        } else if prop_lower.contains("address") || prop_lower.contains("street") {
            let streets = ["123 Main St", "456 Oak Ave", "789 Pine Rd", "321 Elm St"];
            Value::String(streets[rng().random_range(0..streets.len())].to_string())
        } else if prop_lower.contains("city") {
            let cities = ["New York", "London", "Tokyo", "Paris", "Sydney"];
            Value::String(cities[rng().random_range(0..cities.len())].to_string())
        } else if prop_lower.contains("country") {
            let countries = ["USA", "UK", "Japan", "France", "Australia"];
            Value::String(countries[rng().random_range(0..countries.len())].to_string())
        } else if prop_lower.contains("company") || prop_lower.contains("organization") {
            let companies = ["Acme Corp", "Tech Solutions", "Global Inc", "Innovate Ltd"];
            Value::String(companies[rng().random_range(0..companies.len())].to_string())
        } else if prop_lower.contains("url") || prop_lower.contains("website") {
            Value::String("https://example.com".to_string())
        } else if prop_lower.contains("age") {
            Value::Number((18 + rng().random_range(0..60)).into())
        } else if prop_lower.contains("count") || prop_lower.contains("quantity") {
            Value::Number((1 + rng().random_range(0..100)).into())
        } else if prop_lower.contains("price")
            || prop_lower.contains("amount")
            || prop_lower.contains("cost")
        {
            Value::Number(
                serde_json::Number::from_f64(
                    (rng().random::<f64>() * 1000.0 * 100.0).round() / 100.0,
                )
                .unwrap(),
            )
        } else if prop_lower.contains("active")
            || prop_lower.contains("enabled")
            || prop_lower.contains("is_")
        {
            Value::Bool(rng().random_bool(0.5))
        } else if prop_lower.contains("date") || prop_lower.contains("time") {
            Value::String(chrono::Utc::now().to_rfc3339())
        } else if prop_lower.contains("description") || prop_lower.contains("comment") {
            Value::String("This is a sample description text.".to_string())
        } else {
            Value::String(format!("example {}", prop_name))
        }
    }

    /// Generate example responses from OpenAPI examples
    pub fn generate_from_examples(
        response: &Response,
        content_type: Option<&str>,
    ) -> Result<Option<Value>> {
        use openapiv3::ReferenceOr;

        // If content_type is specified, look for examples in that media type
        if let Some(content_type) = content_type {
            if let Some(media_type) = response.content.get(content_type) {
                // Check for single example first
                if let Some(example) = &media_type.example {
                    return Ok(Some(example.clone()));
                }

                // Check for multiple examples
                for (_, example_ref) in &media_type.examples {
                    if let ReferenceOr::Item(example) = example_ref {
                        if let Some(value) = &example.value {
                            return Ok(Some(value.clone()));
                        }
                    }
                    // Reference resolution would require spec parameter to be added to this function
                }
            }
        }

        // If no content_type specified or not found, check all media types
        for (_, media_type) in &response.content {
            // Check for single example first
            if let Some(example) = &media_type.example {
                return Ok(Some(example.clone()));
            }

            // Check for multiple examples
            for (_, example_ref) in &media_type.examples {
                if let ReferenceOr::Item(example) = example_ref {
                    if let Some(value) = &example.value {
                        return Ok(Some(value.clone()));
                    }
                }
                // Reference resolution would require spec parameter to be added to this function
            }
        }

        Ok(None)
    }

    /// Expand templates like {{now}} and {{uuid}} in JSON values
    fn expand_templates(value: &Value) -> Value {
        match value {
            Value::String(s) => {
                let expanded = s
                    .replace("{{now}}", &chrono::Utc::now().to_rfc3339())
                    .replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());
                Value::String(expanded)
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, val) in map {
                    new_map.insert(key.clone(), Self::expand_templates(val));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => {
                let new_arr: Vec<Value> = arr.iter().map(|v| Self::expand_templates(v)).collect();
                Value::Array(new_arr)
            }
            _ => value.clone(),
        }
    }
}

/// Mock response data
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Option<Value>,
}

impl MockResponse {
    /// Create a new mock response
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, name: String, value: String) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Set the response body
    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }
}

/// OpenAPI security requirement wrapper
#[derive(Debug, Clone)]
pub struct OpenApiSecurityRequirement {
    /// The security scheme name
    pub scheme: String,
    /// Required scopes (for OAuth2)
    pub scopes: Vec<String>,
}

impl OpenApiSecurityRequirement {
    /// Create a new security requirement
    pub fn new(scheme: String, scopes: Vec<String>) -> Self {
        Self { scheme, scopes }
    }
}

/// OpenAPI operation wrapper with path context
#[derive(Debug, Clone)]
pub struct OpenApiOperation {
    /// The HTTP method
    pub method: String,
    /// The path this operation belongs to
    pub path: String,
    /// The OpenAPI operation
    pub operation: openapiv3::Operation,
}

impl OpenApiOperation {
    /// Create a new OpenApiOperation
    pub fn new(method: String, path: String, operation: openapiv3::Operation) -> Self {
        Self {
            method,
            path,
            operation,
        }
    }
}
