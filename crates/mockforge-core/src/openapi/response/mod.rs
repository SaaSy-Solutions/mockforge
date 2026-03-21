//! OpenAPI response generation and mocking
//!
//! This module provides functionality for generating mock responses
//! based on OpenAPI specifications.

mod ai_assisted;
mod schema_based;

use crate::intelligent_behavior::config::Persona;
use crate::{
    ai_response::{AiResponseConfig, RequestContext},
    OpenApiSpec, Result,
};
use async_trait::async_trait;
use chrono;
use openapiv3::{Operation, ReferenceOr, Response, Responses, Schema};
use rand::{thread_rng, Rng};
use serde_json::Value;
use std::collections::HashMap;
use uuid;

/// Trait for AI response generation
///
/// This trait allows the HTTP layer to provide custom AI generation
/// implementations without creating circular dependencies between crates.
#[async_trait]
pub trait AiGenerator: Send + Sync {
    /// Generate an AI response from a prompt
    ///
    /// # Arguments
    /// * `prompt` - The expanded prompt to send to the LLM
    /// * `config` - The AI response configuration with temperature, max_tokens, etc.
    ///
    /// # Returns
    /// A JSON value containing the generated response
    async fn generate(&self, prompt: &str, config: &AiResponseConfig) -> Result<Value>;
}

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
        Self::generate_response_with_expansion_and_mode(
            spec,
            operation,
            status_code,
            content_type,
            expand_tokens,
            None,
            None,
        )
    }

    /// Generate response with token expansion and selection mode
    pub fn generate_response_with_expansion_and_mode(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
    ) -> Result<Value> {
        Self::generate_response_with_expansion_and_mode_and_persona(
            spec,
            operation,
            status_code,
            content_type,
            expand_tokens,
            selection_mode,
            selector,
            None, // No persona by default
        )
    }

    /// Generate response with token expansion, selection mode, and persona
    #[allow(clippy::too_many_arguments)]
    pub fn generate_response_with_expansion_and_mode_and_persona(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
        persona: Option<&Persona>,
    ) -> Result<Value> {
        Self::generate_response_with_scenario_and_mode_and_persona(
            spec,
            operation,
            status_code,
            content_type,
            expand_tokens,
            None, // No scenario by default
            selection_mode,
            selector,
            persona,
        )
    }

    /// Generate a mock response with scenario support
    ///
    /// This method allows selection of specific example scenarios from the OpenAPI spec.
    /// Scenarios are defined using the standard OpenAPI `examples` field (not the singular `example`).
    ///
    /// # Arguments
    /// * `spec` - The OpenAPI specification
    /// * `operation` - The operation to generate a response for
    /// * `status_code` - The HTTP status code
    /// * `content_type` - Optional content type (e.g., "application/json")
    /// * `expand_tokens` - Whether to expand template tokens like {{now}} and {{uuid}}
    /// * `scenario` - Optional scenario name to select from the examples map
    ///
    /// # Example
    /// ```yaml
    /// responses:
    ///   '200':
    ///     content:
    ///       application/json:
    ///         examples:
    ///           happy:
    ///             value: { "status": "success", "message": "All good!" }
    ///           error:
    ///             value: { "status": "error", "message": "Something went wrong" }
    /// ```
    pub fn generate_response_with_scenario(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
    ) -> Result<Value> {
        Self::generate_response_with_scenario_and_mode(
            spec,
            operation,
            status_code,
            content_type,
            expand_tokens,
            scenario,
            None,
            None,
        )
    }

    /// Generate response with scenario support and selection mode
    #[allow(clippy::too_many_arguments)]
    pub fn generate_response_with_scenario_and_mode(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
    ) -> Result<Value> {
        Self::generate_response_with_scenario_and_mode_and_persona(
            spec,
            operation,
            status_code,
            content_type,
            expand_tokens,
            scenario,
            selection_mode,
            selector,
            None, // No persona by default
        )
    }

    /// Generate response with scenario support, selection mode, and persona
    #[allow(clippy::too_many_arguments)]
    pub fn generate_response_with_scenario_and_mode_and_persona(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
        _persona: Option<&Persona>,
    ) -> Result<Value> {
        // Find the response for the status code
        let response = Self::find_response_for_status(&operation.responses, status_code);

        tracing::debug!(
            "Finding response for status code {}: {:?}",
            status_code,
            if response.is_some() {
                "found"
            } else {
                "not found"
            }
        );

        match response {
            Some(response_ref) => {
                match response_ref {
                    ReferenceOr::Item(response) => {
                        tracing::debug!(
                            "Using direct response item with {} content types",
                            response.content.len()
                        );
                        Self::generate_from_response_with_scenario_and_mode(
                            spec,
                            response,
                            content_type,
                            expand_tokens,
                            scenario,
                            selection_mode,
                            selector,
                        )
                    }
                    ReferenceOr::Reference { reference } => {
                        tracing::debug!("Resolving response reference: {}", reference);
                        // Resolve the reference
                        if let Some(resolved_response) = spec.get_response(reference) {
                            tracing::debug!(
                                "Resolved response reference with {} content types",
                                resolved_response.content.len()
                            );
                            Self::generate_from_response_with_scenario_and_mode(
                                spec,
                                resolved_response,
                                content_type,
                                expand_tokens,
                                scenario,
                                selection_mode,
                                selector,
                            )
                        } else {
                            tracing::warn!("Response reference '{}' not found in spec", reference);
                            // Reference not found, return empty object
                            Ok(Value::Object(serde_json::Map::new()))
                        }
                    }
                }
            }
            None => {
                tracing::warn!(
                    "No response found for status code {} in operation. Available status codes: {:?}",
                    status_code,
                    operation.responses.responses.keys().collect::<Vec<_>>()
                );
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
    #[allow(dead_code)]
    fn generate_from_response(
        spec: &OpenApiSpec,
        response: &Response,
        content_type: Option<&str>,
        expand_tokens: bool,
    ) -> Result<Value> {
        Self::generate_from_response_with_scenario(
            spec,
            response,
            content_type,
            expand_tokens,
            None,
        )
    }

    /// Generate response from a Response object with scenario support
    #[allow(dead_code)]
    fn generate_from_response_with_scenario(
        spec: &OpenApiSpec,
        response: &Response,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
    ) -> Result<Value> {
        Self::generate_from_response_with_scenario_and_mode(
            spec,
            response,
            content_type,
            expand_tokens,
            scenario,
            None,
            None,
        )
    }

    /// Generate response from a Response object with scenario support and selection mode
    fn generate_from_response_with_scenario_and_mode(
        spec: &OpenApiSpec,
        response: &Response,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
    ) -> Result<Value> {
        Self::generate_from_response_with_scenario_and_mode_and_persona(
            spec,
            response,
            content_type,
            expand_tokens,
            scenario,
            selection_mode,
            selector,
            None, // No persona by default
        )
    }

    /// Generate response from a Response object with scenario support, selection mode, and persona
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    fn generate_from_response_with_scenario_and_mode_and_persona(
        spec: &OpenApiSpec,
        response: &Response,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
        persona: Option<&Persona>,
    ) -> Result<Value> {
        // If content_type is specified, look for that media type
        if let Some(content_type) = content_type {
            if let Some(media_type) = response.content.get(content_type) {
                return Self::generate_from_media_type_with_scenario_and_mode_and_persona(
                    spec,
                    media_type,
                    expand_tokens,
                    scenario,
                    selection_mode,
                    selector,
                    persona,
                );
            }
        }

        // If no content_type specified or not found, try common content types
        let preferred_types = ["application/json", "application/xml", "text/plain"];

        for content_type in &preferred_types {
            if let Some(media_type) = response.content.get(*content_type) {
                return Self::generate_from_media_type_with_scenario_and_mode_and_persona(
                    spec,
                    media_type,
                    expand_tokens,
                    scenario,
                    selection_mode,
                    selector,
                    persona,
                );
            }
        }

        // If no suitable content type found, return the first available
        if let Some((_, media_type)) = response.content.iter().next() {
            return Self::generate_from_media_type_with_scenario_and_mode_and_persona(
                spec,
                media_type,
                expand_tokens,
                scenario,
                selection_mode,
                selector,
                persona,
            );
        }

        // No content found, return empty object
        Ok(Value::Object(serde_json::Map::new()))
    }

    /// Generate response from a MediaType with optional scenario selection
    #[allow(dead_code)]
    fn generate_from_media_type(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
    ) -> Result<Value> {
        Self::generate_from_media_type_with_scenario(spec, media_type, expand_tokens, None)
    }

    /// Generate response from a MediaType with scenario support and selection mode
    #[allow(dead_code)]
    fn generate_from_media_type_with_scenario(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
        scenario: Option<&str>,
    ) -> Result<Value> {
        Self::generate_from_media_type_with_scenario_and_mode(
            spec,
            media_type,
            expand_tokens,
            scenario,
            None,
            None,
        )
    }

    /// Generate response from a MediaType with scenario support and selection mode (6 args)
    #[allow(dead_code)]
    fn generate_from_media_type_with_scenario_and_mode(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
    ) -> Result<Value> {
        Self::generate_from_media_type_with_scenario_and_mode_and_persona(
            spec,
            media_type,
            expand_tokens,
            scenario,
            selection_mode,
            selector,
            None, // No persona by default
        )
    }

    /// Generate response from a MediaType with scenario support, selection mode, and persona
    fn generate_from_media_type_with_scenario_and_mode_and_persona(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
        persona: Option<&Persona>,
    ) -> Result<Value> {
        // First, check if there's an explicit example
        // CRITICAL: Always check examples first before falling back to schema generation
        // This ensures GET requests use the correct response format from OpenAPI examples
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

        // Then check examples map - with scenario support and selection modes
        // CRITICAL: Always use examples if available, even if query parameters are missing
        // This fixes the bug where GET requests without query params return POST-style responses
        if !media_type.examples.is_empty() {
            use crate::openapi::response_selection::{ResponseSelectionMode, ResponseSelector};

            tracing::debug!(
                "Found {} examples in media type, available examples: {:?}",
                media_type.examples.len(),
                media_type.examples.keys().collect::<Vec<_>>()
            );

            // If a scenario is specified, try to find it first
            if let Some(scenario_name) = scenario {
                if let Some(example_ref) = media_type.examples.get(scenario_name) {
                    tracing::debug!("Using scenario '{}' from examples map", scenario_name);
                    match Self::extract_example_value_with_persona(
                        spec,
                        example_ref,
                        expand_tokens,
                        persona,
                        media_type.schema.as_ref(),
                    ) {
                        Ok(value) => return Ok(value),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to extract example for scenario '{}': {}, falling back",
                                scenario_name,
                                e
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "Scenario '{}' not found in examples, falling back based on selection mode",
                        scenario_name
                    );
                }
            }

            // Determine selection mode
            let mode = selection_mode.unwrap_or(ResponseSelectionMode::First);

            // Get list of example names for selection
            let example_names: Vec<String> = media_type.examples.keys().cloned().collect();

            if example_names.is_empty() {
                // No examples available, fall back to schema
                tracing::warn!("Examples map is empty, falling back to schema generation");
            } else if mode == ResponseSelectionMode::Scenario && scenario.is_some() {
                // Scenario mode was requested but scenario not found, fall through to selection mode
                tracing::debug!("Scenario not found, using selection mode: {:?}", mode);
            } else {
                // Use selection mode to choose an example
                let selected_index = if let Some(sel) = selector {
                    sel.select(&example_names)
                } else {
                    // Create temporary selector for this selection
                    let temp_selector = ResponseSelector::new(mode);
                    temp_selector.select(&example_names)
                };

                if let Some(example_name) = example_names.get(selected_index) {
                    if let Some(example_ref) = media_type.examples.get(example_name) {
                        tracing::debug!(
                            "Using example '{}' from examples map (mode: {:?}, index: {})",
                            example_name,
                            mode,
                            selected_index
                        );
                        match Self::extract_example_value_with_persona(
                            spec,
                            example_ref,
                            expand_tokens,
                            persona,
                            media_type.schema.as_ref(),
                        ) {
                            Ok(value) => return Ok(value),
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to extract example '{}': {}, trying fallback",
                                    example_name,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            // Fall back to first example if selection failed
            // This is critical - always use the first example if available, even if previous attempts failed
            if let Some((example_name, example_ref)) = media_type.examples.iter().next() {
                tracing::debug!(
                    "Using first example '{}' from examples map as fallback",
                    example_name
                );
                match Self::extract_example_value_with_persona(
                    spec,
                    example_ref,
                    expand_tokens,
                    persona,
                    media_type.schema.as_ref(),
                ) {
                    Ok(value) => {
                        tracing::debug!(
                            "Successfully extracted fallback example '{}'",
                            example_name
                        );
                        return Ok(value);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to extract fallback example '{}': {}, falling back to schema generation",
                            example_name,
                            e
                        );
                        // Continue to schema generation as last resort
                    }
                }
            }
        } else {
            tracing::debug!("No examples found in media type, will use schema generation");
        }

        // Fall back to schema-based generation
        // Pass persona through to schema generation for consistent data patterns
        if let Some(schema_ref) = &media_type.schema {
            Ok(Self::generate_example_from_schema_ref(spec, schema_ref, persona))
        } else {
            Ok(Value::Object(serde_json::Map::new()))
        }
    }

    /// Extract value from an example reference
    /// Optionally expands items arrays based on pagination metadata if persona is provided
    #[allow(dead_code)]
    fn extract_example_value(
        spec: &OpenApiSpec,
        example_ref: &ReferenceOr<openapiv3::Example>,
        expand_tokens: bool,
    ) -> Result<Value> {
        Self::extract_example_value_with_persona(spec, example_ref, expand_tokens, None, None)
    }

    /// Extract value from an example reference with optional persona and schema for pagination expansion
    fn extract_example_value_with_persona(
        spec: &OpenApiSpec,
        example_ref: &ReferenceOr<openapiv3::Example>,
        expand_tokens: bool,
        persona: Option<&Persona>,
        schema_ref: Option<&ReferenceOr<Schema>>,
    ) -> Result<Value> {
        let mut value = match example_ref {
            ReferenceOr::Item(example) => {
                if let Some(v) = &example.value {
                    tracing::debug!("Using example from examples map: {:?}", v);
                    if expand_tokens {
                        Self::expand_templates(v)
                    } else {
                        v.clone()
                    }
                } else {
                    return Ok(Value::Object(serde_json::Map::new()));
                }
            }
            ReferenceOr::Reference { reference } => {
                // Resolve the example reference
                if let Some(example) = spec.get_example(reference) {
                    if let Some(v) = &example.value {
                        tracing::debug!("Using resolved example reference: {:?}", v);
                        if expand_tokens {
                            Self::expand_templates(v)
                        } else {
                            v.clone()
                        }
                    } else {
                        return Ok(Value::Object(serde_json::Map::new()));
                    }
                } else {
                    tracing::warn!("Example reference '{}' not found", reference);
                    return Ok(Value::Object(serde_json::Map::new()));
                }
            }
        };

        // Check for pagination mismatch and expand items array if needed
        value = Self::expand_example_items_if_needed(spec, value, persona, schema_ref);

        Ok(value)
    }

    /// Expand items array in example if pagination metadata suggests more items
    /// Checks for common response structures: { data: { items: [...], total, limit } } or { items: [...], total, limit }
    fn expand_example_items_if_needed(
        _spec: &OpenApiSpec,
        mut example: Value,
        _persona: Option<&Persona>,
        _schema_ref: Option<&ReferenceOr<Schema>>,
    ) -> Value {
        // Try to find items array and pagination metadata in the example
        // Support both nested (data.items) and flat (items) structures
        let has_nested_items = example
            .get("data")
            .and_then(|v| v.as_object())
            .map(|obj| obj.contains_key("items"))
            .unwrap_or(false);

        let has_flat_items = example.get("items").is_some();

        if !has_nested_items && !has_flat_items {
            return example; // No items array found
        }

        // Extract pagination metadata
        let total = example
            .get("data")
            .and_then(|d| d.get("total"))
            .or_else(|| example.get("total"))
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));

        let limit = example
            .get("data")
            .and_then(|d| d.get("limit"))
            .or_else(|| example.get("limit"))
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)));

        // Get current items array
        let items_array = example
            .get("data")
            .and_then(|d| d.get("items"))
            .or_else(|| example.get("items"))
            .and_then(|v| v.as_array())
            .cloned();

        if let (Some(total_val), Some(limit_val), Some(mut items)) = (total, limit, items_array) {
            let current_count = items.len() as u64;
            let expected_count = std::cmp::min(total_val, limit_val);
            let max_items = 100; // Cap at reasonable maximum
            let expected_count = std::cmp::min(expected_count, max_items);

            // If items array is smaller than expected, expand it
            if current_count < expected_count && !items.is_empty() {
                tracing::debug!(
                    "Expanding example items array: {} -> {} items (total={}, limit={})",
                    current_count,
                    expected_count,
                    total_val,
                    limit_val
                );

                // Use first item as template
                let template = items[0].clone();
                let additional_count = expected_count - current_count;

                // Generate additional items
                for i in 0..additional_count {
                    let mut new_item = template.clone();
                    // Use the centralized variation function
                    let item_index = current_count + i + 1;
                    Self::add_item_variation(&mut new_item, item_index);
                    items.push(new_item);
                }

                // Update the items array in the example
                if let Some(data_obj) = example.get_mut("data").and_then(|v| v.as_object_mut()) {
                    data_obj.insert("items".to_string(), Value::Array(items));
                } else if let Some(root_obj) = example.as_object_mut() {
                    root_obj.insert("items".to_string(), Value::Array(items));
                }
            }
        }

        example
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
                let new_arr: Vec<Value> = arr.iter().map(Self::expand_templates).collect();
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
    pub operation: Operation,
}

impl OpenApiOperation {
    /// Create a new OpenApiOperation
    pub fn new(method: String, path: String, operation: Operation) -> Self {
        Self {
            method,
            path,
            operation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::ReferenceOr;
    use serde_json::json;

    // Mock AI generator for testing
    struct MockAiGenerator {
        response: Value,
    }

    #[async_trait]
    impl AiGenerator for MockAiGenerator {
        async fn generate(&self, _prompt: &str, _config: &AiResponseConfig) -> Result<Value> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn generates_example_using_referenced_schemas() {
        let yaml = r#"
openapi: 3.0.3
info:
  title: Test API
  version: "1.0.0"
paths:
  /apiaries:
    get:
      responses:
        '200':
          description: ok
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Apiary'
components:
  schemas:
    Apiary:
      type: object
      properties:
        id:
          type: string
        hive:
          $ref: '#/components/schemas/Hive'
    Hive:
      type: object
      properties:
        name:
          type: string
        active:
          type: boolean
        "#;

        let spec = OpenApiSpec::from_string(yaml, Some("yaml")).expect("load spec");
        let path_item = spec
            .spec
            .paths
            .paths
            .get("/apiaries")
            .and_then(ReferenceOr::as_item)
            .expect("path item");
        let operation = path_item.get.as_ref().expect("GET operation");

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .expect("generate response");

        let obj = response.as_object().expect("response object");
        assert!(obj.contains_key("id"));
        let hive = obj.get("hive").and_then(|value| value.as_object()).expect("hive object");
        assert!(hive.contains_key("name"));
        assert!(hive.contains_key("active"));
    }

    #[tokio::test]
    async fn test_generate_ai_response_with_generator() {
        let ai_config = AiResponseConfig {
            enabled: true,
            mode: crate::ai_response::AiResponseMode::Intelligent,
            prompt: Some("Generate a response for {{method}} {{path}}".to_string()),
            context: None,
            temperature: 0.7,
            max_tokens: 1000,
            schema: None,
            cache_enabled: true,
        };
        let context = RequestContext {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            multipart_fields: HashMap::new(),
            multipart_files: HashMap::new(),
        };
        let mock_generator = MockAiGenerator {
            response: json!({"message": "Generated response"}),
        };

        let result =
            ResponseGenerator::generate_ai_response(&ai_config, &context, Some(&mock_generator))
                .await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["message"], "Generated response");
    }

    #[tokio::test]
    async fn test_generate_ai_response_without_generator() {
        let ai_config = AiResponseConfig {
            enabled: true,
            mode: crate::ai_response::AiResponseMode::Intelligent,
            prompt: Some("Generate a response for {{method}} {{path}}".to_string()),
            context: None,
            temperature: 0.7,
            max_tokens: 1000,
            schema: None,
            cache_enabled: true,
        };
        let context = RequestContext {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            multipart_fields: HashMap::new(),
            multipart_files: HashMap::new(),
        };

        let result = ResponseGenerator::generate_ai_response(&ai_config, &context, None).await;

        // Without a generator, generate_ai_response returns an error
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no AI generator configured"),
            "Expected 'no AI generator configured' error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_generate_ai_response_no_prompt() {
        let ai_config = AiResponseConfig {
            enabled: true,
            mode: crate::ai_response::AiResponseMode::Intelligent,
            prompt: None,
            context: None,
            temperature: 0.7,
            max_tokens: 1000,
            schema: None,
            cache_enabled: true,
        };
        let context = RequestContext {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            multipart_fields: HashMap::new(),
            multipart_files: HashMap::new(),
        };

        let result = ResponseGenerator::generate_ai_response(&ai_config, &context, None).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_response_with_expansion() {
        let spec = OpenApiSpec::from_string(
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

        let response = ResponseGenerator::generate_response_with_expansion(
            &spec,
            operation,
            200,
            Some("application/json"),
            true,
        )
        .unwrap();

        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_scenario() {
        let spec = OpenApiSpec::from_string(
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
              examples:
                happy:
                  value:
                    id: 1
                    name: "Happy User"
                sad:
                  value:
                    id: 2
                    name: "Sad User"
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

        let response = ResponseGenerator::generate_response_with_scenario(
            &spec,
            operation,
            200,
            Some("application/json"),
            false,
            Some("happy"),
        )
        .unwrap();

        assert_eq!(response["id"], 1);
        assert_eq!(response["name"], "Happy User");
    }

    #[test]
    fn test_generate_response_with_referenced_response() {
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          $ref: '#/components/responses/UserResponse'
components:
  responses:
    UserResponse:
      description: User response
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_default_status() {
        let spec = OpenApiSpec::from_string(
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
          content:
            application/json:
              schema:
                type: object
                properties:
                  error:
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

        // Use default response for 500 status
        let response =
            ResponseGenerator::generate_response(&spec, operation, 500, Some("application/json"))
                .unwrap();

        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_example_in_media_type() {
        let spec = OpenApiSpec::from_string(
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
              example:
                id: 1
                name: "Example User"
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        assert_eq!(response["id"], 1);
        assert_eq!(response["name"], "Example User");
    }

    #[test]
    fn test_generate_response_with_schema_example() {
        let spec = OpenApiSpec::from_string(
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
                example:
                  id: 42
                  name: "Schema Example"
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should use schema example if available
        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_referenced_schema() {
        let spec = OpenApiSpec::from_string(
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        assert!(response.is_object());
        assert!(response.get("id").is_some());
        assert!(response.get("name").is_some());
    }

    #[test]
    fn test_generate_response_with_array_schema() {
        let spec = OpenApiSpec::from_string(
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
                type: array
                items:
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        assert!(response.is_array());
    }

    #[test]
    fn test_generate_response_with_different_content_types() {
        let spec = OpenApiSpec::from_string(
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
            text/plain:
              schema:
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

        // Test JSON content type
        let json_response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();
        assert!(json_response.is_object());

        // Test text/plain content type
        let text_response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("text/plain"))
                .unwrap();
        assert!(text_response.is_string());
    }

    #[test]
    fn test_generate_response_without_content_type() {
        let spec = OpenApiSpec::from_string(
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

        // No content type specified - should use first available
        let response = ResponseGenerator::generate_response(&spec, operation, 200, None).unwrap();

        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_no_content() {
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    delete:
      responses:
        '204':
          description: No Content
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
            .and_then(|p| p.delete.as_ref())
            .unwrap();

        let response = ResponseGenerator::generate_response(&spec, operation, 204, None).unwrap();

        // Should return empty object for no content
        assert!(response.is_object());
        assert!(response.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_generate_response_with_expansion_disabled() {
        let spec = OpenApiSpec::from_string(
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

        let response = ResponseGenerator::generate_response_with_expansion(
            &spec,
            operation,
            200,
            Some("application/json"),
            false, // No expansion
        )
        .unwrap();

        assert!(response.is_object());
    }

    #[test]
    fn test_generate_response_with_array_schema_referenced_items() {
        // Test array schema with referenced item schema (lines 1035-1046)
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /items:
    get:
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Item'
components:
  schemas:
    Item:
      type: object
      properties:
        id:
          type: string
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
            .get("/items")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should generate an array with items from referenced schema
        let arr = response.as_array().expect("response should be array");
        assert!(!arr.is_empty());
        if let Some(item) = arr.first() {
            let obj = item.as_object().expect("item should be object");
            assert!(obj.contains_key("id") || obj.contains_key("name"));
        }
    }

    #[test]
    fn test_generate_response_with_array_schema_missing_reference() {
        // Test array schema with missing referenced item schema (line 1045)
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /items:
    get:
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/NonExistentItem'
components:
  schemas: {}
"#,
            Some("yaml"),
        )
        .unwrap();

        let operation = spec
            .spec
            .paths
            .paths
            .get("/items")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should generate an array with empty objects when reference not found
        let arr = response.as_array().expect("response should be array");
        assert!(!arr.is_empty());
    }

    #[test]
    fn test_generate_response_with_array_example_and_pagination() {
        // Test array generation with pagination using example items (lines 1114-1126)
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /products:
    get:
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: array
                example: [{"id": 1, "name": "Product 1"}]
                items:
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
            .get("/products")
            .and_then(|p| p.as_item())
            .and_then(|p| p.get.as_ref())
            .unwrap();

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should generate an array using the example as template
        let arr = response.as_array().expect("response should be array");
        assert!(!arr.is_empty());
        if let Some(item) = arr.first() {
            let obj = item.as_object().expect("item should be object");
            assert!(obj.contains_key("id") || obj.contains_key("name"));
        }
    }

    #[test]
    fn test_generate_response_with_missing_response_reference() {
        // Test response generation with missing response reference (lines 294-298)
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '200':
          $ref: '#/components/responses/NonExistentResponse'
components:
  responses: {}
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

        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should return empty object when reference not found
        assert!(response.is_object());
        assert!(response.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_generate_response_with_no_response_for_status() {
        // Test response generation when no response found for status code (lines 302-310)
        let spec = OpenApiSpec::from_string(
            r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      responses:
        '404':
          description: Not found
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

        // Request status code 200 but only 404 is defined
        let response =
            ResponseGenerator::generate_response(&spec, operation, 200, Some("application/json"))
                .unwrap();

        // Should return empty object when no response found
        assert!(response.is_object());
        assert!(response.as_object().unwrap().is_empty());
    }
}
