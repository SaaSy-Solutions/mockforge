//! Container-style signals must stop the actual CLI without a SIGKILL fallback.
#![cfg(unix)]
#![allow(deprecated)]

use assert_cmd::cargo::CommandCargoExt;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Reap only this test's child, including on assertion failure.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn assert_serve_stops_for(signal: &str, admin_login: bool, grace: Duration) {
    let directory = tempfile::tempdir().expect("isolated server workspace");
    // Child startup must not discover a host .env or fall back to the root
    // drive for its own temporary files after clearing the environment.
    std::fs::write(directory.path().join(".env"), "").expect("empty fixture environment");
    let mut command = Command::cargo_bin("mockforge").expect("compiled CLI binary");
    command
        .env_clear()
        .env("TMPDIR", directory.path())
        .current_dir(directory.path())
        .args([
            "serve",
            "--no-config",
            "--http-port",
            "0",
            "--ws-port",
            "0",
            "--grpc-port",
            "0",
            "--admin",
            "--admin-port",
            "0",
            "--metrics-port",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if admin_login {
        // A configured admin login is hashed on a startup thread; shutdown
        // must not wait for it. There are no built-in users to seed (#1016).
        command.env("MOCKFORGE_ADMIN_PASSWORD", "shutdown-fixture-password");
    }
    let mut child = OwnedChild(command.spawn().expect("start owned CLI process"));
    let stdout = child.0.stdout.take().expect("stdout pipe");
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let mut ready_tx = Some(ready_tx);
        // Retain only the public listener port; discard other startup output.
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let port = line
                .split("HTTP server listening on http://localhost:")
                .nth(1)
                .and_then(|value| value.trim().parse::<u16>().ok());
            if let Some(port) = port {
                if let Some(sender) = ready_tx.take() {
                    let _ = sender.send(port);
                }
            }
        }
    });
    let port = tokio::time::timeout(Duration::from_secs(45), ready_rx)
        .await
        .expect("CLI should bind an HTTP listener")
        .expect("CLI should report its actual port");
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("local test client")
            .get(format!("http://127.0.0.1:{port}/health/live"))
            .header("Connection", "close")
            .send(),
    )
    .await
    .expect("owned HTTP request should finish")
    .expect("owned HTTP server should accept a request");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    response
        .bytes()
        .await
        .expect("finish the health probe before testing idle shutdown");

    let started = Instant::now();
    assert!(Command::new("kill")
        .args([signal, &child.0.id().to_string()])
        .status()
        .expect("send signal only to owned process")
        .success());
    loop {
        if let Some(status) = child.0.try_wait().expect("poll owned child") {
            assert!(status.success(), "signal shutdown should exit normally: {status}");
            break;
        }
        assert!(started.elapsed() < grace, "CLI required a force-kill fallback");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn serve_exits_normally_for_sigterm_after_http_becomes_available() {
    assert_serve_stops_for("-TERM", true, Duration::from_secs(2)).await;
}

#[tokio::test]
async fn serve_exits_normally_for_sigint_after_http_becomes_available() {
    assert_serve_stops_for("-INT", true, Duration::from_secs(2)).await;
}

#[tokio::test]
async fn production_style_admin_startup_stops_within_short_grace_period() {
    assert_serve_stops_for("-TERM", false, Duration::from_secs(2)).await;
}
