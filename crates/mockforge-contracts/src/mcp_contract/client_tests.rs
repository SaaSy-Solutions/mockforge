//! Tests for the read-only MCP client. Each test starts a small TCP server
//! that answers with hand-written HTTP, so the client is checked against exact
//! bytes for both protocol eras and both response styles.

use super::*;
use crate::mcp_contract::model::{Contract, ServerContract};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// One request the test server received.
#[derive(Clone, Debug)]
struct Request {
    method: String,
    headers: BTreeMap<String, String>,
    body: Value,
}

enum Answer {
    /// A status, headers and a body.
    Status(u16, Vec<(&'static str, String)>, String),
    /// Accept the connection and never answer.
    Hang,
}

type Responder = Arc<dyn Fn(&Request) -> Answer + Send + Sync>;

type Log = Arc<Mutex<Vec<Request>>>;

const JSON_TYPE: &str = "application/json";
const SSE_TYPE: &str = "text/event-stream";

fn json(status: u16, value: &Value) -> Answer {
    Answer::Status(status, vec![("Content-Type", JSON_TYPE.to_string())], value.to_string())
}

fn sse(status: u16, headers: Vec<(&'static str, String)>, events: Vec<String>) -> Answer {
    let body: String = events
        .iter()
        .map(|event| format!("event: message\r\ndata: {event}\r\n\r\n"))
        .collect();
    let mut headers = headers;
    headers.push(("Content-Type", SSE_TYPE.to_string()));
    Answer::Status(status, headers, body)
}

fn result(request: &Request, result: Value) -> Answer {
    json(200, &json!({"jsonrpc": "2.0", "id": request.body["id"], "result": result}))
}

fn error(request: &Request, status: u16, code: i64, message: &str, data: Value) -> Answer {
    json(
        status,
        &json!({"jsonrpc": "2.0", "id": request.body["id"], "error": {"code": code, "message": message, "data": data}}),
    )
}

fn tool(name: &str) -> Value {
    json!({"name": name, "description": format!("Tool {name}"), "inputSchema": {"type": "object"}})
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Status",
    }
}

async fn answer(mut stream: TcpStream, responder: Responder, log: Log) {
    let mut buffer: Vec<u8> = Vec::new();
    let header_end = loop {
        let mut chunk = [0u8; 4096];
        let Ok(read) = stream.read(&mut chunk).await else {
            return;
        };
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break at + 4;
        }
        if buffer.len() > 1 << 20 {
            return;
        }
    };
    let head = String::from_utf8_lossy(&buffer[..header_end - 4]).to_string();
    let mut lines = head.lines();
    let method = lines
        .next()
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or_default()
        .to_string();
    let headers: BTreeMap<String, String> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    let length: usize =
        headers.get("content-length").and_then(|value| value.parse().ok()).unwrap_or(0);
    while buffer.len() < header_end + length {
        let mut chunk = [0u8; 4096];
        let Ok(read) = stream.read(&mut chunk).await else {
            return;
        };
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    let request = Request {
        method,
        headers,
        body: serde_json::from_slice(&buffer[header_end..header_end + length])
            .unwrap_or(Value::Null),
    };
    let reply = responder(&request);
    log.lock().unwrap().push(request);
    let Answer::Status(status, headers, body) = reply else {
        tokio::time::sleep(Duration::from_secs(60)).await;
        return;
    };
    let mut response = format!(
        "HTTP/1.1 {status} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        reason(status),
        body.len()
    );
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(body.as_bytes()).await;
}

/// Start a server that answers each request with `responder`. Returns its
/// address and the log of what it received.
async fn serve(responder: Responder) -> (String, Log) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&log);
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(answer(stream, Arc::clone(&responder), Arc::clone(&recorded)));
        }
    });
    (format!("http://{address}/mcp"), log)
}

fn options(url: &str) -> ClientOptions {
    let mut options = ClientOptions::new(url, ClientOptions::default_identity()).unwrap();
    options.timeout = Duration::from_secs(5);
    options
}

fn methods(log: &Log) -> Vec<String> {
    log.lock()
        .unwrap()
        .iter()
        .map(|request| {
            format!("{} {}", request.method, request.body["method"].as_str().unwrap_or(""))
        })
        .collect()
}

/// Nothing but the four read-only methods ever crosses the wire.
fn assert_read_only(log: &Log) {
    for request in log.lock().unwrap().iter() {
        assert!(
            matches!(
                (request.method.as_str(), request.body["method"].as_str()),
                (
                    "POST",
                    Some(
                        "server/discover"
                            | "initialize"
                            | "notifications/initialized"
                            | "tools/list"
                    )
                ) | ("DELETE", None)
            ),
            "an unexpected request was sent: {} {}",
            request.method,
            request.body
        );
    }
}

fn discover_result() -> Value {
    json!({
        "resultType": "complete",
        "supportedVersions": ["2026-07-28"],
        "capabilities": {"tools": {}},
        "instructions": "Be brief.",
        "_meta": {"io.modelcontextprotocol/serverInfo": {"name": "Example", "version": "1.0.0"}}
    })
}

#[tokio::test]
async fn a_modern_server_is_discovered_and_its_pages_are_followed() {
    let (url, log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => result(request, discover_result()),
        Some("tools/list") if request.body["params"]["cursor"].is_null() => {
            result(request, json!({"tools": [tool("a")], "nextCursor": "page-2", "ttlMs": 0}))
        }
        Some("tools/list") => {
            result(request, json!({"resultType": "complete", "tools": [tool("b")]}))
        }
        _ => json(500, &json!({"unexpected": request.body["method"]})),
    }))
    .await;
    let mut options = options(&url);
    options.plain_header("X-Tenant: acme").unwrap();
    options.client = ClientIdentity {
        name: "agent".to_string(),
        version: "9.9.9".to_string(),
    };
    let declared = fetch_declared(&options).await.unwrap();

    assert_eq!(declared.era, Era::Modern);
    assert_eq!(declared.protocol_version, MODERN_PROTOCOL);
    assert_eq!(declared.implementation, json!({"name": "Example", "version": "1.0.0"}));
    assert_eq!(declared.capabilities, json!({"tools": {}}));
    assert_eq!(declared.instructions.as_deref(), Some("Be brief."));
    let names: Vec<&str> = declared.tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);

    // Read the log before locking it: the standard mutex cannot be locked twice.
    let calls = methods(&log);
    let requests = log.lock().unwrap();
    assert_eq!(calls, ["POST server/discover", "POST tools/list", "POST tools/list"]);
    let discover = &requests[0];
    assert_eq!(discover.headers["mcp-protocol-version"], MODERN_PROTOCOL);
    assert_eq!(discover.headers["mcp-method"], "server/discover");
    assert_eq!(discover.headers["accept"], "application/json, text/event-stream");
    assert_eq!(discover.headers["x-tenant"], "acme");
    assert!(
        !discover.headers.contains_key("mcp-session-id"),
        "a modern server has no sessions"
    );
    assert_eq!(
        discover.body["params"]["_meta"],
        json!({
            "io.modelcontextprotocol/protocolVersion": MODERN_PROTOCOL,
            "io.modelcontextprotocol/clientInfo": {"name": "agent", "version": "9.9.9"},
            "io.modelcontextprotocol/clientCapabilities": {},
        })
    );
    assert!(discover.body["params"].get("cursor").is_none());
    assert_eq!(requests[1].body["params"]["cursor"], Value::Null);
    assert_eq!(requests[2].body["params"]["cursor"], "page-2");
    assert_eq!(requests[2].headers["mcp-method"], "tools/list");
    assert_eq!(discover.body["jsonrpc"], "2.0");
    drop(requests);
    assert_read_only(&log);
}

#[tokio::test]
async fn a_modern_server_answering_with_events_is_read_until_its_answer() {
    let (url, log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => sse(
            200,
            vec![("Mcp-Session-Id", "ignored-by-modern".to_string())],
            vec![
                ": keep-alive".to_string(),
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{{\"progress\":1}}}}"
                ),
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
                    request.body["id"],
                    discover_result()
                ),
                "{\"jsonrpc\":\"2.0\",\"id\":999,\"result\":{}}".to_string(),
            ],
        ),
        Some("tools/list") => Answer::Status(
            200,
            vec![("Content-Type", SSE_TYPE.to_string())],
            format!(
                ": comment\n\nevent: other\ndata: {{\"id\":{},\"result\":{{\"tools\":[{{\"name\":\"wrong-event\"}}]}}}}\n\ndata: {{\"jsonrpc\":\"2.0\",\ndata: \"id\":{},\"result\":{{\"tools\":[{}]}}}}\n\n",
                request.body["id"],
                request.body["id"],
                tool("right")
            ),
        ),
        _ => json(500, &json!({})),
    }))
    .await;
    let declared = fetch_declared(&options(&url)).await.unwrap();
    assert_eq!(declared.era, Era::Modern);
    assert_eq!(declared.implementation["name"], "Example");
    let names: Vec<&str> = declared.tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
    assert_eq!(
        names,
        vec!["right"],
        "only the event that answers the request counts, even across two data lines"
    );
    assert_eq!(methods(&log), ["POST server/discover", "POST tools/list"]);
    assert_read_only(&log);
}

#[tokio::test]
async fn a_modern_server_that_rejects_the_version_is_reported_not_retried_the_old_way() {
    let (url, log) = serve(Arc::new(|request| {
        error(
            request,
            400,
            -32022,
            "Unsupported protocol version",
            json!({"supported": ["2027-01-01", "2027-06-01"], "requested": MODERN_PROTOCOL}),
        )
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(error, McpContractError::UnsupportedVersion { ref supported, .. } if *supported == vec!["2027-01-01".to_string(), "2027-06-01".to_string()])
    );
    let text = error.to_string();
    assert!(text.contains("speaks MCP [\"2027-01-01\", \"2027-06-01\"]"), "{text}");
    assert!(text.contains(MODERN_PROTOCOL) && text.contains(LEGACY_PROTOCOL), "{text}");
    assert_eq!(
        methods(&log),
        ["POST server/discover"],
        "no handshake is attempted with a modern server"
    );
}

#[tokio::test]
async fn another_modern_error_is_reported_as_a_refusal() {
    let (url, log) = serve(Arc::new(|request| {
        error(
            request,
            400,
            -32021,
            "Missing required client capability",
            json!({"requiredCapabilities": ["elicitation"]}),
        )
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(&error, McpContractError::Rejected { method, code, .. } if method == "server/discover" && *code == -32021)
    );
    assert!(error.to_string().contains("Missing required client capability"));
    assert_eq!(methods(&log), ["POST server/discover"]);
}

#[tokio::test]
async fn a_legacy_server_with_a_session_is_asked_through_the_handshake() {
    let (url, log) = serve(Arc::new(|request| match request.method.as_str() {
        "DELETE" => Answer::Status(200, vec![], String::new()),
        _ => match request.body["method"].as_str() {
            Some("server/discover") => Answer::Status(
                404,
                vec![("Content-Type", "text/html".to_string())],
                "<html>Not Found</html>".to_string(),
            ),
            Some("initialize") => Answer::Status(
                200,
                vec![
                    ("Content-Type", JSON_TYPE.to_string()),
                    ("Mcp-Session-Id", "sess-1".to_string()),
                ],
                json!({"jsonrpc": "2.0", "id": request.body["id"], "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {"tools": {"listChanged": true}},
                    "serverInfo": {"name": "Old", "version": "0.1.0"},
                    "instructions": "Legacy guidance."
                }})
                .to_string(),
            ),
            Some("notifications/initialized") => Answer::Status(202, vec![], String::new()),
            Some("tools/list")
                if request.headers.get("mcp-session-id").map(String::as_str) != Some("sess-1") =>
            {
                json(400, &json!({"error": "no session"}))
            }
            Some("tools/list") => result(request, json!({"tools": [tool("old")]})),
            _ => json(500, &json!({})),
        },
    }))
    .await;
    let declared = fetch_declared(&options(&url)).await.unwrap();

    assert_eq!(declared.era, Era::Legacy);
    assert_eq!(declared.protocol_version, "2025-06-18");
    assert_eq!(declared.implementation, json!({"name": "Old", "version": "0.1.0"}));
    assert_eq!(declared.capabilities, json!({"tools": {"listChanged": true}}));
    assert_eq!(declared.instructions.as_deref(), Some("Legacy guidance."));
    assert_eq!(declared.tools.len(), 1);

    let calls = methods(&log);
    let requests = log.lock().unwrap();
    assert_eq!(
        calls,
        [
            "POST server/discover",
            "POST initialize",
            "POST notifications/initialized",
            "POST tools/list",
            "DELETE "
        ]
    );
    let initialize = &requests[1];
    assert!(
        !initialize.headers.contains_key("mcp-protocol-version"),
        "the handshake names its version in the body"
    );
    assert!(
        initialize.body["params"].get("_meta").is_none(),
        "no modern metadata: it would select modern behaviour"
    );
    assert_eq!(initialize.body["params"]["protocolVersion"], LEGACY_PROTOCOL);
    assert_eq!(initialize.body["params"]["clientInfo"]["name"], "mockforge");
    let list = &requests[3];
    assert_eq!(list.headers["mcp-protocol-version"], "2025-06-18");
    assert_eq!(list.headers["mcp-session-id"], "sess-1");
    assert!(list.body["params"].get("_meta").is_none());
    assert!(requests[2].body.get("id").is_none(), "a notification carries no id");
    assert_eq!(
        requests[4].headers["mcp-session-id"], "sess-1",
        "the session is ended with DELETE"
    );
    assert_eq!(requests[4].headers["mcp-protocol-version"], "2025-06-18");
    drop(requests);
    assert_read_only(&log);
}

#[tokio::test]
async fn a_legacy_server_that_answers_the_modern_probe_oddly_still_falls_back() {
    // Exa answers the probe with HTTP 400 and a code in the legacy range.
    let (url, log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => error(request, 400, -32000, "Bad Request: Unsupported protocol version: 2026-07-28", json!({})),
        Some("initialize") => sse(
            200,
            vec![("Mcp-Session-Id", "exa".to_string())],
            vec![format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{{}},\"serverInfo\":{{\"name\":\"Exa\",\"version\":\"3.5.1\"}}}}}}",
                request.body["id"]
            )],
        ),
        Some("notifications/initialized") => Answer::Status(202, vec![], String::new()),
        Some("tools/list") => sse(200, vec![], vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"tools\":[{}]}}}}",
            request.body["id"],
            tool("web_search")
        )]),
        _ => json(500, &json!({})),
    }))
    .await;
    let declared = fetch_declared(&options(&url)).await.unwrap();
    assert_eq!((declared.era, declared.protocol_version.as_str()), (Era::Legacy, "2025-11-25"));
    assert_eq!(declared.tools[0]["name"], "web_search");
    assert_read_only(&log);

    // Microsoft Learn answers the probe with HTTP 200 and "method not available" on a stream.
    let (url, log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => sse(200, vec![], vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":-32601,\"message\":\"Method 'server/discover' is not available.\"}}}}",
            request.body["id"]
        )]),
        Some("initialize") => sse(200, vec![("Mcp-Session-Id", "learn".to_string())], vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"Microsoft Learn MCP Server\",\"version\":\"1.0.0\"}}}}}}",
            request.body["id"]
        )]),
        Some("notifications/initialized") => Answer::Status(202, vec![], String::new()),
        Some("tools/list") => sse(200, vec![], vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"tools\":[{}]}}}}",
            request.body["id"],
            tool("microsoft_docs_search")
        )]),
        _ => json(500, &json!({})),
    }))
    .await;
    let declared = fetch_declared(&options(&url)).await.unwrap();
    assert_eq!((declared.era, declared.protocol_version.as_str()), (Era::Legacy, "2025-06-18"));
    assert_eq!(declared.implementation["name"], "Microsoft Learn MCP Server");
    assert_eq!(declared.tools[0]["name"], "microsoft_docs_search");
    assert_read_only(&log);
}

#[tokio::test]
async fn something_that_is_not_an_mcp_endpoint_says_what_both_attempts_received() {
    let (url, log) = serve(Arc::new(|_| {
        Answer::Status(
            200,
            vec![("Content-Type", "text/html".to_string())],
            "<html>a web page</html>".to_string(),
        )
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    let text = error.to_string();
    assert!(matches!(error, McpContractError::NotMcp { .. }));
    assert!(text.contains("is not an MCP endpoint"), "{text}");
    assert!(text.contains("server/discover answered HTTP 200 (text/html: 23 bytes that are not JSON (<html>a web page</html>))"), "{text}");
    assert!(text.contains("initialize answered HTTP 200"), "{text}");
    assert_eq!(methods(&log), ["POST server/discover", "POST initialize"]);
    assert_read_only(&log);
}

#[tokio::test]
async fn a_refused_credential_is_an_http_error_not_a_clean_result() {
    let (url, _log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => error(request, 401, -32000, "Unauthorized", json!({})),
        _ => Answer::Status(
            401,
            vec![("Content-Type", JSON_TYPE.to_string())],
            "{\"error\":\"invalid token\"}".to_string(),
        ),
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(&error, McpContractError::HttpStatus { method, status: 401, .. } if method == "initialize"),
        "{error}"
    );
    assert!(error.to_string().contains("HTTP 401"), "{error}");
    assert!(
        error.to_string().contains("check the credential passed with --auth-env"),
        "{error}"
    );

    // A credential that stops working between the handshake and the list is reported where it failed.
    let (url, _log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => result(request, discover_result()),
        _ => Answer::Status(403, vec![], "forbidden".to_string()),
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(&error, McpContractError::HttpStatus { method, status: 403, .. } if method == "tools/list"),
        "{error}"
    );
    assert!(error.to_string().contains("--auth-env"), "{error}");
}

#[tokio::test]
async fn a_redirect_is_reported_and_never_followed() {
    let (url, log) = serve(Arc::new(|_| {
        Answer::Status(
            302,
            vec![("Location", "http://127.0.0.1:9/elsewhere/mcp".to_string())],
            String::new(),
        )
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(&error, McpContractError::Redirect { location, .. } if location == "http://127.0.0.1:9/elsewhere/mcp"),
        "{error}"
    );
    assert!(error.to_string().contains("pin the final address instead"), "{error}");
    assert_eq!(methods(&log), ["POST server/discover"]);
}

#[tokio::test]
async fn an_oversized_body_and_a_stream_that_never_answers_are_errors() {
    let padding = "x".repeat(MAX_BODY_BYTES + 1);
    let (url, _log) = serve(Arc::new(move |_| json(200, &json!({"padding": padding})))).await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(
        matches!(&error, McpContractError::TooLarge { limit, .. } if *limit == MAX_BODY_BYTES),
        "{error}"
    );

    let (url, _log) = serve(Arc::new(|_| Answer::Hang)).await;
    let mut options = options(&url);
    options.timeout = Duration::from_millis(300);
    let error = fetch_declared(&options).await.unwrap_err();
    assert!(matches!(&error, McpContractError::Transport { .. }), "{error}");
    assert!(error.to_string().contains("did not answer in time"), "{error}");
    assert!(error.is_transport());
}

#[tokio::test]
async fn a_malformed_list_is_an_error_not_an_empty_contract() {
    for (answer, expected) in [
        (json!({"resultType": "input_required", "inputRequests": {}}), "input_required"),
        (json!({"tools": "not a list"}), "no `tools` array"),
        (json!({"tools": [], "nextCursor": "forever"}), "more than 50 pages"),
    ] {
        let expected = expected.to_string();
        let (url, log) = serve(Arc::new(move |request| match request.body["method"].as_str() {
            Some("server/discover") => result(request, discover_result()),
            _ => result(request, answer.clone()),
        }))
        .await;
        let error = fetch_declared(&options(&url)).await.unwrap_err();
        assert!(
            matches!(&error, McpContractError::Protocol { method, .. } if method == "tools/list"),
            "{error}"
        );
        assert!(error.to_string().contains(&expected), "{error}");
        if expected.contains("more than") {
            assert_eq!(methods(&log).len(), 51, "discovery plus fifty pages");
        }
        assert_read_only(&log);
    }

    // A list without a result on a success status is malformed; on an error status it is that status.
    let (url, _log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => result(request, discover_result()),
        _ => json(200, &json!({"jsonrpc": "2.0", "id": request.body["id"]})),
    }))
    .await;
    let error = fetch_declared(&options(&url)).await.unwrap_err();
    assert!(error.to_string().contains("without a result"), "{error}");

    // A tool with no name is refused when the declarations are pinned.
    let (url, _log) = serve(Arc::new(|request| match request.body["method"].as_str() {
        Some("server/discover") => result(request, discover_result()),
        _ => result(
            request,
            json!({"tools": [{"description": "nameless", "inputSchema": {"type": "object"}}]}),
        ),
    }))
    .await;
    let declared = fetch_declared(&options(&url)).await.unwrap();
    let error = Contract::pin(
        ServerContract {
            url: url.clone(),
            era: declared.era,
            protocol_version: declared.protocol_version,
            implementation: declared.implementation,
            capabilities: declared.capabilities,
            instructions: declared.instructions,
            headers: BTreeMap::new(),
            auth: None,
        },
        ClientOptions::default_identity(),
        declared.tools,
        "0.0.0",
        "2026-11-24T00:00:00Z",
    )
    .unwrap_err();
    assert!(error.to_string().contains("a tool has no name"), "{error}");
}

#[test]
fn an_endpoint_is_an_http_address_without_a_credential() {
    let ok = ClientOptions::checked_endpoint("HTTPS://Example.COM/mcp?region=us#fragment").unwrap();
    assert_eq!(ok, "https://example.com/mcp?region=us");

    for (endpoint, expected) in [
        ("example.com/mcp", "is not a URL"),
        ("ftp://example.com/mcp", "not an http or https address"),
    ] {
        let error = ClientOptions::checked_endpoint(endpoint).unwrap_err();
        assert!(error.to_string().contains(expected), "{endpoint}: {error}");
    }

    // A credential in the address is refused, and its value is not repeated in the message.
    for endpoint in [
        "https://user:s3cret@example.com/mcp",
        "https://example.com/mcp?api_key=abc",
        "https://example.com/mcp?API-KEY=abc",
        "https://example.com/mcp?token=abc",
        "https://example.com/mcp?X-Amz-Signature=abc",
    ] {
        let error = ClientOptions::checked_endpoint(endpoint).unwrap_err();
        assert!(matches!(error, McpContractError::Credential { .. }), "{endpoint}: {error}");
        assert!(error.to_string().contains("credential"), "{error}");
        assert!(
            !error.to_string().contains("abc") && !error.to_string().contains("s3cret"),
            "the value is not repeated: {error}"
        );
    }
    // A name that merely ends like a credential word is not one.
    assert_eq!(
        ClientOptions::checked_endpoint("https://example.com/mcp?monkey=1").unwrap(),
        "https://example.com/mcp?monkey=1"
    );
}

#[test]
fn plain_headers_are_kept_and_credentials_are_refused_or_read_from_the_environment() {
    let mut options =
        ClientOptions::new("https://example.com/mcp", ClientOptions::default_identity()).unwrap();
    options.plain_header("X-Tenant: acme").unwrap();
    options.plain_header("x-tenant: other").unwrap();
    options.plain_header("Accept-Language: fr").unwrap();
    assert_eq!(options.timeout, Duration::from_secs(30));
    assert_eq!(
        options.headers,
        vec![
            ("x-tenant".to_string(), "other".to_string()),
            ("accept-language".to_string(), "fr".to_string())
        ]
    );

    // A header that carries a credential is refused, and its value is not repeated in the message.
    for header in [
        "Authorization: Bearer abc",
        "X-Api-Key: abc",
        "Cookie: session=abc",
        "X-Signature: abc",
    ] {
        let error = options.plain_header(header).unwrap_err();
        assert!(matches!(error, McpContractError::Credential { .. }), "{header}: {error}");
        assert!(
            error.to_string().contains("credential") && !error.to_string().contains("abc"),
            "{error}"
        );
    }
    for (header, expected) in [
        ("broken", "is not written \"Name: value\""),
        ("X-Empty:", "is not written \"Name: value\""),
        ("Bad Name: abc", "is not a header name"),
        ("X-Name: abc\r\nInjected: yes", "cannot be sent"),
    ] {
        let error = options.plain_header(header).unwrap_err();
        assert!(error.to_string().contains(expected), "{header}: {error}");
    }
    assert_eq!(options.headers.len(), 2, "a refused header is not added");

    options
        .env_header("Authorization: TOKEN", |_| Some("Bearer abc".to_string()))
        .unwrap();
    assert!(options
        .headers
        .contains(&("authorization".to_string(), "Bearer abc".to_string())));
    let error = options.env_header("X-Api-Key: MISSING", |_| None).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("environment variable MISSING, named for the `X-Api-Key` header"),
        "{error}"
    );
}

#[test]
fn events_are_parsed_line_by_line() {
    let mut parser = EventParser::default();
    assert!(parser.line(": comment").is_none());
    assert!(parser.line("event: message").is_none());
    assert!(parser.line("data: {\"a\":").is_none());
    assert!(parser.line("data: 1}").is_none());
    assert_eq!(parser.line(""), Some((Some("message".to_string()), "{\"a\":\n1}".to_string())));
    assert_eq!(parser.line(""), None, "an empty event is nothing");
    assert!(parser.line("id: 7").is_none());
    assert!(parser.line("data:nospace").is_none());
    assert_eq!(parser.line(""), Some((None, "nospace".to_string())));
}
