//! Mock MCP (Model Context Protocol) server (#913, #79 round 50 follow-up).
//!
//! Serves a JSON-RPC 2.0 endpoint at `POST /mcp` so an agent acting as an MCP
//! client (the role Cursor / Claude Code / custom agents play when they call
//! out to tool servers) can talk to MockForge as a fake MCP server. Answers
//! the core MCP methods with a configurable catalog and canned results:
//! `initialize`, `tools/list`, `tools/call`, `resources/list`,
//! `resources/read`, `resources/templates/list`, `prompts/list`,
//! `prompts/get`, `ping`, plus the `notifications/initialized` notification.
//!
//! This is a MOCK: no real tools run. `tools/call` returns deterministic
//! canned content so agents can be exercised against a predictable (or, with
//! a configured error tool, hostile) MCP server.
//!
//! Round 52 (#79) — Srikanth on 0.3.198 asked whether the mock can "respond
//! with more tools, resources, prompts so that applications are chatty (sees
//! more client-server traffic)". The default catalog now ships a fuller set
//! of tools plus non-empty resources and prompts, and `resources/read` /
//! `prompts/get` are implemented, so a client that lists then reads/gets/calls
//! generates substantially more request/response traffic to capture.
//!
//! Both eras of the protocol are answered, as the public servers of 2026 do.
//! A request that carries the per-request metadata of revision 2026-07-28
//! (`params._meta`, with `server/discover` in place of a handshake) is served
//! on that revision, and the `Mcp-Method` and `MCP-Protocol-Version` headers
//! are checked against the body as the revision requires. A request without
//! that metadata gets the `initialize` handshake that earlier revisions use.
//!
//! Mounted by `mockforge serve --mcp-mock`.

use axum::{
    extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, response::Response,
    routing::post, Json, Router,
};
use serde_json::{json, Value};

/// The protocol revision this mock serves when a request carries the
/// per-request metadata of revision 2026-07-28.
const MODERN_PROTOCOL: &str = "2026-07-28";

/// The protocol revision this mock advertises on the `initialize` handshake
/// that earlier revisions use. Matches the MCP spec revision the reference
/// clients negotiate; clients that send a different version still work
/// because we echo a fixed supported version.
const LEGACY_PROTOCOL: &str = "2024-11-05";

/// `_meta` keys that revision 2026-07-28 requires on every request.
const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// A single mock tool in the catalog.
#[derive(Clone, Debug)]
pub struct McpTool {
    /// Tool name as advertised in `tools/list` and matched by `tools/call`.
    pub name: String,
    /// Human-readable description shown to the agent.
    pub description: String,
    /// JSON Schema for the tool's input (returned verbatim in `tools/list`).
    pub input_schema: Value,
    /// Canned text returned by `tools/call`. When `is_error` is true the call
    /// result is flagged `isError: true` so agents can be tested against a
    /// failing tool.
    pub canned_result: String,
    /// Whether `tools/call` should flag the result as an error (`isError`).
    pub is_error: bool,
}

/// A single mock resource in the catalog.
#[derive(Clone, Debug)]
pub struct McpResource {
    /// Resource URI advertised in `resources/list` and matched by
    /// `resources/read`.
    pub uri: String,
    /// Human-readable resource name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// MIME type advertised for the resource contents.
    pub mime_type: String,
    /// Canned text returned by `resources/read`.
    pub text: String,
}

/// A single argument declared by a mock prompt.
#[derive(Clone, Debug)]
pub struct McpPromptArg {
    /// Argument name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the client must supply this argument.
    pub required: bool,
}

/// A single mock prompt in the catalog.
#[derive(Clone, Debug)]
pub struct McpPrompt {
    /// Prompt name advertised in `prompts/list` and matched by `prompts/get`.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Declared arguments (surfaced in `prompts/list`).
    pub arguments: Vec<McpPromptArg>,
    /// Canned user-message text returned by `prompts/get`.
    pub text: String,
}

/// Runtime configuration for the mock MCP server.
#[derive(Clone, Debug)]
pub struct McpMockConfig {
    /// Server name returned in `initialize` -> `serverInfo.name`.
    pub server_name: String,
    /// Server version returned in `initialize` -> `serverInfo.version`.
    pub server_version: String,
    /// Tool catalog returned by `tools/list` and dispatched by `tools/call`.
    pub tools: Vec<McpTool>,
    /// Resource catalog returned by `resources/list` and read by
    /// `resources/read`.
    pub resources: Vec<McpResource>,
    /// Prompt catalog returned by `prompts/list` and fetched by `prompts/get`.
    pub prompts: Vec<McpPrompt>,
}

impl Default for McpMockConfig {
    fn default() -> Self {
        Self {
            server_name: "mockforge-mcp".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            tools: default_tools(),
            resources: default_resources(),
            prompts: default_prompts(),
        }
    }
}

/// Default tool catalog. Deliberately broad (round 52 #79) so `tools/list`
/// and subsequent `tools/call`s produce meaningful client-server traffic.
fn default_tools() -> Vec<McpTool> {
    let obj = |props: Value, required: Value| {
        json!({
            "type": "object", "properties": props, "required": required,
        })
    };
    vec![
        McpTool {
            name: "echo".to_string(),
            description: "Echo back the provided text.".to_string(),
            input_schema: obj(json!({ "text": { "type": "string" } }), json!(["text"])),
            canned_result: "echo".to_string(),
            is_error: false,
        },
        McpTool {
            name: "get_status".to_string(),
            description: "Return a canned service status payload.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
            canned_result: "{\"status\":\"ok\",\"source\":\"mockforge-mcp\"}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "get_time".to_string(),
            description: "Return a canned current timestamp (UTC, ISO-8601).".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
            canned_result: "{\"now\":\"2026-01-01T00:00:00Z\",\"tz\":\"UTC\"}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "add".to_string(),
            description: "Add two numbers and return their sum.".to_string(),
            input_schema: obj(
                json!({ "a": { "type": "number" }, "b": { "type": "number" } }),
                json!(["a", "b"]),
            ),
            canned_result: "{\"sum\":42}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "search_documents".to_string(),
            description: "Search a canned document index and return matching hits.".to_string(),
            input_schema: obj(
                json!({
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                }),
                json!(["query"]),
            ),
            canned_result: "{\"hits\":[{\"id\":\"doc-1\",\"title\":\"Getting Started\",\"score\":0.98},{\"id\":\"doc-2\",\"title\":\"API Reference\",\"score\":0.71}]}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "get_weather".to_string(),
            description: "Return a canned weather report for a location.".to_string(),
            input_schema: obj(
                json!({ "location": { "type": "string" } }),
                json!(["location"]),
            ),
            canned_result: "{\"location\":\"Sample City\",\"tempC\":21,\"conditions\":\"Partly cloudy\"}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "create_ticket".to_string(),
            description: "Create a canned support ticket and return its id.".to_string(),
            input_schema: obj(
                json!({
                    "title": { "type": "string" },
                    "priority": { "type": "string", "enum": ["low", "medium", "high"] },
                }),
                json!(["title"]),
            ),
            canned_result: "{\"ticketId\":\"TKT-1001\",\"state\":\"open\"}".to_string(),
            is_error: false,
        },
        McpTool {
            name: "fail_tool".to_string(),
            description: "Always returns an error result (for testing hostile tools).".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
            canned_result: "simulated tool failure".to_string(),
            is_error: true,
        },
    ]
}

/// Default resource catalog (round 52 #79). Non-empty so `resources/list`
/// and `resources/read` exercise the resource half of the protocol.
fn default_resources() -> Vec<McpResource> {
    vec![
        McpResource {
            uri: "file:///readme.md".to_string(),
            name: "README".to_string(),
            description: "Project readme served by the mock.".to_string(),
            mime_type: "text/markdown".to_string(),
            text: "# MockForge MCP Mock\n\nCanned readme resource.\n".to_string(),
        },
        McpResource {
            uri: "config://app/settings.json".to_string(),
            name: "App settings".to_string(),
            description: "Canned application settings document.".to_string(),
            mime_type: "application/json".to_string(),
            text: "{\"featureFlags\":{\"beta\":true},\"maxItems\":100}".to_string(),
        },
        McpResource {
            uri: "db://users/schema".to_string(),
            name: "Users schema".to_string(),
            description: "Canned users table schema.".to_string(),
            mime_type: "application/json".to_string(),
            text: "{\"table\":\"users\",\"columns\":[\"id\",\"email\",\"created_at\"]}".to_string(),
        },
        McpResource {
            uri: "log://app/latest".to_string(),
            name: "Latest app log".to_string(),
            description: "Canned tail of the application log.".to_string(),
            mime_type: "text/plain".to_string(),
            text: "2026-01-01T00:00:00Z INFO booted\n2026-01-01T00:00:01Z INFO ready\n".to_string(),
        },
    ]
}

/// Default prompt catalog (round 52 #79). Non-empty so `prompts/list` and
/// `prompts/get` exercise the prompt half of the protocol.
fn default_prompts() -> Vec<McpPrompt> {
    vec![
        McpPrompt {
            name: "summarize".to_string(),
            description: "Summarize the provided text.".to_string(),
            arguments: vec![McpPromptArg {
                name: "text".to_string(),
                description: "The text to summarize.".to_string(),
                required: true,
            }],
            text: "Please summarize the following text in three sentences.".to_string(),
        },
        McpPrompt {
            name: "code_review".to_string(),
            description: "Review a code snippet for bugs and style.".to_string(),
            arguments: vec![
                McpPromptArg {
                    name: "language".to_string(),
                    description: "Programming language of the snippet.".to_string(),
                    required: false,
                },
                McpPromptArg {
                    name: "code".to_string(),
                    description: "The code to review.".to_string(),
                    required: true,
                },
            ],
            text: "Review the following code and list bugs, risks, and style issues.".to_string(),
        },
        McpPrompt {
            name: "translate".to_string(),
            description: "Translate text into a target language.".to_string(),
            arguments: vec![
                McpPromptArg {
                    name: "target_language".to_string(),
                    description: "Language to translate into.".to_string(),
                    required: true,
                },
                McpPromptArg {
                    name: "text".to_string(),
                    description: "The text to translate.".to_string(),
                    required: true,
                },
            ],
            text: "Translate the following text into the requested target language.".to_string(),
        },
    ]
}

/// Build the axum router exposing the mock MCP JSON-RPC endpoint.
pub fn router(config: McpMockConfig) -> Router {
    Router::new().route("/mcp", post(handle_rpc)).with_state(config)
}

/// JSON-RPC error codes (subset of the spec) we may return.
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC invalid-params error code, returned for an unknown resource uri
/// or prompt name, and for a request missing what its revision requires.
const INVALID_PARAMS: i64 = -32602;
/// Revision 2026-07-28: an HTTP header does not match the request body.
const HEADER_MISMATCH: i64 = -32020;
/// Revision 2026-07-28: the requested protocol revision is not supported.
const UNSUPPORTED_VERSION: i64 = -32022;

async fn handle_rpc(
    State(config): State<McpMockConfig>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> Response {
    // Notifications have no `id`; per JSON-RPC the server must not reply with a
    // result. MCP sends `notifications/initialized` after `initialize`.
    let Some(id) = req.get("id").cloned() else {
        // Notification: acknowledge with 202 and no body.
        return StatusCode::ACCEPTED.into_response();
    };
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    if req.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Json(error_response(id, INVALID_REQUEST, "jsonrpc must be \"2.0\""))
            .into_response();
    }

    // A request on the per-request protocol is recognised by its metadata, by
    // the `Mcp-Method` header only that protocol sends, or by `server/discover`,
    // which only it defines. A request with the header but without the
    // metadata is still served on that protocol, so it is refused as malformed
    // instead of being answered as though it were a handshake-era request.
    let carries_version = params
        .get("_meta")
        .and_then(|meta| meta.get(VERSION_KEY))
        .is_some_and(Value::is_string);
    let names_method_in_header = headers.contains_key("mcp-method");
    if carries_version || names_method_in_header || method == "server/discover" {
        return modern(&config, &headers, id, method, &params);
    }
    legacy(&config, id, method, &params)
}

/// Answer a request on the `initialize` handshake that earlier revisions use.
fn legacy(config: &McpMockConfig, id: Value, method: &str, params: &Value) -> Response {
    // Fallible lookups: an unknown uri / prompt name is INVALID_PARAMS, not a
    // successful empty result, so a client can tell "no such resource" apart
    // from "resource has no contents".
    if let Some(found) = lookup(config, method, params) {
        return Json(match found {
            Ok(result) => result_response(id, result),
            Err((code, message)) => error_response(id, code, &message),
        })
        .into_response();
    }

    let result = match method {
        "initialize" => Some(json!({
            "protocolVersion": LEGACY_PROTOCOL,
            "capabilities": capabilities(),
            "serverInfo": { "name": config.server_name, "version": config.server_version },
        })),
        "tools/list" => Some(json!({ "tools": tools(config) })),
        "tools/call" => Some(tools_call(config, params)),
        "resources/list" => Some(json!({ "resources": resources(config) })),
        "resources/templates/list" => Some(json!({ "resourceTemplates": [] })),
        "prompts/list" => Some(json!({ "prompts": prompts(config) })),
        "ping" => Some(json!({})),
        _ => None,
    };

    match result {
        Some(result) => Json(result_response(id, result)).into_response(),
        None => Json(error_response(id, METHOD_NOT_FOUND, &format!("method not found: {method}")))
            .into_response(),
    }
}

/// Answer a request on the per-request protocol, revision 2026-07-28. There is
/// no handshake: each request says which revision it speaks and who sends it.
fn modern(config: &McpMockConfig, headers: &HeaderMap, id: Value, method: &str, params: &Value) -> Response {
    let meta = params.get("_meta");
    let version = meta.and_then(|meta| meta.get(VERSION_KEY)).and_then(Value::as_str);
    let declares_capabilities = meta
        .and_then(|meta| meta.get(CAPABILITIES_KEY))
        .is_some_and(Value::is_object);
    let Some(version) = version.filter(|_| declares_capabilities) else {
        return bad_request(error_response(
            id,
            INVALID_PARAMS,
            &format!("a request on {MODERN_PROTOCOL} carries `{VERSION_KEY}` and `{CAPABILITIES_KEY}` in params._meta"),
        ));
    };
    if version != MODERN_PROTOCOL {
        return bad_request(error_response_with_data(
            id,
            UNSUPPORTED_VERSION,
            "Unsupported protocol version",
            json!({ "supported": [MODERN_PROTOCOL], "requested": version }),
        ));
    }
    if let Some(detail) = header_mismatch(headers, method, params, version) {
        return bad_request(error_response(id, HEADER_MISMATCH, &format!("Header mismatch: {detail}")));
    }
    if let Some(found) = lookup(config, method, params) {
        return match found {
            Ok(result) => Json(result_response(id, complete(result))).into_response(),
            Err((code, message)) => bad_request(error_response(id, code, &message)),
        };
    }

    let result = match method {
        "server/discover" => json!({
            "resultType": "complete",
            "supportedVersions": [MODERN_PROTOCOL],
            "capabilities": capabilities(),
            // The catalog can be reconfigured, so nothing here may be cached.
            "ttlMs": 0,
            "cacheScope": "private",
            "_meta": {
                SERVER_INFO_KEY: {
                    "name": config.server_name,
                    "version": config.server_version,
                }
            },
        }),
        "tools/list" => complete(json!({ "tools": tools(config) })),
        "tools/call" => complete(tools_call(config, params)),
        "resources/list" => complete(json!({ "resources": resources(config) })),
        "resources/templates/list" => complete(json!({ "resourceTemplates": [] })),
        "prompts/list" => complete(json!({ "prompts": prompts(config) })),
        "ping" => complete(json!({})),
        _ => {
            // On this revision an unknown method is an HTTP 404, which tells a
            // client it reached a modern server that lacks the method.
            return (
                StatusCode::NOT_FOUND,
                Json(error_response(id, METHOD_NOT_FOUND, &format!("method not found: {method}"))),
            )
                .into_response();
        }
    };
    Json(result_response(id, result)).into_response()
}

/// What is wrong with the headers a request on 2026-07-28 mirrors its method,
/// revision and name into, or `None` when they match the body.
fn header_mismatch(headers: &HeaderMap, method: &str, params: &Value, version: &str) -> Option<String> {
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    match header("mcp-method") {
        Some(seen) if seen == method => {}
        Some(seen) => {
            return Some(format!("Mcp-Method header value '{seen}' does not match body value '{method}'"));
        }
        None => return Some("Mcp-Method header is missing".to_string()),
    }
    match header("mcp-protocol-version") {
        Some(seen) if seen == version => {}
        Some(seen) => {
            return Some(format!(
                "MCP-Protocol-Version header value '{seen}' does not match body value '{version}'"
            ));
        }
        None => return Some("MCP-Protocol-Version header is missing".to_string()),
    }
    // `Mcp-Name` carries the tool, resource or prompt a request is about.
    if !matches!(method, "tools/call" | "resources/read" | "prompts/get") {
        return None;
    }
    let name = params
        .get("name")
        .or_else(|| params.get("uri"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    // A value that is not plain ASCII travels encoded, behind a marker.
    match header("mcp-name") {
        Some(seen) if seen == name || seen.starts_with("=?base64?") => {}
        Some(seen) => return Some(format!("Mcp-Name header value '{seen}' does not match body value '{name}'")),
        None => return Some("Mcp-Name header is missing".to_string()),
    }
    None
}

/// `resources/read` and `prompts/get`, whose failures are errors rather than
/// results. `None` for every other method.
fn lookup(config: &McpMockConfig, method: &str, params: &Value) -> Option<Result<Value, (i64, String)>> {
    match method {
        "resources/read" => Some(resources_read(config, params)),
        "prompts/get" => Some(prompts_get(config, params)),
        _ => None,
    }
}

/// Mark a result as the whole answer. Revision 2026-07-28 requires
/// `resultType` on every result.
fn complete(mut result: Value) -> Value {
    if let Some(object) = result.as_object_mut() {
        object.insert("resultType".to_string(), Value::String("complete".to_string()));
    }
    result
}

fn capabilities() -> Value {
    json!({
        "tools": { "listChanged": false },
        "resources": { "listChanged": false },
        "prompts": { "listChanged": false },
    })
}

fn tools(config: &McpMockConfig) -> Vec<Value> {
    config
        .tools
        .iter()
        .map(|tool| json!({
            "name": tool.name,
            "description": tool.description,
            "inputSchema": tool.input_schema,
        }))
        .collect()
}

fn resources(config: &McpMockConfig) -> Vec<Value> {
    config
        .resources
        .iter()
        .map(|resource| json!({
            "uri": resource.uri,
            "name": resource.name,
            "description": resource.description,
            "mimeType": resource.mime_type,
        }))
        .collect()
}

fn prompts(config: &McpMockConfig) -> Vec<Value> {
    config
        .prompts
        .iter()
        .map(|prompt| json!({
            "name": prompt.name,
            "description": prompt.description,
            "arguments": prompt.arguments.iter().map(|argument| json!({
                "name": argument.name,
                "description": argument.description,
                "required": argument.required,
            })).collect::<Vec<_>>(),
        }))
        .collect()
}

fn tools_call(config: &McpMockConfig, params: &Value) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

    let Some(tool) = config.tools.iter().find(|t| t.name == name) else {
        return json!({
            "content": [{ "type": "text", "text": format!("unknown tool: {name}") }],
            "isError": true,
        });
    };

    // The `echo` tool reflects its `text` argument so agents can verify the
    // round-trip; everything else returns its canned result verbatim.
    let text = if tool.name == "echo" {
        args.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()
    } else {
        tool.canned_result.clone()
    };

    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": tool.is_error,
    })
}

/// Handle `resources/read`: look the requested `uri` up in the catalog and
/// return its canned contents, or an INVALID_PARAMS error if unknown.
fn resources_read(config: &McpMockConfig, params: &Value) -> Result<Value, (i64, String)> {
    let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
    let Some(resource) = config.resources.iter().find(|r| r.uri == uri) else {
        return Err((INVALID_PARAMS, format!("unknown resource: {uri}")));
    };
    Ok(json!({
        "contents": [{
            "uri": resource.uri,
            "mimeType": resource.mime_type,
            "text": resource.text,
        }],
    }))
}

/// Handle `prompts/get`: look the requested prompt `name` up in the catalog
/// and return a canned single-message conversation, or an INVALID_PARAMS
/// error if unknown.
fn prompts_get(config: &McpMockConfig, params: &Value) -> Result<Value, (i64, String)> {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let Some(prompt) = config.prompts.iter().find(|p| p.name == name) else {
        return Err((INVALID_PARAMS, format!("unknown prompt: {name}")));
    };
    Ok(json!({
        "description": prompt.description,
        "messages": [{
            "role": "user",
            "content": { "type": "text", "text": prompt.text },
        }],
    }))
}

fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// An error with data for the client, as revision 2026-07-28 returns for an
/// unsupported protocol revision.
fn error_response_with_data(id: Value, code: i64, message: &str, data: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message, "data": data } })
}

/// A malformed or refused request on the per-request protocol is an HTTP 400.
fn bad_request(body: Value) -> Response {
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_contracts::mcp_contract::client::{ClientOptions, fetch_declared};
    use mockforge_contracts::mcp_contract::Era;

    async fn body_of(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        }
    }

    /// A request without the per-request metadata, as the handshake era sends.
    async fn call(body: Value) -> Value {
        let response = handle_rpc(State(McpMockConfig::default()), HeaderMap::new(), Json(body)).await;
        body_of(response).await
    }

    /// A request on 2026-07-28 with the given headers. Returns the status and the body.
    async fn modern(method: &str, params: Value, headers: &[(&'static str, String)]) -> (u16, Value) {
        let mut map = HeaderMap::new();
        for (name, value) in headers {
            map.insert(*name, value.parse().unwrap());
        }
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let response = handle_rpc(State(McpMockConfig::default()), map, Json(body)).await;
        let status = response.status().as_u16();
        (status, body_of(response).await)
    }

    /// The per-request metadata of a conforming client on 2026-07-28.
    fn on(version: &str, mut params: Value) -> Value {
        params["_meta"] = json!({ VERSION_KEY: version, CAPABILITIES_KEY: {} });
        params
    }

    /// The headers a conforming client sends for `method`.
    fn proper(method: &str) -> Vec<(&'static str, String)> {
        vec![
            ("mcp-method", method.to_string()),
            ("mcp-protocol-version", MODERN_PROTOCOL.to_string()),
        ]
    }

    #[tokio::test]
    async fn initialize_returns_server_info_and_capabilities() {
        let v = call(json!({"jsonrpc":"2.0","id":1,"method":"initialize"})).await;
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["protocolVersion"], LEGACY_PROTOCOL);
        assert_eq!(v["result"]["serverInfo"]["name"], "mockforge-mcp");
        assert!(v["result"]["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn tools_list_returns_catalog() {
        let v = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
        let tools = v["result"]["tools"].as_array().unwrap();
        // Round 52 (#79) — the default catalog is deliberately broad so the
        // list response is chatty; keep echo/get_status and add several more.
        assert!(tools.len() >= 6, "expected a rich default tool catalog");
        assert!(tools.iter().any(|t| t["name"] == "echo"));
        assert!(tools.iter().any(|t| t["name"] == "search_documents"));
        assert!(tools[0]["inputSchema"].is_object());
    }

    #[tokio::test]
    async fn tools_call_echo_reflects_argument() {
        let v = call(json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params": { "name": "echo", "arguments": { "text": "hello mcp" } }
        }))
        .await;
        assert_eq!(v["result"]["content"][0]["text"], "hello mcp");
        assert_eq!(v["result"]["isError"], false);
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_is_error() {
        let v = call(json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params": { "name": "nope", "arguments": {} }
        }))
        .await;
        assert_eq!(v["result"]["isError"], true);
    }

    #[tokio::test]
    async fn unknown_method_returns_jsonrpc_error() {
        let v = call(json!({"jsonrpc":"2.0","id":5,"method":"does/not/exist"})).await;
        assert_eq!(v["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn notification_without_id_returns_no_body() {
        // notifications/initialized has no `id`; must not produce a JSON-RPC reply.
        let v = call(json!({"jsonrpc":"2.0","method":"notifications/initialized"})).await;
        assert_eq!(v, Value::Null);
    }

    #[tokio::test]
    async fn resources_and_prompts_list_are_populated() {
        // Round 52 (#79) — the mock now ships non-empty resource and prompt
        // catalogs so the resource/prompt halves of the protocol produce
        // real client-server traffic.
        let r = call(json!({"jsonrpc":"2.0","id":6,"method":"resources/list"})).await;
        let resources = r["result"]["resources"].as_array().unwrap();
        assert!(!resources.is_empty());
        assert!(resources.iter().all(|x| x["uri"].is_string() && x["mimeType"].is_string()));

        let p = call(json!({"jsonrpc":"2.0","id":7,"method":"prompts/list"})).await;
        let prompts = p["result"]["prompts"].as_array().unwrap();
        assert!(!prompts.is_empty());
        assert!(prompts.iter().any(|x| x["name"] == "summarize"));
    }

    #[tokio::test]
    async fn resources_read_returns_canned_contents() {
        let v = call(json!({
            "jsonrpc":"2.0","id":8,"method":"resources/read",
            "params": { "uri": "file:///readme.md" }
        }))
        .await;
        let contents = v["result"]["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["uri"], "file:///readme.md");
        assert!(contents[0]["text"].as_str().unwrap().contains("MockForge"));
    }

    #[tokio::test]
    async fn resources_read_unknown_uri_is_invalid_params() {
        let v = call(json!({
            "jsonrpc":"2.0","id":9,"method":"resources/read",
            "params": { "uri": "file:///nope" }
        }))
        .await;
        assert_eq!(v["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn prompts_get_returns_message() {
        let v = call(json!({
            "jsonrpc":"2.0","id":10,"method":"prompts/get",
            "params": { "name": "summarize" }
        }))
        .await;
        let messages = v["result"]["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"]["type"], "text");
    }

    #[tokio::test]
    async fn prompts_get_unknown_name_is_invalid_params() {
        let v = call(json!({
            "jsonrpc":"2.0","id":11,"method":"prompts/get",
            "params": { "name": "nope" }
        }))
        .await;
        assert_eq!(v["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn discover_answers_the_current_protocol_without_a_handshake() {
        let (status, v) = modern("server/discover", on(MODERN_PROTOCOL, json!({})), &proper("server/discover")).await;
        assert_eq!(status, 200);
        assert_eq!(v["result"]["supportedVersions"], json!([MODERN_PROTOCOL]));
        assert_eq!(v["result"]["resultType"], "complete");
        assert_eq!(v["result"]["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(v["result"]["_meta"][SERVER_INFO_KEY]["name"], "mockforge-mcp");
        assert_eq!(v["result"]["ttlMs"], 0, "the catalog can change, so nothing may be cached");

        // The discover request may carry nothing but its metadata.
        let only_meta = json!({ "_meta": { VERSION_KEY: MODERN_PROTOCOL, CAPABILITIES_KEY: {} } });
        let (status, v) = modern("server/discover", only_meta, &proper("server/discover")).await;
        assert_eq!((status, v["result"]["supportedVersions"].clone()), (200, json!([MODERN_PROTOCOL])));
    }

    #[tokio::test]
    async fn modern_results_carry_their_type_and_the_same_catalog() {
        let (status, v) = modern("tools/list", on(MODERN_PROTOCOL, json!({})), &proper("tools/list")).await;
        assert_eq!(status, 200);
        assert_eq!(v["result"]["resultType"], "complete");
        assert_eq!(v["result"]["tools"], json!(tools(&McpMockConfig::default())));

        let mut headers = proper("tools/call");
        headers.push(("mcp-name", "echo".to_string()));
        let echo = json!({ "name": "echo", "arguments": { "text": "hello mcp" } });
        let (status, v) = modern("tools/call", on(MODERN_PROTOCOL, echo), &headers).await;
        assert_eq!(status, 200);
        assert_eq!(v["result"]["resultType"], "complete");
        assert_eq!(v["result"]["content"][0]["text"], "hello mcp");

        // The handshake era lists the same tools, without the type marker.
        let handshake = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
        assert_eq!(handshake["result"]["tools"], json!(tools(&McpMockConfig::default())));
        assert!(handshake["result"].get("resultType").is_none());
    }

    #[tokio::test]
    async fn modern_requests_are_checked_against_the_revision() {
        // Modern headers without the metadata the revision requires.
        let (status, v) = modern("tools/list", json!({}), &proper("tools/list")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(INVALID_PARAMS)));
        let (status, v) = modern("server/discover", json!({}), &proper("server/discover")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(INVALID_PARAMS)));

        // A revision this mock does not speak, answered with the ones it does.
        let (status, v) = modern("tools/list", on("1900-01-01", json!({})), &proper("tools/list")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(UNSUPPORTED_VERSION)));
        assert_eq!(v["error"]["data"], json!({ "supported": [MODERN_PROTOCOL], "requested": "1900-01-01" }));

        // Headers that disagree with the body, or are missing.
        let (status, v) = modern("tools/list", on(MODERN_PROTOCOL, json!({})), &proper("ping")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(HEADER_MISMATCH)));
        assert!(v["error"]["message"].as_str().unwrap().contains("Mcp-Method header value 'ping'"));
        let only_version = [("mcp-protocol-version", MODERN_PROTOCOL.to_string())];
        let (status, v) = modern("tools/list", on(MODERN_PROTOCOL, json!({})), &only_version).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(HEADER_MISMATCH)));
        assert!(v["error"]["message"].as_str().unwrap().contains("Mcp-Method header is missing"));

        // A tool call names its tool in a header as well as in the body.
        let (status, v) = modern("tools/call", on(MODERN_PROTOCOL, json!({ "name": "echo" })), &proper("tools/call")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(HEADER_MISMATCH)));
        assert!(v["error"]["message"].as_str().unwrap().contains("Mcp-Name"));
    }

    #[tokio::test]
    async fn a_modern_unknown_method_is_a_404_and_a_bad_lookup_is_a_400() {
        let (status, v) = modern("does/not/exist", on(MODERN_PROTOCOL, json!({})), &proper("does/not/exist")).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (404, Some(METHOD_NOT_FOUND)));

        let mut headers = proper("resources/read");
        headers.push(("mcp-name", "file:///nope".to_string()));
        let read = json!({ "uri": "file:///nope" });
        let (status, v) = modern("resources/read", on(MODERN_PROTOCOL, read), &headers).await;
        assert_eq!((status, v["error"]["code"].as_i64()), (400, Some(INVALID_PARAMS)));
    }

    #[tokio::test]
    async fn the_contract_client_reads_the_mock_as_a_modern_server() {
        // The real client against the real mock: the loop a team can rehearse
        // pin and check on without a third-party server.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router(McpMockConfig::default())).await.unwrap();
        });
        let options = ClientOptions::new(&format!("http://{address}/mcp"), ClientOptions::default_identity()).unwrap();
        let declared = fetch_declared(&options).await.unwrap();
        assert_eq!(declared.era, Era::Modern);
        assert_eq!(declared.protocol_version, MODERN_PROTOCOL);
        assert_eq!(declared.tools, tools(&McpMockConfig::default()));
        assert_eq!(declared.implementation["name"], "mockforge-mcp");
    }
}
