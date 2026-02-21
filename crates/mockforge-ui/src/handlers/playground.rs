//! Playground API handlers for Admin UI
//!
//! Provides endpoints for the interactive GraphQL + REST playground that allows
//! users to test and visualize mock endpoints.

use axum::{
    extract::{Path, State},
    response::Json,
};
use chrono::Utc;
use mockforge_core::request_logger::{get_global_logger, RequestLogEntry};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::handlers::AdminState;
use crate::models::ApiResponse;

/// Endpoint information for playground
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundEndpoint {
    /// Protocol type (rest, graphql)
    pub protocol: String,
    /// HTTP method (for REST) or operation type (for GraphQL)
    pub method: String,
    /// Endpoint path or GraphQL operation name
    pub path: String,
    /// Optional description
    pub description: Option<String>,
    /// Whether this endpoint is enabled
    pub enabled: bool,
}

/// Request to execute a REST endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRestRequest {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Optional request headers
    pub headers: Option<HashMap<String, String>>,
    /// Optional request body
    pub body: Option<Value>,
    /// Base URL (defaults to HTTP server address)
    pub base_url: Option<String>,
    /// Whether to use MockAI for response generation
    #[serde(default)]
    pub use_mockai: bool,
    /// Optional workspace ID for workspace-scoped requests
    pub workspace_id: Option<String>,
}

/// Request to execute a GraphQL query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteGraphQLRequest {
    /// GraphQL query string
    pub query: String,
    /// Optional variables
    pub variables: Option<HashMap<String, Value>>,
    /// Optional operation name
    pub operation_name: Option<String>,
    /// Base URL (defaults to GraphQL server address)
    pub base_url: Option<String>,
    /// Optional workspace ID for workspace-scoped requests
    pub workspace_id: Option<String>,
}

/// Response from executing a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResponse {
    /// Response status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Value,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Request ID for history tracking
    pub request_id: String,
    /// Error message if any
    pub error: Option<String>,
}

/// GraphQL introspection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLIntrospectionResult {
    /// Full introspection query result
    pub schema: Value,
    /// Available query types
    pub query_types: Vec<String>,
    /// Available mutation types
    pub mutation_types: Vec<String>,
    /// Available subscription types
    pub subscription_types: Vec<String>,
}

/// Request history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundHistoryEntry {
    /// Request ID
    pub id: String,
    /// Protocol type
    pub protocol: String,
    /// HTTP method or GraphQL operation type
    pub method: String,
    /// Request path or GraphQL query
    pub path: String,
    /// Response status code
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
    /// Request headers (for REST)
    pub request_headers: Option<HashMap<String, String>>,
    /// Request body (for REST)
    pub request_body: Option<Value>,
    /// GraphQL query (for GraphQL)
    pub graphql_query: Option<String>,
    /// GraphQL variables (for GraphQL)
    pub graphql_variables: Option<HashMap<String, Value>>,
}

/// Code snippet generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippetRequest {
    /// Protocol type
    pub protocol: String,
    /// HTTP method (for REST)
    pub method: Option<String>,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: Option<HashMap<String, String>>,
    /// Request body
    pub body: Option<Value>,
    /// GraphQL query (for GraphQL)
    pub graphql_query: Option<String>,
    /// GraphQL variables (for GraphQL)
    pub graphql_variables: Option<HashMap<String, Value>>,
    /// Base URL
    pub base_url: String,
}

/// Code snippet response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippetResponse {
    /// Generated code snippets by language
    pub snippets: HashMap<String, String>,
}

/// List available endpoints for playground
pub async fn list_playground_endpoints(
    State(state): State<AdminState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<PlaygroundEndpoint>>> {
    // Get workspace_id from query params (optional)
    let workspace_id = params.get("workspace_id");
    let mut endpoints = Vec::new();

    // Get REST endpoints from HTTP server
    if let Some(http_addr) = state.http_server_addr {
        let mut url = format!("http://{}/__mockforge/routes", http_addr);

        // Add workspace_id to route query if provided
        if let Some(ws_id) = workspace_id {
            url = format!("{}?workspace_id={}", url, ws_id);
        }

        if let Ok(response) = reqwest::get(&url).await {
            if response.status().is_success() {
                if let Ok(body) = response.json::<Value>().await {
                    if let Some(routes) = body.get("routes").and_then(|r| r.as_array()) {
                        for route in routes {
                            // Filter by workspace if workspace_id is provided and route has workspace info
                            if let Some(ws_id) = workspace_id {
                                if let Some(route_workspace) =
                                    route.get("workspace_id").and_then(|w| w.as_str())
                                {
                                    if route_workspace != ws_id {
                                        continue; // Skip routes not belonging to this workspace
                                    }
                                }
                            }

                            if let (Some(method), Some(path)) = (
                                route.get("method").and_then(|m| m.as_str()),
                                route.get("path").and_then(|p| p.as_str()),
                            ) {
                                endpoints.push(PlaygroundEndpoint {
                                    protocol: "rest".to_string(),
                                    method: method.to_string(),
                                    path: path.to_string(),
                                    description: route
                                        .get("description")
                                        .and_then(|d| d.as_str())
                                        .map(|s| s.to_string()),
                                    enabled: true,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Get GraphQL endpoint if GraphQL server is available
    if state.graphql_server_addr.is_some() {
        endpoints.push(PlaygroundEndpoint {
            protocol: "graphql".to_string(),
            method: "query".to_string(),
            path: "/graphql".to_string(),
            description: Some("GraphQL endpoint".to_string()),
            enabled: true,
        });
    }

    Json(ApiResponse::success(endpoints))
}

/// Execute a REST request
pub async fn execute_rest_request(
    State(state): State<AdminState>,
    Json(request): Json<ExecuteRestRequest>,
) -> Json<ApiResponse<ExecuteResponse>> {
    let start_time = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    // Determine base URL
    let base_url = request.base_url.unwrap_or_else(|| {
        state
            .http_server_addr
            .map(|addr| format!("http://{}", addr))
            .unwrap_or_else(|| "http://localhost:3000".to_string())
    });

    // Build full URL
    let url = if request.path.starts_with("http") {
        request.path.clone()
    } else {
        format!("{}{}", base_url, request.path)
    };

    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Build request
    let mut http_request = match request.method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => {
            return Json(ApiResponse::error(format!(
                "Unsupported HTTP method: {}",
                request.method
            )));
        }
    };

    // Add headers
    let mut headers = request.headers.clone().unwrap_or_default();

    // Add MockAI preview header if requested
    if request.use_mockai {
        headers.insert("X-MockAI-Preview".to_string(), "true".to_string());
    }

    // Add workspace ID header if provided
    if let Some(ws_id) = &request.workspace_id {
        headers.insert("X-Workspace-ID".to_string(), ws_id.clone());
    }

    for (key, value) in &headers {
        http_request = http_request.header(key, value);
    }

    // Add body
    if let Some(body) = &request.body {
        http_request = http_request.json(body);
    }

    // Execute request
    let response = http_request.send().await;

    let response_time_ms = start_time.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status().as_u16();

            // Get response headers
            let mut headers = HashMap::new();
            for (key, value) in resp.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key.to_string(), value_str.to_string());
                }
            }

            // Get response body
            let body = resp
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "error": "Failed to parse response as JSON" }));

            // Log request
            if let Some(logger) = get_global_logger() {
                // Store workspace_id in metadata for filtering
                let mut metadata = HashMap::new();
                if let Some(ws_id) =
                    request.workspace_id.as_ref().or_else(|| headers.get("X-Workspace-ID"))
                {
                    metadata.insert("workspace_id".to_string(), ws_id.clone());
                }

                let log_entry = RequestLogEntry {
                    id: request_id.clone(),
                    timestamp: Utc::now(),
                    server_type: "http".to_string(),
                    method: request.method.clone(),
                    path: request.path.clone(),
                    status_code,
                    response_time_ms,
                    client_ip: None,
                    user_agent: Some("MockForge-Playground".to_string()),
                    headers: headers.clone(),
                    query_params: HashMap::new(), // Query params not available in playground
                    response_size_bytes: serde_json::to_string(&body)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0),
                    error_message: None,
                    metadata,
                    reality_metadata: None,
                };
                logger.log_request(log_entry).await;
            }

            Json(ApiResponse::success(ExecuteResponse {
                status_code,
                headers,
                body: body.clone(),
                response_time_ms,
                request_id,
                error: None,
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            Json(ApiResponse::success(ExecuteResponse {
                status_code: 0,
                headers: HashMap::new(),
                body: json!({ "error": error_msg }),
                response_time_ms,
                request_id,
                error: Some(error_msg),
            }))
        }
    }
}

/// Execute a GraphQL query
pub async fn execute_graphql_query(
    State(state): State<AdminState>,
    Json(request): Json<ExecuteGraphQLRequest>,
) -> Json<ApiResponse<ExecuteResponse>> {
    let start_time = std::time::Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();

    // Determine base URL
    let base_url = request.base_url.unwrap_or_else(|| {
        state
            .graphql_server_addr
            .map(|addr| format!("http://{}", addr))
            .unwrap_or_else(|| "http://localhost:4000".to_string())
    });

    // Build GraphQL request
    let mut graphql_body = json!({
        "query": request.query
    });

    if let Some(variables) = &request.variables {
        graphql_body["variables"] = json!(variables);
    }

    if let Some(operation_name) = &request.operation_name {
        graphql_body["operationName"] = json!(operation_name);
    }

    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Execute GraphQL request
    let url = format!("{}/graphql", base_url);
    let mut graphql_request = client.post(&url).header("Content-Type", "application/json");

    // Add workspace ID header if provided
    if let Some(ws_id) = &request.workspace_id {
        graphql_request = graphql_request.header("X-Workspace-ID", ws_id);
    }

    let response = graphql_request.json(&graphql_body).send().await;

    let response_time_ms = start_time.elapsed().as_millis() as u64;

    match response {
        Ok(resp) => {
            let status_code = resp.status().as_u16();

            // Get response headers
            let mut headers = HashMap::new();
            for (key, value) in resp.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers.insert(key.to_string(), value_str.to_string());
                }
            }

            // Get response body
            let body = resp
                .json::<Value>()
                .await
                .unwrap_or_else(|_| json!({ "error": "Failed to parse response as JSON" }));

            // Log request
            if let Some(logger) = get_global_logger() {
                // Store workspace_id and GraphQL query/variables in metadata
                let mut metadata = HashMap::new();
                if let Some(ws_id) = &request.workspace_id {
                    metadata.insert("workspace_id".to_string(), ws_id.clone());
                }
                metadata.insert("query".to_string(), request.query.clone());
                if let Some(variables) = &request.variables {
                    if let Ok(vars_str) = serde_json::to_string(variables) {
                        metadata.insert("variables".to_string(), vars_str);
                    }
                }

                let has_errors = body.get("errors").is_some();
                let log_entry = RequestLogEntry {
                    id: request_id.clone(),
                    timestamp: Utc::now(),
                    server_type: "graphql".to_string(),
                    method: "POST".to_string(),
                    path: "/graphql".to_string(),
                    status_code,
                    response_time_ms,
                    client_ip: None,
                    user_agent: Some("MockForge-Playground".to_string()),
                    headers: HashMap::new(),
                    query_params: HashMap::new(), // Query params not available in playground
                    response_size_bytes: serde_json::to_string(&body)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0),
                    error_message: if has_errors {
                        Some("GraphQL errors in response".to_string())
                    } else {
                        None
                    },
                    reality_metadata: None,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("query".to_string(), request.query.clone());
                        if let Some(vars) = &request.variables {
                            if let Ok(vars_str) = serde_json::to_string(vars) {
                                meta.insert("variables".to_string(), vars_str);
                            }
                        }
                        meta
                    },
                };
                logger.log_request(log_entry).await;
            }

            let has_errors = body.get("errors").is_some();
            Json(ApiResponse::success(ExecuteResponse {
                status_code,
                headers,
                body: body.clone(),
                response_time_ms,
                request_id,
                error: if has_errors {
                    Some("GraphQL errors in response".to_string())
                } else {
                    None
                },
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            Json(ApiResponse::success(ExecuteResponse {
                status_code: 0,
                headers: HashMap::new(),
                body: json!({ "error": error_msg }),
                response_time_ms,
                request_id,
                error: Some(error_msg),
            }))
        }
    }
}

/// Perform GraphQL introspection
pub async fn graphql_introspect(
    State(state): State<AdminState>,
) -> Json<ApiResponse<GraphQLIntrospectionResult>> {
    // Determine base URL
    let base_url = state
        .graphql_server_addr
        .map(|addr| format!("http://{}", addr))
        .unwrap_or_else(|| "http://localhost:4000".to_string());

    // Standard GraphQL introspection query
    let introspection_query = r#"
        query IntrospectionQuery {
            __schema {
                queryType { name }
                mutationType { name }
                subscriptionType { name }
                types {
                    ...FullType
                }
                directives {
                    name
                    description
                    locations
                    args {
                        ...InputValue
                    }
                }
            }
        }

        fragment FullType on __Type {
            kind
            name
            description
            fields(includeDeprecated: true) {
                name
                description
                args {
                    ...InputValue
                }
                type {
                    ...TypeRef
                }
                isDeprecated
                deprecationReason
            }
            inputFields {
                ...InputValue
            }
            interfaces {
                ...TypeRef
            }
            enumValues(includeDeprecated: true) {
                name
                description
                isDeprecated
                deprecationReason
            }
            possibleTypes {
                ...TypeRef
            }
        }

        fragment InputValue on __InputValue {
            name
            description
            type {
                ...TypeRef
            }
            defaultValue
        }

        fragment TypeRef on __Type {
            kind
            name
            ofType {
                kind
                name
                ofType {
                    kind
                    name
                    ofType {
                        kind
                        name
                        ofType {
                            kind
                            name
                            ofType {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                    ofType {
                                        kind
                                        name
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = format!("{}/graphql", base_url);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": introspection_query
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(data) = body.get("data").and_then(|d| d.get("__schema")) {
                    let schema = data.clone();

                    // Extract query, mutation, and subscription types
                    let query_types = schema
                        .get("queryType")
                        .and_then(|q| q.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|_| vec!["Query".to_string()])
                        .unwrap_or_default();

                    let mutation_types = schema
                        .get("mutationType")
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|_| vec!["Mutation".to_string()])
                        .unwrap_or_default();

                    let subscription_types = schema
                        .get("subscriptionType")
                        .and_then(|s| s.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|_| vec!["Subscription".to_string()])
                        .unwrap_or_default();

                    Json(ApiResponse::success(GraphQLIntrospectionResult {
                        schema: schema.clone(),
                        query_types,
                        mutation_types,
                        subscription_types,
                    }))
                } else {
                    Json(ApiResponse::error("Failed to parse introspection response".to_string()))
                }
            } else {
                Json(ApiResponse::error("Failed to parse response".to_string()))
            }
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to execute introspection query: {}", e))),
    }
}

/// Get request history
pub async fn get_request_history(
    State(_state): State<AdminState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<PlaygroundHistoryEntry>>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    // Get limit from query params
    let limit = params.get("limit").and_then(|l| l.parse::<usize>().ok()).unwrap_or(100);

    // Get protocol filter
    let protocol_filter = params.get("protocol");

    // Get workspace_id filter
    let workspace_id_filter = params.get("workspace_id");

    // Get logs
    let mut logs = if let Some(protocol) = protocol_filter {
        logger
            .get_logs_by_server(protocol, Some(limit * 2)) // Get more to account for filtering
            .await
    } else {
        logger.get_recent_logs(Some(limit * 2)).await
    };

    // Filter by workspace_id if provided
    if let Some(ws_id) = workspace_id_filter {
        logs.retain(|log| log.metadata.get("workspace_id").map(|w| w == ws_id).unwrap_or(false));
    }

    // Limit after filtering
    logs.truncate(limit);

    // Convert to playground history entries
    let history: Vec<PlaygroundHistoryEntry> = logs
        .into_iter()
        .map(|log| {
            // Extract GraphQL query and variables from metadata
            let graphql_query = log.metadata.get("query").cloned();
            let graphql_variables = log
                .metadata
                .get("variables")
                .and_then(|v| serde_json::from_str::<HashMap<String, Value>>(v).ok());

            PlaygroundHistoryEntry {
                id: log.id,
                protocol: log.server_type.clone(),
                method: log.method.clone(),
                path: log.path.clone(),
                status_code: log.status_code,
                response_time_ms: log.response_time_ms,
                timestamp: log.timestamp,
                request_headers: if log.server_type == "http" {
                    Some(log.headers.clone())
                } else {
                    None
                },
                request_body: None, // Request body not stored in logs currently
                graphql_query,
                graphql_variables,
            }
        })
        .collect();

    Json(ApiResponse::success(history))
}

/// Replay a request from history
pub async fn replay_request(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> Json<ApiResponse<ExecuteResponse>> {
    let logger = match get_global_logger() {
        Some(logger) => logger,
        None => {
            return Json(ApiResponse::error("Request logger not initialized".to_string()));
        }
    };

    // Get all logs and find the one with matching ID
    let logs = logger.get_recent_logs(None).await;
    let log_entry = logs.into_iter().find(|log| log.id == id);

    match log_entry {
        Some(log) => {
            if log.server_type == "graphql" {
                // Replay GraphQL request
                if let Some(query) = log.metadata.get("query") {
                    let variables = log
                        .metadata
                        .get("variables")
                        .and_then(|v| serde_json::from_str::<HashMap<String, Value>>(v).ok());

                    let graphql_request = ExecuteGraphQLRequest {
                        query: query.clone(),
                        variables,
                        operation_name: None,
                        base_url: None,
                        workspace_id: log.metadata.get("workspace_id").cloned(),
                    };

                    execute_graphql_query(State(state), Json(graphql_request)).await
                } else {
                    Json(ApiResponse::error("GraphQL query not found in log entry".to_string()))
                }
            } else {
                // Replay REST request
                let rest_request = ExecuteRestRequest {
                    method: log.method.clone(),
                    path: log.path.clone(),
                    headers: Some(log.headers.clone()),
                    body: None, // Request body not stored in logs
                    base_url: None,
                    use_mockai: false,
                    workspace_id: log.metadata.get("workspace_id").cloned(),
                };

                execute_rest_request(State(state), Json(rest_request)).await
            }
        }
        None => Json(ApiResponse::error(format!("Request with ID {} not found", id))),
    }
}

/// Generate code snippets
pub async fn generate_code_snippet(
    State(_state): State<AdminState>,
    Json(request): Json<CodeSnippetRequest>,
) -> Json<ApiResponse<CodeSnippetResponse>> {
    let mut snippets = HashMap::new();

    if request.protocol == "rest" {
        // Generate curl snippet
        let mut curl_parts = vec!["curl".to_string()];
        if let Some(method) = &request.method {
            if method != "GET" {
                curl_parts.push(format!("-X {}", method));
            }
        }

        if let Some(headers) = &request.headers {
            for (key, value) in headers {
                curl_parts.push(format!("-H \"{}: {}\"", key, value));
            }
        }

        if let Some(body) = &request.body {
            curl_parts.push(format!("-d '{}'", serde_json::to_string(body).unwrap_or_default()));
        }

        let url = if request.path.starts_with("http") {
            request.path.clone()
        } else {
            format!("{}{}", request.base_url, request.path)
        };
        curl_parts.push(format!("\"{}\"", url));

        snippets.insert("curl".to_string(), curl_parts.join(" \\\n  "));

        // Generate JavaScript fetch snippet
        let mut js_code = String::new();
        js_code.push_str("fetch(");
        js_code.push_str(&format!("\"{}\"", url));
        js_code.push_str(", {\n");

        if let Some(method) = &request.method {
            js_code.push_str(&format!("  method: \"{}\",\n", method));
        }

        if let Some(headers) = &request.headers {
            js_code.push_str("  headers: {\n");
            for (key, value) in headers {
                js_code.push_str(&format!("    \"{}\": \"{}\",\n", key, value));
            }
            js_code.push_str("  },\n");
        }

        if let Some(body) = &request.body {
            js_code.push_str(&format!(
                "  body: JSON.stringify({}),\n",
                serde_json::to_string(body).unwrap_or_default()
            ));
        }

        js_code.push_str("})");
        snippets.insert("javascript".to_string(), js_code);

        // Generate Python requests snippet
        let mut python_code = String::new();
        python_code.push_str("import requests\n\n");
        python_code.push_str("response = requests.");

        let method = request.method.as_deref().unwrap_or("get").to_lowercase();
        python_code.push_str(&method);
        python_code.push_str("(\n");
        python_code.push_str(&format!("    \"{}\"", url));

        if let Some(headers) = &request.headers {
            python_code.push_str(",\n    headers={\n");
            for (key, value) in headers {
                python_code.push_str(&format!("        \"{}\": \"{}\",\n", key, value));
            }
            python_code.push_str("    }");
        }

        if let Some(body) = &request.body {
            python_code.push_str(",\n    json=");
            python_code.push_str(&serde_json::to_string(body).unwrap_or_default());
        }

        python_code.push_str("\n)");
        snippets.insert("python".to_string(), python_code);
    } else if request.protocol == "graphql" {
        // Generate GraphQL snippets
        if let Some(query) = &request.graphql_query {
            // curl snippet
            let mut curl_parts = vec!["curl".to_string(), "-X POST".to_string()];
            curl_parts.push("-H \"Content-Type: application/json\"".to_string());

            let mut graphql_body = json!({ "query": query });
            if let Some(vars) = &request.graphql_variables {
                graphql_body["variables"] = json!(vars);
            }

            curl_parts
                .push(format!("-d '{}'", serde_json::to_string(&graphql_body).unwrap_or_default()));
            curl_parts.push(format!("\"{}/graphql\"", request.base_url));

            snippets.insert("curl".to_string(), curl_parts.join(" \\\n  "));

            // JavaScript fetch snippet
            let mut js_code = String::new();
            js_code.push_str("fetch(\"");
            js_code.push_str(&format!("{}/graphql", request.base_url));
            js_code.push_str("\", {\n");
            js_code.push_str("  method: \"POST\",\n");
            js_code.push_str("  headers: {\n");
            js_code.push_str("    \"Content-Type\": \"application/json\",\n");
            js_code.push_str("  },\n");
            js_code.push_str("  body: JSON.stringify({\n");
            js_code.push_str(&format!("    query: `{}`,\n", query.replace('`', "\\`")));
            if let Some(vars) = &request.graphql_variables {
                js_code.push_str("    variables: ");
                js_code.push_str(&serde_json::to_string(vars).unwrap_or_default());
                js_code.push_str(",\n");
            }
            js_code.push_str("  }),\n");
            js_code.push_str("})");
            snippets.insert("javascript".to_string(), js_code);
        }
    }

    Json(ApiResponse::success(CodeSnippetResponse { snippets }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_code_snippet_generation_rest_get() {
        let request = CodeSnippetRequest {
            protocol: "rest".to_string(),
            method: Some("GET".to_string()),
            path: "/api/users".to_string(),
            headers: None,
            body: None,
            graphql_query: None,
            graphql_variables: None,
            base_url: "http://localhost:3000".to_string(),
        };

        // Test that we can serialize the request
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("GET"));
        assert!(serialized.contains("/api/users"));
    }

    #[test]
    fn test_code_snippet_generation_rest_post() {
        let request = CodeSnippetRequest {
            protocol: "rest".to_string(),
            method: Some("POST".to_string()),
            path: "/api/users".to_string(),
            headers: Some({
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            }),
            body: Some(json!({ "name": "John" })),
            graphql_query: None,
            graphql_variables: None,
            base_url: "http://localhost:3000".to_string(),
        };

        // Test that we can serialize the request
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("POST"));
        assert!(serialized.contains("Content-Type"));
    }

    #[test]
    fn test_code_snippet_generation_graphql() {
        let request = CodeSnippetRequest {
            protocol: "graphql".to_string(),
            method: None,
            path: "/graphql".to_string(),
            headers: None,
            body: None,
            graphql_query: Some("query { user(id: 1) { name } }".to_string()),
            graphql_variables: None,
            base_url: "http://localhost:4000".to_string(),
        };

        // Test that we can serialize the request
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("graphql"));
        assert!(serialized.contains("user(id: 1)"));
    }

    #[test]
    fn test_playground_endpoint_serialization() {
        let endpoint = PlaygroundEndpoint {
            protocol: "rest".to_string(),
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            description: Some("Get users".to_string()),
            enabled: true,
        };

        let serialized = serde_json::to_string(&endpoint).unwrap();
        assert!(serialized.contains("rest"));
        assert!(serialized.contains("GET"));
        assert!(serialized.contains("/api/users"));
    }

    #[test]
    fn test_execute_response_serialization() {
        let response = ExecuteResponse {
            status_code: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: json!({ "success": true }),
            response_time_ms: 150,
            request_id: "test-id".to_string(),
            error: None,
        };

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("200"));
        assert!(serialized.contains("test-id"));
    }
}
