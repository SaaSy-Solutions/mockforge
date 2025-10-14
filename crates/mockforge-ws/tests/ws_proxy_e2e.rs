use futures_util::{SinkExt, StreamExt};
use mockforge_core::ws_proxy::{WsProxyConfig, WsProxyHandler, WsProxyRule};
use tokio_tungstenite::tungstenite::protocol::Message;

#[tokio::test]
async fn ws_proxy_routes_to_upstream() {
    // Create a mock upstream WebSocket server
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream_listener.local_addr().unwrap();

    // Start mock upstream server that echoes messages
    let upstream_server = tokio::spawn(async move {
        while let Ok((stream, _)) = upstream_listener.accept().await {
            let mut ws_stream = match tokio_tungstenite::accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("Failed to accept WebSocket connection: {}", e);
                    continue;
                }
            };

            // Handle the connection in a separate task
            tokio::spawn(async move {
                while let Some(msg) = ws_stream.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            let response = format!("UPSTREAM_ECHO: {}", text);
                            if let Err(e) = ws_stream.send(Message::Text(response.into())).await {
                                eprintln!("Failed to send response: {}", e);
                                break;
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Ok(Message::Ping(data)) => {
                            let _ = ws_stream.send(Message::Pong(data)).await;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            });
        }
    });

    // Configure proxy to route /ws/proxy to upstream
    let mut config = WsProxyConfig::new(format!("ws://{}", upstream_addr));
    config.enabled = true;
    config.passthrough_by_default = false; // Only proxy specific paths
    config.rules.push(WsProxyRule {
        pattern: "/ws/proxy".to_string(),
        upstream_url: format!("ws://{}", upstream_addr),
        enabled: true,
    });

    let proxy_handler = WsProxyHandler::new(config);

    // Start proxy server
    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    let proxy_server = tokio::spawn(async move {
        let app = mockforge_ws::router_with_proxy(proxy_handler);
        axum::serve(proxy_listener, app).await.unwrap()
    });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test proxied connection
    let url = format!("ws://{}/ws/proxy", proxy_addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    // Send a message and expect it to be echoed by upstream
    ws_stream.send(Message::Text("test_message".into())).await.unwrap();

    if let Some(Ok(Message::Text(response))) = ws_stream.next().await {
        println!("Received response: {}", response);
        assert!(response.contains("UPSTREAM_ECHO: test_message"));
    } else {
        panic!("Expected text message from upstream");
    }

    // Test local connection (should not be proxied)
    let local_url = format!("ws://{}/ws", proxy_addr);
    let (mut local_ws_stream, _) = tokio_tungstenite::connect_async(local_url).await.unwrap();

    // Send a message and expect local echo
    local_ws_stream.send(Message::Text("local_test".into())).await.unwrap();

    if let Some(Ok(Message::Text(response))) = local_ws_stream.next().await {
        assert!(response.contains("echo: local_test"));
    } else {
        panic!("Expected local echo response");
    }

    // Clean up
    drop(proxy_server);
    drop(upstream_server);
}

#[tokio::test]
async fn ws_proxy_passthrough_by_default() {
    // Create a mock upstream WebSocket server
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream_listener.local_addr().unwrap();

    // Start mock upstream server
    let upstream_server = tokio::spawn(async move {
        let (stream, _) = upstream_listener.accept().await.unwrap();
        let mut ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let response = format!("UPSTREAM: {}", text);
                    let _ = ws_stream.send(Message::Text(response.into())).await;
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    });

    // Configure proxy with passthrough by default
    let mut config = WsProxyConfig::new(format!("ws://{}", upstream_addr));
    config.enabled = true;
    config.passthrough_by_default = true; // Proxy all connections by default

    let proxy_handler = WsProxyHandler::new(config);

    // Start proxy server
    let proxy_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();
    let proxy_server = tokio::spawn(async move {
        let app = mockforge_ws::router_with_proxy(proxy_handler);
        axum::serve(proxy_listener, app).await.unwrap()
    });

    // Test that all connections are proxied
    let url = format!("ws://{}/ws", proxy_addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();

    ws_stream.send(Message::Text("passthrough_test".into())).await.unwrap();

    if let Some(Ok(Message::Text(response))) = ws_stream.next().await {
        assert!(response.contains("UPSTREAM: passthrough_test"));
    } else {
        panic!("Expected proxied response");
    }

    // Clean up
    drop(proxy_server);
    drop(upstream_server);
}
