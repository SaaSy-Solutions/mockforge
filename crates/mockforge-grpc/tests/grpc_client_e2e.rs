//! End-to-end regression: a real `tonic` client calls `SayHello` on the
//! bundled Greeter service and gets a well-formed `HelloReply` back.
//! The existing `grpc_server_e2e_test.rs` tests service *discovery*
//! (registry, schemas, mock generation) but never starts the server and
//! never calls it through the wire. This closes that gap so we catch
//! regressions in the transport + dispatch path.

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grpc_real_client_roundtrip_say_hello() {
    let port = free_port().await;

    // Disable the HTTP bridge and reflection — we only need the gRPC socket
    // for this test, and proto_dir must be the bundled one (the default
    // `"proto"` relative path only works when CWD happens to be the crate
    // root, which isn't guaranteed for `cargo test --test`).
    let config = DynamicGrpcConfig {
        proto_dir: proto_dir(),
        enable_reflection: false,
        excluded_services: vec![],
        http_bridge: None,
        tls: None,
    };

    let server = tokio::spawn(async move {
        mockforge_grpc::start_with_config(port, None, config).await.unwrap();
    });

    wait_for_port(port, Duration::from_secs(5)).await;

    // Connect a real tonic client and call SayHello.
    let endpoint = format!("http://127.0.0.1:{port}");
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
