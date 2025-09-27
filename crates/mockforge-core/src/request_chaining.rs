//! Request chaining for MockForge
//!
//! This module provides functionality to chain multiple HTTP requests together,
//! allowing responses from previous requests to be used as input for subsequent requests.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Configuration for request chaining
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    /// Enable request chaining
    pub enabled: bool,
    /// Maximum chain length to prevent infinite loops
    pub max_chain_length: usize,
    /// Global timeout for chain execution
    pub global_timeout_secs: u64,
    /// Parallel execution when dependencies allow
    pub enable_parallel_execution: bool,
}

/// Context store for maintaining state across a chain execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainContext {
    /// Responses from previous requests in the chain
    #[serde(default)]
    pub responses: HashMap<String, ChainResponse>,
    /// Global variables shared across the chain
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
    /// Chain execution metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ChainContext {
    /// Create a new empty chain context
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            variables: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Store a response with the given name
    pub fn store_response(&mut self, name: String, response: ChainResponse) {
        self.responses.insert(name, response);
    }

    /// Get a stored response by name
    pub fn get_response(&self, name: &str) -> Option<&ChainResponse> {
        self.responses.get(name)
    }

    /// Store a global variable
    pub fn set_variable(&mut self, name: String, value: serde_json::Value) {
        self.variables.insert(name, value);
    }

    /// Get a global variable
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
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

impl Default for ChainContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre/Post request scripting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestScripting {
    /// Script to execute before the request
    pub pre_script: Option<String>,
    /// Script to execute after the request completes
    pub post_script: Option<String>,
    /// Scripting runtime (javascript, typescript)
    #[serde(default = "default_script_runtime")]
    pub runtime: String,
    /// Script timeout in milliseconds
    #[serde(default = "default_script_timeout")]
    pub timeout_ms: u64,
}

/// Request body types supported by MockForge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RequestBody {
    /// JSON or text body (default)
    #[serde(rename = "json")]
    Json(serde_json::Value),
    /// Binary file body - references a file on disk
    #[serde(rename = "binary_file")]
    BinaryFile {
        /// Path to the binary file
        path: String,
        /// Optional content type
        content_type: Option<String>,
    },
}

impl RequestBody {
    /// Create a JSON request body from a serde_json::Value
    pub fn json(value: serde_json::Value) -> Self {
        Self::Json(value)
    }

    /// Create a binary file request body
    pub fn binary_file(path: String, content_type: Option<String>) -> Self {
        Self::BinaryFile { path, content_type }
    }

    /// Convert the request body to bytes for HTTP transmission
    pub async fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        match self {
            RequestBody::Json(value) => Ok(serde_json::to_vec(value)?),
            RequestBody::BinaryFile { path, .. } => tokio::fs::read(path).await.map_err(|e| {
                crate::Error::generic(format!("Failed to read binary file '{}': {}", path, e))
            }),
        }
    }

    /// Get the content type for this request body
    pub fn content_type(&self) -> Option<&str> {
        match self {
            RequestBody::Json(_) => Some("application/json"),
            RequestBody::BinaryFile { content_type, .. } => content_type.as_deref(),
        }
    }
}

/// A single request in a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainRequest {
    /// Unique identifier for this request in the chain
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Request URL (can contain template variables)
    pub url: String,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (can contain template variables)
    pub body: Option<RequestBody>,
    /// Dependencies - IDs of other requests that must complete first
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Timeout for this individual request
    pub timeout_secs: Option<u64>,
    /// Expected status code range (optional validation)
    pub expected_status: Option<Vec<u16>>,
    /// Pre/Post request scripting
    #[serde(default)]
    pub scripting: Option<RequestScripting>,
}

/// Response from a chain request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Option<serde_json::Value>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp when the request was executed
    pub executed_at: String,
    /// Any error that occurred
    pub error: Option<String>,
}

/// A single link in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainLink {
    /// Request definition
    pub request: ChainRequest,
    /// Extract variables from the response
    #[serde(default)]
    pub extract: HashMap<String, String>,
    /// Store the entire response with this name
    pub store_as: Option<String>,
}

/// Chain definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainDefinition {
    /// Unique identifier for the chain
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this chain does
    pub description: Option<String>,
    /// Chain configuration
    pub config: ChainConfig,
    /// Ordered list of requests to execute
    pub links: Vec<ChainLink>,
    /// Initial variables to set
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Script execution context for pre/post request scripts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionContext {
    /// Chain context with stored responses
    pub chain_context: ChainContext,
    /// Request-scoped variables
    pub request_variables: HashMap<String, serde_json::Value>,
    /// Current request being executed
    pub current_request: Option<ChainRequest>,
    /// Current response (for post-scripts)
    pub current_response: Option<ChainResponse>,
}

/// Context for template expansion during chain execution
#[derive(Debug, Clone)]
pub struct ChainTemplatingContext {
    /// Chain context with stored responses
    pub chain_context: ChainContext,
    /// Request-scoped variables
    pub request_variables: HashMap<String, serde_json::Value>,
    /// Current request being executed
    pub current_request: Option<ChainRequest>,
}

impl ChainTemplatingContext {
    /// Create a new templating context
    pub fn new(chain_context: ChainContext) -> Self {
        Self {
            chain_context,
            request_variables: HashMap::new(),
            current_request: None,
        }
    }

    /// Set request-scoped variable
    pub fn set_request_variable(&mut self, name: String, value: serde_json::Value) {
        self.request_variables.insert(name, value);
    }

    /// Set current request
    pub fn set_current_request(&mut self, request: ChainRequest) {
        self.current_request = Some(request);
    }

    /// Extract a value using JSONPath-like syntax
    pub fn extract_value(&self, path: &str) -> Option<serde_json::Value> {
        // Split path like "response1.body.user.id"
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return None;
        }

        let root = parts[0];

        // Get the root object
        let root_value = if let Some(resp) = self.chain_context.get_response(root) {
            // For response references, get body
            resp.body.clone()?
        } else if let Some(var) = self.chain_context.get_variable(root) {
            var.clone()
        } else if let Some(var) = self.request_variables.get(root) {
            var.clone()
        } else {
            return None;
        };

        // Navigate the path
        self.navigate_json_path(&root_value, &parts[1..])
    }

    /// Navigate JSON value using path segments
    fn navigate_json_path(
        &self,
        value: &serde_json::Value,
        path: &[&str],
    ) -> Option<serde_json::Value> {
        if path.is_empty() {
            return Some(value.clone());
        }

        match value {
            serde_json::Value::Object(map) => {
                if let Some(next_value) = map.get(path[0]) {
                    self.navigate_json_path(next_value, &path[1..])
                } else {
                    None
                }
            }
            serde_json::Value::Array(arr) => {
                // Handle array indexing like [0]
                if path[0].starts_with('[') && path[0].ends_with(']') {
                    let index_str = &path[0][1..path[0].len() - 1];
                    if let Ok(index) = index_str.parse::<usize>() {
                        if let Some(item) = arr.get(index) {
                            self.navigate_json_path(item, &path[1..])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Chain store for managing multiple chain definitions
#[derive(Debug)]
pub struct ChainStore {
    /// Registry of chain definitions
    chains: RwLock<HashMap<String, ChainDefinition>>,
    /// Configuration
    config: ChainConfig,
}

impl ChainStore {
    /// Create a new chain store
    pub fn new(config: ChainConfig) -> Self {
        Self {
            chains: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Register a chain definition
    pub async fn register_chain(&self, chain: ChainDefinition) -> Result<()> {
        let mut chains = self.chains.write().await;
        chains.insert(chain.id.clone(), chain);
        Ok(())
    }

    /// Get a chain definition by ID
    pub async fn get_chain(&self, id: &str) -> Option<ChainDefinition> {
        let chains = self.chains.read().await;
        chains.get(id).cloned()
    }

    /// List all registered chains
    pub async fn list_chains(&self) -> Vec<String> {
        let chains = self.chains.read().await;
        chains.keys().cloned().collect()
    }

    /// Remove a chain definition
    pub async fn remove_chain(&self, id: &str) -> Result<()> {
        let mut chains = self.chains.write().await;
        chains.remove(id);
        Ok(())
    }

    /// Update chain configuration
    pub fn update_config(&mut self, config: ChainConfig) {
        self.config = config;
    }
}

/// Context for chain execution
#[derive(Debug)]
pub struct ChainExecutionContext {
    /// Chain definition being executed
    pub definition: ChainDefinition,
    /// Chain templating context
    pub templating: ChainTemplatingContext,
    /// Execution start time
    pub start_time: std::time::Instant,
    /// Chain configuration
    pub config: ChainConfig,
}

impl ChainExecutionContext {
    /// Create a new execution context
    pub fn new(definition: ChainDefinition) -> Self {
        let chain_context = ChainContext::new();
        let templating = ChainTemplatingContext::new(chain_context);
        let config = definition.config.clone();

        Self {
            definition,
            templating,
            start_time: std::time::Instant::now(),
            config,
        }
    }

    /// Get elapsed time
    pub fn elapsed_ms(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }
}

/// Main registry for managing request chains
#[derive(Debug)]
pub struct RequestChainRegistry {
    /// Chain store
    store: ChainStore,
}

impl RequestChainRegistry {
    /// Create a new registry
    pub fn new(config: ChainConfig) -> Self {
        Self {
            store: ChainStore::new(config),
        }
    }

    /// Register a chain from YAML string
    pub async fn register_from_yaml(&self, yaml: &str) -> Result<String> {
        let chain: ChainDefinition = serde_yaml::from_str(yaml)
            .map_err(|e| Error::generic(format!("Failed to parse chain YAML: {}", e)))?;
        self.store.register_chain(chain.clone()).await?;
        Ok(chain.id.clone())
    }

    /// Register a chain from JSON string
    pub async fn register_from_json(&self, json: &str) -> Result<String> {
        let chain: ChainDefinition = serde_json::from_str(json)
            .map_err(|e| Error::generic(format!("Failed to parse chain JSON: {}", e)))?;
        self.store.register_chain(chain.clone()).await?;
        Ok(chain.id.clone())
    }

    /// Get a chain by ID
    pub async fn get_chain(&self, id: &str) -> Option<ChainDefinition> {
        self.store.get_chain(id).await
    }

    /// List all chains
    pub async fn list_chains(&self) -> Vec<String> {
        self.store.list_chains().await
    }

    /// Remove a chain
    pub async fn remove_chain(&self, id: &str) -> Result<()> {
        self.store.remove_chain(id).await
    }

    /// Validate chain dependencies and structure
    pub async fn validate_chain(&self, chain: &ChainDefinition) -> Result<()> {
        if chain.links.is_empty() {
            return Err(Error::generic("Chain must have at least one link"));
        }

        if chain.links.len() > self.store.config.max_chain_length {
            return Err(Error::generic(format!(
                "Chain length {} exceeds maximum allowed length {}",
                chain.links.len(),
                self.store.config.max_chain_length
            )));
        }

        // Check for circular dependencies and invalid references
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for link in &chain.links {
            self.validate_link_dependencies(link, &mut visited, &mut rec_stack, chain)?;
        }

        // Check for duplicate request IDs
        let request_ids: std::collections::HashSet<_> =
            chain.links.iter().map(|link| &link.request.id).collect();

        if request_ids.len() != chain.links.len() {
            return Err(Error::generic("Duplicate request IDs found in chain"));
        }

        Ok(())
    }

    /// Validate link dependencies for circular references
    fn validate_link_dependencies(
        &self,
        link: &ChainLink,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        chain: &ChainDefinition,
    ) -> Result<()> {
        if rec_stack.contains(&link.request.id) {
            return Err(Error::generic(format!(
                "Circular dependency detected involving request '{}'",
                link.request.id
            )));
        }

        if visited.contains(&link.request.id) {
            return Ok(());
        }

        visited.insert(link.request.id.clone());
        rec_stack.insert(link.request.id.clone());

        for dep in &link.request.depends_on {
            // Check if dependency exists in the chain
            if !chain.links.iter().any(|l| &l.request.id == dep) {
                return Err(Error::generic(format!(
                    "Request '{}' depends on '{}' which does not exist in the chain",
                    link.request.id, dep
                )));
            }

            // Recursively check the dependency
            if let Some(dep_link) = chain.links.iter().find(|l| &l.request.id == dep) {
                self.validate_link_dependencies(dep_link, visited, rec_stack, chain)?;
            }
        }

        rec_stack.remove(&link.request.id);
        Ok(())
    }

    /// Get the chain store (for internal use)
    pub fn store(&self) -> &ChainStore {
        &self.store
    }
}

fn default_script_runtime() -> String {
    "javascript".to_string()
}

fn default_script_timeout() -> u64 {
    5000 // 5 seconds
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_chain_length: 20,
            global_timeout_secs: 300,
            enable_parallel_execution: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_chain_context() {
        let mut ctx = ChainContext::new();

        // Test variable storage
        ctx.set_variable("user_id".to_string(), json!("12345"));
        assert_eq!(ctx.get_variable("user_id"), Some(&json!("12345")));

        // Test response storage
        let response = ChainResponse {
            status: 200,
            headers: HashMap::new(),
            body: Some(json!({"user": {"id": 123, "name": "John"}})),
            duration_ms: 150,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };
        ctx.store_response("login".to_string(), response.clone());
        assert_eq!(ctx.get_response("login"), Some(&response));
    }

    #[test]
    fn test_chain_context_comprehensive() {
        let mut ctx = ChainContext::new();

        // Test multiple variables
        ctx.set_variable("user_id".to_string(), json!("12345"));
        ctx.set_variable("token".to_string(), json!("abc-def-ghi"));
        ctx.set_variable("environment".to_string(), json!("production"));
        ctx.set_variable("timeout".to_string(), json!(30));

        assert_eq!(ctx.get_variable("user_id"), Some(&json!("12345")));
        assert_eq!(ctx.get_variable("token"), Some(&json!("abc-def-ghi")));
        assert_eq!(ctx.get_variable("environment"), Some(&json!("production")));
        assert_eq!(ctx.get_variable("timeout"), Some(&json!(30)));

        // Test non-existent variable
        assert_eq!(ctx.get_variable("nonexistent"), None);

        // Test variable overwriting
        ctx.set_variable("user_id".to_string(), json!("67890"));
        assert_eq!(ctx.get_variable("user_id"), Some(&json!("67890")));

        // Test metadata
        ctx.set_metadata("chain_id".to_string(), "test-chain-123".to_string());
        ctx.set_metadata("version".to_string(), "1.0.0".to_string());
        assert_eq!(ctx.get_metadata("chain_id"), Some(&"test-chain-123".to_string()));
        assert_eq!(ctx.get_metadata("version"), Some(&"1.0.0".to_string()));

        // Test multiple responses
        let response1 = ChainResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: Some(json!({"message": "success1"})),
            duration_ms: 100,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };

        let response2 = ChainResponse {
            status: 201,
            headers: vec![("Location".to_string(), "/users/123".to_string())].into_iter().collect(),
            body: Some(json!({"id": 123, "name": "John"})),
            duration_ms: 150,
            executed_at: "2023-01-01T00:00:01Z".to_string(),
            error: None,
        };

        ctx.store_response("step1".to_string(), response1.clone());
        ctx.store_response("step2".to_string(), response2.clone());

        assert_eq!(ctx.get_response("step1"), Some(&response1));
        assert_eq!(ctx.get_response("step2"), Some(&response2));
        assert_eq!(ctx.get_response("nonexistent"), None);

        // Test response overwriting
        let updated_response = ChainResponse {
            status: 202,
            headers: HashMap::new(),
            body: Some(json!({"message": "updated"})),
            duration_ms: 200,
            executed_at: "2023-01-01T00:00:02Z".to_string(),
            error: None,
        };
        ctx.store_response("step1".to_string(), updated_response.clone());
        assert_eq!(ctx.get_response("step1"), Some(&updated_response));
    }

    #[test]
    fn test_chain_context_serialization() {
        let mut ctx = ChainContext::new();

        // Add some data
        ctx.set_variable("test_var".to_string(), json!("test_value"));
        ctx.set_metadata("test_meta".to_string(), "test_value".to_string());

        let response = ChainResponse {
            status: 200,
            headers: HashMap::new(),
            body: Some(json!({"data": "test"})),
            duration_ms: 100,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };
        ctx.store_response("test_response".to_string(), response);

        // Test serialization
        let json_str = serde_json::to_string(&ctx).unwrap();
        assert!(json_str.contains("test_var"));
        assert!(json_str.contains("test_value"));
        assert!(json_str.contains("test_meta"));
        assert!(json_str.contains("test_response"));

        // Test deserialization
        let deserialized: ChainContext = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.get_variable("test_var"), Some(&json!("test_value")));
        assert_eq!(deserialized.get_metadata("test_meta"), Some(&"test_value".to_string()));
        assert!(deserialized.get_response("test_response").is_some());
    }

    #[test]
    fn test_chain_request_serialization() {
        let request = ChainRequest {
            id: "test-req".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/test".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: Some(RequestBody::Json(json!({"key": "value"}))),
            depends_on: vec!["req1".to_string(), "req2".to_string()],
            timeout_secs: Some(30),
            expected_status: Some(vec![200, 201, 202]),
            scripting: Some(RequestScripting {
                pre_script: Some("console.log('pre');".to_string()),
                post_script: Some("console.log('post');".to_string()),
                runtime: "javascript".to_string(),
                timeout_ms: 5000,
            }),
        };

        let json_str = serde_json::to_string(&request).unwrap();
        assert!(json_str.contains("test-req"));
        assert!(json_str.contains("POST"));
        assert!(json_str.contains("Content-Type"));
        assert!(json_str.contains("req1"));
        assert!(json_str.contains("pre_script"));

        let deserialized: ChainRequest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, request.id);
        assert_eq!(deserialized.method, request.method);
        assert_eq!(deserialized.depends_on, request.depends_on);
    }

    #[test]
    fn test_chain_response_serialization() {
        let response = ChainResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Request-ID".to_string(), "req-123".to_string()),
            ]
            .into_iter()
            .collect(),
            body: Some(json!({"result": "success", "data": [1, 2, 3]})),
            duration_ms: 150,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: None,
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("200"));
        assert!(json_str.contains("application/json"));
        assert!(json_str.contains("success"));

        let deserialized: ChainResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.status, response.status);
        assert_eq!(deserialized.duration_ms, response.duration_ms);
        assert_eq!(deserialized.body, response.body);
    }

    #[test]
    fn test_chain_response_with_error() {
        let error_response = ChainResponse {
            status: 500,
            headers: HashMap::new(),
            body: None,
            duration_ms: 50,
            executed_at: "2023-01-01T00:00:00Z".to_string(),
            error: Some("Internal server error".to_string()),
        };

        let json_str = serde_json::to_string(&error_response).unwrap();
        assert!(json_str.contains("500"));
        assert!(json_str.contains("Internal server error"));

        let deserialized: ChainResponse = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.error, Some("Internal server error".to_string()));
        assert!(deserialized.body.is_none());
    }

    #[test]
    fn test_request_body_types() {
        // Test JSON body
        let json_body = RequestBody::Json(json!({"key": "value", "number": 42}));
        assert!(matches!(json_body, RequestBody::Json(_)));

        // Test string body
        let string_body =
            RequestBody::Json(serde_json::Value::String("raw text content".to_string()));
        assert!(matches!(string_body, RequestBody::Json(_)));

        // Test binary file body
        let binary_body = RequestBody::BinaryFile {
            path: "/path/to/file.bin".to_string(),
            content_type: Some("application/octet-stream".to_string()),
        };
        assert!(matches!(binary_body, RequestBody::BinaryFile { .. }));

        // Test serialization of different body types
        let test_cases = vec![
            RequestBody::Json(json!({"test": "json"})),
            RequestBody::Json(serde_json::Value::String("test string".to_string())),
            RequestBody::BinaryFile {
                path: "/path/to/bytes.bin".to_string(),
                content_type: None,
            },
        ];

        for body in test_cases {
            let json_str = serde_json::to_string(&body).unwrap();
            let deserialized: RequestBody = serde_json::from_str(&json_str).unwrap();
            assert_eq!(format!("{:?}", body), format!("{:?}", deserialized));
        }
    }

    #[test]
    fn test_chain_link_dependencies() {
        let link1 = ChainLink {
            request: ChainRequest {
                id: "req1".to_string(),
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
                headers: HashMap::new(),
                body: None,
                depends_on: vec![], // No dependencies
                timeout_secs: None,
                expected_status: None,
                scripting: None,
            },
            extract: HashMap::new(),
            store_as: Some("users".to_string()),
        };

        let link2 = ChainLink {
            request: ChainRequest {
                id: "req2".to_string(),
                method: "POST".to_string(),
                url: "https://api.example.com/posts".to_string(),
                headers: HashMap::new(),
                body: Some(RequestBody::Json(json!({"title": "Test"}))),
                depends_on: vec!["req1".to_string()], // Depends on req1
                timeout_secs: Some(30),
                expected_status: Some(vec![200, 201]),
                scripting: None,
            },
            extract: HashMap::new(),
            store_as: Some("post".to_string()),
        };

        let link3 = ChainLink {
            request: ChainRequest {
                id: "req3".to_string(),
                method: "PUT".to_string(),
                url: "https://api.example.com/posts/{{chain.post.id}}".to_string(),
                headers: HashMap::new(),
                body: None,
                depends_on: vec!["req1".to_string(), "req2".to_string()], // Multiple dependencies
                timeout_secs: None,
                expected_status: None,
                scripting: None,
            },
            extract: HashMap::new(),
            store_as: None,
        };

        assert!(link1.request.depends_on.is_empty());
        assert_eq!(link2.request.depends_on, vec!["req1".to_string()]);
        assert_eq!(link3.request.depends_on, vec!["req1".to_string(), "req2".to_string()]);
    }

    #[test]
    fn test_chain_config_validation() {
        // Test valid config
        let valid_config = ChainConfig {
            enabled: true,
            max_chain_length: 10,
            global_timeout_secs: 300,
            enable_parallel_execution: true,
        };

        // Test invalid config
        let invalid_config = ChainConfig {
            enabled: true,
            max_chain_length: 0, // Invalid: must be > 0
            global_timeout_secs: 300,
            enable_parallel_execution: true,
        };

        assert!(valid_config.max_chain_length > 0);
        assert!(invalid_config.max_chain_length == 0);

        // Test edge cases
        let edge_config = ChainConfig {
            enabled: false,
            max_chain_length: 1,
            global_timeout_secs: 0,
            enable_parallel_execution: false,
        };
        assert_eq!(edge_config.max_chain_length, 1);
        assert_eq!(edge_config.global_timeout_secs, 0);
        assert!(!edge_config.enabled);
    }

    #[test]
    fn test_request_scripting_config() {
        let scripting = RequestScripting {
            pre_script: Some("console.log('Starting request');".to_string()),
            post_script: Some("console.log('Request completed');".to_string()),
            runtime: "javascript".to_string(),
            timeout_ms: 5000,
        };

        assert_eq!(scripting.runtime, "javascript");
        assert_eq!(scripting.timeout_ms, 5000);
        assert!(scripting.pre_script.is_some());
        assert!(scripting.post_script.is_some());

        // Test serialization
        let json_str = serde_json::to_string(&scripting).unwrap();
        assert!(json_str.contains("javascript"));
        assert!(json_str.contains("Starting request"));
        assert!(json_str.contains("Request completed"));

        let deserialized: RequestScripting = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.runtime, scripting.runtime);
        assert_eq!(deserialized.timeout_ms, scripting.timeout_ms);
    }

    #[test]
    fn test_chain_definition_structure() {
        let definition = ChainDefinition {
            id: "test-chain".to_string(),
            name: "Test Chain".to_string(),
            description: Some("A comprehensive test chain".to_string()),
            config: ChainConfig::default(),
            links: vec![ChainLink {
                request: ChainRequest {
                    id: "req1".to_string(),
                    method: "GET".to_string(),
                    url: "https://api.example.com/users".to_string(),
                    headers: HashMap::new(),
                    body: None,
                    depends_on: vec![],
                    timeout_secs: None,
                    expected_status: None,
                    scripting: None,
                },
                extract: vec![("user_id".to_string(), "$.users[0].id".to_string())]
                    .into_iter()
                    .collect(),
                store_as: Some("users".to_string()),
            }],
            variables: vec![
                ("api_key".to_string(), json!("test-key")),
                ("base_url".to_string(), json!("https://api.example.com")),
            ]
            .into_iter()
            .collect(),
            tags: vec!["test".to_string(), "integration".to_string()],
        };

        assert_eq!(definition.id, "test-chain");
        assert_eq!(definition.name, "Test Chain");
        assert!(definition.description.is_some());
        assert_eq!(definition.links.len(), 1);
        assert_eq!(definition.variables.len(), 2);
        assert_eq!(definition.tags.len(), 2);

        // Test serialization
        let json_str = serde_json::to_string(&definition).unwrap();
        assert!(json_str.contains("test-chain"));
        assert!(json_str.contains("Test Chain"));
        assert!(json_str.contains("comprehensive test chain"));
        assert!(json_str.contains("api_key"));
        assert!(json_str.contains("test-key"));
    }

    #[test]
    fn test_chain_execution_context() {
        let chain_def = ChainDefinition {
            id: "test_chain".to_string(),
            name: "Test Chain".to_string(),
            description: Some("Test chain for unit tests".to_string()),
            config: ChainConfig::default(),
            links: vec![],
            tags: vec![],
            variables: HashMap::new(),
        };
        let exec_ctx = ChainExecutionContext::new(chain_def);

        // Test elapsed time
        assert!(exec_ctx.elapsed_ms() > 0);
    }

    #[tokio::test]
    async fn test_chain_definition_validation() {
        let registry = RequestChainRegistry::new(ChainConfig::default());

        let valid_chain = ChainDefinition {
            id: "test-chain".to_string(),
            name: "Test Chain".to_string(),
            description: Some("A test chain for validation".to_string()),
            config: ChainConfig::default(),
            links: vec![
                ChainLink {
                    request: ChainRequest {
                        id: "req1".to_string(),
                        method: "GET".to_string(),
                        url: "https://api.example.com/users".to_string(),
                        headers: HashMap::new(),
                        body: None,
                        depends_on: vec![],
                        timeout_secs: None,
                        expected_status: None,
                        scripting: None,
                    },
                    extract: HashMap::new(),
                    store_as: Some("users".to_string()),
                },
                ChainLink {
                    request: ChainRequest {
                        id: "req2".to_string(),
                        method: "POST".to_string(),
                        url: "https://api.example.com/users/{{chain.users.body[0].id}}/posts"
                            .to_string(),
                        headers: HashMap::new(),
                        body: Some(RequestBody::Json(json!({"title": "Hello World"}))),
                        depends_on: vec!["req1".to_string()],
                        timeout_secs: None,
                        expected_status: None,
                        scripting: None,
                    },
                    extract: HashMap::new(),
                    store_as: Some("post".to_string()),
                },
            ],
            variables: {
                let mut vars = HashMap::new();
                vars.insert("api_key".to_string(), json!("test-key-123"));
                vars
            },
            tags: vec!["test".to_string()],
        };

        // Should validate successfully
        assert!(registry.validate_chain(&valid_chain).await.is_ok());

        // Test invalid chain (empty)
        let invalid_chain = ChainDefinition {
            id: "empty-chain".to_string(),
            name: "Empty Chain".to_string(),
            description: None,
            config: ChainConfig::default(),
            links: vec![],
            variables: HashMap::new(),
            tags: vec![],
        };
        assert!(registry.validate_chain(&invalid_chain).await.is_err());

        // Test chain with self-dependency
        let self_dep_chain = ChainDefinition {
            id: "self-dep-chain".to_string(),
            name: "Self Dependency Chain".to_string(),
            description: None,
            config: ChainConfig::default(),
            links: vec![ChainLink {
                request: ChainRequest {
                    id: "req1".to_string(),
                    method: "GET".to_string(),
                    url: "https://api.example.com/users".to_string(),
                    headers: HashMap::new(),
                    body: None,
                    depends_on: vec!["req1".to_string()], // Self dependency
                    timeout_secs: None,
                    expected_status: None,
                    scripting: None,
                },
                extract: HashMap::new(),
                store_as: None,
            }],
            variables: HashMap::new(),
            tags: vec![],
        };
        assert!(registry.validate_chain(&self_dep_chain).await.is_err());
    }
}
