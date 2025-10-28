//! Port discovery integration tests

use mockforge_sdk::MockServer;

#[tokio::test]
async fn test_auto_port_discovery() {
    let mut server = MockServer::new()
        .auto_port()
        .start()
        .await
        .expect("Failed to start server");

    // Port should be automatically assigned
    let port = server.port();
    assert!(port > 0);
    assert!(port >= 30000); // Default range starts at 30000

    // Server should be running
    assert!(server.is_running());

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_custom_port_range() {
    let mut server = MockServer::new()
        .auto_port()
        .port_range(40000, 40100)
        .start()
        .await
        .expect("Failed to start server");

    let port = server.port();
    assert!(port >= 40000);
    assert!(port <= 40100);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_multiple_servers_auto_port() {
    // Start multiple servers with auto-port
    let mut server1 = MockServer::new()
        .auto_port()
        .start()
        .await
        .expect("Failed to start server 1");

    let mut server2 = MockServer::new()
        .auto_port()
        .start()
        .await
        .expect("Failed to start server 2");

    let mut server3 = MockServer::new()
        .auto_port()
        .start()
        .await
        .expect("Failed to start server 3");

    // All servers should have different ports
    let port1 = server1.port();
    let port2 = server2.port();
    let port3 = server3.port();

    assert_ne!(port1, port2);
    assert_ne!(port2, port3);
    assert_ne!(port1, port3);

    // All servers should be running
    assert!(server1.is_running());
    assert!(server2.is_running());
    assert!(server3.is_running());

    // Clean up
    server1.stop().await.expect("Failed to stop server 1");
    server2.stop().await.expect("Failed to stop server 2");
    server3.stop().await.expect("Failed to stop server 3");
}

#[tokio::test]
async fn test_explicit_port_overrides_auto() {
    let mut server = MockServer::new()
        .auto_port()
        .port(35000) // This should override auto_port
        .start()
        .await
        .expect("Failed to start server");

    assert_eq!(server.port(), 35000);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_port_zero_uses_random() {
    let mut server = MockServer::new()
        .port(0) // Port 0 means "assign any available port"
        .start()
        .await
        .expect("Failed to start server");

    let port = server.port();
    assert!(port > 0);

    server.stop().await.expect("Failed to stop server");
}
