//! What the agent sent to the mock, as `serve --mcp-mock-record` writes it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{bad_call, Result};

/// How the agent names itself to the server.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Client {
    /// The name sent as `clientInfo.name`.
    pub name: String,
    /// The version sent as `clientInfo.version`.
    pub version: String,
}

/// What the mock answered a tool call. A log written by a MockForge that
/// recorded only the requests has none, and neither has a call the mock did
/// not answer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Answer {
    /// Whether the tool flagged its result as an error.
    #[serde(rename = "isError", default)]
    pub is_error: bool,
    /// The text of a result the tool flagged as an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Why the mock refused the request before any tool was called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

impl Answer {
    /// Why nothing was done, when nothing was: the tool's error, or the
    /// mock's refusal of the request.
    pub fn refusal(&self) -> Option<&str> {
        self.refused
            .as_deref()
            .or_else(|| self.text.as_deref().filter(|_| self.is_error))
    }
}

/// One request an agent sent to the mock MCP server.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Call {
    /// Where the request came in the run, counted from 1.
    pub seq: u64,
    /// When the mock received it, as an RFC 3339 timestamp.
    pub at: String,
    /// The JSON-RPC method, such as `tools/list` or `tools/call`.
    pub method: String,
    /// For `tools/call`: the tool that was called.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// For `tools/call`: the arguments the agent sent, in full. Nothing is
    /// left out: if the agent sent a secret, the secret is here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    /// For `tools/call`: what the mock answered, when the log holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<Answer>,
    /// The agent, when the request said who sends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<Client>,
}

impl Call {
    /// A tool call, as a suite's tests spell one.
    pub fn tool_call(seq: u64, tool: &str, arguments: Value) -> Self {
        Self {
            seq,
            at: String::new(),
            method: "tools/call".to_string(),
            tool: Some(tool.to_string()),
            arguments: Some(arguments),
            answer: None,
            client: None,
        }
    }

    /// [`Self::tool_call`], with what the mock answered it.
    pub fn answered(mut self, answer: Answer) -> Self {
        self.answer = Some(answer);
        self
    }
}

/// Read a call log: one call record per line, in the order they arrived.
/// A line that is not a call record is an error with its line number, never
/// skipped: a log that cannot be read in full cannot be judged.
pub fn read_calls(text: &str, path: &str) -> Result<Vec<Call>> {
    let mut calls = Vec::new();
    let ends_with_newline = text.ends_with('\n');
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let call: Call = serde_json::from_str(line).map_err(|error| {
            let cut = if error.is_eof() && !ends_with_newline {
                "; the log ends here, as though it was cut off while being written"
            } else {
                ""
            };
            bad_call(path, index + 1, &format!("not a call record: {error}{cut}"))
        })?;
        calls.push(call);
    }
    Ok(calls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_is_read_line_by_line_and_blank_lines_are_nothing() {
        let text = concat!(
            r#"{"seq":1,"at":"2026-11-25T03:00:00.1Z","method":"tools/list","client":{"name":"agent","version":"1"}}"#,
            "\n\n",
            r#"{"seq":2,"at":"2026-11-25T03:00:00.2Z","method":"tools/call","tool":"search","arguments":{"q":"x"}}"#,
            "\n",
        );
        let calls = read_calls(text, "calls.jsonl").unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].method, "tools/list");
        assert_eq!(calls[0].client.as_ref().map(|client| client.name.as_str()), Some("agent"));
        assert_eq!(calls[1].tool.as_deref(), Some("search"));
        assert_eq!(calls[1].arguments, Some(serde_json::json!({ "q": "x" })));
        assert!(calls[1].client.is_none());
    }

    #[test]
    fn a_line_that_is_not_a_call_record_names_its_line() {
        let text = concat!(
            r#"{"seq":1,"at":"t","method":"tools/list"}"#,
            "\n",
            "{\"seq\":2,\"at\":\"t\",\"method\":\"tools/call\",\"tool\":\"search\"",
        );
        let error = match read_calls(text, "calls.jsonl") {
            Ok(_) => panic!("a cut-off log was read"),
            Err(error) => error.to_string(),
        };
        assert!(error.starts_with("calls.jsonl: line 2: not a call record: "), "{error}");
        assert!(
            error.ends_with("the log ends here, as though it was cut off while being written"),
            "{error}"
        );
        let error = read_calls("{\"method\":\"tools/list\"}\n", "calls.jsonl").unwrap_err();
        assert!(
            error.to_string().contains("line 1: not a call record: missing field `seq`"),
            "{error}"
        );
        assert!(
            read_calls("{\"seq\":1,\"at\":\"t\",\"method\":\"ping\",\"extra\":1}\n", "c").is_ok()
        );
    }
}
