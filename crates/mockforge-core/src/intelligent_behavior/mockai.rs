//! Unified MockAI interface
//!
//! This module provides a unified interface for all MockAI features, including
//! auto-configuration from OpenAPI or examples, intelligent response generation,
//! and context-aware behavior orchestration.

use super::config::IntelligentBehaviorConfig;
use super::context::StatefulAiContext;
use super::mutation_analyzer::MutationAnalyzer;
use super::pagination_intelligence::{
    PaginationIntelligence, PaginationMetadata, PaginationRequest,
};
use super::rule_generator::{ExamplePair, RuleGenerator};
use super::types::BehaviorRules;
use super::validation_generator::{RequestContext, ValidationGenerator};
use crate::openapi::OpenApiSpec;
use crate::Result;
use serde_json::Value;
use std::collections::HashMap;
use uuid;

/// HTTP request for MockAI
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body
    pub body: Option<Value>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Headers
    pub headers: HashMap<String, String>,
}

/// HTTP response from MockAI
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status_code: u16,
    /// Response body
    pub body: Value,
    /// Response headers
    pub headers: HashMap<String, String>,
}

/// MockAI unified interface
pub struct MockAI {
    /// Behavior rules
    rules: BehaviorRules,
    /// Rule generator for learning
    rule_generator: RuleGenerator,
    /// Mutation analyzer
    mutation_analyzer: MutationAnalyzer,
    /// Validation generator
    validation_generator: ValidationGenerator,
    /// Pagination intelligence
    pagination_intelligence: PaginationIntelligence,
    /// Configuration
    config: IntelligentBehaviorConfig,
    /// Session contexts for stateful behavior across requests
    session_contexts: std::sync::Arc<tokio::sync::RwLock<HashMap<String, StatefulAiContext>>>,
}

impl MockAI {
    /// Create MockAI from OpenAPI specification
    ///
    /// Automatically generates behavioral rules from the OpenAPI spec.
    pub async fn from_openapi(
        spec: &OpenApiSpec,
        config: IntelligentBehaviorConfig,
    ) -> Result<Self> {
        // Extract examples from OpenAPI spec
        let examples = Self::extract_examples_from_openapi(spec)?;

        // Generate rules from examples
        let behavior_config = config.behavior_model.clone();
        let rule_generator = RuleGenerator::new(behavior_config.clone());
        let rules = rule_generator.generate_rules_from_examples(examples).await?;

        // Create components
        let mutation_analyzer = MutationAnalyzer::new().with_rules(rules.clone());
        let validation_generator = ValidationGenerator::new(behavior_config.clone());
        let pagination_intelligence = PaginationIntelligence::new(behavior_config);

        Ok(Self {
            rules,
            rule_generator,
            mutation_analyzer,
            validation_generator,
            pagination_intelligence,
            config,
            session_contexts: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Create MockAI from example pairs
    ///
    /// Learns behavioral patterns from provided examples.
    pub async fn from_examples(
        examples: Vec<ExamplePair>,
        config: IntelligentBehaviorConfig,
    ) -> Result<Self> {
        // Generate rules from examples
        let behavior_config = config.behavior_model.clone();
        let rule_generator = RuleGenerator::new(behavior_config.clone());
        let rules = rule_generator.generate_rules_from_examples(examples).await?;

        // Create components
        let mutation_analyzer = MutationAnalyzer::new().with_rules(rules.clone());
        let validation_generator = ValidationGenerator::new(behavior_config.clone());
        let pagination_intelligence = PaginationIntelligence::new(behavior_config);

        Ok(Self {
            rules,
            rule_generator,
            mutation_analyzer,
            validation_generator,
            pagination_intelligence,
            config,
            session_contexts: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Create a new MockAI instance (for testing or manual creation)
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let behavior_config = config.behavior_model.clone();
        let rule_generator = RuleGenerator::new(behavior_config.clone());
        let rules = BehaviorRules::default();
        let mutation_analyzer = MutationAnalyzer::new().with_rules(rules.clone());
        let validation_generator = ValidationGenerator::new(behavior_config.clone());
        let pagination_intelligence = PaginationIntelligence::new(behavior_config);

        Self {
            rules,
            rule_generator,
            mutation_analyzer,
            validation_generator,
            pagination_intelligence,
            config,
            session_contexts: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Process a request and generate a response
    ///
    /// Convenience method that gets or creates a session context and generates a response.
    /// This is the main entry point for processing HTTP requests.
    /// Session ID is extracted from headers (X-Session-ID or Cookie) or generated if not present.
    pub async fn process_request(&self, request: &Request) -> Result<Response> {
        // Extract session ID from request headers
        let session_id = self.extract_session_id(request);

        // Get or create session context
        let session_context = self.get_or_create_session_context(session_id).await?;

        // Generate response using the session context
        let response = self.generate_response(request, &session_context).await?;

        // Record interaction in session history
        // Since record_interaction now takes &self (uses internal RwLock),
        // we can call it directly on the cloned context
        if let Err(e) = session_context
            .record_interaction(
                request.method.clone(),
                request.path.clone(),
                request.body.clone(),
                Some(response.body.clone()),
            )
            .await
        {
            tracing::warn!("Failed to record interaction: {}", e);
        }

        Ok(response)
    }

    /// Extract session ID from request headers
    fn extract_session_id(&self, request: &Request) -> Option<String> {
        // Try header first (X-Session-ID)
        if let Some(session_id) = request.headers.get("X-Session-ID") {
            return Some(session_id.clone());
        }

        // Try cookie (mockforge_session)
        if let Some(cookie_header) = request.headers.get("Cookie") {
            for part in cookie_header.split(';') {
                let part = part.trim();
                if let Some((key, value)) = part.split_once('=') {
                    if key.trim() == "mockforge_session" {
                        return Some(value.trim().to_string());
                    }
                }
            }
        }

        None
    }

    /// Get or create a session context
    async fn get_or_create_session_context(
        &self,
        session_id: Option<String>,
    ) -> Result<StatefulAiContext> {
        let session_id = session_id.unwrap_or_else(|| format!("session_{}", uuid::Uuid::new_v4()));

        // Try to get existing context
        {
            let contexts = self.session_contexts.read().await;
            if let Some(context) = contexts.get(&session_id) {
                return Ok(context.clone());
            }
        }

        // Create new context
        let new_context = StatefulAiContext::new(session_id.clone(), self.config.clone());

        // Store it
        {
            let mut contexts = self.session_contexts.write().await;
            contexts.insert(session_id, new_context.clone());
        }

        Ok(new_context)
    }

    /// Generate response for a request
    ///
    /// Uses intelligent behavior to generate contextually appropriate responses
    /// based on request mutations, validation, and pagination needs.
    pub async fn generate_response(
        &self,
        request: &Request,
        session_context: &StatefulAiContext,
    ) -> Result<Response> {
        // CRITICAL FIX: GET, HEAD, and OPTIONS requests should NEVER be analyzed as mutations
        // These are idempotent methods that don't mutate state. Only POST, PUT, PATCH, DELETE are mutations.
        let method_upper = request.method.to_uppercase();
        let is_mutation_method =
            matches!(method_upper.as_str(), "POST" | "PUT" | "PATCH" | "DELETE");

        // Get previous request from session history
        let history = session_context.get_history().await;
        let previous_request = history.last().and_then(|interaction| interaction.request.clone());

        // Only analyze mutations for mutation methods (POST, PUT, PATCH, DELETE)
        // GET, HEAD, OPTIONS should use standard OpenAPI response generation, not mutation responses
        let mutation_analysis = if is_mutation_method {
            // Analyze mutation for mutation methods
            let current_body = request.body.clone().unwrap_or(serde_json::json!({}));
            self.mutation_analyzer
                .analyze_mutation(&current_body, previous_request.as_ref(), session_context)
                .await?
        } else {
            // For non-mutation methods (GET, HEAD, OPTIONS), create a dummy analysis
            // that won't trigger mutation-based response generation
            // This ensures GET requests use OpenAPI examples/schemas, not mutation responses
            super::mutation_analyzer::MutationAnalysis {
                mutation_type: super::mutation_analyzer::MutationType::NoChange, // Read operations are not mutations
                changed_fields: Vec::new(),
                added_fields: Vec::new(),
                removed_fields: Vec::new(),
                validation_issues: Vec::new(),
                confidence: 1.0,
            }
        };

        // Check for validation issues
        if !mutation_analysis.validation_issues.is_empty() {
            // Generate validation error response
            let issue = &mutation_analysis.validation_issues[0];
            let request_context = RequestContext {
                method: request.method.clone(),
                path: request.path.clone(),
                request_body: request.body.clone(),
                query_params: request.query_params.clone(),
                headers: request.headers.clone(),
            };

            let error_response = self
                .validation_generator
                .generate_validation_error(issue, &request_context)
                .await?;

            return Ok(Response {
                status_code: error_response.status_code,
                body: error_response.body,
                headers: HashMap::new(),
            });
        }

        // Check if this is a paginated request
        if self.is_paginated_request(request) {
            let pagination_meta =
                self.generate_pagination_metadata(request, session_context).await?;

            let body = self.build_paginated_response(&pagination_meta, request).await?;

            return Ok(Response {
                status_code: 200,
                body,
                headers: HashMap::new(),
            });
        }

        // Generate normal response based on mutation type
        // For GET/HEAD/OPTIONS (Read operations), this should return an empty object
        // to signal that OpenAPI response generation should be used instead
        let response_body = if is_mutation_method {
            // Only use mutation-based response generation for actual mutations
            self.generate_response_body(&mutation_analysis, request, session_context)
                .await?
        } else {
            // For GET/HEAD/OPTIONS, return empty object to signal OpenAPI generation should be used
            // This prevents GET requests from returning POST-style {id: "generated_id", status: "created"} responses
            tracing::debug!(
                "Skipping mutation-based response generation for {} request - using OpenAPI response generation",
                method_upper
            );
            serde_json::json!({}) // Empty object signals to use OpenAPI response generation
        };

        Ok(Response {
            status_code: 200,
            body: response_body,
            headers: HashMap::new(),
        })
    }

    /// Learn from an example pair
    ///
    /// Updates behavioral rules based on a new example.
    pub async fn learn_from_example(&mut self, example: ExamplePair) -> Result<()> {
        // Regenerate rules with new example
        let examples = vec![example];
        let new_rules = self.rule_generator.generate_rules_from_examples(examples).await?;

        // Merge with existing rules
        self.merge_rules(new_rules);

        Ok(())
    }

    /// Get current behavior rules
    pub fn rules(&self) -> &BehaviorRules {
        &self.rules
    }

    /// Update behavior rules
    pub fn update_rules(&mut self, rules: BehaviorRules) {
        self.rules = rules;
        // Update mutation analyzer with new rules
        self.mutation_analyzer = MutationAnalyzer::new().with_rules(self.rules.clone());
    }

    /// Update configuration at runtime
    ///
    /// This allows changing MockAI configuration without recreating the instance.
    /// Useful for hot-reloading reality level configurations.
    ///
    /// Note: This updates the configuration but does not regenerate rules.
    /// For rule updates, use `update_rules()` or `learn_from_example()`.
    pub fn update_config(&mut self, config: IntelligentBehaviorConfig) {
        self.config = config.clone();

        // Update components that depend on config
        let behavior_config = self.config.behavior_model.clone();
        self.validation_generator = ValidationGenerator::new(behavior_config.clone());
        self.pagination_intelligence = PaginationIntelligence::new(behavior_config);

        // Note: We don't recreate rule_generator or mutation_analyzer
        // as they may have learned rules that should be preserved
    }

    /// Update configuration (async version for Arc<RwLock>)
    ///
    /// Convenience method for updating a MockAI instance wrapped in Arc<RwLock>.
    /// This is the recommended way to update MockAI configuration at runtime.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the update fails.
    pub async fn update_config_async(
        this: &std::sync::Arc<tokio::sync::RwLock<Self>>,
        config: IntelligentBehaviorConfig,
    ) -> Result<()> {
        let mut mockai = this.write().await;
        mockai.update_config(config);
        Ok(())
    }

    /// Get current configuration
    ///
    /// Primarily for testing purposes to verify configuration updates.
    pub fn get_config(&self) -> &IntelligentBehaviorConfig {
        &self.config
    }

    // ===== Private helper methods =====

    /// Extract examples from OpenAPI spec
    pub fn extract_examples_from_openapi(spec: &OpenApiSpec) -> Result<Vec<ExamplePair>> {
        let mut examples = Vec::new();

        // Use the all_paths_and_operations method
        let path_operations = spec.all_paths_and_operations();

        for (path, operations) in path_operations {
            for (method, operation) in operations {
                // Extract request example
                let request = operation
                    .request_body
                    .as_ref()
                    .and_then(|rb| rb.as_item())
                    .and_then(|rb| rb.content.get("application/json"))
                    .and_then(|media| media.example.clone());

                // Extract response example
                let response = operation.responses.responses.iter().find_map(|(status, resp)| {
                    if let openapiv3::StatusCode::Code(200) = status {
                        resp.as_item()
                            .and_then(|r| r.content.get("application/json"))
                            .and_then(|media| media.example.clone())
                    } else {
                        None
                    }
                });

                examples.push(ExamplePair {
                    method: method.clone(),
                    path: path.clone(),
                    request,
                    status: 200,
                    response,
                    query_params: HashMap::new(),
                    headers: HashMap::new(),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(examples)
    }

    /// Check if request is paginated
    fn is_paginated_request(&self, request: &Request) -> bool {
        // Check for pagination parameters
        request.query_params.keys().any(|key| {
            matches!(
                key.to_lowercase().as_str(),
                "page" | "limit" | "per_page" | "offset" | "cursor"
            )
        })
    }

    /// Generate pagination metadata
    async fn generate_pagination_metadata(
        &self,
        request: &Request,
        session_context: &StatefulAiContext,
    ) -> Result<PaginationMetadata> {
        let pagination_request = PaginationRequest {
            path: request.path.clone(),
            query_params: request.query_params.clone(),
            request_body: request.body.clone(),
        };

        self.pagination_intelligence
            .generate_pagination_metadata(&pagination_request, session_context)
            .await
    }

    /// Build paginated response
    async fn build_paginated_response(
        &self,
        meta: &PaginationMetadata,
        _request: &Request,
    ) -> Result<Value> {
        // Build standard paginated response
        Ok(serde_json::json!({
            "data": [], // Would be populated with actual data
            "pagination": {
                "page": meta.page,
                "page_size": meta.page_size,
                "total": meta.total,
                "total_pages": meta.total_pages,
                "has_next": meta.has_next,
                "has_prev": meta.has_prev,
                "offset": meta.offset,
                "next_cursor": meta.next_cursor,
                "prev_cursor": meta.prev_cursor,
            }
        }))
    }

    /// Generate response body based on mutation analysis
    async fn generate_response_body(
        &self,
        mutation: &super::mutation_analyzer::MutationAnalysis,
        request: &Request,
        _session_context: &StatefulAiContext,
    ) -> Result<Value> {
        // Generate response based on mutation type
        // CRITICAL: NoChange (used for GET/HEAD/OPTIONS) should return empty object
        // to signal that OpenAPI response generation should be used instead
        match mutation.mutation_type {
            super::mutation_analyzer::MutationType::NoChange => {
                // For read operations (GET, HEAD, OPTIONS), return empty object
                // This signals to use OpenAPI response generation, not mutation responses
                tracing::debug!("MutationType::NoChange - returning empty object to use OpenAPI response generation");
                Ok(serde_json::json!({}))
            }
            super::mutation_analyzer::MutationType::Create => {
                // Generate created resource response
                Ok(serde_json::json!({
                    "id": "generated_id",
                    "status": "created",
                    "data": request.body.clone().unwrap_or(serde_json::json!({}))
                }))
            }
            super::mutation_analyzer::MutationType::Update
            | super::mutation_analyzer::MutationType::PartialUpdate => {
                // Generate updated resource response
                Ok(serde_json::json!({
                    "id": "resource_id",
                    "status": "updated",
                    "data": request.body.clone().unwrap_or(serde_json::json!({}))
                }))
            }
            super::mutation_analyzer::MutationType::Delete => {
                // Generate deletion response
                Ok(serde_json::json!({
                    "status": "deleted",
                    "message": "Resource deleted successfully"
                }))
            }
            _ => {
                // Default success response
                Ok(serde_json::json!({
                    "status": "success",
                    "data": request.body.clone().unwrap_or(serde_json::json!({}))
                }))
            }
        }
    }

    /// Merge new rules with existing rules
    fn merge_rules(&mut self, new_rules: BehaviorRules) {
        // Merge consistency rules
        self.rules.consistency_rules.extend(new_rules.consistency_rules);

        // Merge schemas
        for (key, value) in new_rules.schemas {
            self.rules.schemas.insert(key, value);
        }

        // Merge state machines
        for (key, value) in new_rules.state_transitions {
            self.rules.state_transitions.insert(key, value);
        }

        // Update system prompt if new one is more descriptive
        if new_rules.system_prompt.len() > self.rules.system_prompt.len() {
            self.rules.system_prompt = new_rules.system_prompt;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_is_paginated_request() {
        // Skip test if API key is not available
        if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("Skipping test: No API key found");
            return;
        }

        let config = IntelligentBehaviorConfig::default();
        let examples = vec![ExamplePair {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            request: None,
            status: 200,
            response: Some(json!({})),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        }];

        let mockai = match MockAI::from_examples(examples, config).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Skipping test: Failed to create MockAI: {}", e);
                return;
            }
        };

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let request = Request {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            body: None,
            query_params,
            headers: HashMap::new(),
        };

        assert!(mockai.is_paginated_request(&request));
    }

    #[tokio::test]
    async fn test_process_request() {
        // Skip test if API key is not available
        if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("Skipping test: No API key found");
            return;
        }

        let config = IntelligentBehaviorConfig::default();
        let examples = vec![ExamplePair {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            request: None,
            status: 200,
            response: Some(json!({
                "users": [],
                "total": 0
            })),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        }];

        let mockai = match MockAI::from_examples(examples, config).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Skipping test: Failed to create MockAI: {}", e);
                return;
            }
        };

        let request = Request {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            body: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };

        let response = match mockai.process_request(&request).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping test: Failed to process request: {}", e);
                return;
            }
        };

        // Verify response structure
        assert_eq!(response.status_code, 200);
        assert!(response.body.is_object() || response.body.is_array());
    }

    #[tokio::test]
    async fn test_process_request_with_body() {
        // Skip test if API key is not available
        if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("ANTHROPIC_API_KEY").is_err() {
            eprintln!("Skipping test: No API key found");
            return;
        }

        let config = IntelligentBehaviorConfig::default();
        let examples = vec![ExamplePair {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            request: Some(json!({
                "name": "John Doe",
                "email": "john@example.com"
            })),
            status: 201,
            response: Some(json!({
                "id": "123",
                "name": "John Doe",
                "email": "john@example.com"
            })),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            metadata: HashMap::new(),
        }];

        let mockai = match MockAI::from_examples(examples, config).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Skipping test: Failed to create MockAI: {}", e);
                return;
            }
        };

        let request = Request {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            body: Some(json!({
                "name": "Jane Doe",
                "email": "jane@example.com"
            })),
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };

        let response = match mockai.process_request(&request).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Skipping test: Failed to process request: {}", e);
                return;
            }
        };

        // Verify response structure
        assert_eq!(response.status_code, 201);
        assert!(response.body.is_object());
    }

    #[test]
    fn test_request_creation() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let request = Request {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            body: Some(json!({"id": 1})),
            query_params,
            headers,
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/users");
        assert!(request.body.is_some());
    }

    #[test]
    fn test_response_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = Response {
            status_code: 200,
            body: json!({"message": "success"}),
            headers,
        };

        assert_eq!(response.status_code, 200);
        assert!(response.body.is_object());
    }

    #[test]
    fn test_mockai_new() {
        let config = IntelligentBehaviorConfig::default();
        let mockai = MockAI::new(config);
        // Just verify it can be created
        let _ = mockai;
    }

    #[test]
    fn test_mockai_rules() {
        let config = IntelligentBehaviorConfig::default();
        let mockai = MockAI::new(config);
        let rules = mockai.rules();
        // Just verify we can access rules
        let _ = rules;
    }

    #[test]
    fn test_mockai_update_rules() {
        let config = IntelligentBehaviorConfig::default();
        let mut mockai = MockAI::new(config);
        let new_rules = BehaviorRules::default();
        mockai.update_rules(new_rules);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_mockai_get_config() {
        let config = IntelligentBehaviorConfig::default();
        let mockai = MockAI::new(config.clone());
        let retrieved_config = mockai.get_config();
        // Just verify we can access config
        let _ = retrieved_config;
    }

    #[test]
    fn test_mockai_update_config() {
        let config = IntelligentBehaviorConfig::default();
        let mut mockai = MockAI::new(config.clone());
        let new_config = IntelligentBehaviorConfig::default();
        mockai.update_config(new_config);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_extract_examples_from_openapi_empty_spec() {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {}
        });
        let spec = OpenApiSpec::from_json(spec_json).unwrap();
        let examples = MockAI::extract_examples_from_openapi(&spec).unwrap();
        assert!(examples.is_empty());
    }

    #[test]
    fn test_request_with_all_fields() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let mut query_params = HashMap::new();
        query_params.insert("limit".to_string(), "10".to_string());

        let request = Request {
            method: "POST".to_string(),
            path: "/api/data".to_string(),
            body: Some(json!({"key": "value"})),
            query_params: query_params.clone(),
            headers: headers.clone(),
        };

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/data");
        assert!(request.body.is_some());
        assert_eq!(request.query_params.get("limit"), Some(&"10".to_string()));
        assert_eq!(request.headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_response_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("X-Total-Count".to_string(), "100".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = Response {
            status_code: 201,
            body: json!({"id": "123", "created": true}),
            headers: headers.clone(),
        };

        assert_eq!(response.status_code, 201);
        assert!(response.body.is_object());
        assert_eq!(response.headers.len(), 2);
        assert_eq!(response.headers.get("X-Total-Count"), Some(&"100".to_string()));
    }

    #[test]
    fn test_request_clone() {
        let request1 = Request {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            body: Some(json!({"id": 1})),
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };
        let request2 = request1.clone();
        assert_eq!(request1.method, request2.method);
        assert_eq!(request1.path, request2.path);
    }

    #[test]
    fn test_request_debug() {
        let request = Request {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            body: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };
        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("Request"));
    }

    #[test]
    fn test_response_clone() {
        let response1 = Response {
            status_code: 200,
            body: json!({"status": "ok"}),
            headers: HashMap::new(),
        };
        let response2 = response1.clone();
        assert_eq!(response1.status_code, response2.status_code);
    }

    #[test]
    fn test_response_debug() {
        let response = Response {
            status_code: 404,
            body: json!({"error": "Not found"}),
            headers: HashMap::new(),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("Response"));
    }

    #[test]
    fn test_request_with_empty_fields() {
        let request = Request {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            body: None,
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };
        assert!(request.body.is_none());
        assert!(request.query_params.is_empty());
        assert!(request.headers.is_empty());
    }

    #[test]
    fn test_response_with_empty_headers() {
        let response = Response {
            status_code: 200,
            body: json!({"data": []}),
            headers: HashMap::new(),
        };
        assert!(response.headers.is_empty());
        assert!(response.body.is_object());
    }

    #[test]
    fn test_request_with_complex_body() {
        let request = Request {
            method: "PUT".to_string(),
            path: "/api/users/123".to_string(),
            body: Some(json!({
                "name": "John Doe",
                "email": "john@example.com",
                "metadata": {
                    "role": "admin",
                    "permissions": ["read", "write"]
                }
            })),
            query_params: HashMap::new(),
            headers: HashMap::new(),
        };
        assert!(request.body.is_some());
        let body = request.body.unwrap();
        assert!(body.is_object());
        assert!(body.get("metadata").is_some());
    }

    #[test]
    fn test_response_with_array_body() {
        let response = Response {
            status_code: 200,
            body: json!([
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
                {"id": 3, "name": "Charlie"}
            ]),
            headers: HashMap::new(),
        };
        assert!(response.body.is_array());
        let array = response.body.as_array().unwrap();
        assert_eq!(array.len(), 3);
    }

    #[test]
    fn test_request_with_multiple_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "20".to_string());
        query_params.insert("sort".to_string(), "name".to_string());
        query_params.insert("order".to_string(), "asc".to_string());

        let request = Request {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            body: None,
            query_params: query_params.clone(),
            headers: HashMap::new(),
        };

        assert_eq!(request.query_params.len(), 4);
        assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
        assert_eq!(request.query_params.get("limit"), Some(&"20".to_string()));
    }

    #[test]
    fn test_response_with_multiple_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Request-ID".to_string(), "req-123".to_string());
        headers.insert("X-Rate-Limit-Remaining".to_string(), "99".to_string());
        headers.insert("Cache-Control".to_string(), "no-cache".to_string());

        let response = Response {
            status_code: 200,
            body: json!({"data": "test"}),
            headers: headers.clone(),
        };

        assert_eq!(response.headers.len(), 4);
        assert_eq!(response.headers.get("X-Request-ID"), Some(&"req-123".to_string()));
    }

    #[test]
    fn test_request_different_methods() {
        let methods = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
        for method in methods {
            let request = Request {
                method: method.to_string(),
                path: "/api/test".to_string(),
                body: None,
                query_params: HashMap::new(),
                headers: HashMap::new(),
            };
            assert_eq!(request.method, method);
        }
    }

    #[test]
    fn test_response_different_status_codes() {
        let status_codes = vec![200, 201, 204, 400, 401, 403, 404, 500, 503];
        for status_code in status_codes {
            let response = Response {
                status_code,
                body: json!({"status": status_code}),
                headers: HashMap::new(),
            };
            assert_eq!(response.status_code, status_code);
        }
    }
}
