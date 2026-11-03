//! Regression guards for the billing-bypass vulnerability (#733).
//!
//! The org `plan` column is server-authoritative: the ONLY writer is the
//! Stripe billing webhook (`handlers/billing.rs`). A client must never be able
//! to provision or self-upgrade to a paid plan through the organization CRUD
//! API. These tests prove both attack vectors are closed:
//!
//!   1. `POST /api/v1/organizations` with `{"plan":"team"}` does NOT yield a
//!      Team org — the request is rejected (400) and, even if it weren't, the
//!      resulting org would be Free.
//!   2. `PATCH /api/v1/organizations/{id}` with `{"plan":"team"}` does NOT
//!      change the plan — it is rejected (400) and the plan stays Free.
//!
//! Requires the same scaffolding as the other `*_e2e.rs` tests in this crate
//! (Postgres + registry-server). See `signup_flow_e2e.rs` for the env vars.
//!
//! Run manually with:
//! ```
//! REGISTRY_URL=http://localhost:8080 \
//! cargo test -p mockforge-registry-server --test billing_bypass_guard_e2e \
//!   -- --ignored --nocapture
//! ```

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

/// Register a fresh user and return (client, access_token).
async fn register(base_url: &str) -> (Client, String) {
    let client = Client::new();
    let ts = chrono::Utc::now().timestamp_micros();
    let username = format!("bypass_{}", ts);
    let email = format!("bypass_{}@e2e-test.local", ts);
    let password = "SecureP@ssw0rd!2024";

    let res = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&json!({ "username": username, "email": email, "password": password }))
        .send()
        .await
        .expect("register failed");
    let status = res.status();
    let body: Value = res.json().await.expect("register not JSON");
    assert!(status.is_success(), "register {}: {}", status, body);
    let token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();
    (client, token)
}

/// Create an org with the given JSON body and return the raw response.
async fn create_org(
    client: &Client,
    base_url: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.unwrap_or(json!({"raw": "non-json"}));
    (status, body)
}

#[tokio::test]
#[ignore] // needs running registry server + Postgres
async fn create_org_with_paid_plan_is_blocked() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let (client, token) = register(&base_url).await;
    let ts = chrono::Utc::now().timestamp_micros();

    // Attempt to provision a Team org directly via the create API.
    let (status, body) = create_org(
        &client,
        &base_url,
        &token,
        json!({
            "name": format!("Bypass Create {}", ts),
            "slug": format!("bypass-create-{}", ts),
            "plan": "team",
        }),
    )
    .await;

    // The fix rejects a non-free plan on create with a 400. Even if a future
    // refactor were to accept the body, the resulting plan must be Free — so we
    // accept either outcome, but never a Team/Pro org.
    if status.is_success() {
        assert_eq!(
            body["plan"].as_str(),
            Some("free"),
            "org create must never yield a paid plan; got {}",
            body
        );
    } else {
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected 400 rejecting client-set plan, got {}: {}",
            status,
            body
        );
    }
}

#[tokio::test]
#[ignore] // needs running registry server + Postgres
async fn create_org_with_free_plan_is_allowed() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let (client, token) = register(&base_url).await;
    let ts = chrono::Utc::now().timestamp_micros();

    // An explicit "free" plan is still allowed (it matches the only value a
    // client may legitimately send).
    let (status, body) = create_org(
        &client,
        &base_url,
        &token,
        json!({
            "name": format!("Bypass Free {}", ts),
            "slug": format!("bypass-free-{}", ts),
            "plan": "free",
        }),
    )
    .await;
    assert!(status.is_success(), "free-plan create should succeed, got {}: {}", status, body);
    assert_eq!(body["plan"].as_str(), Some("free"));
}

#[tokio::test]
#[ignore] // needs running registry server + Postgres
async fn owner_cannot_self_upgrade_plan_via_patch() {
    let base_url =
        std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let (client, token) = register(&base_url).await;
    let ts = chrono::Utc::now().timestamp_micros();

    // Create a (Free) org as the owner.
    let (status, body) = create_org(
        &client,
        &base_url,
        &token,
        json!({
            "name": format!("Bypass Patch {}", ts),
            "slug": format!("bypass-patch-{}", ts),
        }),
    )
    .await;
    assert!(status.is_success(), "create org should succeed, got {}: {}", status, body);
    let org_id = body["id"].as_str().expect("no org id").to_string();
    assert_eq!(body["plan"].as_str(), Some("free"), "fresh org should be Free");

    // Owner attempts to self-upgrade to Team via PATCH.
    let res = client
        .patch(format!("{}/api/v1/organizations/{}", base_url, org_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&json!({ "plan": "team" }))
        .send()
        .await
        .expect("patch org failed");
    let patch_status = res.status();
    let patch_body: Value = res.json().await.unwrap_or(json!({"raw": "non-json"}));

    // The fix rejects the plan mutation with a 400.
    assert_eq!(
        patch_status,
        StatusCode::BAD_REQUEST,
        "owner self-upgrade via PATCH must be rejected, got {}: {}",
        patch_status,
        patch_body
    );

    // And the plan must still be Free when re-read.
    let res = client
        .get(format!("{}/api/v1/organizations/{}", base_url, org_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("get org failed");
    let body: Value = res.json().await.expect("get org not JSON");
    assert_eq!(
        body["plan"].as_str(),
        Some("free"),
        "plan must be unchanged after a rejected self-upgrade; got {}",
        body
    );
}
