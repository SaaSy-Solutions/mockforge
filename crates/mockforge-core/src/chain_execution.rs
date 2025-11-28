//! Chain execution engine for request chaining
//!
//! This module provides the execution engine that manages chain execution with
//! dependency resolution, parallel execution when possible, and proper error handling.

use crate::request_chaining::{
    ChainConfig, ChainDefinition, ChainExecutionContext, ChainLink, ChainResponse,
    ChainTemplatingContext, RequestChainRegistry,
};
use crate::request_scripting::{ScriptContext, ScriptEngine};
use crate::templating::{expand_str_with_context, TemplatingContext};
use crate::{Error, Result};
use chrono::Utc;
use futures::future::join_all;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

/// Record of a chain execution with timestamp
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// ISO 8601 timestamp when the chain was executed
    pub executed_at: String,
    /// Result of the chain execution
    pub result: ChainExecutionResult,
}

/// Engine for executing request chains
#[derive(Debug)]
pub struct ChainExecutionEngine {
    /// HTTP client for making requests
    http_client: Client,
    /// Chain registry
    registry: Arc<RequestChainRegistry>,
    /// Global configuration
    config: ChainConfig,
    /// Execution history storage (chain_id -> Vec<ExecutionRecord>)
    execution_history: Arc<Mutex<HashMap<String, Vec<ExecutionRecord>>>>,
    /// JavaScript scripting engine for pre/post request scripts
    script_engine: ScriptEngine,
}

impl ChainExecutionEngine {
    /// Create a new chain execution engine
    ///
    /// # Panics
    ///
    /// This method will panic if the HTTP client cannot be created, which typically
    /// indicates a system configuration issue. For better error handling, use `try_new()`.
    pub fn new(registry: Arc<RequestChainRegistry>, config: ChainConfig) -> Self {
        Self::try_new(registry, config)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create HTTP client for chain execution engine: {}. \
                    This typically indicates a system configuration issue (e.g., invalid timeout value).",
                    e
                )
            })
    }

    /// Try to create a new chain execution engine
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn try_new(registry: Arc<RequestChainRegistry>, config: ChainConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.global_timeout_secs))
            .build()
            .map_err(|e| {
                Error::generic(format!(
                    "Failed to create HTTP client: {}. \
                Check that the timeout value ({}) is valid.",
                    e, config.global_timeout_secs
                ))
            })?;

        Ok(Self {
            http_client,
            registry,
            config,
            execution_history: Arc::new(Mutex::new(HashMap::new())),
            script_engine: ScriptEngine::new(),
        })
    }

    /// Execute a chain by ID
    pub async fn execute_chain(
        &self,
        chain_id: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<ChainExecutionResult> {
        let chain = self
            .registry
            .get_chain(chain_id)
            .await
            .ok_or_else(|| Error::generic(format!("Chain '{}' not found", chain_id)))?;

        let result = self.execute_chain_definition(&chain, variables).await?;

        // Store execution in history
        let record = ExecutionRecord {
            executed_at: Utc::now().to_rfc3339(),
            result: result.clone(),
        };

        let mut history = self.execution_history.lock().await;
        history.entry(chain_id.to_string()).or_insert_with(Vec::new).push(record);

        Ok(result)
    }

    /// Get execution history for a chain
    pub async fn get_chain_history(&self, chain_id: &str) -> Vec<ExecutionRecord> {
        let history = self.execution_history.lock().await;
        history.get(chain_id).cloned().unwrap_or_default()
    }

    /// Execute a chain definition
    pub async fn execute_chain_definition(
        &self,
        chain_def: &ChainDefinition,
        variables: Option<serde_json::Value>,
    ) -> Result<ChainExecutionResult> {
        // First validate the chain
        self.registry.validate_chain(chain_def).await?;

        let start_time = std::time::Instant::now();
        let mut execution_context = ChainExecutionContext::new(chain_def.clone());

        // Initialize context with chain variables
        for (key, value) in &chain_def.variables {
            execution_context
                .templating
                .chain_context
                .set_variable(key.clone(), value.clone());
        }

        // Merge custom variables from request
        if let Some(serde_json::Value::Object(map)) = variables {
            for (key, value) in map {
                execution_context.templating.chain_context.set_variable(key, value);
            }
        }

        if self.config.enable_parallel_execution {
            self.execute_with_parallelism(&mut execution_context).await
        } else {
            self.execute_sequential(&mut execution_context).await
        }
        .map(|_| ChainExecutionResult {
            chain_id: chain_def.id.clone(),
            status: ChainExecutionStatus::Successful,
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            request_results: execution_context.templating.chain_context.responses.clone(),
            error_message: None,
        })
    }

    /// Execute chain using topological sorting for parallelism
    async fn execute_with_parallelism(
        &self,
        execution_context: &mut ChainExecutionContext,
    ) -> Result<()> {
        let dep_graph = self.build_dependency_graph(&execution_context.definition.links);
        let topo_order = self.topological_sort(&dep_graph)?;

        // Group requests by dependency level
        let mut level_groups = vec![];
        let mut processed = HashSet::new();

        for request_id in topo_order {
            if !processed.contains(&request_id) {
                let mut level = vec![];
                self.collect_dependency_level(request_id, &dep_graph, &mut level, &mut processed);
                level_groups.push(level);
            }
        }

        // Execute levels in parallel
        for level in level_groups {
            if level.len() == 1 {
                // Single request, execute directly
                let request_id = &level[0];
                let link = execution_context
                    .definition
                    .links
                    .iter()
                    .find(|l| l.request.id == *request_id)
                    .unwrap();

                let link_clone = link.clone();
                self.execute_request(&link_clone, execution_context).await?;
            } else {
                // Execute level in parallel
                let tasks = level
                    .into_iter()
                    .map(|request_id| {
                        let link = execution_context
                            .definition
                            .links
                            .iter()
                            .find(|l| l.request.id == request_id)
                            .unwrap()
                            .clone();
                        // Create a new context for parallel execution
                        let parallel_context = ChainExecutionContext {
                            definition: execution_context.definition.clone(),
                            templating: execution_context.templating.clone(),
                            start_time: std::time::Instant::now(),
                            config: execution_context.config.clone(),
                        };

                        let context = Arc::new(Mutex::new(parallel_context));
                        let engine =
                            ChainExecutionEngine::new(self.registry.clone(), self.config.clone());

                        tokio::spawn(async move {
                            let mut ctx = context.lock().await;
                            engine.execute_request(&link, &mut ctx).await
                        })
                    })
                    .collect::<Vec<_>>();

                let results = join_all(tasks).await;
                for result in results {
                    result
                        .map_err(|e| Error::generic(format!("Task join error: {}", e)))?
                        .map_err(|e| Error::generic(format!("Request execution error: {}", e)))?;
                }
            }
        }

        Ok(())
    }

    /// Execute requests sequentially
    async fn execute_sequential(
        &self,
        execution_context: &mut ChainExecutionContext,
    ) -> Result<()> {
        let links = execution_context.definition.links.clone();
        for link in &links {
            self.execute_request(link, execution_context).await?;
        }
        Ok(())
    }

    /// Execute a single request in the chain
    async fn execute_request(
        &self,
        link: &ChainLink,
        execution_context: &mut ChainExecutionContext,
    ) -> Result<()> {
        let request_start = std::time::Instant::now();

        // Prepare the request with templating
        execution_context.templating.set_current_request(link.request.clone());

        let method = Method::from_bytes(link.request.method.as_bytes()).map_err(|e| {
            Error::generic(format!("Invalid HTTP method '{}': {}", link.request.method, e))
        })?;

        let url = self.expand_template(&link.request.url, &execution_context.templating);

        // Prepare headers
        let mut headers = HeaderMap::new();
        for (key, value) in &link.request.headers {
            let expanded_value = self.expand_template(value, &execution_context.templating);
            let header_name = HeaderName::from_str(key)
                .map_err(|e| Error::generic(format!("Invalid header name '{}': {}", key, e)))?;
            let header_value = HeaderValue::from_str(&expanded_value).map_err(|e| {
                Error::generic(format!("Invalid header value for '{}': {}", key, e))
            })?;
            headers.insert(header_name, header_value);
        }

        // Prepare request builder
        let mut request_builder = self.http_client.request(method, &url).headers(headers.clone());

        // Add body if present
        if let Some(body) = &link.request.body {
            match body {
                crate::request_chaining::RequestBody::Json(json_value) => {
                    let expanded_body =
                        self.expand_template_in_json(json_value, &execution_context.templating);
                    request_builder = request_builder.json(&expanded_body);
                }
                crate::request_chaining::RequestBody::BinaryFile { path, content_type } => {
                    // Create templating context for path expansion
                    let templating_context =
                        TemplatingContext::with_chain(execution_context.templating.clone());

                    // Expand templates in the file path
                    let expanded_path = expand_str_with_context(path, &templating_context);

                    // Create a new body with expanded path
                    let binary_body = crate::request_chaining::RequestBody::binary_file(
                        expanded_path,
                        content_type.clone(),
                    );

                    // Read the binary file
                    match binary_body.to_bytes().await {
                        Ok(file_bytes) => {
                            request_builder = request_builder.body(file_bytes);

                            // Set content type if specified
                            if let Some(ct) = content_type {
                                let mut headers = headers.clone();
                                headers.insert(
                                    "content-type",
                                    ct.parse().unwrap_or_else(|_| {
                                        reqwest::header::HeaderValue::from_static(
                                            "application/octet-stream",
                                        )
                                    }),
                                );
                                request_builder = request_builder.headers(headers);
                            }
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Set timeout if specified
        if let Some(timeout_secs) = link.request.timeout_secs {
            request_builder = request_builder.timeout(Duration::from_secs(timeout_secs));
        }

        // Execute pre-request script if configured
        if let Some(scripting) = &link.request.scripting {
            if let Some(pre_script) = &scripting.pre_script {
                let script_context = ScriptContext {
                    request: Some(link.request.clone()),
                    response: None,
                    chain_context: execution_context.templating.chain_context.variables.clone(),
                    variables: HashMap::new(),
                    env_vars: std::env::vars().collect(),
                };

                match self
                    .script_engine
                    .execute_script(pre_script, &script_context, scripting.timeout_ms)
                    .await
                {
                    Ok(script_result) => {
                        // Merge script-modified variables into chain context
                        for (key, value) in script_result.modified_variables {
                            execution_context.templating.chain_context.set_variable(key, value);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Pre-script execution failed for request '{}': {}",
                            link.request.id,
                            e
                        );
                        // Continue execution even if script fails
                    }
                }
            }
        }

        // Execute the request
        let response_result =
            timeout(Duration::from_secs(self.config.global_timeout_secs), request_builder.send())
                .await;

        let response = match response_result {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                return Err(Error::generic(format!("Request '{}' failed: {}", link.request.id, e)));
            }
            Err(_) => {
                return Err(Error::generic(format!("Request '{}' timed out", link.request.id)));
            }
        };

        let status = response.status();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body_text = response.text().await.unwrap_or_default();
        let body_json: Option<Value> = serde_json::from_str(&body_text).ok();

        let duration_ms = request_start.elapsed().as_millis() as u64;
        let executed_at = Utc::now().to_rfc3339();

        let chain_response = ChainResponse {
            status: status.as_u16(),
            headers,
            body: body_json,
            duration_ms,
            executed_at,
            error: None,
        };

        // Validate expected status if specified
        if let Some(expected) = &link.request.expected_status {
            if !expected.contains(&status.as_u16()) {
                let error_msg = format!(
                    "Request '{}' returned status {} but expected one of {:?}",
                    link.request.id,
                    status.as_u16(),
                    expected
                );
                return Err(Error::generic(error_msg));
            }
        }

        // Store the response
        if let Some(store_name) = &link.store_as {
            execution_context
                .templating
                .chain_context
                .store_response(store_name.clone(), chain_response.clone());
        }

        // Extract variables from response
        for (var_name, extraction_path) in &link.extract {
            if let Some(value) = self.extract_from_response(&chain_response, extraction_path) {
                execution_context.templating.chain_context.set_variable(var_name.clone(), value);
            }
        }

        // Execute post-request script if configured
        if let Some(scripting) = &link.request.scripting {
            if let Some(post_script) = &scripting.post_script {
                let script_context = ScriptContext {
                    request: Some(link.request.clone()),
                    response: Some(chain_response.clone()),
                    chain_context: execution_context.templating.chain_context.variables.clone(),
                    variables: HashMap::new(),
                    env_vars: std::env::vars().collect(),
                };

                match self
                    .script_engine
                    .execute_script(post_script, &script_context, scripting.timeout_ms)
                    .await
                {
                    Ok(script_result) => {
                        // Merge script-modified variables into chain context
                        for (key, value) in script_result.modified_variables {
                            execution_context.templating.chain_context.set_variable(key, value);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Post-script execution failed for request '{}': {}",
                            link.request.id,
                            e
                        );
                        // Continue execution even if script fails
                    }
                }
            }
        }

        // Also store by request ID as fallback
        execution_context
            .templating
            .chain_context
            .store_response(link.request.id.clone(), chain_response);

        Ok(())
    }

    /// Build dependency graph from chain links
    fn build_dependency_graph(&self, links: &[ChainLink]) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        for link in links {
            graph
                .entry(link.request.id.clone())
                .or_insert_with(Vec::new)
                .extend(link.request.depends_on.iter().cloned());
        }

        graph
    }

    /// Perform topological sort on dependency graph
    fn topological_sort(&self, graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut result = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                self.topo_sort_util(node, graph, &mut visited, &mut rec_stack, &mut result)?;
            }
        }

        result.reverse();
        Ok(result)
    }

    /// Utility function for topological sort
    #[allow(clippy::only_used_in_recursion)]
    fn topo_sort_util(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(dependencies) = graph.get(node) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    self.topo_sort_util(dep, graph, visited, rec_stack, result)?;
                } else if rec_stack.contains(dep) {
                    return Err(Error::generic(format!(
                        "Circular dependency detected involving '{}'",
                        node
                    )));
                }
            }
        }

        rec_stack.remove(node);
        result.push(node.to_string());
        Ok(())
    }

    /// Collect requests that can be executed in parallel (same dependency level)
    fn collect_dependency_level(
        &self,
        request_id: String,
        _graph: &HashMap<String, Vec<String>>,
        level: &mut Vec<String>,
        processed: &mut HashSet<String>,
    ) {
        level.push(request_id.clone());
        processed.insert(request_id);
    }

    /// Expand template string with chain context
    fn expand_template(&self, template: &str, context: &ChainTemplatingContext) -> String {
        let templating_context = crate::templating::TemplatingContext {
            chain_context: Some(context.clone()),
            env_context: None,
            virtual_clock: None,
        };
        crate::templating::expand_str_with_context(template, &templating_context)
    }

    /// Expand template variables in JSON value
    fn expand_template_in_json(&self, value: &Value, context: &ChainTemplatingContext) -> Value {
        match value {
            Value::String(s) => Value::String(self.expand_template(s, context)),
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.expand_template_in_json(v, context)).collect())
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(
                        self.expand_template(k, context),
                        self.expand_template_in_json(v, context),
                    );
                }
                Value::Object(new_map)
            }
            _ => value.clone(),
        }
    }

    /// Extract value from response using JSON path-like syntax
    fn extract_from_response(&self, response: &ChainResponse, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() || parts[0] != "body" {
            return None;
        }

        let mut current = response.body.as_ref()?;

        for part in &parts[1..] {
            match current {
                Value::Object(map) => {
                    current = map.get(*part)?;
                }
                Value::Array(arr) => {
                    if part.starts_with('[') && part.ends_with(']') {
                        let index_str = &part[1..part.len() - 1];
                        if let Ok(index) = index_str.parse::<usize>() {
                            current = arr.get(index)?;
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }
}

/// Result of executing a request chain
#[derive(Debug, Clone)]
pub struct ChainExecutionResult {
    /// Unique identifier for the executed chain
    pub chain_id: String,
    /// Overall execution status
    pub status: ChainExecutionStatus,
    /// Total duration of chain execution in milliseconds
    pub total_duration_ms: u64,
    /// Results of individual requests in the chain, keyed by request ID
    pub request_results: HashMap<String, ChainResponse>,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

/// Status of chain execution
#[derive(Debug, Clone, PartialEq)]
pub enum ChainExecutionStatus {
    /// All requests in the chain succeeded
    Successful,
    /// Some requests succeeded but others failed
    PartialSuccess,
    /// Chain execution failed
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_engine_creation() {
        let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
        let _engine = ChainExecutionEngine::new(registry, ChainConfig::default());

        // Engine should be created successfully
    }

    #[tokio::test]
    async fn test_topological_sort() {
        let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
        let engine = ChainExecutionEngine::new(registry, ChainConfig::default());

        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec![]);
        graph.insert("B".to_string(), vec!["A".to_string()]);
        graph.insert("C".to_string(), vec!["A".to_string()]);
        graph.insert("D".to_string(), vec!["B".to_string(), "C".to_string()]);

        let topo_order = engine.topological_sort(&graph).unwrap();

        // Verify this is a valid topological ordering
        // D should come before B and C (its dependencies)
        // B should come before A (its dependency)
        // C should come before A (its dependency)
        let d_pos = topo_order.iter().position(|x| x == "D").unwrap();
        let b_pos = topo_order.iter().position(|x| x == "B").unwrap();
        let c_pos = topo_order.iter().position(|x| x == "C").unwrap();
        let a_pos = topo_order.iter().position(|x| x == "A").unwrap();

        assert!(d_pos < b_pos, "D should come before B");
        assert!(d_pos < c_pos, "D should come before C");
        assert!(b_pos < a_pos, "B should come before A");
        assert!(c_pos < a_pos, "C should come before A");
        assert_eq!(topo_order.len(), 4, "Should have all 4 nodes");
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let registry = Arc::new(RequestChainRegistry::new(ChainConfig::default()));
        let engine = ChainExecutionEngine::new(registry, ChainConfig::default());

        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string()]);
        graph.insert("B".to_string(), vec!["A".to_string()]); // Circular dependency

        let result = engine.topological_sort(&graph);
        assert!(result.is_err());
    }
}
