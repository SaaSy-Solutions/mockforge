//! `http.response_template_expand` in the configuration file, against the real
//! `mockforge serve`. The guide says it turns the random and time tokens of an
//! example on, `{{uuid}}` and `{{now}}` among them. It reached no spec before:
//! a route read the environment variable alone.
//!
//! Each test starts the server in a directory of its own, with the flag in its
//! `mockforge.yaml` and the variable set to what the test names, or not set.

use assert_cmd::prelude::*;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

const SPEC: &str = r#"{ "openapi": "3.0.3", "info": { "title": "Tokens", "version": "1" },
  "paths": { "/token": { "get": { "responses": { "200": { "description": "ok",
    "content": { "application/json": { "example": { "id": "{{uuid}}", "made": "{{now}}" } } } } } } } } }"#;

const ON: &str = "http:\n  response_template_expand: true\n";
const OFF: &str = "http:\n  response_template_expand: false\n";

/// The real `mockforge serve`, with `configuration` in the `mockforge.yaml` of
/// a directory of its own, `spec.json` served from there, and the variable as
/// `variable` says, or not set. It is stopped when dropped.
struct Server {
    /// The server's directory, kept alive for as long as it runs.
    _directory: tempfile::TempDir,
    child: Child,
    /// The port its HTTP listener was given.
    port: u16,
}

impl Server {
    /// Start the server, or say what it printed when it stopped instead.
    fn start(configuration: &str, variable: Option<&str>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        // Nothing of the machine's own is found: no environment file, no other
        // configuration.
        fs::write(directory.path().join(".env"), "").unwrap();
        fs::write(directory.path().join("spec.json"), SPEC).unwrap();
        fs::write(directory.path().join("mockforge.yaml"), configuration).unwrap();
        let spec = directory.path().join("spec.json").display().to_string();
        let mut command = Command::cargo_bin("mockforge").unwrap();
        command
            .env_clear()
            .env("TMPDIR", directory.path())
            .current_dir(directory.path())
            .args([
                "serve",
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
        if let Some(value) = variable {
            command.env("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", value);
        }
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
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// What `/token` answered, as one JSON document.
fn answer(port: u16) -> serde_json::Value {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let request =
        format!("GET /token HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let body = response.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or("");
    serde_json::from_str(body)
        .unwrap_or_else(|error| panic!("/token did not answer JSON ({error}): {response}"))
}

/// The tokens are expanded: `id` is a UUID and `made` holds no token.
fn assert_expanded(answer: &serde_json::Value) {
    let id = answer["id"].as_str().unwrap_or_else(|| panic!("no id: {answer}"));
    uuid::Uuid::parse_str(id)
        .unwrap_or_else(|error| panic!("the id is not expanded, it is {id}: {error}"));
    let made = answer["made"].as_str().unwrap();
    assert!(!made.contains("{{"), "the time is not expanded, it is {made}");
}

/// The tokens are as the spec wrote them.
fn assert_as_written(answer: &serde_json::Value) {
    assert_eq!(answer["id"], "{{uuid}}");
    assert_eq!(answer["made"], "{{now}}");
}

#[test]
fn the_configuration_turns_the_tokens_on_when_the_variable_is_not_set() {
    let server = Server::start(ON, None);
    assert_expanded(&answer(server.port));
}

#[test]
fn the_variable_turns_them_on_though_the_configuration_does_not() {
    let server = Server::start(OFF, Some("true"));
    assert_expanded(&answer(server.port));
}

#[test]
fn the_variable_turns_them_off_though_the_configuration_does_not() {
    let server = Server::start(ON, Some("false"));
    assert_as_written(&answer(server.port));
}

#[test]
fn nothing_on_leaves_the_tokens_as_they_are() {
    let server = Server::start(OFF, None);
    assert_as_written(&answer(server.port));
}
