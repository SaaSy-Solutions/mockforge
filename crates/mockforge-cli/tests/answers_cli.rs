//! Answers by request, against the real `mockforge serve`: an operation's
//! `x-mockforge-answers` extension, and a response header set to the value
//! the spec gives it.
//!
//! An operation has one answer, its example. An answer the extension names is
//! given to the requests its `when` names, and to no other: a weakness
//! planted that way is shown to the request that triggers it, and an agent
//! that did nothing wrong is shown nothing.

mod support;
use assert_cmd::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

const SPEC: &str = r#"{ "openapi": "3.0.3", "info": { "title": "Answers", "version": "1" },
  "paths": {
    "/search": { "get": {
      "parameters": [{ "in": "query", "name": "q", "schema": { "type": "string" } }],
      "responses": { "200": { "description": "ok",
        "content": { "application/json": { "example": { "results": ["shoes"] } } } } },
      "x-mockforge-answers": [
        { "when": { "query_contains": { "q": "OR 1=1" } },
          "then": { "status": 200, "headers": { "X-Query-Debug": "on" },
            "body": { "results": ["every order of every customer"], "asked": "{{request.query.q}}" } } }
      ] } },
    "/me": { "put": {
      "responses": { "200": { "description": "ok",
        "content": { "application/json": { "example": { "id": 77, "name": "Ada" } } } } },
      "x-mockforge-answers": [
        { "when": { "body_contains": "\"role\"" },
          "then": { "status": 200, "body": { "id": 77, "name": "Ada", "role": "{{request.body.role}}" } } }
      ] } },
    "/ping": { "post": {
      "responses": { "200": { "description": "ok",
        "content": { "application/json": { "example": { "pong": true } } } } },
      "x-mockforge-answers": [
        { "when": { "headers": { "X-Debug": "1" } }, "then": { "status": 500, "body": { "error": "ping: command failed" } } }
      ] } },
    "/headers": { "get": {
      "responses": { "200": { "description": "ok",
        "headers": {
          "Access-Control-Allow-Origin": { "schema": { "type": "string" }, "example": "*" },
          "Access-Control-Allow-Credentials": { "schema": { "type": "string", "example": "true" } },
          "X-Page-Size": { "schema": { "type": "integer", "default": 100 } },
          "X-Made-Up": { "schema": { "type": "string" } }
        },
        "content": { "application/json": { "example": { "ok": true } } } } } } }
  } }"#;

/// The real `mockforge serve`, with `spec.json` served from a directory of
/// its own. It is stopped when dropped.
struct Server {
    /// The server's directory, kept alive for as long as it runs.
    _directory: tempfile::TempDir,
    child: Child,
    /// The port its HTTP listener was given.
    port: u16,
}

impl Server {
    /// Start the server, or say what it printed when it stopped instead.
    fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        // Nothing of the machine's own is found: no environment file, no other
        // configuration.
        fs::write(directory.path().join(".env"), "").unwrap();
        fs::write(directory.path().join("spec.json"), SPEC).unwrap();
        let spec = directory.path().join("spec.json").display().to_string();
        let mut command = Command::cargo_bin("mockforge").unwrap();
        support::isolated(&mut command, directory.path())
            .current_dir(directory.path())
            .args([
                "serve",
                "--no-config",
                "--spec",
                spec.as_str(),
                "--http-port",
                "0",
                "--ws-port",
                "0",
                "--grpc-port",
                "0",
                "--admin-port",
                "0",
                "--metrics-port",
                "0",
            ]);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        // What the server prints is read to the end and let go of: a server
        // whose pipe fills up stops serving.
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let port = line
                    .split("HTTP server listening on http://localhost:")
                    .nth(1)
                    .and_then(|rest| rest.trim().parse::<u16>().ok());
                if let Some(port) = port {
                    let _ = ready_tx.send(Ok(port));
                }
            }
            let mut error = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut error);
            let _ = ready_tx.send(Err(error));
        });
        let port = match ready_rx.recv_timeout(std::time::Duration::from_secs(120)) {
            Ok(Ok(port)) => port,
            Ok(Err(error)) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the server stopped instead of listening:\n{error}");
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the server did not listen within two minutes");
            }
        };
        Self {
            _directory: directory,
            child,
            port,
        }
    }

    /// What the server answered to a request: the status, the headers by
    /// their names in lower case, and the body when it is JSON.
    fn ask(
        &self,
        method: &str,
        target: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (u16, BTreeMap<String, String>, serde_json::Value) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        let mut request = format!(
            "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        if !body.is_empty() {
            request.push_str("Content-Type: application/json\r\n");
        }
        request.push_str("\r\n");
        request.push_str(body);
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
        let status = head
            .split("\r\n")
            .next()
            .and_then(|first| first.split(' ').nth(1))
            .and_then(|status| status.parse::<u16>().ok())
            .unwrap_or_else(|| panic!("no status line in: {response}"));
        let headers = head
            .split("\r\n")
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_string()))
            .collect();
        let json = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        (status, headers, json)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_request_an_answer_names_is_answered_with_it() {
    let server = Server::start();
    // As it was sent, encoded; the mock reads it decoded, and says it back.
    let (status, headers, body) = server.ask("GET", "/search?q=shoes%27%20OR%201%3D1--", &[], "");
    assert_eq!(status, 200);
    assert_eq!(
        body,
        serde_json::json!({ "results": ["every order of every customer"], "asked": "shoes' OR 1=1--" })
    );
    assert_eq!(headers.get("x-query-debug").map(String::as_str), Some("on"));
}

#[test]
fn every_other_request_of_the_operation_still_gets_the_example() {
    let server = Server::start();
    for target in ["/search?q=shoes", "/search", "/search?q=organic"] {
        let (status, headers, body) = server.ask("GET", target, &[], "");
        assert_eq!(status, 200, "{target}");
        assert_eq!(body, serde_json::json!({ "results": ["shoes"] }), "{target}");
        assert!(!headers.contains_key("x-query-debug"), "{target}");
    }
}

#[test]
fn an_answer_may_name_the_body_and_say_what_it_holds() {
    let server = Server::start();
    // A field only the server may set, set by the client: the answer is the
    // profile with the role the client sent. A profile updated with its own
    // fields is the profile.
    let (status, _, body) = server.ask("PUT", "/me", &[], r#"{"role": "admin"}"#);
    assert_eq!(
        (status, body),
        (200, serde_json::json!({ "id": 77, "name": "Ada", "role": "admin" }))
    );
    let (status, _, body) = server.ask("PUT", "/me", &[], r#"{"name": "Grace"}"#);
    assert_eq!((status, body), (200, serde_json::json!({ "id": 77, "name": "Ada" })));
}

#[test]
fn a_header_tells_the_requests_an_answer_is_given_to() {
    let server = Server::start();
    let (status, _, body) = server.ask("POST", "/ping", &[("X-Debug", "1")], "{}");
    assert_eq!((status, body), (500, serde_json::json!({ "error": "ping: command failed" })));
    let (status, _, body) = server.ask("POST", "/ping", &[], "{}");
    assert_eq!((status, body), (200, serde_json::json!({ "pong": true })));
}

#[test]
fn a_response_header_is_answered_with_the_value_the_spec_gives_it() {
    let server = Server::start();
    let (status, headers, body) = server.ask("GET", "/headers", &[], "");
    assert_eq!((status, body), (200, serde_json::json!({ "ok": true })));
    assert_eq!(headers.get("access-control-allow-origin").map(String::as_str), Some("*"));
    assert_eq!(
        headers.get("access-control-allow-credentials").map(String::as_str),
        Some("true")
    );
    assert_eq!(headers.get("x-page-size").map(String::as_str), Some("100"));
    // One the spec gives no value is made up, as before.
    assert_eq!(headers.get("x-made-up").map(String::as_str), Some("mockforge-mock-value"));
}
