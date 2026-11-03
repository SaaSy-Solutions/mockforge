//! End-to-end coverage for the tables that became Postgres-RLS-forced through
//! JOIN-based policies in migration `20250101000087_rls_join_policies.sql`
//! (issue #1087): tenant tables with no `org_id` column of their own, scoped
//! through a parent (`workspace_id`, `deployment_id`, `run_id`, ...).
//!
//! Under `scripts/rls-e2e-gate.sh` the request path runs on a `NOBYPASSRLS`
//! role, so a handler that touches one of these tables without binding the
//! org that owns the parent fail-closes: an empty list right after a create,
//! a 4xx on a row that exists, or a 500 from a `WITH CHECK` violation. Each
//! test does a real round trip plus a CROSS-TENANT check (another org's user
//! must not see or mutate the rows).
//!
//! The tests are split into one module per table family under `rls_join/`.
//!
//! Run with:
//!   scripts/rls-e2e-gate.sh test --test rls_join_coverage_e2e
//! or against any running registry:
//!   REGISTRY_URL=http://localhost:8080 \
//!   cargo test -p mockforge-registry-server --test rls_join_coverage_e2e -- --ignored --nocapture --test-threads=1

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use hmac::{Hmac, Mac};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{json, Value};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Stripe webhook secret the e2e registry server is started with. Must match
/// `STRIPE_WEBHOOK_SECRET` in `.github/workflows/registry-e2e.yml` and
/// `scripts/rls-e2e-gate.sh` (see `paid_flow_e2e.rs`, same literal).
const DEFAULT_WEBHOOK_SECRET: &str = "whsec_e2e_test_secret";

const PASSWORD: &str = "SecureP@ssw0rd!2024";

fn base_url() -> String {
    std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Per-run unique suffix: timestamp micros plus a process-local counter so two
/// users registered in the same microsecond still get distinct names.
fn unique() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}{}", chrono::Utc::now().timestamp_micros(), n)
}

// ─── HTTP helpers ────────────────────────────────────────────────────────────

/// Thin wrapper around reqwest::Client that tracks auth + org headers.
#[derive(Clone)]
struct E2e {
    client: Client,
    base_url: String,
    access_token: String,
    user_id: String,
    /// Org sent as `X-Organization-Id`; empty means "send no org header"
    /// (the server then falls back to the caller's newest owned org).
    org_id: String,
}

impl E2e {
    /// Same user, different `X-Organization-Id` header.
    fn as_org(&self, org_id: &str) -> E2e {
        E2e {
            org_id: org_id.to_string(),
            ..self.clone()
        }
    }

    /// Same user, no `X-Organization-Id` header at all.
    fn without_org_header(&self) -> E2e {
        self.as_org("")
    }

    fn req(&self, method: Method, path: &str) -> RequestBuilder {
        let rb = self
            .client
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.access_token));
        if self.org_id.is_empty() {
            rb
        } else {
            rb.header("X-Organization-Id", &self.org_id)
        }
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        send(self.req(Method::GET, path)).await
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        send(self.req(Method::POST, path).json(&body)).await
    }

    async fn put(&self, path: &str, body: Value) -> (StatusCode, Value) {
        send(self.req(Method::PUT, path).json(&body)).await
    }

    async fn patch(&self, path: &str, body: Value) -> (StatusCode, Value) {
        send(self.req(Method::PATCH, path).json(&body)).await
    }

    async fn delete(&self, path: &str) -> (StatusCode, Value) {
        send(self.req(Method::DELETE, path)).await
    }
}

/// Send a request and return (status, body). Non-JSON bodies come back as a
/// JSON string so assertion messages always show what the server said.
async fn send(rb: RequestBuilder) -> (StatusCode, Value) {
    let res = rb.send().await.expect("request failed to send");
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    let body = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    };
    (status, body)
}

/// Assert a 2xx and return the body.
#[track_caller]
fn ok(what: &str, (status, body): (StatusCode, Value)) -> Value {
    assert!(status.is_success(), "{what}: expected 2xx, got {status}: {body}");
    body
}

/// Assert a 4xx (the handler's deliberate "not found / other org / forbidden"),
/// never a 2xx (leak) or a 5xx (RLS WITH CHECK or unbound-query failure).
#[track_caller]
fn client_err(what: &str, (status, body): (StatusCode, Value)) -> Value {
    assert!(status.is_client_error(), "{what}: expected 4xx, got {status}: {body}");
    body
}

#[track_caller]
fn expect_status(what: &str, expected: StatusCode, (status, body): (StatusCode, Value)) -> Value {
    assert_eq!(status, expected, "{what}: expected {expected}, got {status}: {body}");
    body
}

fn arr<'a>(what: &str, v: &'a Value) -> &'a Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("{what}: expected a JSON array, got {v}"))
}

fn ids(what: &str, v: &Value) -> Vec<String> {
    arr(what, v)
        .iter()
        .filter_map(|x| x["id"].as_str().map(str::to_string))
        .collect()
}

fn str_field(what: &str, v: &Value, field: &str) -> String {
    v[field]
        .as_str()
        .unwrap_or_else(|| panic!("{what}: missing string field '{field}' in {v}"))
        .to_string()
}

#[track_caller]
fn assert_contains(what: &str, list: &[String], id: &str) {
    assert!(list.iter().any(|x| x == id), "{what}: expected {id} in {list:?}");
}

#[track_caller]
fn assert_absent(what: &str, list: &[String], id: &str) {
    assert!(!list.iter().any(|x| x == id), "{what}: did not expect {id} in {list:?}");
}

// ─── Setup: users, orgs, Team upgrade ────────────────────────────────────────

/// Register a fresh user. The returned E2e has no org header yet.
async fn register_user(prefix: &str) -> (E2e, String) {
    let base_url = base_url();
    let client = Client::new();
    let u = unique();
    let username = format!("{prefix}{u}");
    let email = format!("{prefix}{u}@e2e-test.local");

    let (status, body) = send(
        client
            .post(format!("{base_url}/api/v1/auth/register"))
            .json(&json!({ "username": username, "email": email, "password": PASSWORD })),
    )
    .await;
    assert!(status.is_success(), "register {status}: {body}");
    let access_token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();
    let user_id = str_field("register", &body, "user_id");

    (
        E2e {
            client,
            base_url,
            access_token,
            user_id,
            org_id: String::new(),
        },
        email,
    )
}

/// Create an org owned by `e`'s user (always provisioned Free, #733).
async fn create_org(e: &E2e, prefix: &str) -> String {
    let u = unique();
    let body = ok(
        "create org",
        e.post(
            "/api/v1/organizations",
            json!({ "name": format!("RLS coverage {prefix} {u}"), "slug": format!("{prefix}-{u}") }),
        )
        .await,
    );
    str_field("create org", &body, "id")
}

/// Register a user, create an org, and upgrade it to Team through the Stripe
/// webhook. The returned E2e sends that org as `X-Organization-Id`.
async fn setup_team_org(prefix: &str) -> E2e {
    let (e, _) = register_user(prefix).await;
    let org_id = create_org(&e, prefix).await;
    upgrade_org_to_team(&e.client, &e.base_url, &org_id).await;
    e.as_org(&org_id)
}

/// Register a user and create a Free org for them.
async fn setup_free_org(prefix: &str) -> E2e {
    let (e, _) = register_user(prefix).await;
    let org_id = create_org(&e, prefix).await;
    e.as_org(&org_id)
}

/// Sign a Stripe webhook body with the shared e2e secret, matching what
/// `stripe::Webhook::construct_event` verifies: `t=<ts>,v1=<hex hmac-sha256>`.
fn sign_webhook(secret: &str, payload: &str, timestamp: i64) -> String {
    let signed = format!("{}.{}", timestamp, payload);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts arbitrary key length");
    mac.update(signed.as_bytes());
    format!("t={},v1={}", timestamp, hex::encode(mac.finalize().into_bytes()))
}

/// POST a Stripe-signed `customer.subscription.created` (Team) webhook so the
/// server flips the org's plan to Team. Mirrors `workspace_content_e2e.rs`.
async fn upgrade_org_to_team(client: &Client, base_url: &str, org_id: &str) {
    let secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .unwrap_or_else(|_| DEFAULT_WEBHOOK_SECRET.to_string());
    let now = chrono::Utc::now().timestamp();
    let tag = unique();
    let body = json!({
        "id": format!("evt_rlsc_{}", uuid::Uuid::new_v4()),
        "object": "event",
        "api_version": "2024-04-10",
        "created": now,
        "type": "customer.subscription.created",
        "livemode": false,
        "pending_webhooks": 0,
        "request": { "id": null, "idempotency_key": null },
        "data": {
            "object": {
                "id": format!("sub_rlsc_{}", tag),
                "object": "subscription",
                "customer": format!("cus_rlsc_{}", tag),
                "status": "active",
                "livemode": false,
                "automatic_tax": { "enabled": false },
                "current_period_start": now,
                "current_period_end": now + 30 * 86400,
                "cancel_at_period_end": false,
                "canceled_at": Value::Null,
                "created": now,
                "start_date": now,
                "billing_cycle_anchor": now,
                "collection_method": "charge_automatically",
                "currency": "usd",
                "items": {
                    "object": "list",
                    "data": [{
                        "id": format!("si_rlsc_{}", uuid::Uuid::new_v4()),
                        "object": "subscription_item",
                        "price": {
                            "id": "price_test_team",
                            "object": "price",
                            "active": true,
                            "currency": "usd",
                            "product": "prod_rlsc",
                            "type": "recurring",
                        },
                        "quantity": 1,
                        "subscription": format!("sub_rlsc_{}", tag),
                    }],
                    "has_more": false,
                    "url": "/v1/subscription_items"
                },
                "metadata": { "org_id": org_id },
            }
        }
    })
    .to_string();

    let sig = sign_webhook(&secret, &body, now);
    let res = client
        .post(format!("{}/api/v1/billing/webhook", base_url))
        .header("stripe-signature", sig)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .expect("team-upgrade webhook POST failed");
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    assert!(status.is_success(), "team-upgrade webhook returned {}: {}", status, text);
}

async fn list_ids(e: &E2e, what: &str, path: &str) -> Vec<String> {
    let body = ok(what, e.get(path).await);
    ids(what, &body)
}

/// Create a cloud workspace in `e`'s header org and return its id.
async fn create_workspace(e: &E2e, name: &str) -> String {
    let body = ok(
        "create workspace",
        e.post("/api/v1/workspaces", json!({ "name": name, "description": "rls join e2e" }))
            .await,
    );
    str_field("create workspace", &body, "id")
}

#[path = "rls_join/workspace_content.rs"]
mod workspace_content;

#[path = "rls_join/flows_snapshots_chaos.rs"]
mod flows_snapshots_chaos;

#[path = "rls_join/verification_and_tests.rs"]
mod verification_and_tests;

#[path = "rls_join/hosted_mock_runtime.rs"]
mod hosted_mock_runtime;

#[path = "rls_join/user_keys_and_federation.rs"]
mod user_keys_and_federation;
