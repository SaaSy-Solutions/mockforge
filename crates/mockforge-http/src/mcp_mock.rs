//! Mock MCP (Model Context Protocol) server (#913, #79 round 50 follow-up).
//!
//! Serves a JSON-RPC 2.0 endpoint at `POST /mcp` so an agent acting as an MCP
//! client (the role Cursor / Claude Code / custom agents play when they call
//! out to tool servers) can talk to MockForge as a fake MCP server. Answers
//! the core MCP methods with a configurable tool catalog and canned results:
//! `initialize`, `tools/list`, `tools/call`, `resources/list`, `prompts/list`,
//! plus the `notifications/initialized` notification.
//!
//! This is a MOCK: no real tools run. `tools/call` returns deterministic
//! canned content so agents can be exercised against a predictable (or, with
//! a configured error tool, hostile) MCP server.
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

/// Runtime configuration for the mock MCP server.
#[derive(Clone, Debug)]
pub struct McpMockConfig {
    /// Server name returned in `initialize` -> `serverInfo.name`.
    pub server_name: String,
    /// Server version returned in `initialize` -> `serverInfo.version`.
    pub server_version: String,
    /// Tool catalog returned by `tools/list` and dispatched by `tools/call`.
    pub tools: Vec<McpTool>,
}

impl Default for McpMockConfig {
    fn default() -> Self {
        Self {
            server_name: "mockforge-mcp".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            tools: vec![
                McpTool {
                    name: "echo".to_string(),
                    description: "Echo back the provided text.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": { "text": { "type": "string" } },
                        "required": ["text"],
                    }),
                    canned_result: "echo".to_string(),
                    is_error: false,
                },
                McpTool {
                    name: "get_status".to_string(),
                    description: "Return a canned status payload.".to_string(),
                    input_schema: json!({ "type": "object", "properties": {} }),
                    canned_result: "{\"status\":\"ok\",\"source\":\"mockforge-mcp\"}".to_string(),
                    is_error: false,
                },
            ],
        }
    }
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
        "resources/list" => Some(json!({ "resources": [] })),
        "prompts/list" => Some(json!({ "prompts": [] })),
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
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t["name"] == "echo"));
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
    async fn resources_and_prompts_list_are_empty() {
        let r = call(json!({"jsonrpc":"2.0","id":6,"method":"resources/list"})).await;
        assert!(r["result"]["resources"].as_array().unwrap().is_empty());
        let p = call(json!({"jsonrpc":"2.0","id":7,"method":"prompts/list"})).await;
        assert!(p["result"]["prompts"].as_array().unwrap().is_empty());
    }
}
