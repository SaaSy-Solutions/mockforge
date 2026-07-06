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
//! Mounted by `mockforge serve --mcp-mock`.

use axum::{
    extract::State, http::StatusCode, response::IntoResponse, response::Response, routing::post,
    Json, Router,
};
use serde_json::{json, Value};

/// Protocol version this mock advertises. Matches the MCP spec revision the
/// reference clients negotiate; clients that send a different version still
/// work because we echo a fixed supported version.
const PROTOCOL_VERSION: &str = "2024-11-05";

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

async fn handle_rpc(State(config): State<McpMockConfig>, Json(req): Json<Value>) -> Response {
    // Notifications have no `id`; per JSON-RPC the server must not reply with a
    // result. MCP sends `notifications/initialized` after `initialize`.
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    if id.is_none() {
        // Notification: acknowledge with 202 and no body.
        return StatusCode::ACCEPTED.into_response();
    }
    let id = id.unwrap();

    if req.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Json(error_response(id, INVALID_REQUEST, "jsonrpc must be \"2.0\""))
            .into_response();
    }

    // Fallible lookups: an unknown uri / prompt name is INVALID_PARAMS, not a
    // successful empty result, so a client can tell "no such resource" apart
    // from "resource has no contents".
    match method {
        "resources/read" => {
            return Json(match resources_read(&config, &params) {
                Ok(r) => result_response(id, r),
                Err((code, msg)) => error_response(id, code, &msg),
            })
            .into_response();
        }
        "prompts/get" => {
            return Json(match prompts_get(&config, &params) {
                Ok(r) => result_response(id, r),
                Err((code, msg)) => error_response(id, code, &msg),
            })
            .into_response();
        }
        _ => {}
    }

    let result = match method {
        "initialize" => Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "listChanged": false },
                "prompts": { "listChanged": false },
            },
            "serverInfo": { "name": config.server_name, "version": config.server_version },
        })),
        "tools/list" => Some(json!({
            "tools": config.tools.iter().map(|t| json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.input_schema,
            })).collect::<Vec<_>>(),
        })),
        "tools/call" => Some(tools_call(&config, &params)),
        "resources/list" => Some(json!({
            "resources": config.resources.iter().map(|r| json!({
                "uri": r.uri,
                "name": r.name,
                "description": r.description,
                "mimeType": r.mime_type,
            })).collect::<Vec<_>>(),
        })),
        "resources/templates/list" => Some(json!({ "resourceTemplates": [] })),
        "prompts/list" => Some(json!({
            "prompts": config.prompts.iter().map(|p| json!({
                "name": p.name,
                "description": p.description,
                "arguments": p.arguments.iter().map(|a| json!({
                    "name": a.name,
                    "description": a.description,
                    "required": a.required,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })),
        "ping" => Some(json!({})),
        _ => None,
    };

    match result {
        Some(r) => Json(result_response(id, r)).into_response(),
        None => Json(error_response(id, METHOD_NOT_FOUND, &format!("method not found: {method}")))
            .into_response(),
    }
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

/// JSON-RPC invalid-params error code, returned for an unknown resource uri
/// or prompt name.
const INVALID_PARAMS: i64 = -32602;

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

#[cfg(test)]
mod tests {
    use super::*;

    async fn call(body: Value) -> Value {
        let resp = handle_rpc(State(McpMockConfig::default()), Json(body)).await;
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        }
    }

    #[tokio::test]
    async fn initialize_returns_server_info_and_capabilities() {
        let v = call(json!({"jsonrpc":"2.0","id":1,"method":"initialize"})).await;
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["protocolVersion"], PROTOCOL_VERSION);
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
}
