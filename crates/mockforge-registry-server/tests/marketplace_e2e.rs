//! End-to-end tests for marketplace workflows
//!
//! Tests complete workflows for:
//! - Plugin marketplace (publish, browse, install, review)
//! - Template marketplace (publish, browse, install, review)
//! - Scenario marketplace (publish, browse, install, review)

// Note: This test requires the registry server to be running
// Run with: REGISTRY_URL=http://localhost:8080 cargo test --test marketplace_e2e -- --ignored
use base64::Engine;
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
    async fn register_user(
        &mut self,
        username: &str,
        email: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(format!("{}/api/v1/auth/register", self.base_url))
            .json(&json!({
                "username": username,
                "email": email,
                "password": "testpassword123"
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            panic!("User registration failed: {} - {}", status, body_text);
        }
        let body: serde_json::Value = response.json().await?;
        // Handler returns AuthResponseV2 with access_token/refresh_token (not legacy `token`).
        self.auth_token = body["access_token"].as_str().map(|s| s.to_string());
        self.user_id = body["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        Ok(())
    }

    /// Login with existing user
    #[allow(dead_code)]
    async fn login(
        &mut self,
        email: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(format!("{}/api/v1/auth/login", self.base_url))
            .json(&json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        assert!(response.status().is_success(), "Login failed");
        let body: serde_json::Value = response.json().await?;
        self.auth_token = body["access_token"].as_str().map(|s| s.to_string());
        self.user_id = body["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
        Ok(())
    }

    /// Create an organization
    async fn create_org(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.auth_token.as_ref().ok_or("Not authenticated")?;
        let response = self
            .client
            .post(format!("{}/api/v1/organizations", self.base_url))
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
            0x1F, 0x8B, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x03, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let test_id = Uuid::new_v4().simple();
    helper
        .register_user(&format!("testuser_{}", test_id), &format!("test_{}@example.com", test_id))
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", test_id))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a plugin
    let wasm_data = MarketplaceTestHelper::create_test_wasm();
    let checksum = MarketplaceTestHelper::calculate_checksum(&wasm_data);
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(&wasm_data);

    let publish_response = helper
        .client
        .post(format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", test_id),
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

    let publish_body: serde_json::Value =
        publish_response.json().await.expect("Failed to parse response");
    assert_eq!(publish_body["success"], true);
    let plugin_name = format!("test-plugin-{}", test_id);

    // The tracked download must return the uploaded bytes. With S3 storage it
    // redirects to a presigned URL; the bucket rejects unsigned reads (#1239).
    let download = helper
        .client
        .get(format!(
            "{}/api/v1/plugins/{}/versions/1.0.0/download",
            helper.base_url, plugin_name
        ))
        .send()
        .await
        .expect("Failed to download plugin");
    assert!(download.status().is_success(), "Plugin download failed: {}", download.status());
    assert_eq!(download.bytes().await.unwrap().as_ref(), wasm_data.as_slice());

    // Step 4: Search for the plugin
    //
    // `SearchQuery` in `mockforge-plugin-registry` uses camelCase and requires
    // `tags` and `sort` to be present (no `#[serde(default)]`). Sending only
    // `query`/`page`/`perPage` makes axum reject the body with 422 before the
    // handler runs.
    let search_response = helper
        .client
        .post(format!("{}/api/v1/plugins/search", helper.base_url))
        .json(&json!({
            "query": plugin_name.clone(),
            "tags": json!([]),
            "sort": "downloads",
            "page": 0,
            "perPage": 10
        }))
        .send()
        .await
        .expect("Failed to search plugins");

    assert!(
        search_response.status().is_success(),
        "Plugin search failed: {:?}",
        search_response.text().await
    );
    let search_body: serde_json::Value =
        search_response.json().await.expect("Failed to parse response");
    assert!(!search_body["plugins"].as_array().unwrap().is_empty());

    // Every sort mode the UI offers must succeed. "popular" (the UI default)
    // and "security" order by expressions, which Postgres rejects when the
    // query uses SELECT DISTINCT; that shipped as a 500 on the registry page.
    for (sort, tags) in [
        ("downloads", json!([])),
        ("rating", json!([])),
        ("recent", json!([])),
        ("name", json!([])),
        ("popular", json!([])),
        ("security", json!([])),
        ("popular", json!(["e2e"])),
    ] {
        let response = helper
            .client
            .post(format!("{}/api/v1/plugins/search", helper.base_url))
            .json(&json!({ "tags": tags, "sort": sort, "page": 0, "perPage": 24 }))
            .send()
            .await
            .expect("Failed to search plugins");
        assert!(
            response.status().is_success(),
            "Plugin search with sort={sort} tags={tags} failed: {:?}",
            response.text().await
        );
    }

    // Step 5: Get plugin details
    let get_response = helper
        .client
        .get(format!("{}/api/v1/plugins/{}", helper.base_url, plugin_name))
        .send()
        .await
        .expect("Failed to get plugin");

    assert!(get_response.status().is_success());
    let plugin_body: serde_json::Value =
        get_response.json().await.expect("Failed to parse response");
    assert_eq!(plugin_body["name"], plugin_name);

    // Step 6: Submit a review.
    //
    // `SubmitReviewRequest` requires `version` + `rating` + `comment`. Without
    // `version` the body is rejected with 422 before the handler runs.
    let review_response = helper
        .client
        .post(format!("{}/api/v1/plugins/{}/reviews", helper.base_url, plugin_name))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "version": "1.0.0",
            "rating": 5,
            "title": "Great plugin!",
            "comment": "This plugin works perfectly for testing purposes."
        }))
        .send()
        .await
        .expect("Failed to submit review");

    assert!(
        review_response.status().is_success(),
        "Plugin review submit failed: {:?}",
        review_response.text().await
    );

    // Step 7: Get reviews
    let reviews_response = helper
        .client
        .get(format!("{}/api/v1/plugins/{}/reviews", helper.base_url, plugin_name))
        .send()
        .await
        .expect("Failed to get reviews");

    assert!(reviews_response.status().is_success());
    let reviews_body: serde_json::Value =
        reviews_response.json().await.expect("Failed to parse response");
    assert!(!reviews_body["reviews"].as_array().unwrap().is_empty());
}

/// Test template marketplace workflow
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_template_marketplace_workflow() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let test_id = Uuid::new_v4().simple();
    helper
        .register_user(&format!("testuser_{}", test_id), &format!("test_{}@example.com", test_id))
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", test_id))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a template
    let package_data = MarketplaceTestHelper::create_test_package();
    let checksum = MarketplaceTestHelper::calculate_checksum(&package_data);
    let package_base64 = base64::engine::general_purpose::STANDARD.encode(&package_data);

    // Templates publish under the `/marketplace/` prefix. `PublishTemplateRequest`
    // uses `content_json` (not `content`) and `TemplateCategory` is a kebab-case
    // enum — `"chaos"` is not a valid variant; use `"network-chaos"`.
    let template_name = format!("test-template-{}", test_id);
    let publish_response = helper
        .client
        .post(format!("{}/api/v1/marketplace/templates/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .header("X-Org-Id", helper.org_id.unwrap().to_string())
        .json(&json!({
            "name": template_name.clone(),
            "slug": template_name.clone(),
            "description": "Test template for E2E testing",
            "version": "1.0.0",
            "category": "network-chaos",
            "content_json": json!({
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
    //
    // `TemplateSearchQuery` lives under `/marketplace/templates/search` and uses
    // snake_case fields; `tags` is required (no `#[serde(default)]`).
    //
    // The publish step stored the template with the caller's org_id, so the
    // search must replay the auth header: `OptionalAuthUser` resolves the
    // authenticated user from the bearer token and `search_templates` includes
    // that user's org-scoped rows. Without auth the WHERE clause narrows to
    // `org_id IS NULL` (public only) and excludes the row we just published
    // (regression guarded — see issue #724).
    let search_response = helper
        .client
        .post(format!("{}/api/v1/marketplace/templates/search", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "query": template_name.clone(),
            "tags": json!([]),
            "page": 0,
            "per_page": 10
        }))
        .send()
        .await
        .expect("Failed to search templates");

    assert!(
        search_response.status().is_success(),
        "Template search failed: {:?}",
        search_response.text().await
    );
    let search_body: serde_json::Value =
        search_response.json().await.expect("Failed to parse response");
    assert!(
        !search_body["templates"].as_array().unwrap().is_empty(),
        "published template missing from search results (issue #724)"
    );

    // Step 5: Get template details. Templates are addressed by name + version
    // under the marketplace prefix.
    let get_response = helper
        .client
        .get(format!(
            "{}/api/v1/marketplace/templates/{}/1.0.0",
            helper.base_url, template_name
        ))
        .send()
        .await
        .expect("Failed to get template");

    assert!(
        get_response.status().is_success(),
        "Get template failed: {:?}",
        get_response.text().await
    );
}

/// Test scenario marketplace workflow
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_scenario_marketplace_workflow() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Step 1: Register and authenticate
    let test_id = Uuid::new_v4().simple();
    helper
        .register_user(&format!("testuser_{}", test_id), &format!("test_{}@example.com", test_id))
        .await
        .expect("Failed to register user");

    // Step 2: Create organization
    helper
        .create_org(&format!("test-org-{}", test_id))
        .await
        .expect("Failed to create organization");

    // Step 3: Publish a scenario
    let package_data = MarketplaceTestHelper::create_test_package();
    let checksum = MarketplaceTestHelper::calculate_checksum(&package_data);
    let package_base64 = base64::engine::general_purpose::STANDARD.encode(&package_data);

    let scenario_name = format!("test-scenario-{}", test_id);
    let manifest = json!({
        "name": scenario_name.clone(),
        "version": "1.0.0",
        "description": "Test scenario for E2E testing"
    });

    // Scenarios publish/search live under the `/marketplace/` prefix.
    // The bare `/api/v1/scenarios/*` paths are reserved for org-level listing
    // (GET /api/v1/scenarios, GET /api/v1/scenarios/{id}) and return 405 for POST.
    let publish_response = helper
        .client
        .post(format!("{}/api/v1/marketplace/scenarios/publish", helper.base_url))
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

    // The advertised package URL must be fetchable (#1239). Local-storage
    // `/storage/` paths aren't routed, so only check absolute URLs.
    let version_response = helper
        .client
        .get(format!(
            "{}/api/v1/marketplace/scenarios/{}/versions/1.0.0",
            helper.base_url, scenario_name
        ))
        .header("Authorization", helper.auth_header().unwrap())
        .send()
        .await
        .expect("Failed to get scenario version");
    assert!(
        version_response.status().is_success(),
        "Scenario version lookup failed: {}",
        version_response.status()
    );
    let version_body: serde_json::Value = version_response.json().await.unwrap();
    let package_url = version_body["download_url"].as_str().expect("download_url").to_string();
    if package_url.starts_with("http") {
        let package =
            helper.client.get(&package_url).send().await.expect("Failed to fetch package");
        assert!(package.status().is_success(), "Package fetch failed: {}", package.status());
        assert_eq!(package.bytes().await.unwrap().as_ref(), package_data.as_slice());
    }

    // Step 4: Search for the scenario
    //
    // `ScenarioSearchQuery` uses snake_case; `tags` is required (no
    // `#[serde(default)]`).
    //
    // The publish step stored the scenario with the caller's org_id. The search
    // replays the auth header so `OptionalAuthUser` resolves the user from the
    // bearer token and `search_scenarios` includes that user's org-scoped rows.
    // Without auth the WHERE clause narrows to `org_id IS NULL`, hiding the row
    // we just published (regression guarded — see issue #724).
    let search_response = helper
        .client
        .post(format!("{}/api/v1/marketplace/scenarios/search", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "query": scenario_name.clone(),
            "tags": json!([]),
            "page": 0,
            "per_page": 10
        }))
        .send()
        .await
        .expect("Failed to search scenarios");

    assert!(
        search_response.status().is_success(),
        "Scenario search failed: {:?}",
        search_response.text().await
    );
    let search_body: serde_json::Value =
        search_response.json().await.expect("Failed to parse response");
    assert!(
        !search_body["scenarios"].as_array().unwrap().is_empty(),
        "published scenario missing from search results (issue #724)"
    );

    // Step 5: Get scenario details (by name, under the marketplace prefix).
    let get_response = helper
        .client
        .get(format!("{}/api/v1/marketplace/scenarios/{}", helper.base_url, scenario_name))
        .send()
        .await
        .expect("Failed to get scenario");

    assert!(
        get_response.status().is_success(),
        "Get scenario failed: {:?}",
        get_response.text().await
    );

    // Step 6: Submit a review (under the marketplace prefix).
    let review_response = helper
        .client
        .post(format!(
            "{}/api/v1/marketplace/scenarios/{}/reviews",
            helper.base_url, scenario_name
        ))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "rating": 5,
            "title": "Great scenario!",
            "comment": "This scenario works perfectly for testing purposes."
        }))
        .send()
        .await
        .expect("Failed to submit review");

    assert!(
        review_response.status().is_success(),
        "Submit scenario review failed: {:?}",
        review_response.text().await
    );
}

/// Test validation errors
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_upload_validation_errors() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    // Register and authenticate
    let test_id = Uuid::new_v4().simple();
    helper
        .register_user(&format!("testuser_{}", test_id), &format!("test_{}@example.com", test_id))
        .await
        .expect("Failed to register user");

    // Test 1: Invalid WASM file (wrong magic bytes)
    let invalid_wasm = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    let checksum = MarketplaceTestHelper::calculate_checksum(&invalid_wasm);
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(&invalid_wasm);

    let response = helper
        .client
        .post(format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", test_id),
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": Vec::<String>::new(),
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
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(&wasm_data);

    let response = helper
        .client
        .post(format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": "../../etc/passwd",
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": Vec::<String>::new(),
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

    // Test 3: File too large - create an 11 MB buffer with WASM magic bytes
    let mut large_wasm = vec![0u8; 11 * 1024 * 1024]; // 11 MB
    large_wasm[..4].copy_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
    let checksum = MarketplaceTestHelper::calculate_checksum(&large_wasm);
    let wasm_base64 = base64::engine::general_purpose::STANDARD.encode(&large_wasm);

    let response = helper
        .client
        .post(format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": format!("test-plugin-{}", test_id),
            "version": "1.0.0",
            "description": "Test plugin",
            "category": "testing",
            "license": "MIT",
            "tags": Vec::<String>::new(),
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

/// Star a template, verify the count goes up, toggle off, verify it goes down,
/// and verify that the star count exposed by the public search/get endpoints
/// reflects the live row count (not a denormalized counter on `templates`).
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_template_star_flow() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut helper = MarketplaceTestHelper::new(base_url);

    let test_id = Uuid::new_v4().simple();
    helper
        .register_user(&format!("staruser_{}", test_id), &format!("star_{}@example.com", test_id))
        .await
        .expect("Failed to register user");
    helper
        .create_org(&format!("star-org-{}", test_id))
        .await
        .expect("Failed to create organization");

    // Publish a template via the marketplace path.
    let package_data = MarketplaceTestHelper::create_test_package();
    let checksum = MarketplaceTestHelper::calculate_checksum(&package_data);
    let package_base64 = base64::engine::general_purpose::STANDARD.encode(&package_data);
    let template_name = format!("star-template-{}", test_id);
    let version = "1.0.0";

    let publish_response = helper
        .client
        .post(format!("{}/api/v1/marketplace/templates/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .header("X-Org-Id", helper.org_id.unwrap().to_string())
        .json(&json!({
            "name": template_name.clone(),
            "slug": template_name.clone(),
            "description": "Star-flow regression template",
            "version": version,
            "category": "network-chaos",
            "content_json": json!({ "name": template_name, "version": version }),
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

    let star_path = format!(
        "{}/api/v1/marketplace/templates/{}/{}/star",
        helper.base_url, template_name, version
    );

    // Toggle on.
    let resp = helper
        .client
        .post(&star_path)
        .header("Authorization", helper.auth_header().unwrap())
        .send()
        .await
        .expect("Failed to star template");
    assert!(resp.status().is_success(), "Star toggle on failed");
    let body: serde_json::Value = resp.json().await.expect("parse toggle response");
    assert_eq!(body["starred"], json!(true), "should report starred=true");
    assert_eq!(body["stars"], json!(1), "should report stars=1");

    // Toggle off.
    let resp = helper
        .client
        .post(&star_path)
        .header("Authorization", helper.auth_header().unwrap())
        .send()
        .await
        .expect("Failed to unstar template");
    assert!(resp.status().is_success(), "Star toggle off failed");
    let body: serde_json::Value = resp.json().await.expect("parse toggle response");
    assert_eq!(body["starred"], json!(false), "should report starred=false");
    assert_eq!(body["stars"], json!(0), "should report stars=0");

    // GET on the marketplace endpoint should reflect the live count, not a
    // cached value from stats_json.
    let get = helper
        .client
        .get(format!(
            "{}/api/v1/marketplace/templates/{}/{}",
            helper.base_url, template_name, version
        ))
        .send()
        .await
        .expect("Failed to get template");
    assert!(get.status().is_success());
    let got: serde_json::Value = get.json().await.expect("parse get response");
    assert_eq!(got["stats"]["stars"], json!(0), "live stars should be 0");

    // Anonymous POST to star endpoint must be rejected.
    let anon = Client::new().post(&star_path).send().await.expect("anonymous request");
    assert_eq!(anon.status().as_u16(), 401, "anonymous star should be 401");
}

async fn publish_plugin_bytes(
    helper: &MarketplaceTestHelper,
    name: &str,
    wasm: &[u8],
) -> reqwest::StatusCode {
    helper
        .client
        .post(format!("{}/api/v1/plugins/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .json(&json!({
            "name": name,
            "version": "1.0.0",
            "description": "Overwrite guard test",
            "category": "testing",
            "license": "MIT",
            "tags": vec!["test"],
            "checksum": MarketplaceTestHelper::calculate_checksum(wasm),
            "file_size": wasm.len() as i64,
            "wasm_data": base64::engine::general_purpose::STANDARD.encode(wasm),
            "dependencies": {}
        }))
        .send()
        .await
        .expect("Failed to publish plugin")
        .status()
}

async fn publish_scenario_bytes(
    helper: &MarketplaceTestHelper,
    name: &str,
    package: &[u8],
) -> reqwest::StatusCode {
    let manifest = json!({ "name": name, "version": "1.0.0", "description": "Overwrite guard" });
    helper
        .client
        .post(format!("{}/api/v1/marketplace/scenarios/publish", helper.base_url))
        .header("Authorization", helper.auth_header().unwrap())
        .header("X-Org-Id", helper.org_id.unwrap().to_string())
        .json(&json!({
            "manifest": manifest.to_string(),
            "checksum": MarketplaceTestHelper::calculate_checksum(package),
            "size": package.len() as u64,
            "package": base64::engine::general_purpose::STANDARD.encode(package)
        }))
        .send()
        .await
        .expect("Failed to publish scenario")
        .status()
}

/// Another tenant can't publish over an existing plugin or scenario, and an
/// owner can't replace a published version. Rejections happen before the
/// upload, so the served artifact keeps the original bytes.
#[tokio::test]
#[ignore] // Requires running registry server
async fn test_publish_cannot_overwrite_existing_artifacts() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let test_id = Uuid::new_v4().simple();
    let mut owner = MarketplaceTestHelper::new(base_url.clone());
    owner
        .register_user(&format!("owner_{test_id}"), &format!("owner_{test_id}@example.com"))
        .await
        .expect("Failed to register owner");
    owner
        .create_org(&format!("owner-org-{test_id}"))
        .await
        .expect("Failed to create org");
    let mut other = MarketplaceTestHelper::new(base_url);
    other
        .register_user(&format!("other_{test_id}"), &format!("other_{test_id}@example.com"))
        .await
        .expect("Failed to register other user");
    other
        .create_org(&format!("other-org-{test_id}"))
        .await
        .expect("Failed to create org");

    let original = MarketplaceTestHelper::create_test_wasm();
    // Same header plus an empty custom section: valid wasm, different bytes.
    let mut replacement = original.clone();
    replacement.extend_from_slice(&[0x00, 0x02, 0x01, b'x']);
    let plugin = format!("guard-plugin-{test_id}");
    assert!(publish_plugin_bytes(&owner, &plugin, &original).await.is_success());
    assert_eq!(
        publish_plugin_bytes(&other, &plugin, &replacement).await,
        reqwest::StatusCode::FORBIDDEN
    );
    assert_eq!(
        publish_plugin_bytes(&owner, &plugin, &replacement).await,
        reqwest::StatusCode::CONFLICT
    );
    let served = owner
        .client
        .get(format!("{}/api/v1/plugins/{}/versions/1.0.0/download", owner.base_url, plugin))
        .send()
        .await
        .expect("Failed to download plugin");
    assert!(served.status().is_success());
    assert_eq!(served.bytes().await.unwrap().as_ref(), original.as_slice());

    let package = MarketplaceTestHelper::create_test_package();
    let mut other_package = package.clone();
    *other_package.last_mut().unwrap() = 0x01;
    let scenario = format!("guard-scenario-{test_id}");
    assert!(publish_scenario_bytes(&owner, &scenario, &package).await.is_success());
    assert_eq!(
        publish_scenario_bytes(&other, &scenario, &other_package).await,
        reqwest::StatusCode::FORBIDDEN
    );
    // 409 for a published version, unless the plan's scenario limit (1 by
    // default) rejects first with 400; either way nothing is uploaded.
    let republish = publish_scenario_bytes(&owner, &scenario, &other_package).await;
    assert!(republish.is_client_error(), "owner republish returned {republish}");
    let version: serde_json::Value = owner
        .client
        .get(format!(
            "{}/api/v1/marketplace/scenarios/{}/versions/1.0.0",
            owner.base_url, scenario
        ))
        .header("Authorization", owner.auth_header().unwrap())
        .send()
        .await
        .expect("Failed to get scenario version")
        .json()
        .await
        .unwrap();
    let url = version["download_url"].as_str().unwrap().to_string();
    if url.starts_with("http") {
        let served = owner.client.get(&url).send().await.expect("Failed to fetch package");
        assert_eq!(served.bytes().await.unwrap().as_ref(), package.as_slice());
    }
}
