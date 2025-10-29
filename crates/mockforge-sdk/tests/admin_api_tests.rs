//! Admin API client integration tests

use mockforge_sdk::{AdminClient, MockConfigBuilder, MockServer};
use serde_json::json;

#[tokio::test]
async fn test_admin_client_list_mocks() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    // Initially should have no mocks
    let mocks = admin_client.list_mocks().await.expect("Failed to list mocks");
    assert_eq!(mocks.total, 0);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_admin_client_create_mock() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    // Create a mock
    let mock_config = MockConfigBuilder::new("GET", "/api/test")
        .name("Test Mock")
        .status(200)
        .body(json!({"message": "Hello, World!"}))
        .build();

    let created = admin_client.create_mock(mock_config).await.expect("Failed to create mock");

    assert_eq!(created.method, "GET");
    assert_eq!(created.path, "/api/test");
    assert_eq!(created.name, "Test Mock");
    assert!(!created.id.is_empty());

    // Verify the mock was created
    let mocks = admin_client.list_mocks().await.expect("Failed to list mocks");
    assert_eq!(mocks.total, 1);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_admin_client_get_mock() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    // Create a mock
    let mock_config = MockConfigBuilder::new("GET", "/api/users")
        .name("Get Users")
        .body(json!([]))
        .build();

    let created = admin_client.create_mock(mock_config).await.expect("Failed to create mock");

    // Get the mock by ID
    let fetched = admin_client.get_mock(&created.id).await.expect("Failed to get mock");

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.method, "GET");
    assert_eq!(fetched.path, "/api/users");

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_admin_client_update_mock() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    // Create a mock
    let mock_config = MockConfigBuilder::new("GET", "/api/data")
        .name("Original")
        .status(200)
        .body(json!({"value": 1}))
        .build();

    let created = admin_client.create_mock(mock_config).await.expect("Failed to create mock");

    // Update the mock
    let updated_config = MockConfigBuilder::new("GET", "/api/data")
        .id(&created.id)
        .name("Updated")
        .status(200)
        .body(json!({"value": 2}))
        .build();

    let updated = admin_client
        .update_mock(&created.id, updated_config)
        .await
        .expect("Failed to update mock");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.response.body["value"], 2);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_admin_client_delete_mock() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    // Create a mock
    let mock_config =
        MockConfigBuilder::new("DELETE", "/api/delete-test").name("To Delete").build();

    let created = admin_client.create_mock(mock_config).await.expect("Failed to create mock");

    // Delete the mock
    admin_client.delete_mock(&created.id).await.expect("Failed to delete mock");

    // Verify it's deleted
    let result = admin_client.get_mock(&created.id).await;
    assert!(result.is_err());

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_admin_client_get_stats() {
    let mut server = MockServer::new().auto_port().start().await.expect("Failed to start server");

    let admin_client = AdminClient::new(server.url());

    let stats = admin_client.get_stats().await.expect("Failed to get stats");

    assert!(stats.uptime_seconds >= 0);
    assert_eq!(stats.active_mocks, 0);

    server.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_mock_config_builder() {
    let mock = MockConfigBuilder::new("POST", "/api/create")
        .name("Create Resource")
        .status(201)
        .body(json!({"id": 123, "created": true}))
        .latency_ms(50)
        .header("X-Custom-Header", "custom-value")
        .enabled(true)
        .build();

    assert_eq!(mock.method, "POST");
    assert_eq!(mock.path, "/api/create");
    assert_eq!(mock.name, "Create Resource");
    assert_eq!(mock.status_code, Some(201));
    assert_eq!(mock.latency_ms, Some(50));
    assert!(mock.enabled);
    assert!(mock.response.headers.is_some());
    assert_eq!(
        mock.response.headers.as_ref().unwrap().get("X-Custom-Header"),
        Some(&"custom-value".to_string())
    );
}
