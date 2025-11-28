//! GraphQL Handler System
//!
//! Provides a flexible handler-based system for GraphQL operations, similar to the WebSocket handler architecture.
//! Handlers can intercept and customize query, mutation, and subscription resolution.

use async_graphql::{Name, Request, Response, ServerError, Value, Variables};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Result type for handler operations
pub type HandlerResult<T> = Result<T, HandlerError>;

/// Errors that can occur during handler execution
#[derive(Debug, Error)]
pub enum HandlerError {
    /// Error sending response
    #[error("Send error: {0}")]
    SendError(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Operation matching error
    #[error("Operation error: {0}")]
    OperationError(String),

    /// Upstream passthrough error
    #[error("Upstream error: {0}")]
    UpstreamError(String),

    /// Generic handler error
    #[error("{0}")]
    Generic(String),
}

/// Context for GraphQL handler execution
pub struct GraphQLContext {
    /// Operation name (query/mutation name)
    pub operation_name: Option<String>,

    /// Operation type (query, mutation, subscription)
    pub operation_type: OperationType,

    /// GraphQL query string
    pub query: String,

    /// Variables passed to the operation
    pub variables: Variables,

    /// Request metadata (headers, etc.)
    pub metadata: HashMap<String, String>,

    /// Custom data storage for handlers
    pub data: HashMap<String, serde_json::Value>,
}

/// Type of GraphQL operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationType {
    /// Query operation
    Query,
    /// Mutation operation
    Mutation,
    /// Subscription operation
    Subscription,
}

impl GraphQLContext {
    /// Create a new GraphQL context
    pub fn new(
        operation_name: Option<String>,
        operation_type: OperationType,
        query: String,
        variables: Variables,
    ) -> Self {
        Self {
            operation_name,
            operation_type,
            query,
            variables,
            metadata: HashMap::new(),
            data: HashMap::new(),
        }
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(&Name::new(name))
    }

    /// Set custom data
    pub fn set_data(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }

    /// Get custom data
    pub fn get_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Trait for handling GraphQL operations
#[async_trait]
pub trait GraphQLHandler: Send + Sync {
    /// Called before query/mutation execution
    /// Return None to proceed with default resolution, Some(Response) to override
    async fn on_operation(&self, _ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
        Ok(None)
    }

    /// Called after successful query/mutation execution
    /// Allows modification of the response
    async fn after_operation(
        &self,
        _ctx: &GraphQLContext,
        response: Response,
    ) -> HandlerResult<Response> {
        Ok(response)
    }

    /// Called when an error occurs
    async fn on_error(&self, _ctx: &GraphQLContext, error: String) -> HandlerResult<Response> {
        let server_error = ServerError::new(error, None);
        Ok(Response::from_errors(vec![server_error]))
    }

    /// Check if this handler should handle the given operation
    fn handles_operation(
        &self,
        operation_name: Option<&str>,
        _operation_type: &OperationType,
    ) -> bool {
        // Default: handle all operations
        operation_name.is_some()
    }

    /// Priority of this handler (higher = executes first)
    fn priority(&self) -> i32 {
        0
    }
}

/// Registry for managing GraphQL handlers
pub struct HandlerRegistry {
    handlers: Vec<Arc<dyn GraphQLHandler>>,
    /// Upstream GraphQL server URL for passthrough
    upstream_url: Option<String>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            upstream_url: None,
        }
    }

    /// Create a handler registry with upstream URL
    pub fn with_upstream(upstream_url: Option<String>) -> Self {
        Self {
            handlers: Vec::new(),
            upstream_url,
        }
    }

    /// Register a handler
    pub fn register<H: GraphQLHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Arc::new(handler));
        // Sort by priority (highest first)
        self.handlers.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Get handlers for a specific operation
    pub fn get_handlers(
        &self,
        operation_name: Option<&str>,
        operation_type: &OperationType,
    ) -> Vec<Arc<dyn GraphQLHandler>> {
        self.handlers
            .iter()
            .filter(|h| h.handles_operation(operation_name, operation_type))
            .cloned()
            .collect()
    }

    /// Execute handlers for an operation
    pub async fn execute_operation(&self, ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
        let handlers = self.get_handlers(ctx.operation_name.as_deref(), &ctx.operation_type);

        for handler in handlers {
            if let Some(response) = handler.on_operation(ctx).await? {
                return Ok(Some(response));
            }
        }

        Ok(None)
    }

    /// Execute after_operation hooks
    pub async fn after_operation(
        &self,
        ctx: &GraphQLContext,
        mut response: Response,
    ) -> HandlerResult<Response> {
        let handlers = self.get_handlers(ctx.operation_name.as_deref(), &ctx.operation_type);

        for handler in handlers {
            response = handler.after_operation(ctx, response).await?;
        }

        Ok(response)
    }

    /// Passthrough request to upstream server
    pub async fn passthrough(&self, request: &Request) -> HandlerResult<Response> {
        let upstream = self
            .upstream_url
            .as_ref()
            .ok_or_else(|| HandlerError::UpstreamError("No upstream URL configured".to_string()))?;

        let client = reqwest::Client::new();
        let body = json!({
            "query": request.query.clone(),
            "variables": request.variables.clone(),
            "operationName": request.operation_name.clone(),
        });

        let resp = client
            .post(upstream)
            .json(&body)
            .send()
            .await
            .map_err(|e| HandlerError::UpstreamError(e.to_string()))?;

        let response_data: serde_json::Value =
            resp.json().await.map_err(|e| HandlerError::UpstreamError(e.to_string()))?;

        // Convert JSON response to GraphQL Response
        // Extract data and errors from the GraphQL response
        let has_errors = response_data.get("errors").is_some();

        // For now, return a null response with error status if there are errors
        // In a full implementation, you would properly convert the JSON response to GraphQL types
        if has_errors {
            let error_msg = response_data
                .get("errors")
                .and_then(|e| e.as_array())
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Upstream GraphQL error");

            let server_error = async_graphql::ServerError::new(error_msg.to_string(), None);
            Ok(Response::from_errors(vec![server_error]))
        } else {
            // Return successful response
            // Note: Proper implementation would convert serde_json::Value to async_graphql::Value
            Ok(Response::new(Value::Null))
        }
    }

    /// Get upstream URL
    pub fn upstream_url(&self) -> Option<&str> {
        self.upstream_url.as_deref()
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable matcher for filtering operations by variable values
#[derive(Debug, Clone)]
pub struct VariableMatcher {
    patterns: HashMap<String, VariablePattern>,
}

impl VariableMatcher {
    /// Create a new variable matcher
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
        }
    }

    /// Add a pattern for a variable
    pub fn with_pattern(mut self, name: String, pattern: VariablePattern) -> Self {
        self.patterns.insert(name, pattern);
        self
    }

    /// Check if variables match the patterns
    pub fn matches(&self, variables: &Variables) -> bool {
        for (name, pattern) in &self.patterns {
            if !pattern.matches(variables.get(&Name::new(name))) {
                return false;
            }
        }
        true
    }
}

impl Default for VariableMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for matching variable values
#[derive(Debug, Clone)]
pub enum VariablePattern {
    /// Exact value match
    Exact(Value),
    /// Regular expression match (for strings)
    Regex(String),
    /// Any value (always matches)
    Any,
    /// Value must be present
    Present,
    /// Value must be null or absent
    Null,
}

impl VariablePattern {
    /// Check if a value matches this pattern
    pub fn matches(&self, value: Option<&Value>) -> bool {
        match (self, value) {
            (VariablePattern::Any, _) => true,
            (VariablePattern::Present, Some(_)) => true,
            (VariablePattern::Present, None) => false,
            (VariablePattern::Null, None) | (VariablePattern::Null, Some(Value::Null)) => true,
            (VariablePattern::Null, Some(_)) => false,
            (VariablePattern::Exact(expected), Some(actual)) => expected == actual,
            (VariablePattern::Exact(_), None) => false,
            (VariablePattern::Regex(pattern), Some(Value::String(s))) => {
                regex::Regex::new(pattern).ok().map(|re| re.is_match(s)).unwrap_or(false)
            }
            (VariablePattern::Regex(_), _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        operation_name: String,
    }

    #[async_trait]
    impl GraphQLHandler for TestHandler {
        async fn on_operation(&self, ctx: &GraphQLContext) -> HandlerResult<Option<Response>> {
            if ctx.operation_name.as_deref() == Some(&self.operation_name) {
                // Return a simple null response for testing
                Ok(Some(Response::new(Value::Null)))
            } else {
                Ok(None)
            }
        }

        fn handles_operation(&self, operation_name: Option<&str>, _: &OperationType) -> bool {
            operation_name == Some(&self.operation_name)
        }
    }

    #[tokio::test]
    async fn test_handler_registry_new() {
        let registry = HandlerRegistry::new();
        assert_eq!(registry.handlers.len(), 0);
        assert!(registry.upstream_url.is_none());
    }

    #[tokio::test]
    async fn test_handler_registry_with_upstream() {
        let registry =
            HandlerRegistry::with_upstream(Some("http://example.com/graphql".to_string()));
        assert_eq!(registry.upstream_url(), Some("http://example.com/graphql"));
    }

    #[tokio::test]
    async fn test_handler_registry_register() {
        let mut registry = HandlerRegistry::new();
        let handler = TestHandler {
            operation_name: "getUser".to_string(),
        };
        registry.register(handler);
        assert_eq!(registry.handlers.len(), 1);
    }

    #[tokio::test]
    async fn test_handler_execution() {
        let mut registry = HandlerRegistry::new();
        registry.register(TestHandler {
            operation_name: "getUser".to_string(),
        });

        let ctx = GraphQLContext::new(
            Some("getUser".to_string()),
            OperationType::Query,
            "query { user { id } }".to_string(),
            Variables::default(),
        );

        let result = registry.execute_operation(&ctx).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_variable_matcher_any() {
        let matcher = VariableMatcher::new().with_pattern("id".to_string(), VariablePattern::Any);

        let mut vars = Variables::default();
        vars.insert(Name::new("id"), Value::String("123".to_string()));

        assert!(matcher.matches(&vars));
    }

    #[test]
    fn test_variable_matcher_exact() {
        let matcher = VariableMatcher::new().with_pattern(
            "id".to_string(),
            VariablePattern::Exact(Value::String("123".to_string())),
        );

        let mut vars = Variables::default();
        vars.insert(Name::new("id"), Value::String("123".to_string()));

        assert!(matcher.matches(&vars));

        let mut vars2 = Variables::default();
        vars2.insert(Name::new("id"), Value::String("456".to_string()));

        assert!(!matcher.matches(&vars2));
    }

    #[test]
    fn test_variable_pattern_present() {
        assert!(VariablePattern::Present.matches(Some(&Value::String("test".to_string()))));
        assert!(!VariablePattern::Present.matches(None));
    }

    #[test]
    fn test_variable_pattern_null() {
        assert!(VariablePattern::Null.matches(None));
        assert!(VariablePattern::Null.matches(Some(&Value::Null)));
        assert!(!VariablePattern::Null.matches(Some(&Value::String("test".to_string()))));
    }

    #[test]
    fn test_graphql_context_new() {
        let ctx = GraphQLContext::new(
            Some("getUser".to_string()),
            OperationType::Query,
            "query { user { id } }".to_string(),
            Variables::default(),
        );

        assert_eq!(ctx.operation_name, Some("getUser".to_string()));
        assert_eq!(ctx.operation_type, OperationType::Query);
    }

    #[test]
    fn test_graphql_context_metadata() {
        let mut ctx = GraphQLContext::new(
            Some("getUser".to_string()),
            OperationType::Query,
            "query { user { id } }".to_string(),
            Variables::default(),
        );

        ctx.set_metadata("Authorization".to_string(), "Bearer token".to_string());
        assert_eq!(ctx.get_metadata("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn test_graphql_context_data() {
        let mut ctx = GraphQLContext::new(
            Some("getUser".to_string()),
            OperationType::Query,
            "query { user { id } }".to_string(),
            Variables::default(),
        );

        ctx.set_data("custom_key".to_string(), json!({"test": "value"}));
        assert_eq!(ctx.get_data("custom_key"), Some(&json!({"test": "value"})));
    }
}
