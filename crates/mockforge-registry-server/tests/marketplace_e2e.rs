//! End-to-end tests for marketplace workflows
//!
//! Tests complete workflows for:
//! - Plugin marketplace (publish, browse, install, review)
//! - Template marketplace (publish, browse, install, review)
//! - Scenario marketplace (publish, browse, install, review)

// Note: This test requires the registry server to be running
// Run with: REGISTRY_URL=http://localhost:8080 cargo test --test marketplace_e2e -- --ignored
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

/// Test helper for marketplace E2E tests
struct MarketplaceTestHelper {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
    user_id: Option<Uuid>,
    org_id: Option<Uuid>,
}

impl MarketplaceTestHelper {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            auth_token: None,
            user_id: None,
            org_id: None,
        }
    }

    /// Register a new test user
    async fn register_user(&mut self, username: &str, email: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&format!("{}/api/v1/auth/register", self.base_url))
            .json(&json!({
                "username": username,
                "email": email,
                "password": "testpassword123"
            }))
            .send()
            .await?;

        assert!(response.status().is_success(), "User registration failed");
        let body: serde_json::Value = response.json().await?;
        self.auth_token = body["token"].as_str().map(|s| s.to_string());
        self.user_id = body["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        Ok(())
    }

    /// Login with existing user
    async fn login(&mut self, email: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&format!("{}/api/v1/auth/login", self.base_url))
            .json(&json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        assert!(response.status().is_success(), "Login failed");
        let body: serde_json::Value = response.json().await?;
        self.auth_token = body["token"].as_str().map(|s| s.to_string());
        self.user_id = body["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        Ok(())
    }

    /// Create an organization
    async fn create_org(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.auth_token.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .post(&format!("{}/api/v1/orgs", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&json!({
                "name": name,
                "slug": name.to_lowercase().replace(" ", "-")
            }))
            .send()
            .await?;

        assert!(response.status().is_success(), "Organization creation failed");
        let body: serde_json::Value = response.json().await?;
        self.org_id = body["id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        Ok(())
    }

    /// Get authorization header
    fn auth_header(&self) -> Option<String> {
        self.auth_token.as_ref().map(|t| format!("Bearer {}", t))
    }

    /// Create a minimal valid WASM file for testing
    fn create_test_wasm() -> Vec<u8> {
        // Minimal valid WASM file (magic bytes + version)
        vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00]
    }

    /// Create a minimal valid tar.gz package for testing
    fn create_test_package() -> Vec<u8> {
        // Minimal valid gzip file (magic bytes + minimal header)
        vec![
            0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    }

    /// Calculate SHA-256 checksum
    fn calculate_checksum(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

/// Test plugin marketplace workflow
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_plugin_marketplace_workflow() {
    let base_url = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let timestamp = chrono::Utc::now().timestamp();
    helper
        .register_user(
            &format!("testuser_{}", timestamp),
            &format!("test_{}@example.com", timestamp),
        )
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", timestamp))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a plugin
    let wasm_data = MarketplaceTestHelper::create_test_wasm();
    let checksum = MarketplaceTestHelper::calculate_checksum(&wasm_data);
    let wasm_base64 = base64::encode(&wasm_data);

    let publish_response = helper
        .client
        .post(&format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", timestamp),
            "version": "1.0.0",
            "description": "Test plugin for E2E testing",
            "category": "testing",
            "license": "MIT",
            "tags": vec!["test", "e2e"],
            "checksum": checksum,
            "file_size": wasm_data.len() as i64,
            "wasm_data": wasm_base64,
            "dependencies": {}
        }))
        .send()
        .await
        .expect("Failed to publish plugin");

    assert!(
        publish_response.status().is_success(),
        "Plugin publish failed: {:?}",
        publish_response.text().await
    );

    let publish_body: serde_json::Value = publish_response.json().await.expect("Failed to parse response");
    assert_eq!(publish_body["success"], true);
    let plugin_name = format!("test-plugin-{}", timestamp);

    // Step 4: Search for the plugin
    let search_response = helper
        .client
        .post(&format!("{}/api/v1/plugins/search", helper.base_url))
        .json(&json!({
            "query": plugin_name.clone(),
            "page": 0,
            "per_page": 10
        }))
        .send()
        .await
        .expect("Failed to search plugins");

    assert!(search_response.status().is_success());
    let search_body: serde_json::Value = search_response.json().await.expect("Failed to parse response");
    assert!(search_body["plugins"].as_array().unwrap().len() > 0);

    // Step 5: Get plugin details
    let get_response = helper
        .client
        .get(&format!("{}/api/v1/plugins/{}", helper.base_url, plugin_name))
        .send()
        .await
        .expect("Failed to get plugin");

    assert!(get_response.status().is_success());
    let plugin_body: serde_json::Value = get_response.json().await.expect("Failed to parse response");
    assert_eq!(plugin_body["name"], plugin_name);

    // Step 6: Submit a review
    let review_response = helper
        .client
        .post(&format!("{}/api/v1/plugins/{}/reviews", helper.base_url, plugin_name))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "rating": 5,
            "title": "Great plugin!",
            "comment": "This plugin works perfectly for testing purposes."
        }))
        .send()
        .await
        .expect("Failed to submit review");

    assert!(review_response.status().is_success());

    // Step 7: Get reviews
    let reviews_response = helper
        .client
        .get(&format!("{}/api/v1/plugins/{}/reviews", helper.base_url, plugin_name))
        .send()
        .await
        .expect("Failed to get reviews");

    assert!(reviews_response.status().is_success());
    let reviews_body: serde_json::Value = reviews_response.json().await.expect("Failed to parse response");
    assert!(reviews_body["reviews"].as_array().unwrap().len() > 0);
}

/// Test template marketplace workflow
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_template_marketplace_workflow() {
    let base_url = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let timestamp = chrono::Utc::now().timestamp();
    helper
        .register_user(
            &format!("testuser_{}", timestamp),
            &format!("test_{}@example.com", timestamp),
        )
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", timestamp))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a template
    let package_data = MarketplaceTestHelper::create_test_package();
    let checksum = MarketplaceTestHelper::calculate_checksum(&package_data);
    let package_base64 = base64::encode(&package_data);

    let template_name = format!("test-template-{}", timestamp);
    let publish_response = helper
        .client
        .post(&format!("{}/api/v1/templates/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .header("X-Org-Id", helper.org_id.unwrap().to_string())
        .json(&json!({
            "name": template_name.clone(),
            "slug": template_name.clone(),
            "description": "Test template for E2E testing",
            "version": "1.0.0",
            "category": "chaos",
            "tags": vec!["test", "e2e"],
            "content": json!({
                "name": template_name,
                "version": "1.0.0",
                "description": "Test template"
            }),
            "checksum": checksum,
            "file_size": package_data.len() as i64,
            "package": package_base64
        }))
        .send()
        .await
        .expect("Failed to publish template");

    assert!(
        publish_response.status().is_success(),
        "Template publish failed: {:?}",
        publish_response.text().await
    );

    // Step 4: Search for the template
    let search_response = helper
        .client
        .post(&format!("{}/api/v1/templates/search", helper.base_url))
        .json(&json!({
            "query": template_name.clone(),
            "page": 0,
            "per_page": 10
        }))
        .send()
        .await
        .expect("Failed to search templates");

    assert!(search_response.status().is_success());
    let search_body: serde_json::Value = search_response.json().await.expect("Failed to parse response");
    assert!(search_body["templates"].as_array().unwrap().len() > 0);

    // Step 5: Get template details
    let get_response = helper
        .client
        .get(&format!("{}/api/v1/templates/{}", helper.base_url, template_name))
        .send()
        .await
        .expect("Failed to get template");

    assert!(get_response.status().is_success());
}

/// Test scenario marketplace workflow
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_scenario_marketplace_workflow() {
    let base_url = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let timestamp = chrono::Utc::now().timestamp();
    helper
        .register_user(
            &format!("testuser_{}", timestamp),
            &format!("test_{}@example.com", timestamp),
        )
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", timestamp))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a scenario
    let package_data = MarketplaceTestHelper::create_test_package();
    let checksum = MarketplaceTestHelper::calculate_checksum(&package_data);
    let package_base64 = base64::encode(&package_data);

    let scenario_name = format!("test-scenario-{}", timestamp);
    let manifest = json!({
        "name": scenario_name.clone(),
        "version": "1.0.0",
        "description": "Test scenario for E2E testing"
    });

    let publish_response = helper
        .client
        .post(&format!("{}/api/v1/scenarios/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .header("X-Org-Id", helper.org_id.unwrap().to_string())
        .json(&json!({
            "manifest": manifest.to_string(),
            "checksum": checksum,
            "size": package_data.len() as u64,
            "package": package_base64
        }))
        .send()
        .await
        .expect("Failed to publish scenario");

    assert!(
        publish_response.status().is_success(),
        "Scenario publish failed: {:?}",
        publish_response.text().await
    );

    // Step 4: Search for the scenario
    let search_response = helper
        .client
        .post(&format!("{}/api/v1/scenarios/search", helper.base_url))
        .json(&json!({
            "query": scenario_name.clone(),
            "page": 0,
            "per_page": 10
        }))
        .send()
        .await
        .expect("Failed to search scenarios");

    assert!(search_response.status().is_success());
    let search_body: serde_json::Value = search_response.json().await.expect("Failed to parse response");
    assert!(search_body["scenarios"].as_array().unwrap().len() > 0);

    // Step 5: Get scenario details
    let get_response = helper
        .client
        .get(&format!("{}/api/v1/scenarios/{}", helper.base_url, scenario_name))
        .send()
        .await
        .expect("Failed to get scenario");

    assert!(get_response.status().is_success());

    // Step 6: Submit a review
    let review_response = helper
        .client
        .post(&format!("{}/api/v1/scenarios/{}/reviews", helper.base_url, scenario_name))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "rating": 5,
            "title": "Great scenario!",
            "comment": "This scenario works perfectly for testing purposes."
        }))
        .send()
        .await
        .expect("Failed to submit review");

    assert!(review_response.status().is_success());
}

/// Test validation errors
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_upload_validation_errors() {
    let base_url = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Register and authenticate
    let timestamp = chrono::Utc::now().timestamp();
    helper
        .register_user(
            &format!("testuser_{}", timestamp),
            &format!("test_{}@example.com", timestamp),
        )
        .await
        .expect("Failed to register user");

    // Test 1: Invalid WASM file (wrong magic bytes)
    let invalid_wasm = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    let checksum = MarketplaceTestHelper::calculate_checksum(&invalid_wasm);
    let wasm_base64 = base64::encode(&invalid_wasm);

    let response = helper
        .client
        .post(&format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", timestamp),
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": vec![],
            "checksum": checksum,
            "file_size": invalid_wasm.len() as i64,
            "wasm_data": wasm_base64,
            "dependencies": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should fail validation
    assert!(!response.status().is_success(), "Should reject invalid WASM file");

    // Test 2: Path traversal in name
    let wasm_data = MarketplaceTestHelper::create_test_wasm();
    let checksum = MarketplaceTestHelper::calculate_checksum(&wasm_data);
    let wasm_base64 = base64::encode(&wasm_data);

    let response = helper
        .client
        .post(&format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": "../../etc/passwd",
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": vec![],
            "checksum": checksum,
            "file_size": wasm_data.len() as i64,
            "wasm_data": wasm_base64,
            "dependencies": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should fail validation
    assert!(!response.status().is_success(), "Should reject path traversal in name");

    // Test 3: File too large
    let large_wasm = vec![0x00, 0x61, 0x73, 0x6D; 11 * 1024 * 1024]; // 11 MB
    let checksum = MarketplaceTestHelper::calculate_checksum(&large_wasm);
    let wasm_base64 = base64::encode(&large_wasm);

    let response = helper
        .client
        .post(&format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", timestamp),
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": vec![],
            "checksum": checksum,
            "file_size": large_wasm.len() as i64,
            "wasm_data": wasm_base64,
            "dependencies": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should fail validation
    assert!(!response.status().is_success(), "Should reject file that's too large");
}
