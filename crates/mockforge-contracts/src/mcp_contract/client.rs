//! A read-only MCP client: it asks a tool server what it declares, and nothing
//! else.
//!
//! The client speaks both eras of the protocol. Revision 2026-07-28 and later
//! carry the protocol version on every request and answer `server/discover`;
//! earlier revisions begin with an `initialize` handshake. The client tries the
//! modern request first and falls back to the handshake when the answer did
//! not come from a modern server, which is the order the specification gives.
//! A server may answer each request with one JSON object or with a stream of
//! server-sent events, so both are read.
//!
//! The client only ever sends `server/discover`, `initialize`,
//! `notifications/initialized` and `tools/list`. It never calls a tool, never
//! follows a redirect, and never fetches anything a declaration points at.

use reqwest::header::{ACCEPT, LOCATION};
use reqwest::redirect::Policy;
use serde_json::{json, Map, Value};
use std::cell::Cell;
use std::time::Duration;

use super::error::{McpContractError, Result};
use super::model::{ClientIdentity, Era};

/// The per-request protocol revision this client speaks.
pub const MODERN_PROTOCOL: &str = "2026-07-28";

/// The newest handshake revision this client offers to a server.
pub const LEGACY_PROTOCOL: &str = "2025-11-25";

/// The largest response body read from one request.
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

/// The most `tools/list` pages read before the server is declared malformed.
pub const MAX_PAGES: usize = 50;

/// JSON-RPC error codes that only a server speaking 2026-07-28 returns.
const MODERN_ERROR_CODES: [i64; 3] = [-32020, -32021, -32022];

const UNSUPPORTED_VERSION: i64 = -32022;

/// Words in a header or query-parameter name that mark it as a credential.
const CREDENTIAL_WORDS: [&str; 12] = [
    "authorization",
    "cookie",
    "token",
    "secret",
    "password",
    "passwd",
    "key",
    "apikey",
    "credential",
    "session",
    "sig",
    "signature",
];

/// How to ask a server, and as whom.
#[derive(Clone, Debug)]
pub struct ClientOptions {
    /// The MCP endpoint, `http` or `https`, without a fragment or credential.
    pub endpoint: String,
    /// Headers sent with every request, by lower-case name.
    pub headers: Vec<(String, String)>,
    /// How the client names itself to the server.
    pub client: ClientIdentity,
    /// Time allowed for each request.
    pub timeout: Duration,
}

/// What a server declared when it was asked.
#[derive(Clone, Debug)]
pub struct Declared {
    /// Which era answered.
    pub era: Era,
    /// The protocol revision the answer was given in.
    pub protocol_version: String,
    /// What the server says it is.
    pub implementation: Value,
    /// The features the server offers.
    pub capabilities: Value,
    /// The server-wide guidance for the model.
    pub instructions: Option<String>,
    /// The tools as the server sent them, every page in order.
    pub tools: Vec<Value>,
}

impl ClientOptions {
    /// How `mockforge` names itself unless told to imitate the agent's client.
    pub fn default_identity() -> ClientIdentity {
        ClientIdentity {
            name: "mockforge".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Options for `endpoint`, validated by [`ClientOptions::checked_endpoint`].
    pub fn new(endpoint: &str, client: ClientIdentity) -> Result<Self> {
        Ok(Self {
            endpoint: Self::checked_endpoint(endpoint)?,
            headers: Vec::new(),
            client,
            timeout: Duration::from_secs(30),
        })
    }

    /// The endpoint as it is stored and requested: an `http` or `https` URL
    /// with no user information, no credential in its query and no fragment.
    pub fn checked_endpoint(endpoint: &str) -> Result<String> {
        let mut url = reqwest::Url::parse(endpoint)
            .map_err(|_| McpContractError::Endpoint(format!("\"{endpoint}\" is not a URL")))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(McpContractError::Endpoint(format!(
                "\"{endpoint}\" is not an http or https address"
            )));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(McpContractError::Credential {
                what: "the user information in the endpoint".to_string(),
            });
        }
        for (name, _) in url.query_pairs() {
            if credential_like(&name) {
                return Err(McpContractError::Credential {
                    what: format!("the endpoint query parameter `{name}`"),
                });
            }
        }
        url.set_fragment(None);
        Ok(url.to_string())
    }

    /// Send a plain header with every request, given as `Name: value`. A
    /// header that carries a credential is refused: use
    /// [`ClientOptions::env_header`] for those.
    pub fn plain_header(&mut self, header: &str) -> Result<()> {
        let (name, value) = split_header(header)?;
        if credential_like(&name) {
            return Err(McpContractError::Credential {
                what: format!("the header `{name}`"),
            });
        }
        self.add_header(name, value)
    }

    /// Send a header whose value is read from an environment variable each
    /// time the server is asked, given as `Name: ENV_VAR`. The value is held
    /// for the life of these options and is never written to a contract.
    pub fn env_header(
        &mut self,
        header: &str,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<()> {
        let (name, variable) = split_header(header)?;
        let value = lookup(&variable).ok_or_else(|| {
            McpContractError::Header(format!(
                "environment variable {variable}, named for the `{name}` header, is not set"
            ))
        })?;
        self.add_header(name, value)
    }

    fn add_header(&mut self, name: String, value: String) -> Result<()> {
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| McpContractError::Header(format!("\"{name}\" is not a header name")))?;
        reqwest::header::HeaderValue::from_str(&value).map_err(|_| {
            McpContractError::Header(format!("the value of the `{name}` header cannot be sent"))
        })?;
        let name = name.as_str().to_string();
        self.headers.retain(|(existing, _)| *existing != name);
        self.headers.push((name, value));
        Ok(())
    }
}

/// Ask the server what it declares.
pub async fn fetch_declared(options: &ClientOptions) -> Result<Declared> {
    let http = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(options.timeout)
        .user_agent(format!("mockforge-mcp-contract/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| transport(&options.endpoint, &error))?;
    let transport = Transport {
        http,
        options,
        next_id: Cell::new(1),
    };
    let detected = transport.detect().await?;
    let tools = transport.list_tools(&detected.wire).await?;
    if detected.wire.session.is_some() {
        transport.end_session(&detected.wire).await;
    }
    Ok(Declared {
        era: detected.wire.era,
        protocol_version: detected.wire.protocol_version,
        implementation: detected.implementation,
        capabilities: detected.capabilities,
        instructions: detected.instructions,
        tools,
    })
}

fn credential_like(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.split(['-', '_']).any(|word| CREDENTIAL_WORDS.contains(&word))
        || CREDENTIAL_WORDS.contains(&lower.as_str())
}

fn split_header(header: &str) -> Result<(String, String)> {
    let Some((name, value)) = header.split_once(':') else {
        return Err(McpContractError::Header(format!(
            "\"{header}\" is not written \"Name: value\""
        )));
    };
    let name = name.trim();
    if name.is_empty() || value.trim().is_empty() {
        return Err(McpContractError::Header(format!(
            "\"{header}\" is not written \"Name: value\""
        )));
    }
    Ok((name.to_string(), value.trim().to_string()))
}

/// The detail of an HTTP error. A refusal names the credential to look at.
fn refusal(status: u16, note: String) -> String {
    if matches!(status, 401 | 403) {
        format!("{note}; the server refused the request, so check the credential passed with --auth-env and the address")
    } else {
        note
    }
}

fn transport(endpoint: &str, error: &reqwest::Error) -> McpContractError {
    McpContractError::Transport {
        url: endpoint.to_string(),
        reason: if error.is_timeout() {
            "it did not answer in time".to_string()
        } else {
            error.to_string()
        },
    }
}

/// What the server answered to when this client was speaking.
struct Wire {
    era: Era,
    protocol_version: String,
    session: Option<String>,
}

/// One answer from the server.
struct Reply {
    status: u16,
    session: Option<String>,
    value: Option<Value>,
    /// What the body was, when it did not hold the answer.
    note: String,
}

struct Detected {
    wire: Wire,
    implementation: Value,
    capabilities: Value,
    instructions: Option<String>,
}

struct Transport<'a> {
    http: reqwest::Client,
    options: &'a ClientOptions,
    next_id: Cell<u64>,
}

impl Transport<'_> {
    /// Find out which era the server speaks, the modern way first.
    async fn detect(&self) -> Result<Detected> {
        let modern = Wire {
            era: Era::Modern,
            protocol_version: MODERN_PROTOCOL.to_string(),
            session: None,
        };
        let discover = self.call("server/discover", json!({}), true, &modern).await?;
        if let Some(result) = result_of(discover.value.as_ref()) {
            // The per-request protocol has no sessions, whatever headers came back.
            return Ok(detected(modern, None, result));
        }
        if let Some(error) = error_of(discover.value.as_ref()) {
            if error
                .get("code")
                .and_then(Value::as_i64)
                .is_some_and(|code| MODERN_ERROR_CODES.contains(&code))
            {
                return Err(self.modern_rejection(error));
            }
        }
        let first = describe("server/discover", &discover);

        // The handshake carries no modern metadata: a server that serves both
        // eras picks the legacy behaviour for it.
        let handshake = Wire {
            era: Era::Legacy,
            protocol_version: String::new(),
            session: None,
        };
        let initialize = self
            .call(
                "initialize",
                json!({
                    "protocolVersion": LEGACY_PROTOCOL,
                    "capabilities": {},
                    "clientInfo": {"name": self.options.client.name, "version": self.options.client.version},
                }),
                true,
                &handshake,
            )
            .await?;
        let second = describe("initialize", &initialize);
        if matches!(initialize.status, 401 | 403) {
            return Err(McpContractError::HttpStatus {
                method: "initialize".to_string(),
                status: initialize.status,
                detail: refusal(initialize.status, initialize.note.clone()),
            });
        }
        if let Some(error) = error_of(initialize.value.as_ref()) {
            return Err(McpContractError::Rejected {
                method: "initialize".to_string(),
                code: error.get("code").and_then(Value::as_i64).unwrap_or_default(),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("no message")
                    .to_string(),
            });
        }
        let legacy_result = result_of(initialize.value.as_ref());
        let Some(result) = legacy_result
            .filter(|result| result.get("protocolVersion").and_then(Value::as_str).is_some())
        else {
            return Err(McpContractError::NotMcp {
                url: self.options.endpoint.clone(),
                detail: format!("{first}; {second}"),
            });
        };
        let legacy = Wire {
            era: Era::Legacy,
            protocol_version: result["protocolVersion"]
                .as_str()
                .unwrap_or(LEGACY_PROTOCOL)
                .to_string(),
            session: initialize.session.clone(),
        };
        // The notification has no id and no answer to read. A server that
        // rejects it is still asked for its tools.
        let _ = self.call("notifications/initialized", json!({}), false, &legacy).await;
        Ok(detected(legacy, initialize.session, result))
    }

    /// A modern server that refused the modern request.
    fn modern_rejection(&self, error: &Value) -> McpContractError {
        let message =
            error.get("message").and_then(Value::as_str).unwrap_or("no message").to_string();
        if error.get("code").and_then(Value::as_i64) == Some(UNSUPPORTED_VERSION) {
            let supported = error
                .get("data")
                .and_then(|data| data.get("supported"))
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
            return McpContractError::UnsupportedVersion {
                url: self.options.endpoint.clone(),
                supported,
                modern: MODERN_PROTOCOL.to_string(),
                legacy: LEGACY_PROTOCOL.to_string(),
            };
        }
        McpContractError::Rejected {
            method: "server/discover".to_string(),
            code: error.get("code").and_then(Value::as_i64).unwrap_or_default(),
            message,
        }
    }

    /// Every tool the server offers, following pagination.
    async fn list_tools(&self, wire: &Wire) -> Result<Vec<Value>> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut params = Map::new();
            if let Some(cursor) = &cursor {
                params.insert("cursor".to_string(), Value::String(cursor.clone()));
            }
            let reply = self.call("tools/list", Value::Object(params), true, wire).await?;
            let malformed = |detail: &str| McpContractError::Protocol {
                method: "tools/list".to_string(),
                detail: detail.to_string(),
            };
            if let Some(error) = error_of(reply.value.as_ref()) {
                return Err(McpContractError::Rejected {
                    method: "tools/list".to_string(),
                    code: error.get("code").and_then(Value::as_i64).unwrap_or_default(),
                    message: error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("no message")
                        .to_string(),
                });
            }
            let Some(result) = result_of(reply.value.as_ref()) else {
                if (200..300).contains(&reply.status) {
                    return Err(malformed(&format!(
                        "it answered HTTP {} without a result: {}",
                        reply.status, reply.note
                    )));
                }
                return Err(McpContractError::HttpStatus {
                    method: "tools/list".to_string(),
                    status: reply.status,
                    detail: refusal(reply.status, reply.note),
                });
            };
            match result.get("resultType").and_then(Value::as_str) {
                None | Some("complete") => {}
                Some(other) => {
                    return Err(malformed(&format!(
                        "it returned `resultType: {other}`, not a complete list"
                    )))
                }
            }
            let Some(list) = result.get("tools").and_then(Value::as_array) else {
                return Err(malformed("it returned no `tools` array"));
            };
            tools.extend(list.iter().cloned());
            cursor = result.get("nextCursor").and_then(Value::as_str).map(str::to_string);
            if cursor.is_none() {
                return Ok(tools);
            }
        }
        Err(McpContractError::Protocol {
            method: "tools/list".to_string(),
            detail: format!("it listed more than {MAX_PAGES} pages"),
        })
    }

    /// End a handshake session. Purely polite: nothing is read from the answer.
    async fn end_session(&self, wire: &Wire) {
        let Some(session) = &wire.session else {
            return;
        };
        let request = self
            .http
            .delete(&self.options.endpoint)
            .header("Mcp-Session-Id", session.as_str())
            .header("MCP-Protocol-Version", wire.protocol_version.as_str());
        let _ = request.send().await;
    }

    /// Send one JSON-RPC message and read the one answer to it. A request
    /// has an id; a notification does not and expects no answer.
    async fn call(
        &self,
        method: &str,
        params: Value,
        is_request: bool,
        wire: &Wire,
    ) -> Result<Reply> {
        let malformed = || McpContractError::Protocol {
            method: method.to_string(),
            detail: "the parameters of the request could not be built".to_string(),
        };
        let Value::Object(mut params) = params else {
            return Err(malformed());
        };
        let id = self.next_id.replace(self.next_id.get() + 1);
        if wire.era == Era::Modern {
            params.insert(
                "_meta".to_string(),
                json!({
                    "io.modelcontextprotocol/protocolVersion": wire.protocol_version,
                    "io.modelcontextprotocol/clientInfo": {
                        "name": self.options.client.name,
                        "version": self.options.client.version,
                    },
                    "io.modelcontextprotocol/clientCapabilities": {},
                }),
            );
        }
        let mut message = Map::new();
        message.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
        message.insert("method".to_string(), Value::String(method.to_string()));
        message.insert("params".to_string(), Value::Object(params));
        if is_request {
            message.insert("id".to_string(), Value::from(id));
        }
        let mut request = self
            .http
            .post(&self.options.endpoint)
            .header(ACCEPT, "application/json, text/event-stream")
            .json(&Value::Object(message));
        for (name, value) in &self.options.headers {
            request = request.header(name.as_str(), value.as_str());
        }
        if wire.era == Era::Modern {
            request = request.header("MCP-Protocol-Version", wire.protocol_version.as_str());
            request = request.header("Mcp-Method", method);
        } else if !wire.protocol_version.is_empty() {
            request = request.header("MCP-Protocol-Version", wire.protocol_version.as_str());
        }
        if let Some(session) = &wire.session {
            request = request.header("Mcp-Session-Id", session.as_str());
        }
        let response = request
            .send()
            .await
            .map_err(|error| transport(&self.options.endpoint, &error))?;
        let status = response.status().as_u16();
        if (300..400).contains(&status) {
            return Err(McpContractError::Redirect {
                url: self.options.endpoint.clone(),
                location: response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("an address it did not name")
                    .to_string(),
            });
        }
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let mut note;
        let value = if !is_request || status == 202 {
            note = "accepted".to_string();
            None
        } else if content_type == "text/event-stream" {
            let (value, streamed) = self.read_stream(response, id).await?;
            note = streamed;
            value
        } else {
            let (value, read) = self.read_body(response).await?;
            note = read;
            match value {
                Some(value) => match response_to(value, id) {
                    Some(answer) => Some(answer),
                    None => {
                        note = "a JSON message that is not the answer to this request".to_string();
                        None
                    }
                },
                None => None,
            }
        };
        Ok(Reply {
            status,
            session,
            value,
            note: if content_type.is_empty() {
                note
            } else {
                format!("{content_type}: {note}")
            },
        })
    }

    async fn read_body(&self, response: reqwest::Response) -> Result<(Option<Value>, String)> {
        let mut response = response;
        let mut body: Vec<u8> = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| transport(&self.options.endpoint, &error))?
        {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(McpContractError::TooLarge {
                    url: self.options.endpoint.clone(),
                    limit: MAX_BODY_BYTES,
                });
            }
            body.extend_from_slice(&chunk);
        }
        let text = String::from_utf8_lossy(&body);
        if text.trim().is_empty() {
            return Ok((None, "an empty body".to_string()));
        }
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok((Some(value), "json".to_string())),
            Err(_) => {
                Ok((None, format!("{} bytes that are not JSON ({})", body.len(), excerpt(&text))))
            }
        }
    }

    /// Read a stream of server-sent events until the answer to `id` arrives,
    /// then stop: a server may keep the stream open after it.
    async fn read_stream(
        &self,
        response: reqwest::Response,
        id: u64,
    ) -> Result<(Option<Value>, String)> {
        let mut response = response;
        let mut parser = EventParser::default();
        let mut buffer: Vec<u8> = Vec::new();
        let mut read = 0usize;
        loop {
            let chunk = response
                .chunk()
                .await
                .map_err(|error| transport(&self.options.endpoint, &error))?;
            let Some(chunk) = chunk else { break };
            read += chunk.len();
            if read > MAX_BODY_BYTES {
                return Err(McpContractError::TooLarge {
                    url: self.options.endpoint.clone(),
                    limit: MAX_BODY_BYTES,
                });
            }
            buffer.extend_from_slice(&chunk);
            while let Some(at) = buffer.iter().position(|byte| *byte == b'\n') {
                let line =
                    String::from_utf8_lossy(&buffer[..at]).trim_end_matches('\r').to_string();
                buffer.drain(..=at);
                if let Some((event, data)) = parser.line(&line) {
                    if !matches!(event.as_deref(), None | Some("message")) {
                        continue;
                    }
                    if let Ok(value) = serde_json::from_str::<Value>(&data) {
                        if let Some(answer) = response_to(value, id) {
                            return Ok((Some(answer), "a stream of events".to_string()));
                        }
                    }
                }
            }
        }
        Ok((None, "a stream that closed before the answer".to_string()))
    }
}

fn detected(wire: Wire, session: Option<String>, result: &Value) -> Detected {
    let field = |key: &str| result.get(key).cloned().unwrap_or(Value::Null);
    let (implementation, instructions) = match wire.era {
        Era::Modern => (
            result
                .get("_meta")
                .and_then(|meta| meta.get("io.modelcontextprotocol/serverInfo"))
                .cloned()
                .unwrap_or(Value::Null),
            field("instructions").as_str().map(str::to_string),
        ),
        Era::Legacy => (field("serverInfo"), field("instructions").as_str().map(str::to_string)),
    };
    Detected {
        wire: Wire { session, ..wire },
        implementation,
        capabilities: field("capabilities"),
        instructions,
    }
}

/// The `result` member of a JSON-RPC answer, when there is one.
fn result_of(value: Option<&Value>) -> Option<&Value> {
    value.and_then(|value| value.get("result")).filter(|result| result.is_object())
}

/// The `error` member of a JSON-RPC answer, when there is one.
fn error_of(value: Option<&Value>) -> Option<&Value> {
    value.and_then(|value| value.get("error")).filter(|error| error.is_object())
}

/// Whether a JSON object is the answer to the request numbered `id`.
fn response_to(value: Value, id: u64) -> Option<Value> {
    (value.get("id") == Some(&Value::from(id))
        && (value.get("result").is_some() || value.get("error").is_some()))
    .then_some(value)
}

fn describe(method: &str, reply: &Reply) -> String {
    match (&reply.value, reply.status) {
        (Some(value), _) if error_of(Some(value)).is_some() => {
            let error = &value["error"];
            format!(
                "{method} returned an error: {} (code {})",
                error.get("message").and_then(Value::as_str).unwrap_or("no message"),
                error.get("code").and_then(Value::as_i64).unwrap_or_default()
            )
        }
        (Some(_), status) => format!("{method} answered HTTP {status} without a result"),
        (None, status) => format!("{method} answered HTTP {status} ({})", reply.note),
    }
}

fn excerpt(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = single_line.chars().take(60).collect();
    if single_line.chars().count() > 60 {
        format!("{cut}…")
    } else {
        cut
    }
}

/// Reads server-sent events one line at a time.
#[derive(Default)]
struct EventParser {
    data: Vec<String>,
    event: Option<String>,
}

impl EventParser {
    /// Feed one line without its line ending. Returns the event name and data
    /// when the line ends an event.
    fn line(&mut self, line: &str) -> Option<(Option<String>, String)> {
        if line.is_empty() {
            let event = (!self.data.is_empty()).then(|| (self.event.take(), self.data.join("\n")));
            self.data.clear();
            self.event = None;
            return event;
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => self.data.push(value.to_string()),
            "event" => self.event = Some(value.to_string()),
            _ => {}
        }
        None
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
