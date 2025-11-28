//! OpenAPI response generation and mocking
//!
//! This module provides functionality for generating mock responses
//! based on OpenAPI specifications.

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
    /// Generate an AI-assisted response using LLM
    ///
    /// This method generates a dynamic response based on request context
    /// using the configured LLM provider (OpenAI, Anthropic, etc.)
    ///
    /// # Arguments
    /// * `ai_config` - The AI response configuration
    /// * `context` - The request context for prompt expansion
    /// * `generator` - Optional AI generator implementation (if None, returns placeholder)
    ///
    /// # Returns
    /// A JSON value containing the generated response
    pub async fn generate_ai_response(
        ai_config: &AiResponseConfig,
        context: &RequestContext,
        generator: Option<&dyn AiGenerator>,
    ) -> Result<Value> {
        // Get the prompt template and expand it with request context
        let prompt_template = ai_config
            .prompt
            .as_ref()
            .ok_or_else(|| crate::Error::generic("AI prompt is required"))?;

        // Note: expand_prompt_template is now in mockforge-template-expansion crate
        // For now, we'll do a simple string replacement as a fallback
        // In the future, this should be refactored to use the template expansion crate
        let expanded_prompt = prompt_template
            .replace("{{method}}", &context.method)
            .replace("{{path}}", &context.path);

        tracing::info!("AI response generation requested with prompt: {}", expanded_prompt);

        // Use the provided generator if available
        if let Some(gen) = generator {
            tracing::debug!("Using provided AI generator for response");
            return gen.generate(&expanded_prompt, ai_config).await;
        }

        // Fallback: return a descriptive placeholder if no generator is provided
        tracing::warn!("No AI generator provided, returning placeholder response");
        Ok(serde_json::json!({
            "ai_response": "AI generation placeholder",
            "note": "This endpoint is configured for AI-assisted responses, but no AI generator was provided",
            "expanded_prompt": expanded_prompt,
            "mode": format!("{:?}", ai_config.mode),
            "temperature": ai_config.temperature,
            "implementation_note": "Pass an AiGenerator implementation to ResponseGenerator::generate_ai_response to enable actual AI generation"
        }))
    }

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
    pub fn generate_response_with_scenario_and_mode_and_persona(
        spec: &OpenApiSpec,
        operation: &Operation,
        status_code: u16,
        content_type: Option<&str>,
        expand_tokens: bool,
        scenario: Option<&str>,
        selection_mode: Option<crate::openapi::response_selection::ResponseSelectionMode>,
        selector: Option<&crate::openapi::response_selection::ResponseSelector>,
        persona: Option<&Persona>,
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
    fn generate_from_media_type(
        spec: &OpenApiSpec,
        media_type: &openapiv3::MediaType,
        expand_tokens: bool,
    ) -> Result<Value> {
        Self::generate_from_media_type_with_scenario(spec, media_type, expand_tokens, None)
    }

    /// Generate response from a MediaType with scenario support and selection mode
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

    /// Generate response from a MediaType with scenario support and selection mode
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
        spec: &OpenApiSpec,
        mut example: Value,
        persona: Option<&Persona>,
        schema_ref: Option<&ReferenceOr<Schema>>,
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

    fn generate_example_from_schema_ref(
        spec: &OpenApiSpec,
        schema_ref: &ReferenceOr<Schema>,
        persona: Option<&Persona>,
    ) -> Value {
        match schema_ref {
            ReferenceOr::Item(schema) => Self::generate_example_from_schema(spec, schema, persona),
            ReferenceOr::Reference { reference } => spec
                .get_schema(reference)
                .map(|schema| Self::generate_example_from_schema(spec, &schema.schema, persona))
                .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
        }
    }

    /// Generate example data from an OpenAPI schema
    ///
    /// Priority order:
    /// 1. Schema-level example (schema.schema_data.example)
    /// 2. Property-level examples when generating objects
    /// 3. Generated values based on schema type
    /// 4. Persona traits (if persona provided)
    fn generate_example_from_schema(
        spec: &OpenApiSpec,
        schema: &Schema,
        persona: Option<&Persona>,
    ) -> Value {
        // First, check for schema-level example in schema_data
        // OpenAPI v3 stores examples in schema_data.example
        if let Some(example) = schema.schema_data.example.as_ref() {
            tracing::debug!("Using schema-level example: {:?}", example);
            return example.clone();
        }

        // Note: schema-level example check happens at the top of the function (line 380-383)
        // At this point, if we have a schema-level example, we've already returned it
        // So we only generate defaults when no example exists
        match &schema.schema_kind {
            openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => {
                // Use faker for string fields based on field name hints
                Value::String("example string".to_string())
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => Value::Number(42.into()),
            openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => {
                Value::Number(serde_json::Number::from_f64(std::f64::consts::PI).unwrap())
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Boolean(_)) => Value::Bool(true),
            openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) => {
                // First pass: Scan for pagination metadata (total, page, limit)
                // This helps us generate the correct number of array items
                let mut pagination_metadata: Option<(u64, u64, u64)> = None; // (total, page, limit)

                // Check if this looks like a paginated response by scanning properties
                // Look for "items" array property and pagination fields
                let has_items =
                    obj.properties.iter().any(|(name, _)| name.to_lowercase() == "items");

                if has_items {
                    // Try to extract pagination metadata from schema properties
                    let mut total_opt = None;
                    let mut page_opt = None;
                    let mut limit_opt = None;

                    for (prop_name, prop_schema) in &obj.properties {
                        let prop_lower = prop_name.to_lowercase();
                        // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema> for extraction
                        let schema_ref: ReferenceOr<Schema> = match prop_schema {
                            ReferenceOr::Item(boxed) => ReferenceOr::Item(boxed.as_ref().clone()),
                            ReferenceOr::Reference { reference } => ReferenceOr::Reference {
                                reference: reference.clone(),
                            },
                        };
                        if prop_lower == "total" || prop_lower == "count" || prop_lower == "size" {
                            total_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        } else if prop_lower == "page" {
                            page_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        } else if prop_lower == "limit" || prop_lower == "per_page" {
                            limit_opt = Self::extract_numeric_value_from_schema(&schema_ref);
                        }
                    }

                    // If we found a total, use it (with defaults for page/limit)
                    if let Some(total) = total_opt {
                        let page = page_opt.unwrap_or(1);
                        let limit = limit_opt.unwrap_or(20);
                        pagination_metadata = Some((total, page, limit));
                        tracing::debug!(
                            "Detected pagination metadata: total={}, page={}, limit={}",
                            total,
                            page,
                            limit
                        );
                    } else {
                        // Phase 3: If no total found in schema, try to infer from parent entity
                        // Look for "items" array to determine child entity name
                        if obj.properties.contains_key("items") {
                            // Try to infer parent/child relationship from schema names
                            // This is a heuristic: if we're generating a paginated response,
                            // check if we can find a parent entity schema with a count field
                            if let Some(inferred_total) =
                                Self::try_infer_total_from_context(spec, obj)
                            {
                                let page = page_opt.unwrap_or(1);
                                let limit = limit_opt.unwrap_or(20);
                                pagination_metadata = Some((inferred_total, page, limit));
                                tracing::debug!(
                                    "Inferred pagination metadata from parent entity: total={}, page={}, limit={}",
                                    inferred_total, page, limit
                                );
                            } else {
                                // Phase 4: Try to use persona traits if available
                                if let Some(persona) = persona {
                                    // Look for count-related traits (e.g., "hive_count", "apiary_count")
                                    // Try common patterns
                                    let count_keys =
                                        ["hive_count", "apiary_count", "item_count", "total_count"];
                                    for key in &count_keys {
                                        if let Some(count) = persona.get_numeric_trait(key) {
                                            let page = page_opt.unwrap_or(1);
                                            let limit = limit_opt.unwrap_or(20);
                                            pagination_metadata = Some((count, page, limit));
                                            tracing::debug!(
                                                "Using persona trait '{}' for pagination: total={}, page={}, limit={}",
                                                key, count, page, limit
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut map = serde_json::Map::new();
                for (prop_name, prop_schema) in &obj.properties {
                    let prop_lower = prop_name.to_lowercase();

                    // Check if this is an array property that should use pagination metadata
                    let is_items_array = prop_lower == "items" && pagination_metadata.is_some();

                    let value = match prop_schema {
                        ReferenceOr::Item(prop_schema) => {
                            // If this is an items array with pagination metadata, always use generate_array_with_count
                            // (it will use the example as a template if one exists)
                            if is_items_array {
                                // Generate array with count based on pagination metadata
                                Self::generate_array_with_count(
                                    spec,
                                    prop_schema.as_ref(),
                                    pagination_metadata.unwrap(),
                                    persona,
                                )
                            } else if let Some(prop_example) =
                                prop_schema.schema_data.example.as_ref()
                            {
                                // Check for property-level example (only if not items array)
                                tracing::debug!(
                                    "Using example for property '{}': {:?}",
                                    prop_name,
                                    prop_example
                                );
                                prop_example.clone()
                            } else {
                                Self::generate_example_from_schema(
                                    spec,
                                    prop_schema.as_ref(),
                                    persona,
                                )
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            // Try to resolve reference
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                // If this is an items array with pagination metadata, always use generate_array_with_count
                                if is_items_array {
                                    // Generate array with count based on pagination metadata
                                    Self::generate_array_with_count(
                                        spec,
                                        &resolved_schema.schema,
                                        pagination_metadata.unwrap(),
                                        persona,
                                    )
                                } else if let Some(ref_example) =
                                    resolved_schema.schema.schema_data.example.as_ref()
                                {
                                    // Check for example from referenced schema (only if not items array)
                                    tracing::debug!(
                                        "Using example from referenced schema '{}': {:?}",
                                        reference,
                                        ref_example
                                    );
                                    ref_example.clone()
                                } else {
                                    Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    )
                                }
                            } else {
                                Self::generate_example_for_property(prop_name)
                            }
                        }
                    };
                    let value = match value {
                        Value::Null => Self::generate_example_for_property(prop_name),
                        Value::Object(ref obj) if obj.is_empty() => {
                            Self::generate_example_for_property(prop_name)
                        }
                        _ => value,
                    };
                    map.insert(prop_name.clone(), value);
                }

                // Ensure pagination metadata is set if we detected it
                if let Some((total, page, limit)) = pagination_metadata {
                    map.insert("total".to_string(), Value::Number(total.into()));
                    map.insert("page".to_string(), Value::Number(page.into()));
                    map.insert("limit".to_string(), Value::Number(limit.into()));
                }

                Value::Object(map)
            }
            openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
                // Check for array-level example (schema.schema_data.example contains the full array)
                // Note: This check is actually redundant since we check at the top,
                // but keeping it here for clarity and defensive programming
                // If the array schema itself has an example, it's already handled at the top

                match &arr.items {
                    Some(item_schema) => {
                        let example_item = match item_schema {
                            ReferenceOr::Item(item_schema) => {
                                // Recursively generate example for array item
                                // This will check for item-level examples
                                Self::generate_example_from_schema(
                                    spec,
                                    item_schema.as_ref(),
                                    persona,
                                )
                            }
                            ReferenceOr::Reference { reference } => {
                                // Try to resolve reference and generate example
                                // This will check for examples in referenced schema
                                if let Some(resolved_schema) = spec.get_schema(reference) {
                                    Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    )
                                } else {
                                    Value::Object(serde_json::Map::new())
                                }
                            }
                        };
                        Value::Array(vec![example_item])
                    }
                    None => Value::Array(vec![Value::String("item".to_string())]),
                }
            }
            _ => Value::Object(serde_json::Map::new()),
        }
    }

    /// Extract numeric value from a schema (from example or default)
    /// Returns None if no numeric value can be extracted
    fn extract_numeric_value_from_schema(schema_ref: &ReferenceOr<Schema>) -> Option<u64> {
        match schema_ref {
            ReferenceOr::Item(schema) => {
                // Check for example value first
                if let Some(example) = schema.schema_data.example.as_ref() {
                    if let Some(num) = example.as_u64() {
                        return Some(num);
                    } else if let Some(num) = example.as_f64() {
                        return Some(num as u64);
                    }
                }
                // Check for default value
                if let Some(default) = schema.schema_data.default.as_ref() {
                    if let Some(num) = default.as_u64() {
                        return Some(num);
                    } else if let Some(num) = default.as_f64() {
                        return Some(num as u64);
                    }
                }
                // For integer types, try to extract from schema constraints
                // Note: IntegerType doesn't have a default field in openapiv3
                // Defaults are stored in schema_data.default instead
                None
            }
            ReferenceOr::Reference { reference: _ } => {
                // For references, we'd need to resolve them, but for now return None
                // This can be enhanced later if needed
                None
            }
        }
    }

    /// Generate an array with a specific count based on pagination metadata
    /// Respects the limit (e.g., if total=50 and limit=20, generates 20 items)
    fn generate_array_with_count(
        spec: &OpenApiSpec,
        array_schema: &Schema,
        pagination: (u64, u64, u64), // (total, page, limit)
        persona: Option<&Persona>,
    ) -> Value {
        let (total, _page, limit) = pagination;

        // Determine how many items to generate
        // Respect pagination: generate min(total, limit) items
        let count = std::cmp::min(total, limit);

        // Cap at reasonable maximum to avoid performance issues
        let max_items = 100;
        let count = std::cmp::min(count, max_items);

        tracing::debug!("Generating array with count={} (total={}, limit={})", count, total, limit);

        // Check if array schema has an example with items
        if let Some(example) = array_schema.schema_data.example.as_ref() {
            if let Some(example_array) = example.as_array() {
                if !example_array.is_empty() {
                    // Use first example item as template
                    let template_item = &example_array[0];
                    let items: Vec<Value> = (0..count)
                        .map(|i| {
                            // Clone template and add variation
                            let mut item = template_item.clone();
                            Self::add_item_variation(&mut item, i + 1);
                            item
                        })
                        .collect();
                    return Value::Array(items);
                }
            }
        }

        // Generate items from schema
        if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) = &array_schema.schema_kind
        {
            if let Some(item_schema) = &arr.items {
                let items: Vec<Value> = match item_schema {
                    ReferenceOr::Item(item_schema) => {
                        (0..count)
                            .map(|i| {
                                let mut item = Self::generate_example_from_schema(
                                    spec,
                                    item_schema.as_ref(),
                                    persona,
                                );
                                // Add variation to make items unique
                                Self::add_item_variation(&mut item, i + 1);
                                item
                            })
                            .collect()
                    }
                    ReferenceOr::Reference { reference } => {
                        if let Some(resolved_schema) = spec.get_schema(reference) {
                            (0..count)
                                .map(|i| {
                                    let mut item = Self::generate_example_from_schema(
                                        spec,
                                        &resolved_schema.schema,
                                        persona,
                                    );
                                    // Add variation to make items unique
                                    Self::add_item_variation(&mut item, i + 1);
                                    item
                                })
                                .collect()
                        } else {
                            vec![Value::Object(serde_json::Map::new()); count as usize]
                        }
                    }
                };
                return Value::Array(items);
            }
        }

        // Fallback: generate simple items
        Value::Array((0..count).map(|i| Value::String(format!("item_{}", i + 1))).collect())
    }

    /// Add variation to an item to make it unique (for array generation)
    /// Varies IDs, names, addresses, and coordinates based on item index
    fn add_item_variation(item: &mut Value, item_index: u64) {
        if let Some(obj) = item.as_object_mut() {
            // Update ID fields to be unique
            if let Some(id_val) = obj.get_mut("id") {
                if let Some(id_str) = id_val.as_str() {
                    // Extract base ID (remove any existing suffix)
                    let base_id = id_str.split('_').next().unwrap_or(id_str);
                    *id_val = Value::String(format!("{}_{:03}", base_id, item_index));
                } else if let Some(id_num) = id_val.as_u64() {
                    *id_val = Value::Number((id_num + item_index).into());
                }
            }

            // Update name fields - add variation for all names
            if let Some(name_val) = obj.get_mut("name") {
                if let Some(name_str) = name_val.as_str() {
                    if name_str.contains('#') {
                        // Pattern like "Hive #1" -> "Hive #2"
                        *name_val = Value::String(format!("Hive #{}", item_index));
                    } else {
                        // Pattern like "Meadow Apiary" -> use rotation of varied names
                        // 60+ unique apiary names with geographic diversity for realistic demo
                        let apiary_names = [
                            // Midwest/Prairie names
                            "Meadow Apiary",
                            "Prairie Apiary",
                            "Sunset Valley Apiary",
                            "Golden Fields Apiary",
                            "Miller Family Apiary",
                            "Heartland Honey Co.",
                            "Cornfield Apiary",
                            "Harvest Moon Apiary",
                            "Prairie Winds Apiary",
                            "Amber Fields Apiary",
                            // California/Coastal names
                            "Coastal Apiary",
                            "Sunset Coast Apiary",
                            "Pacific Grove Apiary",
                            "Golden Gate Apiary",
                            "Napa Valley Apiary",
                            "Coastal Breeze Apiary",
                            "Pacific Heights Apiary",
                            "Bay Area Apiary",
                            "Sunset Valley Honey Co.",
                            "Coastal Harvest Apiary",
                            // Texas/Ranch names
                            "Lone Star Apiary",
                            "Texas Ranch Apiary",
                            "Big Sky Apiary",
                            "Prairie Rose Apiary",
                            "Hill Country Apiary",
                            "Lone Star Honey Co.",
                            "Texas Pride Apiary",
                            "Wildflower Ranch",
                            "Desert Bloom Apiary",
                            "Cactus Creek Apiary",
                            // Florida/Grove names
                            "Orange Grove Apiary",
                            "Citrus Grove Apiary",
                            "Palm Grove Apiary",
                            "Tropical Breeze Apiary",
                            "Everglades Apiary",
                            "Sunshine State Apiary",
                            "Florida Keys Apiary",
                            "Grove View Apiary",
                            "Tropical Harvest Apiary",
                            "Palm Coast Apiary",
                            // Northeast/Valley names
                            "Mountain View Apiary",
                            "Valley Apiary",
                            "Riverside Apiary",
                            "Hilltop Apiary",
                            "Forest Apiary",
                            "Mountain Apiary",
                            "Lakeside Apiary",
                            "Ridge Apiary",
                            "Brook Apiary",
                            "Hillside Apiary",
                            // Generic/Professional names
                            "Field Apiary",
                            "Creek Apiary",
                            "Woodland Apiary",
                            "Farm Apiary",
                            "Orchard Apiary",
                            "Pasture Apiary",
                            "Green Valley Apiary",
                            "Blue Sky Apiary",
                            "Sweet Honey Apiary",
                            "Nature's Best Apiary",
                            // Business/Commercial names
                            "Premium Honey Co.",
                            "Artisan Apiary",
                            "Heritage Apiary",
                            "Summit Apiary",
                            "Crystal Springs Apiary",
                            "Maple Grove Apiary",
                            "Wildflower Apiary",
                            "Thistle Apiary",
                            "Clover Field Apiary",
                            "Honeycomb Apiary",
                        ];
                        let name_index = (item_index - 1) as usize % apiary_names.len();
                        *name_val = Value::String(apiary_names[name_index].to_string());
                    }
                }
            }

            // Update location/address fields
            if let Some(location_val) = obj.get_mut("location") {
                if let Some(location_obj) = location_val.as_object_mut() {
                    // Update address
                    if let Some(address_val) = location_obj.get_mut("address") {
                        if let Some(address_str) = address_val.as_str() {
                            // Extract street number if present, otherwise add variation
                            if let Some(num_str) = address_str.split_whitespace().next() {
                                if let Ok(num) = num_str.parse::<u64>() {
                                    *address_val =
                                        Value::String(format!("{} Farm Road", num + item_index));
                                } else {
                                    *address_val =
                                        Value::String(format!("{} Farm Road", 100 + item_index));
                                }
                            } else {
                                *address_val =
                                    Value::String(format!("{} Farm Road", 100 + item_index));
                            }
                        }
                    }

                    // Vary coordinates slightly
                    if let Some(lat_val) = location_obj.get_mut("latitude") {
                        if let Some(lat) = lat_val.as_f64() {
                            *lat_val = Value::Number(
                                serde_json::Number::from_f64(lat + (item_index as f64 * 0.01))
                                    .unwrap(),
                            );
                        }
                    }
                    if let Some(lng_val) = location_obj.get_mut("longitude") {
                        if let Some(lng) = lng_val.as_f64() {
                            *lng_val = Value::Number(
                                serde_json::Number::from_f64(lng + (item_index as f64 * 0.01))
                                    .unwrap(),
                            );
                        }
                    }
                } else if let Some(address_str) = location_val.as_str() {
                    // Flat address string
                    if let Some(num_str) = address_str.split_whitespace().next() {
                        if let Ok(num) = num_str.parse::<u64>() {
                            *location_val =
                                Value::String(format!("{} Farm Road", num + item_index));
                        } else {
                            *location_val =
                                Value::String(format!("{} Farm Road", 100 + item_index));
                        }
                    }
                }
            }

            // Update address field if it exists at root level
            if let Some(address_val) = obj.get_mut("address") {
                if let Some(address_str) = address_val.as_str() {
                    if let Some(num_str) = address_str.split_whitespace().next() {
                        if let Ok(num) = num_str.parse::<u64>() {
                            *address_val = Value::String(format!("{} Farm Road", num + item_index));
                        } else {
                            *address_val = Value::String(format!("{} Farm Road", 100 + item_index));
                        }
                    }
                }
            }

            // Vary status fields (common enum values)
            if let Some(status_val) = obj.get_mut("status") {
                if let Some(status_str) = status_val.as_str() {
                    let statuses = [
                        "healthy",
                        "sick",
                        "needs_attention",
                        "quarantined",
                        "active",
                        "inactive",
                    ];
                    let status_index = (item_index - 1) as usize % statuses.len();
                    // Bias towards "healthy" and "active" (70% of items)
                    let final_status = if (item_index - 1) % 10 < 7 {
                        statuses[0] // "healthy" or "active"
                    } else {
                        statuses[status_index]
                    };
                    *status_val = Value::String(final_status.to_string());
                }
            }

            // Vary hive_type fields
            if let Some(hive_type_val) = obj.get_mut("hive_type") {
                if hive_type_val.as_str().is_some() {
                    let hive_types = ["langstroth", "top_bar", "warre", "flow_hive", "national"];
                    let type_index = (item_index - 1) as usize % hive_types.len();
                    *hive_type_val = Value::String(hive_types[type_index].to_string());
                }
            }

            // Vary nested queen breed fields
            if let Some(queen_val) = obj.get_mut("queen") {
                if let Some(queen_obj) = queen_val.as_object_mut() {
                    if let Some(breed_val) = queen_obj.get_mut("breed") {
                        if breed_val.as_str().is_some() {
                            let breeds =
                                ["italian", "carniolan", "russian", "buckfast", "caucasian"];
                            let breed_index = (item_index - 1) as usize % breeds.len();
                            *breed_val = Value::String(breeds[breed_index].to_string());
                        }
                    }
                    // Vary queen age
                    if let Some(age_val) = queen_obj.get_mut("age_days") {
                        if let Some(base_age) = age_val.as_u64() {
                            *age_val = Value::Number((base_age + (item_index * 10) % 200).into());
                        } else if let Some(base_age) = age_val.as_i64() {
                            *age_val =
                                Value::Number((base_age + (item_index as i64 * 10) % 200).into());
                        }
                    }
                    // Vary queen mark color
                    if let Some(color_val) = queen_obj.get_mut("mark_color") {
                        if color_val.as_str().is_some() {
                            let colors = ["yellow", "white", "red", "green", "blue"];
                            let color_index = (item_index - 1) as usize % colors.len();
                            *color_val = Value::String(colors[color_index].to_string());
                        }
                    }
                }
            }

            // Vary description fields if they exist
            if let Some(desc_val) = obj.get_mut("description") {
                if let Some(desc_str) = desc_val.as_str() {
                    let descriptions = [
                        "Production apiary",
                        "Research apiary",
                        "Commercial operation",
                        "Backyard apiary",
                        "Educational apiary",
                    ];
                    let desc_index = (item_index - 1) as usize % descriptions.len();
                    *desc_val = Value::String(descriptions[desc_index].to_string());
                }
            }

            // Vary timestamp fields (created_at, updated_at, timestamp, date) for realistic time-series data
            // Generate timestamps spanning 12-24 months with proper distribution
            let timestamp_fields = [
                "created_at",
                "updated_at",
                "timestamp",
                "date",
                "forecastDate",
                "predictedDate",
            ];
            for field_name in &timestamp_fields {
                if let Some(timestamp_val) = obj.get_mut(*field_name) {
                    if let Some(_timestamp_str) = timestamp_val.as_str() {
                        // Generate realistic timestamp: distribute items over past 12-18 months
                        // Use item_index to create variation (not all same date)
                        let months_ago = 12 + ((item_index - 1) % 6); // Distribute over 6 months (12-18 months ago)
                        let days_offset = (item_index - 1) % 28; // Distribute within month (cap at 28)
                        let hours_offset = ((item_index * 7) % 24) as u8; // Distribute throughout day
                        let minutes_offset = ((item_index * 11) % 60) as u8; // Vary minutes

                        // Calculate timestamp relative to current date (November 2024)
                        // Format: ISO 8601 (e.g., "2024-11-12T14:30:00Z")
                        let base_year = 2024;
                        let base_month = 11;

                        // Calculate target month (going back in time)
                        let target_year = if months_ago >= base_month as u64 {
                            base_year - 1
                        } else {
                            base_year
                        };
                        let target_month = if months_ago >= base_month as u64 {
                            12 - (months_ago - base_month as u64) as u8
                        } else {
                            (base_month as u64 - months_ago) as u8
                        };
                        let target_day = std::cmp::min(28, 1 + days_offset as u8); // Start from day 1, cap at 28

                        // Format as ISO 8601
                        let timestamp = format!(
                            "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
                            target_year, target_month, target_day, hours_offset, minutes_offset
                        );
                        *timestamp_val = Value::String(timestamp);
                    }
                }
            }
        }
    }

    /// Try to infer total count from context (parent entity schemas)
    /// This is a heuristic that looks for common relationship patterns
    fn try_infer_total_from_context(
        spec: &OpenApiSpec,
        obj_type: &openapiv3::ObjectType,
    ) -> Option<u64> {
        // Look for "items" array to determine what we're generating
        if let Some(items_schema_ref) = obj_type.properties.get("items") {
            // Try to determine child entity name from items schema
            // This is a heuristic: check schema names in the spec
            if let Some(components) = &spec.spec.components {
                let schemas = &components.schemas;
                // Look through all schemas to find potential parent entities
                // that might have count fields matching the items type
                for (schema_name, schema_ref) in schemas {
                    if let ReferenceOr::Item(schema) = schema_ref {
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) =
                            &schema.schema_kind
                        {
                            // Look for count fields that might match
                            for (prop_name, prop_schema) in &obj.properties {
                                let prop_lower = prop_name.to_lowercase();
                                if prop_lower.ends_with("_count") {
                                    // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema>
                                    let schema_ref: ReferenceOr<Schema> = match prop_schema {
                                        ReferenceOr::Item(boxed) => {
                                            ReferenceOr::Item(boxed.as_ref().clone())
                                        }
                                        ReferenceOr::Reference { reference } => {
                                            ReferenceOr::Reference {
                                                reference: reference.clone(),
                                            }
                                        }
                                    };
                                    // Found a count field, try to extract its value
                                    if let Some(count) =
                                        Self::extract_numeric_value_from_schema(&schema_ref)
                                    {
                                        // Use a reasonable default if count is very large
                                        if count > 0 && count <= 1000 {
                                            tracing::debug!(
                                                "Inferred count {} from parent schema {} field {}",
                                                count,
                                                schema_name,
                                                prop_name
                                            );
                                            return Some(count);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Infer relationship count from parent entity schema
    /// When generating a child entity list, check if parent entity has a count field
    fn infer_count_from_parent_schema(
        spec: &OpenApiSpec,
        parent_entity_name: &str,
        child_entity_name: &str,
    ) -> Option<u64> {
        // Look for parent entity schema
        let parent_schema_name = parent_entity_name.to_string();
        let count_field_name = format!("{}_count", child_entity_name);

        // Try to find the schema
        if let Some(components) = &spec.spec.components {
            let schemas = &components.schemas;
            // Look for parent schema (case-insensitive)
            for (schema_name, schema_ref) in schemas {
                let schema_name_lower = schema_name.to_lowercase();
                if schema_name_lower.contains(&parent_entity_name.to_lowercase()) {
                    if let ReferenceOr::Item(schema) = schema_ref {
                        // Check if this schema has the count field
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) =
                            &schema.schema_kind
                        {
                            for (prop_name, prop_schema) in &obj.properties {
                                if prop_name.to_lowercase() == count_field_name.to_lowercase() {
                                    // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema>
                                    let schema_ref: ReferenceOr<Schema> = match prop_schema {
                                        ReferenceOr::Item(boxed) => {
                                            ReferenceOr::Item(boxed.as_ref().clone())
                                        }
                                        ReferenceOr::Reference { reference } => {
                                            ReferenceOr::Reference {
                                                reference: reference.clone(),
                                            }
                                        }
                                    };
                                    // Extract count value from schema
                                    return Self::extract_numeric_value_from_schema(&schema_ref);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Generate example value for a property based on its name
    fn generate_example_for_property(prop_name: &str) -> Value {
        let prop_lower = prop_name.to_lowercase();

        // Generate realistic data based on property name patterns
        if prop_lower.contains("id") || prop_lower.contains("uuid") {
            Value::String(uuid::Uuid::new_v4().to_string())
        } else if prop_lower.contains("email") {
            Value::String(format!("user{}@example.com", thread_rng().random_range(1000..=9999)))
        } else if prop_lower.contains("name") || prop_lower.contains("title") {
            let names = ["John Doe", "Jane Smith", "Bob Johnson", "Alice Brown"];
            Value::String(names[thread_rng().random_range(0..names.len())].to_string())
        } else if prop_lower.contains("phone") || prop_lower.contains("mobile") {
            Value::String(format!("+1-555-{:04}", thread_rng().random_range(1000..=9999)))
        } else if prop_lower.contains("address") || prop_lower.contains("street") {
            let streets = ["123 Main St", "456 Oak Ave", "789 Pine Rd", "321 Elm St"];
            Value::String(streets[thread_rng().random_range(0..streets.len())].to_string())
        } else if prop_lower.contains("city") {
            let cities = ["New York", "London", "Tokyo", "Paris", "Sydney"];
            Value::String(cities[thread_rng().random_range(0..cities.len())].to_string())
        } else if prop_lower.contains("country") {
            let countries = ["USA", "UK", "Japan", "France", "Australia"];
            Value::String(countries[thread_rng().random_range(0..countries.len())].to_string())
        } else if prop_lower.contains("company") || prop_lower.contains("organization") {
            let companies = ["Acme Corp", "Tech Solutions", "Global Inc", "Innovate Ltd"];
            Value::String(companies[thread_rng().random_range(0..companies.len())].to_string())
        } else if prop_lower.contains("url") || prop_lower.contains("website") {
            Value::String("https://example.com".to_string())
        } else if prop_lower.contains("age") {
            Value::Number((18 + thread_rng().random_range(0..60)).into())
        } else if prop_lower.contains("count") || prop_lower.contains("quantity") {
            Value::Number((1 + thread_rng().random_range(0..100)).into())
        } else if prop_lower.contains("price")
            || prop_lower.contains("amount")
            || prop_lower.contains("cost")
        {
            Value::Number(
                serde_json::Number::from_f64(
                    (thread_rng().random::<f64>() * 1000.0 * 100.0).round() / 100.0,
                )
                .unwrap(),
            )
        } else if prop_lower.contains("active")
            || prop_lower.contains("enabled")
            || prop_lower.contains("is_")
        {
            Value::Bool(thread_rng().random_bool(0.5))
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
                let new_arr: Vec<Value> = arr.iter().map(Self::expand_templates).collect();
                Value::Array(new_arr)
            }
            _ => value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::ReferenceOr;

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
