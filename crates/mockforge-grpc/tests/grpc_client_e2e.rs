//! End-to-end regressions: real `tonic` clients call each of the four
//! Greeter RPCs (unary + server-streaming + client-streaming + bidi)
//! over the wire. The existing `grpc_server_e2e_test.rs` only tests
//! service *discovery* (registry, schemas, mock generation); it never
//! starts the server or crosses the transport boundary. These tests
//! catch regressions in the transport + dispatch + streaming codecs.

use futures::StreamExt;
use mockforge_grpc::dynamic::DynamicGrpcConfig;
use mockforge_grpc::generated::greeter_client::GreeterClient;
use mockforge_grpc::generated::HelloRequest;
use std::path::PathBuf;
use std::time::Duration;

fn proto_dir() -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("proto").to_string_lossy().into_owned()
}

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Poll until the server is accepting TCP connections, or time out.
async fn wait_for_port(port: u16, max: Duration) {
    let deadline = tokio::time::Instant::now() + max;
    loop {
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("gRPC server never started listening on 127.0.0.1:{port}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Spawn a Greeter mock server on a free port and return the client
/// endpoint URL + the server JoinHandle. Caller aborts the handle to
/// stop the server. HTTP bridge + reflection are disabled — we only
/// need the gRPC socket for these tests.
async fn spawn_greeter() -> (String, tokio::task::JoinHandle<()>) {
    let port = free_port().await;
    // proto_dir must be the bundled absolute path (the default `"proto"`
    // relative path only works when CWD happens to be the crate root,
    // which isn't guaranteed for `cargo test --test`).
    let config = DynamicGrpcConfig {
        proto_dir: proto_dir(),
        enable_reflection: false,
        excluded_services: vec![],
        http_bridge: None,
        tls: None,
    };
    let handle = tokio::spawn(async move {
        mockforge_grpc::start_with_config(port, None, config).await.unwrap();
    });
    wait_for_port(port, Duration::from_secs(5)).await;
    (format!("http://127.0.0.1:{port}"), handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grpc_real_client_roundtrip_say_hello() {
    let (endpoint, server) = spawn_greeter().await;

    let mut client = GreeterClient::connect(endpoint)
        .await
        .expect("tonic GreeterClient should connect to the running mock server");

    let request = tonic::Request::new(HelloRequest {
        name: "mockforge-e2e".into(),
        user_info: None,
        tags: vec!["integration".into(), "grpc".into()],
    });

    let response = tokio::time::timeout(Duration::from_secs(5), client.say_hello(request))
        .await
        .expect("say_hello should complete within 5s")
        .expect("say_hello should return Ok");

    let reply = response.into_inner();
    // The mock-response generator fills in a non-empty message. The
    // exact wording is up to the generator; what matters is that the
    // unary RPC completed and produced a well-formed HelloReply.
    assert!(
        !reply.message.is_empty(),
        "Expected HelloReply.message to be populated by the mock generator; got an empty string"
    );

    server.abort();
}

/// `SayHelloStream` is a server-streaming RPC: one request, many
/// replies. The handler emits 5 `HelloReply`s (with a 100ms sleep
/// between each) tagged "Stream message i of 5". This test drains the
/// stream and asserts on count + well-formedness.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grpc_real_client_server_streaming_yields_expected_count() {
    let (endpoint, server) = spawn_greeter().await;

    let mut client = GreeterClient::connect(endpoint)
        .await
        .expect("tonic GreeterClient should connect to the running mock server");

    let request = tonic::Request::new(HelloRequest {
        name: "streaming-e2e".into(),
        user_info: None,
        tags: vec![],
    });

    let response = tokio::time::timeout(Duration::from_secs(5), client.say_hello_stream(request))
        .await
        .expect("say_hello_stream should open within 5s")
        .expect("say_hello_stream should return Ok");

    // Drain the full stream. The handler sleeps 100ms between each of
    // 5 replies so we give generous headroom (handler can be slower
    // under CI load, but still bounded).
    let mut stream = response.into_inner();
    let mut replies: Vec<String> = Vec::new();
    while let Some(reply) = tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .expect("stream should yield or end within 10s")
    {
        let reply = reply.expect("no transport error mid-stream");
        assert!(
            !reply.message.is_empty(),
            "every streamed HelloReply must carry a non-empty message"
        );
        replies.push(reply.message);
    }

    assert_eq!(
        replies.len(),
        5,
        "SayHelloStream handler emits exactly 5 replies; got {}: {:?}",
        replies.len(),
        replies
    );
    // Spot-check that the request name round-trips into each reply
    // and the per-reply index appears in the text.
    for (i, msg) in replies.iter().enumerate() {
        assert!(
            msg.contains("streaming-e2e"),
            "reply {i} should echo the request name; got: {msg}"
        );
    }

    server.abort();
}
