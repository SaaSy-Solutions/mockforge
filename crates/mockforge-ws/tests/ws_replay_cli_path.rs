//! Regression test for the CLI replay-file plumbing.
//!
//! Before this PR, passing `--ws-replay-file` to `mockforge serve` wrote to
//! `config.websocket.replay_file` but never reached the WS handler (which
//! only consulted `MOCKFORGE_WS_REPLAY_FILE` directly). The fix in
//! `mockforge-cli` now pushes the resolved config value back into the env
//! var before spawning the WS server, so all three input paths (CLI flag,
//! YAML config, direct env) converge.
//!
//! This test pins the contract: given the env var set to a custom replay
//! file (the same way the fix sets it from config), a real
//! `tokio-tungstenite` client sees the scripted replay instead of the
//! fallback echo mode.

use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_replay_file_path_reaches_the_handler() {
    // Custom replay file in a temp location that the default demo path
    // couldn't find — proves the env var is actually read (not the default
    // examples/ws-demo.jsonl).
    let replay = NamedTempFile::new().unwrap();
    let replay_path = replay.path().to_path_buf();
    std::fs::write(
        &replay_path,
        // {{uuid}} so we can assert template expansion happened AND that
        // this specific file's contents shipped (no accidental fallback).
        r#"{"ts":0,"dir":"out","text":"CLI-REPLAY-MARKER {{uuid}}","waitFor":"^CLIENT_READY$"}
"#,
    )
    .unwrap();

    // Simulate what `mockforge-cli::serve` does after resolving
    // `config.websocket.replay_file`:
    std::env::set_var("MOCKFORGE_WS_REPLAY_FILE", &replay_path);
    std::env::set_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND", "true");

    // Spin up the WS server on an ephemeral port.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let _server =
        tokio::spawn(async move { axum::serve(listener, mockforge_ws::router()).await.unwrap() });

    // Give axum a tick to start accepting connections.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect, send the replay's `waitFor` trigger, and read the scripted
    // reply with timeout.
    let url = format!("ws://{}/ws", addr);
    let (mut ws, _) =
        tokio::time::timeout(Duration::from_secs(3), tokio_tungstenite::connect_async(url))
            .await
            .expect("ws handshake should not time out")
            .expect("ws handshake ok");

    ws.send(Message::Text("CLIENT_READY".into())).await.unwrap();

    let got = tokio::time::timeout(Duration::from_secs(3), ws.next())
        .await
        .expect("replay frame should arrive")
        .expect("stream produced a message")
        .expect("no transport error");
    let text = match got {
        Message::Text(t) => t.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    };

    // Content must come from OUR file (the marker string), not the bundled
    // demo replay and not the fallback echo.
    assert!(
        text.contains("CLI-REPLAY-MARKER"),
        "expected replay content from the temp file, got: {text}"
    );
    // Template expansion must have happened — the literal `{{uuid}}` token
    // should NOT survive the round-trip.
    assert!(!text.contains("{{uuid}}"), "template token should have been expanded: {text}");

    // Clean up so we don't leak env into sibling tests.
    std::env::remove_var("MOCKFORGE_WS_REPLAY_FILE");
    std::env::remove_var("MOCKFORGE_RESPONSE_TEMPLATE_EXPAND");
}
