//! OpenAPI route generation from specifications
//!
//! This module provides functionality for generating Axum routes
//! from OpenAPI path definitions.

use crate::response_selection::{ResponseSelectionMode, ResponseSelector};
use crate::spec::OpenApiSpec;
use mockforge_foundation::ai_response::AiResponseConfig;
use mockforge_foundation::error::Result;
use mockforge_foundation::intelligent_behavior::Persona;
use openapiv3::{Operation, PathItem, ReferenceOr};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Extract path parameters from an OpenAPI path template
fn extract_path_parameters(path_template: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_param = false;
    let mut current_param = String::new();

    for ch in path_template.chars() {
        match ch {
            '{' => {
                in_param = true;
                current_param.clear();
            }
            '}' => {
                if in_param {
                    params.push(current_param.clone());
                    in_param = false;
                }
            }
            ch if in_param => {
                current_param.push(ch);
            }
            _ => {}
        }
    }

    params
}

/// OpenAPI route wrapper with additional metadata
#[derive(Debug, Clone)]
pub struct OpenApiRoute {
    /// The HTTP method
    pub method: String,
    /// The path pattern
    pub path: String,
    /// The OpenAPI operation
    pub operation: Operation,
    /// Route-specific metadata
    pub metadata: BTreeMap<String, String>,
    /// Path parameters extracted from the path
    pub parameters: Vec<String>,
    /// Reference to the OpenAPI spec for response generation
    pub spec: Arc<OpenApiSpec>,
    /// AI response configuration (parsed from x-mockforge-ai extension)
    pub ai_config: Option<AiResponseConfig>,
    /// Response selection mode (parsed from x-mockforge-response-selection extension)
    pub response_selection_mode: ResponseSelectionMode,
    /// Response selector for sequential/random modes (shared across requests)
    pub response_selector: Arc<ResponseSelector>,
    /// Active persona for consistent data generation (optional)
    pub persona: Option<Arc<Persona>>,
}

impl OpenApiRoute {
    /// Create a new OpenApiRoute
    pub fn new(method: String, path: String, operation: Operation, spec: Arc<OpenApiSpec>) -> Self {
        Self::new_with_persona(method, path, operation, spec, None)
    }

    /// Create a new OpenApiRoute with persona
    pub fn new_with_persona(
        method: String,
        path: String,
        operation: Operation,
        spec: Arc<OpenApiSpec>,
        persona: Option<Arc<Persona>>,
    ) -> Self {
        let parameters = extract_path_parameters(&path);

        // Parse AI configuration from x-mockforge-ai vendor extension
        let ai_config = Self::parse_ai_config(&operation);

        // Parse response selection mode from x-mockforge-response-selection extension
        let response_selection_mode = Self::parse_response_selection_mode(&operation);
        let response_selector = Arc::new(ResponseSelector::new(response_selection_mode));

        Self {
            method,
            path,
            operation,
            metadata: BTreeMap::new(),
            parameters,
            spec,
            ai_config,
            response_selection_mode,
            response_selector,
            persona,
        }
    }

    /// Parse AI configuration from OpenAPI operation's vendor extensions
    fn parse_ai_config(operation: &Operation) -> Option<AiResponseConfig> {
        // Check for x-mockforge-ai extension
        if let Some(ai_config_value) = operation.extensions.get("x-mockforge-ai") {
            // Try to deserialize the AI config from the extension value
            match serde_json::from_value::<AiResponseConfig>(ai_config_value.clone()) {
                Ok(config) => {
                    if config.is_active() {
                        tracing::debug!(
                            "Parsed AI config for operation {}: mode={:?}, prompt={:?}",
                            operation.operation_id.as_deref().unwrap_or("unknown"),
                            config.mode,
                            config.prompt
                        );
                        return Some(config);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse x-mockforge-ai extension for operation {}: {}",
                        operation.operation_id.as_deref().unwrap_or("unknown"),
                        e
                    );
                }
            }
        }
        None
    }

    /// Parse response selection mode from OpenAPI operation's vendor extensions
    fn parse_response_selection_mode(operation: &Operation) -> ResponseSelectionMode {
        // Check for environment variable override (per-operation or global)
        let op_id = operation.operation_id.as_deref().unwrap_or("unknown");

        // Try operation-specific env var first: MOCKFORGE_RESPONSE_SELECTION_<OPERATION_ID>
        if let Ok(op_env_var) = std::env::var(format!(
            "MOCKFORGE_RESPONSE_SELECTION_{}",
            op_id.to_uppercase().replace('-', "_")
        )) {
            if let Some(mode) = ResponseSelectionMode::from_str(&op_env_var) {
                tracing::debug!(
                    "Using response selection mode from env var for operation {}: {:?}",
                    op_id,
                    mode
                );
                return mode;
            }
        }

        // Check global env var: MOCKFORGE_RESPONSE_SELECTION_MODE
        if let Ok(global_mode_str) = std::env::var("MOCKFORGE_RESPONSE_SELECTION_MODE") {
            if let Some(mode) = ResponseSelectionMode::from_str(&global_mode_str) {
                tracing::debug!("Using global response selection mode from env var: {:?}", mode);
                return mode;
            }
        }

        // Check for x-mockforge-response-selection extension
        if let Some(selection_value) = operation.extensions.get("x-mockforge-response-selection") {
            // Try to parse as string first
            if let Some(mode_str) = selection_value.as_str() {
                if let Some(mode) = ResponseSelectionMode::from_str(mode_str) {
                    tracing::debug!(
                        "Parsed response selection mode for operation {}: {:?}",
                        op_id,
                        mode
                    );
                    return mode;
                }
            }
            // Try to parse as object with mode field
            if let Some(obj) = selection_value.as_object() {
                if let Some(mode_str) = obj.get("mode").and_then(|v| v.as_str()) {
                    if let Some(mode) = ResponseSelectionMode::from_str(mode_str) {
                        tracing::debug!(
                            "Parsed response selection mode for operation {}: {:?}",
                            op_id,
                            mode
                        );
                        return mode;
                    }
                }
            }
            tracing::warn!(
                "Failed to parse x-mockforge-response-selection extension for operation {}",
                op_id
            );
        }
        // Default to First mode
        ResponseSelectionMode::First
    }

    /// Create an OpenApiRoute from an operation
    pub fn from_operation(
        method: &str,
        path: String,
        operation: &Operation,
        spec: Arc<OpenApiSpec>,
    ) -> Self {
        Self::from_operation_with_persona(method, path, operation, spec, None)
    }

    /// Create a new OpenApiRoute from an operation with optional persona
    pub fn from_operation_with_persona(
        method: &str,
        path: String,
        operation: &Operation,
        spec: Arc<OpenApiSpec>,
        persona: Option<Arc<Persona>>,
    ) -> Self {
        Self::new_with_persona(method.to_string(), path, operation.clone(), spec, persona)
    }

    /// Convert OpenAPI path to Axum-compatible path format
    pub fn axum_path(&self) -> String {
        // Strip query string if present (some non-standard OpenAPI specs embed query params in path)
        // Axum v0.7+ uses {param} format, same as OpenAPI
        let path = self.path.split('?').next().unwrap_or(&self.path);

        // Handle empty function call parens: functionName() → functionName
        if path.contains("()") {
            let path = path.replace("()", "");
            return path;
        }

        // Handle OData function call syntax: functionName(key='{param}',key2={param2})
        // Also handles Microsoft Graph style: functionName(key='{param}') where quotes wrap braces
        // Convert to: functionName/{param}/{param2}
        // This prevents Axum from panicking on multiple params per segment or invalid chars
        if path.contains('(') && path.contains('=') {
            let mut result = String::with_capacity(path.len());
            let mut chars = path.chars().peekable();

            while let Some(ch) = chars.next() {
                if ch == '(' {
                    // Extract params from inside parentheses
                    let mut paren_content = String::new();
                    for c in chars.by_ref() {
                        if c == ')' {
                            break;
                        }
                        paren_content.push(c);
                    }
                    // Parse key='{value}' or key={value} pairs
                    for part in paren_content.split(',') {
                        if let Some((_key, value)) = part.split_once('=') {
                            let param = value.trim_matches(|c| c == '\'' || c == '"');
                            result.push('/');
                            result.push_str(param);
                        }
                    }
                } else {
                    result.push(ch);
                }
            }
            return result;
        }

        path.to_string()
    }

    /// Returns true if this route's path can be registered with Axum's router.
    ///
    /// Paths that contain characters Axum can't handle (e.g., unmatched braces,
    /// multiple params per segment after conversion) are considered invalid.
    pub fn is_valid_axum_path(&self) -> bool {
        let path = self.axum_path();
        // If parentheses survived conversion, the path is invalid for Axum
        if path.contains('(') || path.contains(')') {
            return false;
        }
        // Each segment may contain at most one `{param}` capture
        for segment in path.split('/') {
            let brace_count = segment.matches('{').count();
            if brace_count > 1 {
                return false;
            }
            // A segment with a param must be ONLY the param (e.g. `{id}` not `prefix{id}suffix`)
            // unless it's a wildcard. Axum allows `{*rest}` as a catch-all.
            if brace_count == 1
                && segment
                    != format!(
                        "{{{}}}",
                        segment
                            .trim_matches(|c: char| c != '{' && c != '}')
                            .trim_matches(|c| c == '{' || c == '}')
                    )
            {
                // Segment has a param mixed with literal text — check if it's truly invalid
                // Axum 0.8 allows `{param}` as full segment only
                if !segment.starts_with('{') || !segment.ends_with('}') {
                    return false;
                }
            }
        }
        true
    }

    /// Add metadata to the route
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate a mock response with status code for this route (async version with AI support)
    ///
    /// This method checks if AI response generation is configured and uses it if available,
    /// otherwise falls back to standard OpenAPI response generation.
    ///
    /// # Arguments
    /// * `context` - The request context for AI prompt expansion
    /// * `ai_generator` - Optional AI generator implementation for actual LLM calls
    pub async fn mock_response_with_status_async(
        &self,
        context: &mockforge_foundation::ai_response::RequestContext,
        ai_generator: Option<&dyn crate::response::AiGenerator>,
    ) -> (u16, serde_json::Value) {
        use crate::response::ResponseGenerator;

        // Find the first available status code from the OpenAPI spec
        let status_code = self.find_first_available_status_code();

        // Check if AI response generation is configured
        if let Some(ai_config) = &self.ai_config {
            if ai_config.is_active() {
                tracing::info!(
                    "Using AI-assisted response generation for {} {}",
                    self.method,
                    self.path
                );

                match ResponseGenerator::generate_ai_response(ai_config, context, ai_generator)
                    .await
                {
                    Ok(response_body) => {
                        tracing::debug!(
                            "AI response generated successfully for {} {}: {:?}",
                            self.method,
                            self.path,
                            response_body
                        );
                        return (status_code, response_body);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "AI response generation failed for {} {}: {}, falling back to standard generation",
                            self.method,
                            self.path,
                            e
                        );
                        // Continue to standard generation on error
                    }
                }
            }
        }

        // Standard OpenAPI-based response generation
        let expand_tokens = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // Use response selection mode for multiple examples
        let mode = Some(self.response_selection_mode);
        let selector = Some(self.response_selector.as_ref());

        // Get persona reference for response generation
        let persona_ref = self.persona.as_deref();

        match ResponseGenerator::generate_response_with_expansion_and_mode_and_persona(
            &self.spec,
            &self.operation,
            status_code,
            Some("application/json"),
            expand_tokens,
            mode,
            selector,
            persona_ref,
        ) {
            Ok(response_body) => {
                tracing::debug!(
                    "ResponseGenerator succeeded for {} {} with status {}: {:?}",
                    self.method,
                    self.path,
                    status_code,
                    response_body
                );
                (status_code, response_body)
            }
            Err(e) => {
                tracing::debug!(
                    "ResponseGenerator failed for {} {}: {}, using fallback",
                    self.method,
                    self.path,
                    e
                );
                // Fallback to simple mock response if schema-based generation fails
                let response_body = serde_json::json!({
                    "message": format!("Mock response for {} {}", self.method, self.path),
                    "operation_id": self.operation.operation_id,
                    "status": status_code
                });
                (status_code, response_body)
            }
        }
    }

    /// Generate a mock response with status code for this route (synchronous version)
    ///
    /// Note: This method does not support AI-assisted response generation.
    /// Use `mock_response_with_status_async` for AI features.
    pub fn mock_response_with_status(&self) -> (u16, serde_json::Value) {
        self.mock_response_with_status_and_scenario(None)
    }

    /// Generate a mock response with status code and scenario selection
    ///
    /// # Arguments
    /// * `scenario` - Optional scenario name to select from the OpenAPI examples
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Select the "error" scenario from examples
    /// let (status, body) = route.mock_response_with_status_and_scenario(Some("error"));
    /// ```
    pub fn mock_response_with_status_and_scenario(
        &self,
        scenario: Option<&str>,
    ) -> (u16, serde_json::Value) {
        self.mock_response_with_status_and_scenario_and_override(scenario, None)
    }

    /// Generate a mock response with status code, scenario, and optional status override
    ///
    /// # Arguments
    /// * `scenario` - Optional scenario name to select from the OpenAPI examples
    /// * `status_override` - Optional HTTP status code to use instead of the default
    pub fn mock_response_with_status_and_scenario_and_override(
        &self,
        scenario: Option<&str>,
        status_override: Option<u16>,
    ) -> (u16, serde_json::Value) {
        let (status, body, _) =
            self.mock_response_with_status_and_scenario_and_trace(scenario, status_override);
        (status, body)
    }

    /// Generate a mock response with status code, scenario selection, and trace collection
    ///
    /// Returns a tuple of (status_code, response_body, trace_data)
    pub fn mock_response_with_status_and_scenario_and_trace(
        &self,
        scenario: Option<&str>,
        status_override: Option<u16>,
    ) -> (
        u16,
        serde_json::Value,
        mockforge_foundation::response_generation_trace::ResponseGenerationTrace,
    ) {
        use crate::response_trace;
        use mockforge_foundation::response_generation_trace::ResponseGenerationTrace;

        // Use status override if the spec has a response for that code, otherwise default
        let status_code = status_override
            .filter(|code| self.has_response_for_status(*code))
            .unwrap_or_else(|| self.find_first_available_status_code());

        // Check if token expansion should be enabled
        let expand_tokens = std::env::var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // Use response selection mode for multiple examples
        let mode = Some(self.response_selection_mode);
        let selector = Some(self.response_selector.as_ref());

        // Try to generate with trace collection
        match response_trace::generate_response_with_trace(
            &self.spec,
            &self.operation,
            status_code,
            Some("application/json"),
            expand_tokens,
            scenario,
            mode,
            selector,
            None, // No persona in basic route
        ) {
            Ok((response_body, trace)) => {
                tracing::debug!(
                    "ResponseGenerator succeeded for {} {} with status {} and scenario {:?}: {:?}",
                    self.method,
                    self.path,
                    status_code,
                    scenario,
                    response_body
                );
                (status_code, response_body, trace)
            }
            Err(e) => {
                tracing::debug!(
                    "ResponseGenerator failed for {} {}: {}, using fallback",
                    self.method,
                    self.path,
                    e
                );
                // Fallback to simple mock response if schema-based generation fails
                let response_body = serde_json::json!({
                    "message": format!("Mock response for {} {}", self.method, self.path),
                    "operation_id": self.operation.operation_id,
                    "status": status_code
                });
                // Create a minimal trace for fallback
                let mut trace = ResponseGenerationTrace::new();
                trace.set_final_payload(response_body.clone());
                trace.add_metadata("fallback".to_string(), serde_json::json!(true));
                trace.add_metadata("error".to_string(), serde_json::json!(e.to_string()));
                (status_code, response_body, trace)
            }
        }
    }

    /// Check if the operation declares a response for the given HTTP status code
    pub fn has_response_for_status(&self, code: u16) -> bool {
        self.operation
            .responses
            .responses
            .iter()
            .any(|(status, _)| matches!(status, openapiv3::StatusCode::Code(c) if *c == code))
    }

    /// Find the first available status code from the OpenAPI operation responses
    pub fn find_first_available_status_code(&self) -> u16 {
        // Look for the first available status code in the responses
        for (status, _) in &self.operation.responses.responses {
            match status {
                openapiv3::StatusCode::Code(code) => {
                    return *code;
                }
                openapiv3::StatusCode::Range(range) => {
                    // For ranges, use the appropriate status code
                    match range {
                        2 => return 200, // 2XX range
                        3 => return 300, // 3XX range
                        4 => return 400, // 4XX range
                        5 => return 500, // 5XX range
                        _ => continue,   // Skip unknown ranges
                    }
                }
            }
        }

        // If no specific status codes found, check for default
        if self.operation.responses.default.is_some() {
            return 200; // Default to 200 for default responses
        }

        // Fallback to 200 if nothing else is available
        200
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

/// Route generation utilities
pub struct RouteGenerator;

impl RouteGenerator {
    /// Generate routes from an OpenAPI path item
    pub fn generate_routes_from_path(
        path: &str,
        path_item: &ReferenceOr<PathItem>,
        spec: &Arc<OpenApiSpec>,
    ) -> Result<Vec<OpenApiRoute>> {
        Self::generate_routes_from_path_with_persona(path, path_item, spec, None)
    }

    /// Generate routes from an OpenAPI path item with optional persona
    pub fn generate_routes_from_path_with_persona(
        path: &str,
        path_item: &ReferenceOr<PathItem>,
        spec: &Arc<OpenApiSpec>,
        persona: Option<Arc<Persona>>,
    ) -> Result<Vec<OpenApiRoute>> {
        let mut routes = Vec::new();

        if let Some(item) = path_item.as_item() {
            // Generate route for each HTTP method
            if let Some(op) = &item.get {
                routes.push(OpenApiRoute::new_with_persona(
                    "GET".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.post {
                routes.push(OpenApiRoute::new_with_persona(
                    "POST".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.put {
                routes.push(OpenApiRoute::new_with_persona(
                    "PUT".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.delete {
                routes.push(OpenApiRoute::new_with_persona(
                    "DELETE".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.patch {
                routes.push(OpenApiRoute::new_with_persona(
                    "PATCH".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.head {
                routes.push(OpenApiRoute::new_with_persona(
                    "HEAD".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.options {
                routes.push(OpenApiRoute::new_with_persona(
                    "OPTIONS".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
            if let Some(op) = &item.trace {
                routes.push(OpenApiRoute::new_with_persona(
                    "TRACE".to_string(),
                    path.to_string(),
                    op.clone(),
                    spec.clone(),
                    persona.clone(),
                ));
            }
        }

        Ok(routes)
    }
}
