//! Writing down what an agent sends the mock, so that it can be judged.
//!
//! `serve --mcp-mock-record FILE` starts a log. Every request that expects an
//! answer becomes one line of JSON, in the shape
//! `mockforge_contracts::agent_redteam::read_calls` reads: when it arrived,
//! the method, the tool and the arguments of a `tools/call`, and the agent's
//! identity when the request carries it. Arguments are written in full: if the
//! agent sent a secret, the secret is in the log.

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

    /// Write a request down as the next line.
    pub async fn record(&self, request: &Value) -> std::io::Result<()> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let line =
            serde_json::to_string(&record(sequence, request)).map_err(std::io::Error::other)?;
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await
    }
}

/// A request as the log holds it.
fn record(sequence: u64, request: &Value) -> Value {
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
        log.record(&modern("tools/list", json!({}))).await.unwrap();
        log.record(&modern(
            "tools/call",
            json!({ "name": "search", "arguments": { "q": "rotation", "nested": { "a": [1, 2] } } }),
        ))
        .await
        .unwrap();
        // The handshake names the client in its own request only.
        log.record(&json!({
            "jsonrpc": "2.0", "id": 2, "method": "initialize",
            "params": { "clientInfo": { "name": "old-agent" }, "protocolVersion": "2024-11-05" }
        }))
        .await
        .unwrap();
        // A call without arguments, and a request from an unnamed client.
        log.record(&modern("tools/call", json!({ "name": "ping" }))).await.unwrap();
        log.record(&json!({ "jsonrpc": "2.0", "id": 3, "method": "ping", "params": {} }))
            .await
            .unwrap();
        drop(log);

        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        let calls = read_calls(&text, "calls.jsonl").unwrap();
        let sequences: Vec<u64> = calls.iter().map(|call| call.seq).collect();
        assert_eq!(sequences, vec![1, 2, 3, 4, 5]);
        let methods: Vec<&str> = calls.iter().map(|call| call.method.as_str()).collect();
        assert_eq!(
            methods,
            vec![
                "tools/list",
                "tools/call",
                "initialize",
                "tools/call",
                "ping"
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
    }

    #[tokio::test]
    async fn a_log_replaces_what_was_there_and_can_be_shared() {
        let (path, directory) = place();
        std::fs::write(&path, "left over\n").unwrap();
        let log = McpCallLog::create(&path).await.unwrap();
        let shared = log.clone();
        shared
            .record(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
            .await
            .unwrap();
        log.record(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
            .await
            .unwrap();
        drop((shared, log));
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(!text.contains("left over"));
        assert!(text.ends_with('\n'));
    }

    #[tokio::test]
    async fn a_log_that_cannot_be_started_is_an_error() {
        let missing = std::env::temp_dir().join("mockforge-no-such-dir").join("calls.jsonl");
        assert!(McpCallLog::create(&missing).await.is_err());
    }
}
