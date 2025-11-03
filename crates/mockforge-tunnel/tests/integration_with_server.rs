//! Integration tests with a test tunnel server

#[cfg(feature = "server")]
use mockforge_tunnel::server::start_test_server;
use mockforge_tunnel::{TunnelConfig, TunnelManager, TunnelProvider};

#[cfg(feature = "server")]
#[tokio::test]
async fn test_full_tunnel_workflow() {
    // Start test server on random port
    let server_addr = start_test_server(0).await.unwrap();
    let server_url = format!("http://{}", server_addr);

    // Create tunnel config
    let config = TunnelConfig::new("http://localhost:3000")
        .with_provider(TunnelProvider::SelfHosted)
        .with_subdomain("test-integration");

    // Set server URL
    let mut config = config;
    config.server_url = Some(server_url.clone());
    config.auth_token = None;

    // Create tunnel manager
    let manager = TunnelManager::new(&config).unwrap();

    // Create tunnel
    let status = manager.create_tunnel(&config).await.unwrap();

    assert!(status.active);
    assert!(status.public_url.contains("test-integration"));
    assert!(!status.tunnel_id.is_empty());

    // Get tunnel status
    let refreshed_status = manager.refresh_status().await.unwrap();
    assert_eq!(refreshed_status.tunnel_id, status.tunnel_id);

    // List tunnels
    let tunnels = manager.list_tunnels().await.unwrap();
    assert!(tunnels.len() > 0);
    assert!(tunnels.iter().any(|t| t.tunnel_id == status.tunnel_id));

    // Stop tunnel
    manager.stop_tunnel().await.unwrap();

    // Verify tunnel is gone
    let tunnels_after = manager.list_tunnels().await.unwrap();
    assert!(!tunnels_after.iter().any(|t| t.tunnel_id == status.tunnel_id));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_tunnel_with_auto_subdomain() {
    let server_addr = start_test_server(0).await.unwrap();
    let server_url = format!("http://{}", server_addr);

    let mut config =
        TunnelConfig::new("http://localhost:3000").with_provider(TunnelProvider::SelfHosted);
    config.server_url = Some(server_url);

    let manager = TunnelManager::new(&config).unwrap();
    let status = manager.create_tunnel(&config).await.unwrap();

    // Should auto-generate subdomain
    assert!(status.public_url.contains("tunnel-"));
    assert!(status.active);
}
