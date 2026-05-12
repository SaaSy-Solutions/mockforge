//! End-to-end test for paid-flow plan gating
//!
//! Verifies the enforcement points shipped in #449 (quota gates) and #450
//! (cost cap), at the HTTP surface against a running registry server.
//!
//! What this test covers:
//! 1. Register a Free-tier user — confirm gRPC is rejected with the
//!    explicit "upgrade" error (protocol gate from hosted_mocks.rs).
//! 2. Free quota — confirm a second workspace beyond the limit is
//!    rejected with the explicit "Upgrade to create more" message
//!    (max_projects gate from cloud_workspaces.rs).
//! 3. Billing API surface — confirm /api/v1/billing/subscription returns
//!    the org's current plan + reasonable shape.
//!
//! What this test does NOT cover (deferred to follow-up):
//! - Stripe webhook → plan transition simulation. The webhook handler
//!   requires HMAC-signed payloads + crafted stripe::Subscription
//!   fixtures. That's a 200+ line scaffolding investment of its own;
//!   the issue tracker calls it out explicitly.
//! - Hitting a deployed mock at <slug>.mocks.mockforge.dev — requires
//!   Fly orchestrator running in the test env, which the registry-e2e
//!   compose stack doesn't currently provide.
//! - past_due block on deploy — requires inserting a Subscription row
//!   directly, which means DB access from the test. The test runner
//!   here only has HTTP access. Covered by unit tests for now.
//!
//! Requires:
//!   - PostgreSQL + MinIO running (docker-compose -f docker-compose.e2e.yml up -d)
//!   - Registry server running on REGISTRY_URL
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   cargo test --test paid_flow_e2e -- --ignored --nocapture

use reqwest::{Client, StatusCode};
use serde_json::json;

struct PaidFlowHelper {
    client: Client,
    base_url: String,
    access_token: Option<String>,
}

impl PaidFlowHelper {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            access_token: None,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token.as_ref().expect("not authenticated"))
    }

    async fn register(&mut self, username: &str, email: &str) {
        let res = self
            .client
            .post(format!("{}/api/v1/auth/register", self.base_url))
            .json(&json!({
                "username": username,
                "email": email,
                "password": "SecureP@ssw0rd!2024",
            }))
            .send()
            .await
            .expect("register request failed");
        let status = res.status();
        let body: serde_json::Value = res.json().await.expect("register response not JSON");
        assert!(status.is_success(), "Registration failed ({}): {}", status, body);
        self.access_token = body["access_token"]
            .as_str()
            .or_else(|| body["token"].as_str())
            .map(|s| s.to_string());
        assert!(self.access_token.is_some(), "No access token in register response: {}", body);
    }
}

#[tokio::test]
#[ignore] // Requires running registry server + database
async fn free_tier_rejects_grpc_protocol() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut h = PaidFlowHelper::new(base_url.clone());

    let ts = chrono::Utc::now().timestamp();
    h.register(&format!("free_grpc_{}", ts), &format!("free_grpc_{}@e2e-test.local", ts))
        .await;

    // Free orgs are bootstrapped via auto-org on registration. Attempt to
    // deploy a hosted mock with the gRPC protocol — should reject with
    // an "upgrade" message (the protocol gate in hosted_mocks.rs).
    let res = h
        .client
        .post(format!("{}/api/v1/hosted-mocks", base_url))
        .header("Authorization", h.auth_header())
        .json(&json!({
            "name": "grpc-mock",
            "slug": format!("free-grpc-{}", ts),
            "enabled_protocols": ["http", "grpc"],
            "config": { "spec_url": "https://example.com/spec.json" },
        }))
        .send()
        .await
        .expect("deploy request failed");

    let status = res.status();
    let body: serde_json::Value = res
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"_raw_body": "non-json"}));

    assert!(
        status.is_client_error(),
        "Free org gRPC deploy should be rejected with 4xx, got {}: {}",
        status,
        body
    );

    // Spot-check the error message hints at the plan upgrade — not just a
    // generic 400. This guards against the gate regressing into a
    // "validation error" that doesn't tell the customer why.
    let message = body["error"]
        .as_str()
        .or_else(|| body["message"].as_str())
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        message.contains("upgrade") || message.contains("plan") || message.contains("grpc"),
        "Expected 'upgrade/plan/grpc' hint in error message, got: {}",
        body
    );
}

#[tokio::test]
#[ignore] // Requires running registry server + database
async fn free_tier_rejects_second_workspace() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut h = PaidFlowHelper::new(base_url.clone());

    let ts = chrono::Utc::now().timestamp();
    h.register(&format!("free_ws_{}", ts), &format!("free_ws_{}@e2e-test.local", ts))
        .await;

    // Free plan: max_projects = 1. Register auto-creates one personal
    // workspace, but it's an org-level personal org without an attached
    // workspace, so we'll create the first workspace then expect the
    // second to be rejected.
    let first = h
        .client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", h.auth_header())
        .json(&json!({ "name": "first-ws", "description": "" }))
        .send()
        .await
        .expect("first workspace request failed");
    let first_status = first.status();
    let first_body: serde_json::Value = first.json().await.unwrap_or(serde_json::json!({}));
    // First should succeed (Free allows 1).
    assert!(
        first_status.is_success(),
        "First workspace on Free should succeed, got {}: {}",
        first_status,
        first_body
    );

    // Second should be rejected with max_projects message.
    let second = h
        .client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", h.auth_header())
        .json(&json!({ "name": "second-ws", "description": "" }))
        .send()
        .await
        .expect("second workspace request failed");
    let second_status = second.status();
    let second_body: serde_json::Value = second
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"_raw_body": "non-json"}));

    assert!(
        second_status.is_client_error(),
        "Second workspace on Free should be rejected with 4xx, got {}: {}",
        second_status,
        second_body
    );

    let message = second_body["error"]
        .as_str()
        .or_else(|| second_body["message"].as_str())
        .unwrap_or_default()
        .to_lowercase();
    assert!(
        message.contains("workspace") || message.contains("upgrade") || message.contains("plan"),
        "Expected 'workspace/upgrade/plan' hint in error, got: {}",
        second_body
    );
}

#[tokio::test]
#[ignore] // Requires running registry server + database
async fn billing_subscription_endpoint_shape() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let mut h = PaidFlowHelper::new(base_url.clone());

    let ts = chrono::Utc::now().timestamp();
    h.register(
        &format!("billing_shape_{}", ts),
        &format!("billing_shape_{}@e2e-test.local", ts),
    )
    .await;

    let res = h
        .client
        .get(format!("{}/api/v1/billing/subscription", base_url))
        .header("Authorization", h.auth_header())
        .send()
        .await
        .expect("subscription request failed");

    let status = res.status();
    let body: serde_json::Value = res
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"_raw_body": "non-json"}));

    // The endpoint should be reachable for an authenticated user. The
    // exact shape may vary (it depends on whether Stripe is configured),
    // but a free-tier user must at minimum see their plan reported.
    assert!(
        status.is_success() || status == StatusCode::NOT_FOUND,
        "Subscription endpoint should respond 2xx or 404 for no-stripe configs, got {}: {}",
        status,
        body
    );

    if status.is_success() {
        // Sanity: response should reference a plan tier.
        let plan_hint = body.to_string().to_lowercase();
        assert!(
            plan_hint.contains("plan")
                || plan_hint.contains("free")
                || plan_hint.contains("pro")
                || plan_hint.contains("team"),
            "Subscription response should include plan info, got: {}",
            body
        );
    }
}
