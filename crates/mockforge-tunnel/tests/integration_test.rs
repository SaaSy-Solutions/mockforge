//! Integration tests for tunnel functionality

use mockforge_tunnel::{TunnelConfig, TunnelManager, TunnelProvider};

#[tokio::test]
async fn test_tunnel_config_creation() {
    let config = TunnelConfig::new("http://localhost:3000")
        .with_provider(TunnelProvider::SelfHosted)
        .with_subdomain("test-api");

    assert_eq!(config.local_url, "http://localhost:3000");
    assert_eq!(config.provider, TunnelProvider::SelfHosted);
    assert_eq!(config.subdomain, Some("test-api".to_string()));
    assert!(config.websocket_enabled);
}

#[tokio::test]
async fn test_tunnel_manager_requires_server_url() {
    let config =
        TunnelConfig::new("http://localhost:3000").with_provider(TunnelProvider::SelfHosted);

    // Should fail without server_url
    let result = TunnelManager::new(&config);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("server_url required"));
    } else {
        panic!("Expected error but got Ok");
    }
}

#[tokio::test]
async fn test_tunnel_provider_parsing() {
    // Test that provider enum works correctly
    let config = TunnelConfig::new("http://localhost:3000").with_provider(TunnelProvider::Cloud);

    assert_eq!(config.provider, TunnelProvider::Cloud);
}
