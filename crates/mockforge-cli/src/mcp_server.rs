//! `mockforge mcp` — Model Context Protocol server over stdio (#835).
//!
//! Speaks newline-delimited JSON-RPC 2.0 on stdin/stdout so local agents
//! (Claude Desktop, Cursor, any MCP client) can drive MockForge's spec
//! tooling directly. Tools map 1:1 onto existing library capabilities:
//!
//! - `list_routes`        — parse an OpenAPI/Swagger spec, list operations
//! - `validate_request`   — run the conformance chain against a synthetic request
//! - `generate_example`   — synthesize a sample response body for an operation
//! - `convert_spec`       — up-convert Swagger 2.0 to OpenAPI 3.x
//!
//! Transport is stdio-only in v1; a token-gated HTTP transport is the
//! documented follow-up.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, Write};

use mockforge_openapi::openapi_routes::OpenApiRouteRegistry;
use mockforge_openapi::schema_ref_resolver::merge_components_into;
use mockforge_openapi::spec::OpenApiSpec;
use openapiv3::ReferenceOr;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the MCP loop until stdin closes.
pub async fn run() -> mockforge_core::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line =
            line.map_err(|e| mockforge_core::Error::io_with_context("mcp stdin", e.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match handle_line(&line).await {
            Ok(Some(v)) => v,
            Ok(None) => continue, // notification
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": null,
                        "error": { "code": code, "message": message } })
            }
        };
        writeln!(stdout, "{response}")
            .map_err(|e| mockforge_core::Error::io_with_context("mcp stdout", e.to_string()))?;
        stdout.flush().ok();
    }
    Ok(())
}

async fn handle_line(line: &str) -> Result<Option<Value>, (i64, String)> {
    let msg: Value = serde_json::from_str(line).map_err(|_| (-32700, "parse error".to_string()))?;
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    if method.starts_with("notifications/") {
        return Ok(None);
    }

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "mockforge",
                "version": env!("CARGO_PKG_VERSION"),
            }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => tools_list(),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match tools_call(name, &arguments).await {
                Ok(text) => Ok(json!({
                    "content": [ { "type": "text", "text": text } ],
                    "isError": false,
                })),
                Err(e) => Ok(json!({
                    "content": [ { "type": "text", "text": e } ],
                    "isError": true,
                })),
            }
        }
        other => Err((-32601, format!("method not found: {other}"))),
    }?;

    let _ = id;
    Ok(Some(
        json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result }),
    ))
}

fn tools_list() -> Result<Value, (i64, String)> {
    Ok(json!({
        "tools": [
            {
                "name": "list_routes",
                "description": "List every operation (method + path + summary) in an OpenAPI/Swagger spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": { "type": "string", "description": "OpenAPI 3.x or Swagger 2.0 document (JSON or YAML)" }
                    }
                }
            },
            {
                "name": "validate_request",
                "description": "Validate a synthetic request against a spec and return conformance violations",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "method", "path"],
                    "properties": {
                        "spec": { "type": "string", "description": "OpenAPI 3.x or Swagger 2.0 document" },
                        "method": { "type": "string" },
                        "path": { "type": "string", "description": "Concrete path, e.g. /users/42" },
                        "query": { "type": "string" },
                        "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                        "body": { "type": ["object", "null"] }
                    }
                }
            },
            {
                "name": "generate_example",
                "description": "Generate a sample JSON response body for an operation's declared success schema",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "method", "path"],
                    "properties": {
                        "spec": { "type": "string" },
                        "method": { "type": "string" },
                        "path": { "type": "string", "description": "Template path, e.g. /users/{id}" },
                        "status": { "type": "integer", "default": 200 }
                    }
                }
            },
            {
                "name": "convert_spec",
                "description": "Up-convert a Swagger/OpenAPI 2.0 document to OpenAPI 3.0",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": { "spec": { "type": "string" } }
                }
            }
        ]
    }))
}

async fn tools_call(name: &str, args: &Value) -> Result<String, String> {
    // Load + normalize the spec (Swagger 2.0 → 3.x) once per call.
    let load_spec = |args: &Value| -> Result<OpenApiRouteRegistry, String> {
        let raw = args
            .get("spec")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing required argument: spec".to_string())?;
        let parsed: Value = serde_json::from_str(raw)
            .or_else(|_| serde_yaml::from_str(raw))
            .map_err(|e| format!("cannot parse spec: {e}"))?;
        let normalized = if parsed.get("swagger").and_then(Value::as_str) == Some("2.0") {
            mockforge_import::import::swagger2_convert::convert_swagger2_to_openapi3(&parsed)?
        } else {
            parsed
        };
        let openapi =
            OpenApiSpec::from_json(normalized).map_err(|e| format!("invalid spec: {e}"))?;
        Ok(OpenApiRouteRegistry::new(openapi))
    };

    match name {
        "list_routes" => {
            let registry = load_spec(args)?;
            let routes: Vec<Value> = registry
                .routes()
                .iter()
                .map(|r| {
                    json!({
                        "method": r.method,
                        "path": r.path,
                        "summary": r.operation.summary,
                        "operationId": r.operation.operation_id,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&routes).map_err(|e| e.to_string())
        }

        "validate_request" => {
            let registry = load_spec(args)?;
            let method = args.get("method").and_then(Value::as_str).unwrap_or("GET").to_uppercase();
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or("missing required argument: path")?;

            let Some((template, path_params)) = find_template(&registry, &method, path) else {
                return Ok(format!("no spec route matches {method} {path}"));
            };
            let query_map: serde_json::Map<String, Value> = args
                .get("query")
                .and_then(Value::as_str)
                .map(|q| {
                    q.split('&')
                        .filter_map(|pair| pair.split_once('='))
                        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let headers: serde_json::Map<String, Value> =
                args.get("headers").and_then(Value::as_object).cloned().unwrap_or_default();
            let body = args.get("body").filter(|b| !b.is_null());

            match registry.run_validation_with_recording_ex(
                &template,
                &method,
                &path_params_value(&path_params),
                &query_map,
                &headers,
                &Default::default(),
                body,
                true,
            ) {
                Ok(()) => Ok(format!("{method} {path} conforms to the spec")),
                Err((status, payload)) => {
                    Ok(format!("{method} {path} violates the spec (status {status}): {payload}"))
                }
            }
        }

        "generate_example" => {
            let registry = load_spec(args)?;
            let method = args.get("method").and_then(Value::as_str).unwrap_or("GET").to_uppercase();
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .ok_or("missing required argument: path")?;
            let want_status = args.get("status").and_then(Value::as_u64).unwrap_or(200);

            let route = registry
                .routes()
                .iter()
                .find(|r| r.method.eq_ignore_ascii_case(&method) && r.path == path);
            let Some(route) = route else {
                return Err(format!("no operation {method} {path}"));
            };

            let declared: Option<Value> =
                route.operation.responses.responses.iter().find_map(|(code, r)| match code {
                    openapiv3::StatusCode::Code(c) if *c as u64 == want_status => {
                        let ReferenceOr::Item(response) = r else {
                            return None;
                        };
                        let media = response.content.get("application/json")?;
                        match &media.schema {
                            Some(ReferenceOr::Item(schema)) => serde_json::to_value(schema).ok(),
                            Some(ReferenceOr::Reference { reference }) => {
                                Some(json!({ "$ref": reference }))
                            }
                            None => None,
                        }
                    }
                    _ => None,
                });
            let Some(schema) = declared else {
                return Err(format!(
                    "no application/json schema declared for status {want_status} on {method} {path}"
                ));
            };
            let merged = merge_components_into(schema, &registry.spec().spec);
            let example =
                mockforge_import::import::schema_data_generator::generate_from_schema(&merged);
            serde_json::to_string_pretty(&example).map_err(|e| e.to_string())
        }

        "convert_spec" => {
            let raw = args
                .get("spec")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing required argument: spec".to_string())?;
            let parsed: Value = serde_json::from_str(raw)
                .or_else(|_| serde_yaml::from_str(raw))
                .map_err(|e| format!("cannot parse spec: {e}"))?;
            let converted =
                mockforge_import::import::swagger2_convert::convert_swagger2_to_openapi3(&parsed)?;
            serde_json::to_string_pretty(&converted).map_err(|e| e.to_string())
        }

        other => Err(format!("unknown tool: {other}")),
    }
}

fn find_template(
    registry: &OpenApiRouteRegistry,
    method: &str,
    concrete_path: &str,
) -> Option<(String, HashMap<String, String>)> {
    registry.routes().iter().find_map(|route| {
        if !route.method.eq_ignore_ascii_case(method) {
            return None;
        }
        let mut params = HashMap::new();
        let mut c_parts = concrete_path.split('/');
        for t in route.path.split('/') {
            let c = c_parts.next()?;
            if t.starts_with('{') && t.ends_with('}') {
                params.insert(t[1..t.len() - 1].to_string(), c.to_string());
            } else if t != c {
                return None;
            }
        }
        if c_parts.next().is_some() {
            return None;
        }
        Some((route.path.clone(), params))
    })
}

fn path_params_value(params: &HashMap<String, String>) -> serde_json::Map<String, Value> {
    params.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{
        "openapi": "3.0.0",
        "info": { "title": "T", "version": "1" },
        "paths": {
            "/users/{id}": {
                "get": {
                    "summary": "Get user",
                    "parameters": [
                        { "name": "id", "in": "path", "required": true,
                          "schema": { "type": "integer" } }
                    ],
                    "responses": {
                        "200": { "description": "ok",
                                 "content": { "application/json": {
                                     "schema": { "type": "object",
                                         "required": ["id"],
                                         "properties": { "id": { "type": "integer" } } } } } }
                    }
                }
            }
        }
    }"#;

    #[tokio::test]
    async fn list_routes_returns_operations() {
        let out = tools_call("list_routes", &json!({ "spec": SPEC })).await.expect("ok");
        assert!(out.contains("/users/{id}"), "route listed: {out}");
        assert!(out.contains("\"Get user\""), "summary included");
    }

    #[tokio::test]
    async fn validate_request_reports_conformance() {
        let ok = tools_call(
            "validate_request",
            &json!({ "spec": SPEC, "method": "GET", "path": "/users/7" }),
        )
        .await
        .unwrap();
        assert!(ok.contains("conforms"), "conforming request: {ok}");

        let bad = tools_call(
            "validate_request",
            &json!({ "spec": SPEC, "method": "DELETE", "path": "/users/7" }),
        )
        .await
        .unwrap();
        assert!(bad.contains("no spec route"), "unmatched method: {bad}");
    }

    #[tokio::test]
    async fn generate_example_produces_schema_conformant_body() {
        let out = tools_call(
            "generate_example",
            &json!({ "spec": SPEC, "method": "GET", "path": "/users/{id}", "status": 200 }),
        )
        .await
        .expect("example generated");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON example");
        assert!(
            parsed["id"].is_i64() || parsed["id"].is_u64(),
            "id generated from schema: {parsed}"
        );
    }

    #[tokio::test]
    async fn unknown_tool_is_reported() {
        let err = tools_call("nope", &json!({})).await.unwrap_err();
        assert!(err.contains("unknown tool"));
    }
}
