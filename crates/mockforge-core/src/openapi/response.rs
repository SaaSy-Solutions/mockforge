//! OpenAPI response generation and mocking
//!
//! This module provides functionality for generating mock responses
//! based on OpenAPI specifications.

use crate::{
    ai_response::{expand_prompt_template, AiResponseConfig, RequestContext},
    OpenApiSpec, Result,
};
use crate::intelligent_behavior::config::Persona;
use async_trait::async_trait;
use chrono;
use openapiv3::{Operation, ReferenceOr, Response, Responses, Schema};
use rand::{rng, Rng};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
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

        let expanded_prompt = expand_prompt_template(prompt_template, context);

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

        match response {
            Some(response_ref) => {
                match response_ref {
                    ReferenceOr::Item(response) => {
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
                        // Resolve the reference
                        if let Some(resolved_response) = spec.get_response(reference) {
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
        if !media_type.examples.is_empty() {
            use crate::openapi::response_selection::{ResponseSelectionMode, ResponseSelector};

            // If a scenario is specified, try to find it first
            if let Some(scenario_name) = scenario {
                if let Some(example_ref) = media_type.examples.get(scenario_name) {
                    tracing::debug!("Using scenario '{}' from examples map", scenario_name);
                    return Self::extract_example_value(spec, example_ref, expand_tokens);
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
            } else if mode == ResponseSelectionMode::Scenario && scenario.is_some() {
                // Scenario mode was requested but scenario not found, fall through to selection mode
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
                        return Self::extract_example_value(spec, example_ref, expand_tokens);
                    }
                }
            }

            // Fall back to first example if selection failed
            if let Some((example_name, example_ref)) = media_type.examples.iter().next() {
                tracing::debug!(
                    "Using first example '{}' from examples map as fallback",
                    example_name
                );
                return Self::extract_example_value(spec, example_ref, expand_tokens);
            }
        }

        // Fall back to schema-based generation
        // Note: Persona is not available at this level, will be None for now
        // This can be enhanced later to pass persona through the call chain
        if let Some(schema_ref) = &media_type.schema {
            Ok(Self::generate_example_from_schema_ref(spec, schema_ref, None))
        } else {
            Ok(Value::Object(serde_json::Map::new()))
        }
    }

    /// Extract value from an example reference
    fn extract_example_value(
        spec: &OpenApiSpec,
        example_ref: &ReferenceOr<openapiv3::Example>,
        expand_tokens: bool,
    ) -> Result<Value> {
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
        Ok(Value::Object(serde_json::Map::new()))
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
    fn generate_example_from_schema(spec: &OpenApiSpec, schema: &Schema, persona: Option<&Persona>) -> Value {
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
                let has_items = obj.properties.iter()
                    .any(|(name, _)| name.to_lowercase() == "items");

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
                            ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
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
                            total, page, limit
                        );
                    } else {
                        // Phase 3: If no total found in schema, try to infer from parent entity
                        // Look for "items" array to determine child entity name
                        if obj.properties.contains_key("items") {
                            // Try to infer parent/child relationship from schema names
                            // This is a heuristic: if we're generating a paginated response,
                            // check if we can find a parent entity schema with a count field
                            if let Some(inferred_total) = Self::try_infer_total_from_context(spec, obj) {
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
                                    let count_keys = ["hive_count", "apiary_count", "item_count", "total_count"];
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
                            // Check for property-level example first
                            if let Some(prop_example) = prop_schema.schema_data.example.as_ref() {
                                tracing::debug!(
                                    "Using example for property '{}': {:?}",
                                    prop_name,
                                    prop_example
                                );
                                prop_example.clone()
                            } else if is_items_array {
                                // Generate array with count based on pagination metadata
                                Self::generate_array_with_count(
                                    spec,
                                    prop_schema.as_ref(),
                                    pagination_metadata.unwrap(),
                                    persona,
                                )
                            } else {
                                Self::generate_example_from_schema(spec, prop_schema.as_ref(), persona)
                            }
                        }
                        ReferenceOr::Reference { reference } => {
                            // Try to resolve reference and check for example
                            if let Some(resolved_schema) = spec.get_schema(reference) {
                                if let Some(ref_example) =
                                    resolved_schema.schema.schema_data.example.as_ref()
                                {
                                    tracing::debug!(
                                        "Using example from referenced schema '{}': {:?}",
                                        reference,
                                        ref_example
                                    );
                                    ref_example.clone()
                                } else if is_items_array {
                                    // Generate array with count based on pagination metadata
                                    Self::generate_array_with_count(
                                        spec,
                                        &resolved_schema.schema,
                                        pagination_metadata.unwrap(),
                                        persona,
                                    )
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
                                Self::generate_example_from_schema(spec, item_schema.as_ref(), persona)
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
    fn extract_numeric_value_from_schema(
        schema_ref: &ReferenceOr<Schema>,
    ) -> Option<u64> {
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

        tracing::debug!(
            "Generating array with count={} (total={}, limit={})",
            count,
            total,
            limit
        );

        // Check if array schema has an example with items
        if let Some(example) = array_schema.schema_data.example.as_ref() {
            if let Some(example_array) = example.as_array() {
                if !example_array.is_empty() {
                    // Use first example item as template
                    let template_item = &example_array[0];
                    let items: Vec<Value> = (0..count)
                        .map(|i| {
                            // Clone template and add variation (e.g., unique IDs)
                            let mut item = template_item.clone();
                            if let Some(obj) = item.as_object_mut() {
                                // Add variation to IDs to make items unique
                                if let Some(id_val) = obj.get_mut("id") {
                                    if let Some(id_str) = id_val.as_str() {
                                        *id_val = Value::String(format!("{}_{}", id_str, i + 1));
                                    }
                                }
                            }
                            item
                        })
                        .collect();
                    return Value::Array(items);
                }
            }
        }

        // Generate items from schema
        if let openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) = &array_schema.schema_kind {
            if let Some(item_schema) = &arr.items {
                let items: Vec<Value> = match item_schema {
                    ReferenceOr::Item(item_schema) => {
                        (0..count)
                            .map(|i| {
                                let mut item = Self::generate_example_from_schema(spec, item_schema.as_ref(), persona);
                                // Add variation to make items unique
                                if let Some(obj) = item.as_object_mut() {
                                    // Update ID fields to be unique
                                    if let Some(id_val) = obj.get_mut("id") {
                                        if let Some(id_str) = id_val.as_str() {
                                            *id_val = Value::String(format!("{}_{}", id_str, i + 1));
                                        }
                                    }
                                }
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
                                    if let Some(obj) = item.as_object_mut() {
                                        if let Some(id_val) = obj.get_mut("id") {
                                            if let Some(id_str) = id_val.as_str() {
                                                *id_val = Value::String(format!("{}_{}", id_str, i + 1));
                                            }
                                        }
                                    }
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
        Value::Array(
            (0..count)
                .map(|i| Value::String(format!("item_{}", i + 1)))
                .collect(),
        )
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
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) = &schema.schema_kind {
                            // Look for count fields that might match
                            for (prop_name, prop_schema) in &obj.properties {
                                let prop_lower = prop_name.to_lowercase();
                                if prop_lower.ends_with("_count") {
                                    // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema>
                                    let schema_ref: ReferenceOr<Schema> = match prop_schema {
                                        ReferenceOr::Item(boxed) => ReferenceOr::Item(boxed.as_ref().clone()),
                                        ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
                                    };
                                    // Found a count field, try to extract its value
                                    if let Some(count) = Self::extract_numeric_value_from_schema(&schema_ref) {
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
        let parent_schema_name = format!("{}", parent_entity_name);
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
                        if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) = &schema.schema_kind {
                            for (prop_name, prop_schema) in &obj.properties {
                                if prop_name.to_lowercase() == count_field_name.to_lowercase() {
                                    // Convert ReferenceOr<Box<Schema>> to ReferenceOr<Schema>
                                    let schema_ref: ReferenceOr<Schema> = match prop_schema {
                                        ReferenceOr::Item(boxed) => ReferenceOr::Item(boxed.as_ref().clone()),
                                        ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
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
