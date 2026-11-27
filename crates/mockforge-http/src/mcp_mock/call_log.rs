//! Writing down what an agent sends the mock, so that it can be judged.
//!
//! `serve --mcp-mock-record FILE` starts a log. Every request that expects an
//! answer becomes one line of JSON, in the shape
//! `mockforge_contracts::agent_redteam::read_calls` reads: when it arrived,
//! the method, the tool and the arguments of a `tools/call`, and the agent's
//! identity when the request carries it. For a tool call the log also holds
//! what the mock answered: whether the tool flagged an error, with its text
//! when it did, or why the mock refused the request before a tool was called.
//! A result that is not an error is not written down. Arguments are written in
//! full: if the agent sent a secret, the secret is in the log.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Map, Value};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::CLIENT_INFO_KEY;

/// A file the mock appends one line to per request.
#[derive(Clone, Debug)]
pub struct McpCallLog {
    file: Arc<Mutex<tokio::fs::File>>,
    /// The last sequence number handed out.
    sequence: Arc<AtomicU64>,
}

impl McpCallLog {
    /// Start the log at `path`. A file that is there is replaced.
    pub async fn create(path: &Path) -> std::io::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            sequence: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Write a request down as the next line, with the mock's answer to it.
    /// When this returns the line is in the file, so whoever reads the log
    /// after a call finds that call in it.
    pub async fn record(&self, request: &Value, answered: Option<&Value>) -> std::io::Result<()> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let line = serde_json::to_string(&record(sequence, request, answered))
            .map_err(std::io::Error::other)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        // A write on tokio's file is handed to a thread and reported done
        // before that thread has run. The flush waits for it: when this
        // returns, the line is where whoever reads the log finds it.
        file.flush().await
    }
}

/// A request as the log holds it, with what the mock answered it.
fn record(sequence: u64, request: &Value, answered: Option<&Value>) -> Value {
    let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = request.get("params");
    let mut record = Map::new();
    record.insert("seq".to_string(), json!(sequence));
    record.insert(
        "at".to_string(),
        json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true)),
    );
    record.insert("method".to_string(), json!(method));
    if method == "tools/call" {
        if let Some(tool) = params.and_then(|params| params.get("name")).and_then(Value::as_str) {
            record.insert("tool".to_string(), json!(tool));
        }
        if let Some(arguments) = params
            .and_then(|params| params.get("arguments"))
            .filter(|value| value.is_object())
        {
            record.insert("arguments".to_string(), arguments.clone());
        }
        if let Some(answer) = answered.map(answer) {
            record.insert("answer".to_string(), answer);
        }
    }
    // A handshake-era request names the client in `params.clientInfo`; a
    // request on 2026-07-28 names it in every request's metadata.
    let client_info = params.and_then(|params| {
        params
            .get("clientInfo")
            .or_else(|| params.get("_meta").and_then(|meta| meta.get(CLIENT_INFO_KEY)))
    });
    if let Some(client) = client_info.and_then(client) {
        record.insert("client".to_string(), client);
    }
    Value::Object(record)
}

/// What the mock answered a tool call, as the log holds it: whether the tool
/// flagged its result as an error, with the text when it did, or why the mock
/// refused the request before a tool was called. A result that is not an error
/// is not written down: it holds nothing a finding needs, and a tool may send
/// an argument, a secret even, straight back.
fn answer(response: &Value) -> Value {
    if let Some(error) = response.get("error") {
        return json!({
            "refused": error.get("message").and_then(Value::as_str).unwrap_or_default(),
        });
    }
    let result = &response["result"];
    let is_error = result.get("isError").and_then(Value::as_bool).unwrap_or(false);
    let mut answer = Map::new();
    answer.insert("isError".to_string(), json!(is_error));
    if is_error {
        let text = result["content"]
            .as_array()
            .and_then(|items| items.first())
            .and_then(|item| item.get("text"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        answer.insert("text".to_string(), json!(text));
    }
    Value::Object(answer)
}

/// A client's identity, when it says one: a name, and a version when given.
fn client(info: &Value) -> Option<Value> {
    let name = info.get("name").and_then(Value::as_str).filter(|name| !name.is_empty())?;
    Some(json!({
        "name": name,
        "version": info.get("version").and_then(Value::as_str).unwrap_or_default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_contracts::agent_redteam::{read_calls, Client};
    use std::path::PathBuf;

    /// The path of a log in a directory of its own, and that directory.
    fn place() -> (PathBuf, PathBuf) {
        let directory =
            std::env::temp_dir().join(format!("mockforge-mcp-log-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        (directory.join("calls.jsonl"), directory)
    }

    /// A request on 2026-07-28 from `my-agent` 2.0.
    fn modern(method: &str, params: Value) -> Value {
        let mut inner = params.as_object().cloned().unwrap_or_default();
        inner.insert(
            "_meta".to_string(),
            json!({ CLIENT_INFO_KEY: { "name": "my-agent", "version": "2.0" } }),
        );
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": Value::Object(inner) })
    }

    #[tokio::test]
    async fn every_request_that_expects_an_answer_is_one_line() {
        let (path, directory) = place();
        let log = McpCallLog::create(&path).await.unwrap();
        log.record(&modern("tools/list", json!({})), None).await.unwrap();
        // A tool that did what was asked: its answer's text is not written down.
        log.record(
            &modern(
                "tools/call",
                json!({ "name": "search", "arguments": { "q": "rotation", "nested": { "a": [1, 2] } } }),
            ),
            Some(&json!({ "jsonrpc": "2.0", "id": 1, "result": {
                "content": [{ "type": "text", "text": "rotation" }], "isError": false
            } })),
        )
        .await
        .unwrap();
        // The handshake names the client in its own request only.
        log.record(
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "initialize",
                "params": { "clientInfo": { "name": "old-agent" }, "protocolVersion": "2024-11-05" }
            }),
            None,
        )
        .await
        .unwrap();
        // A call without arguments, from an unnamed client, that the tool refused.
        log.record(
            &modern("tools/call", json!({ "name": "ping" })),
            Some(&json!({ "jsonrpc": "2.0", "id": 3, "result": {
                "content": [{ "type": "text", "text": "unknown tool: ping" }], "isError": true
            } })),
        )
        .await
        .unwrap();
        log.record(&json!({ "jsonrpc": "2.0", "id": 3, "method": "ping", "params": {} }), None)
            .await
            .unwrap();
        // A call the mock refused before any tool was called.
        log.record(
            &json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "search" } }),
            Some(&json!({ "jsonrpc": "2.0", "id": 4, "error": { "code": -32602, "message": "a request on 2026-07-28 carries its version" } })),
        )
        .await
        .unwrap();
        drop(log);

        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        let calls = read_calls(&text, "calls.jsonl").unwrap();
        let sequences: Vec<u64> = calls.iter().map(|call| call.seq).collect();
        assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6]);
        let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
        assert_eq!(
            methods,
            vec![
                "tools/list",
                "tools/call",
                "initialize",
                "tools/call",
                "ping",
                "tools/call"
            ]
        );
        assert_eq!(calls[1].tool.as_deref(), Some("search"));
        assert_eq!(calls[1].arguments, Some(json!({ "q": "rotation", "nested": { "a": [1, 2] } })));
        assert_eq!(
            calls[1].client,
            Some(Client {
                name: "my-agent".to_string(),
                version: "2.0".to_string()
            })
        );
        assert_eq!(
            calls[2].client,
            Some(Client {
                name: "old-agent".to_string(),
                version: String::new()
            })
        );
        assert!(
            calls[3].arguments.is_none() && calls[4].client.is_none() && calls[4].tool.is_none()
        );
        assert!(calls[0].at.contains('T') && calls[0].at.ends_with('Z'), "{}", calls[0].at);

        // What the mock answered travels with the call, and with a tool call only.
        assert_eq!(calls[1].answer.as_ref().map(|answer| answer.is_error), Some(false));
        assert_eq!(calls[1].answer.as_ref().and_then(|answer| answer.text.as_deref()), None);
        assert_eq!(
            calls[3].answer.as_ref().map(|answer| (answer.is_error, answer.text.as_deref())),
            Some((true, Some("unknown tool: ping")))
        );
        assert_eq!(
            calls[5].answer.as_ref().and_then(|answer| answer.refused.as_deref()),
            Some("a request on 2026-07-28 carries its version")
        );
        assert!(
            calls[0].answer.is_none() && calls[2].answer.is_none() && calls[4].answer.is_none()
        );
        // A log written before answers were recorded reads the same, without them.
        let old = text
            .lines()
            .map(|line| {
                let mut value: Value = serde_json::from_str(line).unwrap();
                value.as_object_mut().unwrap().remove("answer");
                value.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(read_calls(&old, "calls.jsonl")
            .unwrap()
            .iter()
            .all(|call| call.answer.is_none()));
    }

    #[tokio::test]
    async fn a_log_replaces_what_was_there_and_can_be_shared() {
        let (path, directory) = place();
        std::fs::write(&path, "left over\n").unwrap();
        let log = McpCallLog::create(&path).await.unwrap();
        let shared = log.clone();
        shared
            .record(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }), None)
            .await
            .unwrap();
        log.record(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }), None)
            .await
            .unwrap();
        drop((shared, log));
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(!text.contains("left over"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn a_line_is_in_the_file_when_the_log_has_written_it() {
        // A write on tokio's file is handed to a thread of the runtime's
        // blocking pool, and reported done before that thread has run. A
        // runtime with one such thread is made here, and the thread is held
        // across the writes: a log that returned without a flush would leave
        // the newline of its line still to write.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let (path, directory) = place();
            let log = McpCallLog::create(&path).await.unwrap();
            let (holding, held) = std::sync::mpsc::channel();
            tokio::task::spawn_blocking(move || {
                holding.send(()).unwrap();
                // Long enough for the line to wait behind this task.
                std::thread::sleep(std::time::Duration::from_millis(100));
                // The thread is held once more, past the newline being handed
                // to it.
                tokio::task::spawn_blocking(|| {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                });
            });
            held.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
            log.record(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }), None)
                .await
                .unwrap();
            drop(log);
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::remove_dir_all(&directory).unwrap();
            assert!(text.ends_with('\n'), "{text}");
        });
    }

    #[tokio::test]
    async fn a_log_that_cannot_be_started_is_an_error() {
        let missing = std::env::temp_dir().join("mockforge-no-such-dir").join("calls.jsonl");
        assert!(McpCallLog::create(&missing).await.is_err());
    }
}
