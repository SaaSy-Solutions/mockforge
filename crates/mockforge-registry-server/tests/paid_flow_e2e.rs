//! End-to-end test for the paid-customer flow (#451).
//!
//! Drives the registry-side state machine that flips a Free org to Pro on a
//! `customer.subscription.created` webhook and back to Free on a
//! `customer.subscription.deleted` webhook, and proves that the protocol
//! gating on `POST /api/v1/hosted-mocks` follows the plan in both directions:
//!
//!   1. Register a new user (auto-creates a Free personal org)
//!   2. Try to deploy a gRPC hosted mock → 400 (Free disallows gRPC)
//!   3. POST a Stripe-signed `customer.subscription.created` (Pro) webhook
//!      → 200; org plan flips to Pro
//!   4. Re-attempt gRPC deploy → 200 (Pro allows gRPC; deployment row is
//!      created in Pending status — the Fly orchestrator that would actually
//!      ship the machine is a separate process and out of scope here)
//!   5. POST a signed `customer.subscription.deleted` webhook → 200; plan
//!      flips back to Free
//!   6. Re-attempt gRPC deploy → 400 again
//!
//! What this test does **not** cover (deferred to follow-up work, per the
//! #451 acceptance criteria):
//!   - Real Stripe checkout-session creation (needs a Stripe test-mode key)
//!   - Real Fly.io machine launch (deployment orchestrator is async)
//!   - Live proxy request through `<slug>.mocks.mockforge.dev`
//!
//! The pieces above are external-service dependencies that don't usefully
//! exercise registry code — the state machine and gating proven here is the
//! actual product surface that has to work when a real customer pays.
//!
//! Requires the same scaffolding as the other `*_e2e.rs` tests in this
//! crate (Postgres + MinIO + registry-server). See `.github/workflows/
//! registry-e2e.yml`. The registry server must be started with
//! `STRIPE_WEBHOOK_SECRET` set to the same value as `TEST_STRIPE_WEBHOOK_SECRET`
//! below (the workflow wires both to one literal value).
//!
//! Run manually with:
//! ```
//! REGISTRY_URL=http://localhost:8080 \
//! STRIPE_WEBHOOK_SECRET=whsec_e2e_test_secret \
//! cargo test -p mockforge-registry-server --test paid_flow_e2e \
//!   -- --ignored --nocapture
//! ```

use hmac::{Hmac, Mac};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Default webhook secret used by both the test and the server when the
/// CI workflow runs the e2e suite. Keep this in sync with the
/// `STRIPE_WEBHOOK_SECRET` env var in `.github/workflows/registry-e2e.yml`.
const DEFAULT_WEBHOOK_SECRET: &str = "whsec_e2e_test_secret";

struct PaidFlowHelper {
    client: Client,
    base_url: String,
    webhook_secret: String,
    access_token: Option<String>,
    user_id: Option<String>,
    org_id: Option<String>,
    customer_id: String,
    subscription_id: String,
}

impl PaidFlowHelper {
    fn new(base_url: String, webhook_secret: String, ts: i64) -> Self {
        Self {
            client: Client::new(),
            base_url,
            webhook_secret,
            access_token: None,
            user_id: None,
            org_id: None,
            // Stable IDs across all the webhook posts in one test run so the
            // server treats them as one subscription lifecycle (create → delete).
            customer_id: format!("cus_e2e_{}", ts),
            subscription_id: format!("sub_e2e_{}", ts),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token.as_ref().expect("not authenticated"))
    }

    fn org_id_str(&self) -> &str {
        self.org_id.as_deref().expect("org_id not resolved")
    }

    /// Sign a Stripe webhook body with the shared secret. Mirrors what
    /// `stripe::Webhook::construct_event` verifies on the server side:
    /// `t=<timestamp>,v1=<hex hmac-sha256("timestamp.payload")>`.
    fn sign(&self, payload: &str, timestamp: i64) -> String {
        let signed = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .expect("HMAC accepts arbitrary key length");
        mac.update(signed.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        format!("t={},v1={}", timestamp, hex::encode(sig_bytes))
    }

    /// POST a signed event body to `/api/v1/billing/webhook` and assert 2xx.
    async fn post_webhook(&self, body: &str) {
        let ts = chrono::Utc::now().timestamp();
        let signature = self.sign(body, ts);
        let res = self
            .client
            .post(format!("{}/api/v1/billing/webhook", self.base_url))
            .header("stripe-signature", signature)
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .expect("webhook POST failed");

        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        assert!(
            status.is_success(),
            "webhook POST returned {}: {}\nbody was: {}",
            status,
            text,
            body
        );
    }

    /// Build a minimal `customer.subscription.created` event body. The
    /// fields here are the ones `handle_subscription_event` actually reads
    /// (price.id, customer, current_period_*, status, metadata.org_id) —
    /// `stripe::Webhook::construct_event` deserializes via the async-stripe
    /// `Event` type, so any required-by-the-type fields also have to be
    /// present even if the handler ignores them.
    fn subscription_event(&self, event_type: &str, status: &str, price_id: &str) -> String {
        let now = chrono::Utc::now().timestamp();
        json!({
            "id": format!("evt_e2e_{}", uuid::Uuid::new_v4()),
            "object": "event",
            "api_version": "2024-04-10",
            "created": now,
            "type": event_type,
            "livemode": false,
            "pending_webhooks": 0,
            "request": { "id": null, "idempotency_key": null },
            "data": {
                "object": {
                    "id": self.subscription_id,
                    "object": "subscription",
                    "customer": self.customer_id,
                    "status": status,
                    "current_period_start": now,
                    "current_period_end": now + 30 * 86400,
                    "cancel_at_period_end": event_type == "customer.subscription.deleted",
                    "canceled_at": if event_type == "customer.subscription.deleted" {
                        Value::from(now)
                    } else {
                        Value::Null
                    },
                    "created": now,
                    "start_date": now,
                    "billing_cycle_anchor": now,
                    "collection_method": "charge_automatically",
                    "currency": "usd",
                    "items": {
                        "object": "list",
                        "data": [{
                            "id": format!("si_e2e_{}", uuid::Uuid::new_v4()),
                            "object": "subscription_item",
                            "price": {
                                "id": price_id,
                                "object": "price",
                                "active": true,
                                "currency": "usd",
                                "product": "prod_e2e",
                                "type": "recurring",
                            },
                            "quantity": 1,
                            "subscription": self.subscription_id,
                        }],
                        "has_more": false,
                        "url": "/v1/subscription_items"
                    },
                    "metadata": {
                        "org_id": self.org_id_str(),
                    },
                }
            }
        })
        .to_string()
    }

    /// Attempt to create a hosted-mock with gRPC enabled. Returns the HTTP
    /// status so the caller can assert allowed-vs-rejected per plan.
    async fn try_grpc_deploy(&self, slug: &str) -> (StatusCode, String) {
        let res = self
            .client
            .post(format!("{}/api/v1/hosted-mocks", self.base_url))
            .header("Authorization", self.auth_header())
            .header("X-Org-Id", self.org_id_str())
            .json(&json!({
                "name": slug,
                "slug": slug,
                "config_json": {},
                "enabled_protocols": ["http", "grpc"],
            }))
            .send()
            .await
            .expect("hosted-mock create request failed");

        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        (status, body)
    }

    /// Read the current plan via `/api/v1/billing/subscription`.
    async fn fetch_plan(&self) -> String {
        let res = self
            .client
            .get(format!("{}/api/v1/billing/subscription", self.base_url))
            .header("Authorization", self.auth_header())
            .header("X-Org-Id", self.org_id_str())
            .send()
            .await
            .expect("subscription GET failed");

        let status = res.status();
        let body: Value = res.json().await.expect("subscription body not JSON");
        assert!(status.is_success(), "subscription GET {} : {}", status, body);
        body["plan"].as_str().expect("plan missing").to_string()
    }
}

#[tokio::test]
#[ignore] // needs running registry server + Postgres + STRIPE_WEBHOOK_SECRET
async fn test_paid_flow_e2e() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .unwrap_or_else(|_| DEFAULT_WEBHOOK_SECRET.to_string());

    let ts = chrono::Utc::now().timestamp();
    let username = format!("paidflow_{}", ts);
    let email = format!("paidflow_{}@e2e-test.local", ts);
    let password = "SecureP@ssw0rd!2024";

    let mut h = PaidFlowHelper::new(base_url.clone(), webhook_secret, ts);

    // ─────────────────────────────────────────────────────────────────
    // Step 1: Register — auto-creates a Free personal org
    // ─────────────────────────────────────────────────────────────────
    println!("Step 1: Register user {}", username);
    let res = h
        .client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("register request failed");

    let status = res.status();
    let body: Value = res.json().await.expect("register response not JSON");
    assert!(status.is_success(), "Register failed ({}): {}", status, body);

    h.access_token = body["access_token"].as_str().map(str::to_string);
    h.user_id = body["user_id"].as_str().map(str::to_string);
    assert!(h.access_token.is_some(), "no access_token in register response: {}", body);

    // ─────────────────────────────────────────────────────────────────
    // Step 2: Resolve the auto-created org_id
    // ─────────────────────────────────────────────────────────────────
    let res = h
        .client
        .get(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", h.auth_header())
        .send()
        .await
        .expect("list orgs request failed");
    let body: Value = res.json().await.expect("orgs body not JSON");
    let orgs = body
        .as_array()
        .or_else(|| body["organizations"].as_array())
        .expect("organizations payload missing");
    let first_org = orgs.first().expect("user has no organizations");
    h.org_id = first_org["id"].as_str().map(str::to_string);
    assert!(h.org_id.is_some(), "no org id in orgs response: {}", body);
    println!("  -> org_id={} plan={}", h.org_id_str(), first_org["plan"]);

    // Sanity check: the auto-org should start on Free.
    assert_eq!(h.fetch_plan().await, "free", "fresh org should be on Free");

    // ─────────────────────────────────────────────────────────────────
    // Step 3: Free org cannot deploy gRPC
    // ─────────────────────────────────────────────────────────────────
    println!("Step 3: Free org gRPC deploy (must fail)");
    let (status, body) = h.try_grpc_deploy(&format!("paidflow-free-{}", ts)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Free gRPC deploy should be rejected, got {}: {}",
        status,
        body
    );
    assert!(
        body.to_lowercase().contains("plan") || body.to_lowercase().contains("upgrade"),
        "Expected plan-gate error, got: {}",
        body
    );

    // ─────────────────────────────────────────────────────────────────
    // Step 4: Webhook flips org to Pro
    // ─────────────────────────────────────────────────────────────────
    // `price_test_pro` triggers the handler's heuristic-matching fallback
    // in `determine_plan_from_price_id` — no STRIPE_PRICE_ID_PRO env var
    // required.
    println!("Step 4: customer.subscription.created (Pro)");
    let body = h.subscription_event("customer.subscription.created", "active", "price_test_pro");
    h.post_webhook(&body).await;

    let plan = h.fetch_plan().await;
    assert_eq!(plan, "pro", "plan should be Pro after subscription.created webhook");

    // ─────────────────────────────────────────────────────────────────
    // Step 5: Pro org can deploy gRPC
    // ─────────────────────────────────────────────────────────────────
    println!("Step 5: Pro org gRPC deploy (must succeed)");
    let (status, body) = h.try_grpc_deploy(&format!("paidflow-pro-{}", ts)).await;
    assert!(status.is_success(), "Pro gRPC deploy should succeed, got {}: {}", status, body);

    // ─────────────────────────────────────────────────────────────────
    // Step 6: Webhook cancels the subscription → back to Free
    // ─────────────────────────────────────────────────────────────────
    println!("Step 6: customer.subscription.deleted");
    let body = h.subscription_event("customer.subscription.deleted", "canceled", "price_test_pro");
    h.post_webhook(&body).await;

    let plan = h.fetch_plan().await;
    assert_eq!(plan, "free", "plan should revert to Free after subscription.deleted webhook");

    // ─────────────────────────────────────────────────────────────────
    // Step 7: gRPC deploy is rejected again
    // ─────────────────────────────────────────────────────────────────
    println!("Step 7: post-cancel gRPC deploy (must fail again)");
    let (status, body) = h.try_grpc_deploy(&format!("paidflow-cancelled-{}", ts)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Post-cancel gRPC deploy should be rejected, got {}: {}",
        status,
        body
    );

    println!("paid-flow e2e: all 7 steps passed");
}

// ───────────────────────────────── unit-side guards ─────────────────────────────────
//
// These run on every `cargo test` (no `#[ignore]`) so we catch regressions
// in the test's own signing/fixture helpers without needing the e2e stack.

#[test]
fn signature_format_matches_stripe_layout() {
    let h = PaidFlowHelper::new(
        "http://unused".to_string(),
        DEFAULT_WEBHOOK_SECRET.to_string(),
        1234567890,
    );
    let sig = h.sign("{\"hello\":\"world\"}", 1700000000);
    assert!(sig.starts_with("t=1700000000,v1="), "unexpected format: {}", sig);
    // SHA-256 hex digest is 64 chars
    let hex_part = sig.rsplit('=').next().unwrap();
    assert_eq!(hex_part.len(), 64, "hex digest length: {}", sig);
}

#[test]
fn subscription_event_includes_org_id_metadata() {
    let mut h =
        PaidFlowHelper::new("http://unused".to_string(), DEFAULT_WEBHOOK_SECRET.to_string(), 1);
    h.org_id = Some("00000000-0000-0000-0000-000000000001".to_string());
    let body = h.subscription_event("customer.subscription.created", "active", "price_test_pro");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        parsed["data"]["object"]["metadata"]["org_id"],
        "00000000-0000-0000-0000-000000000001"
    );
    assert_eq!(parsed["data"]["object"]["items"]["data"][0]["price"]["id"], "price_test_pro");
}

#[test]
fn deleted_event_marks_canceled_at_set() {
    let mut h =
        PaidFlowHelper::new("http://unused".to_string(), DEFAULT_WEBHOOK_SECRET.to_string(), 1);
    h.org_id = Some("00000000-0000-0000-0000-000000000001".to_string());
    let body = h.subscription_event("customer.subscription.deleted", "canceled", "price_test_pro");
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert!(
        parsed["data"]["object"]["canceled_at"].is_number(),
        "deleted event must have canceled_at set so the server records the cancellation"
    );
}
