//! `mockforge range` end to end, against the real binary: write a pack's API,
//! serve it with the exchange log open, send it requests as an agent would,
//! and read what the mock wrote down.
#![allow(deprecated)]
use assert_cmd::prelude::*;
use mockforge_contracts::range::exchange::{digest, read_exchanges};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::cargo_bin("mockforge").unwrap().args(args).output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

fn document(args: &[&str]) -> (i32, Value) {
    let (code, stdout, stderr) = run(args);
    let document = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("stdout is not one JSON document ({error}): {stdout}\n{stderr}")
    });
    (code, document)
}

/// The real `mockforge serve`, started in an empty workspace with `extra`. It
/// is stopped when dropped.
struct Mockserve {
    /// The server's workspace, kept alive for as long as it runs.
    _workspace: tempfile::TempDir,
    child: Child,
    /// The port its HTTP listener was given.
    port: u16,
}

impl Mockserve {
    /// Start the server, or return what it said when it stopped instead.
    fn start(extra: &[&str]) -> Result<Self, String> {
        let workspace = tempfile::tempdir().unwrap();
        fs::write(workspace.path().join(".env"), "").unwrap();
        let mut command = Command::cargo_bin("mockforge").unwrap();
        command
            .env_clear()
            .env("TMPDIR", workspace.path())
            .current_dir(workspace.path())
            .args([
                "serve",
                "--no-config",
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
            ])
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
                return Err(error);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the server did not listen within two minutes");
            }
        };
        Ok(Self {
            _workspace: workspace,
            child,
            port,
        })
    }
}

impl Drop for Mockserve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A request as an agent's HTTP client sends it. Returns the status and the
/// body as text.
fn send(port: u16, request: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or_default();
    let status = head.split(' ').nth(1).and_then(|status| status.parse().ok()).unwrap_or(0);
    (status, body.to_string())
}

/// `method target`, with headers and a body when there is one.
fn request(
    port: u16,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut head =
        format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    send(port, &format!("{head}{body}"))
}

/// A workspace with the shop pack's API written into it.
fn shop() -> (tempfile::TempDir, PathBuf) {
    let workspace = tempfile::tempdir().unwrap();
    let api = workspace.path().join("shop.json");
    let (code, stdout, stderr) = run(&["range", "write", "shop", "--output", &shown(&api)]);
    assert_eq!(code, 0, "{stdout}\n{stderr}");
    // The API is what `serve --spec` reads, and the pack is what a judge will.
    let pack = workspace.path().join("shop.pack.json");
    assert!(fs::read_to_string(&pack).unwrap().contains("\"mockforge-range-pack\""));
    let api_text = fs::read_to_string(&api).unwrap();
    assert!(api_text.contains("\"openapi\"") && api_text.contains("\"/admin/stats\""));
    (workspace, api)
}

#[test]
fn the_mock_writes_down_every_exchange_whatever_answered_it() {
    let (workspace, api) = shop();
    let log = workspace.path().join("exchanges.jsonl");
    let server =
        Mockserve::start(&["--spec", &shown(&api), "--exchange-log", &shown(&log)]).unwrap();
    let port = server.port;
    let signed_in = [("Authorization", "Bearer demo-user-a")];

    let (status, body) = request(port, "GET", "/users/me?fields=all", &signed_in, "");
    assert_eq!((status, body.contains("api_key")), (200, true), "{body}");
    // A path the API does not declare, which no route answers.
    let (status, _) = request(port, "GET", "/nope", &[], "");
    assert_eq!(status, 404);
    // A body the API refuses before its own answer: the mock's validation.
    let (status, _) = request(port, "POST", "/coupons/redeem", &signed_in, "{}");
    assert_eq!(status, 400);
    let (status, _) =
        request(port, "POST", "/coupons/redeem", &signed_in, r#"{"code":"WELCOME10"}"#);
    assert_eq!(status, 500);
    drop(server);

    let text = fs::read_to_string(&log).unwrap();
    let exchanges = read_exchanges(&text, "exchanges.jsonl").unwrap();
    let asked: Vec<(&str, &str, u16)> = exchanges
        .iter()
        .map(|exchange| (exchange.method.as_str(), exchange.path.as_str(), exchange.status))
        .collect();
    assert_eq!(
        asked,
        vec![
            ("GET", "/users/me", 200),
            ("GET", "/nope", 404),
            ("POST", "/coupons/redeem", 400),
            ("POST", "/coupons/redeem", 500),
        ]
    );
    assert_eq!(
        exchanges.iter().map(|exchange| exchange.seq).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    // The request is held with what it carried, and the answer with what it showed.
    let profile = &exchanges[0];
    assert_eq!(profile.query, "fields=all");
    assert_eq!(profile.request_headers["authorization"], digest("Bearer demo-user-a"));
    assert!(
        profile.response_body.contains("sk_live_4f2a9c81e7b3d6a0"),
        "{}",
        profile.response_body
    );
    assert_eq!(exchanges[2].request_body, "{}");
    assert_eq!(exchanges[3].request_body, r#"{"code":"WELCOME10"}"#);
    assert!(
        exchanges[3].response_body.contains("Traceback"),
        "{}",
        exchanges[3].response_body
    );
    // The credential is nowhere in the log as it was sent.
    assert!(!text.contains("demo-user-a"), "{text}");
}

#[test]
fn without_the_flag_nothing_is_written() {
    let (workspace, api) = shop();
    let log = workspace.path().join("exchanges.jsonl");
    let server = Mockserve::start(&["--spec", &shown(&api)]).unwrap();
    assert_eq!(request(server.port, "GET", "/users/me", &[], "").0, 200);
    drop(server);
    assert!(!log.exists());
}

#[test]
fn a_log_that_cannot_be_opened_stops_the_server_before_it_listens() {
    let (workspace, api) = shop();
    let nowhere = workspace.path().join("no-such-dir").join("exchanges.jsonl");
    let said = match Mockserve::start(&["--spec", &shown(&api), "--exchange-log", &shown(&nowhere)])
    {
        Ok(server) => {
            drop(server);
            panic!("the server started with a log it cannot write")
        }
        Err(said) => said,
    };
    assert!(said.contains(&format!("--exchange-log {}: ", nowhere.display())), "{said}");
    assert!(!said.contains("HTTP server listening"), "{said}");
}

#[test]
fn range_list_and_write_speak_as_the_guide_says() {
    let (code, list, stderr) = run(&["range", "list"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(list.starts_with("shop\n  title     A shop API"), "{list}");
    assert!(
        list.ends_with("  mockforge serve --spec shop.json --http-port 8080\n"),
        "{list}"
    );

    let (code, listed) = document(&["range", "list", "--format", "json"]);
    assert_eq!(code, 0);
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], "shop");
    assert_eq!(listed[0]["operations"], 9);
    assert_eq!(listed[0]["flaws"].as_array().unwrap().len(), 4);
    assert_eq!(listed[0]["credentials"]["Authorization"], "Bearer demo-user-a");

    // A pack that is not shipped is named, and nothing is written.
    let nowhere = std::env::temp_dir().join(format!("mockforge-range-{}", std::process::id()));
    let (code, stdout, stderr) = run(&["range", "write", "nope", "--output", &shown(&nowhere)]);
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert!(stderr.contains("no shipped pack has the id `nope`"), "{stderr}");
    assert!(!nowhere.exists());
}
