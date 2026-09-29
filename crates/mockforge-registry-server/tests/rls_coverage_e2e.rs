//! End-to-end coverage for the HTTP surfaces backed by the tables that became
//! Postgres-RLS-forced in migration `20250101000084_rls_tenant_isolation_coverage.sql`
//! (issue #1087).
//!
//! Under `scripts/rls-e2e-gate.sh` the registry server runs its request-path
//! queries on a `NOBYPASSRLS` role, so any handler that queries a covered table
//! without binding the right org fail-closes: an empty list right after a
//! create, a 404/400 on a row that exists, or a 500 from a `WITH CHECK`
//! violation on insert/update. Each test here does a real round trip
//! (create -> list -> get/update -> delete -> list) so every one of those
//! failure modes is observable, plus two isolation tests:
//!
//!   * MULTI-ORG: one user owning two orgs; handlers must bind the PATH org,
//!     not the request's default/header org.
//!   * CROSS-TENANT: a second user must not see or mutate the first user's rows.
//!
//! Requires:
//!   - PostgreSQL + MinIO running, registry server running (see
//!     signup_flow_e2e.rs header), started with the e2e Stripe webhook secret.
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   cargo test -p mockforge-registry-server --test rls_coverage_e2e -- --ignored --nocapture --test-threads=1
//!
//! or against the RLS-active stack:
//!   scripts/rls-e2e-gate.sh test --test rls_coverage_e2e

use std::sync::atomic::{AtomicU64, Ordering};

use base64::Engine;
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

/// 32 random bytes, standard base64 — a syntactically valid Ed25519 public key
/// as far as the trust-root handler is concerned (it only checks length).
fn random_ed25519_pubkey_b64() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Lowercase alnum subdomain, 3-40 chars, unique per call.
fn unique_subdomain() -> String {
    format!("rlsc{}", unique())
}

// ─── Resource creators shared by several tests ───────────────────────────────

async fn create_channel(e: &E2e, org: &str, name: &str) -> String {
    let body = ok(
        "create notification channel",
        e.post(
            &format!("/api/v1/organizations/{org}/notification-channels"),
            json!({
                "name": name,
                "kind": "email",
                "config": { "recipients": ["ops@e2e-test.local"] },
            }),
        )
        .await,
    );
    assert_eq!(body["org_id"], org, "channel org_id: {body}");
    assert_eq!(body["name"], name, "channel name: {body}");
    assert_eq!(body["kind"], "email", "channel kind: {body}");
    assert_eq!(body["enabled"], true, "channel enabled default: {body}");
    str_field("create channel", &body, "id")
}

async fn create_rule(e: &E2e, org: &str, channel_id: &str) -> String {
    let body = ok(
        "create routing rule",
        e.post(
            &format!("/api/v1/organizations/{org}/routing-rules"),
            json!({
                "priority": 10,
                "match_severity": ["critical", "high"],
                "match_source": ["e2e"],
                "channel_ids": [channel_id],
            }),
        )
        .await,
    );
    assert_eq!(body["org_id"], org, "rule org_id: {body}");
    assert_eq!(body["priority"], 10, "rule priority: {body}");
    assert_eq!(body["channel_ids"], json!([channel_id]), "rule channel_ids: {body}");
    str_field("create rule", &body, "id")
}

async fn list_ids(e: &E2e, what: &str, path: &str) -> Vec<String> {
    let body = ok(what, e.get(path).await);
    ids(what, &body)
}

// ─── 1. Notification channels + routing rules ────────────────────────────────

#[tokio::test]
#[ignore]
async fn notification_channels_and_routing_rules_round_trip() {
    let e = setup_team_org("rlsnc").await;
    let org = e.org_id.clone();
    let channels = format!("/api/v1/organizations/{org}/notification-channels");
    let rules = format!("/api/v1/organizations/{org}/routing-rules");

    // Fresh org: nothing configured.
    assert!(list_ids(&e, "list channels (empty)", &channels).await.is_empty());
    assert!(list_ids(&e, "list rules (empty)", &rules).await.is_empty());

    // Validation still runs before any DB access.
    expect_status(
        "create channel with bad kind",
        StatusCode::BAD_REQUEST,
        e.post(&channels, json!({ "name": "x", "kind": "sms", "config": {} })).await,
    );

    let ch = create_channel(&e, &org, "ops-email").await;
    assert_contains(
        "list channels after create",
        &list_ids(&e, "list channels", &channels).await,
        &ch,
    );

    let body = ok(
        "update channel",
        e.patch(
            &format!("{channels}/{ch}"),
            json!({ "name": "ops-email-renamed", "enabled": false }),
        )
        .await,
    );
    assert_eq!(body["id"], ch.as_str());
    assert_eq!(body["name"], "ops-email-renamed");
    assert_eq!(body["enabled"], false);

    // Routing rule referencing the channel.
    let rule = create_rule(&e, &org, &ch).await;
    assert_contains("list rules after create", &list_ids(&e, "list rules", &rules).await, &rule);

    let body = ok(
        "update rule",
        e.patch(&format!("{rules}/{rule}"), json!({ "priority": 5 })).await,
    );
    assert_eq!(body["id"], rule.as_str());
    assert_eq!(body["priority"], 5);
    assert_eq!(body["match_severity"], json!(["critical", "high"]));

    // The list reflects the update.
    let listed = ok("list rules after update", e.get(&rules).await);
    let row = arr("rules", &listed).iter().find(|r| r["id"] == rule.as_str()).cloned();
    assert_eq!(row.expect("rule missing from list")["priority"], 5);

    // Delete rule, then channel.
    let body = ok("delete rule", e.delete(&format!("{rules}/{rule}")).await);
    assert_eq!(body["deleted"], true);
    assert_absent("list rules after delete", &list_ids(&e, "list rules", &rules).await, &rule);
    client_err("delete rule twice", e.delete(&format!("{rules}/{rule}")).await);

    let body = ok("delete channel", e.delete(&format!("{channels}/{ch}")).await);
    assert_eq!(body["deleted"], true);
    assert_absent(
        "list channels after delete",
        &list_ids(&e, "list channels", &channels).await,
        &ch,
    );
    client_err(
        "update deleted channel",
        e.patch(&format!("{channels}/{ch}"), json!({ "name": "ghost" })).await,
    );
}

// ─── 2. Observability saved queries + dashboards ─────────────────────────────

#[tokio::test]
#[ignore]
async fn observability_saved_queries_and_dashboards_round_trip() {
    let e = setup_team_org("rlsob").await;
    let org = e.org_id.clone();
    let queries = format!("/api/v1/organizations/{org}/observability/saved-queries");
    let dashboards = format!("/api/v1/organizations/{org}/observability/dashboards");

    // Saved query.
    let body = ok(
        "create saved query",
        e.post(
            &queries,
            json!({
                "name": "5xx last hour",
                "description": "e2e",
                "kind": "logs",
                "filters": { "kind": "request_count", "status": 500 },
            }),
        )
        .await,
    );
    let q = str_field("create saved query", &body, "id");
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["kind"], "logs");
    assert_eq!(body["created_by"], e.user_id.as_str());

    assert_contains("list queries", &list_ids(&e, "list queries", &queries).await, &q);
    assert_contains(
        "list queries ?kind=logs",
        &list_ids(&e, "list queries kind", &format!("{queries}?kind=logs")).await,
        &q,
    );
    assert_absent(
        "list queries ?kind=traces",
        &list_ids(&e, "list queries kind", &format!("{queries}?kind=traces")).await,
        &q,
    );

    let body = ok(
        "update saved query",
        e.patch(
            &format!("/api/v1/observability/saved-queries/{q}"),
            json!({ "name": "renamed" }),
        )
        .await,
    );
    assert_eq!(body["name"], "renamed");

    let body = ok(
        "delete saved query",
        e.delete(&format!("/api/v1/observability/saved-queries/{q}")).await,
    );
    assert_eq!(body["deleted"], true);
    assert_absent("list queries after delete", &list_ids(&e, "list queries", &queries).await, &q);

    // Dashboard.
    let body = ok(
        "create dashboard",
        e.post(&dashboards, json!({ "name": "Ops", "layout": { "cols": 12 }, "queries": [] }))
            .await,
    );
    let d = str_field("create dashboard", &body, "id");
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["layout"], json!({ "cols": 12 }));

    assert_contains("list dashboards", &list_ids(&e, "list dashboards", &dashboards).await, &d);

    let body = ok(
        "update dashboard",
        e.patch(
            &format!("/api/v1/observability/dashboards/{d}"),
            json!({ "name": "Ops v2", "layout": { "cols": 6 } }),
        )
        .await,
    );
    assert_eq!(body["name"], "Ops v2");
    assert_eq!(body["layout"], json!({ "cols": 6 }));

    let body = ok(
        "delete dashboard",
        e.delete(&format!("/api/v1/observability/dashboards/{d}")).await,
    );
    assert_eq!(body["deleted"], true);
    assert_absent(
        "list dashboards after delete",
        &list_ids(&e, "list dashboards", &dashboards).await,
        &d,
    );
}

// ─── 3. Org trust roots ──────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn trust_roots_create_list_revoke() {
    let e = setup_team_org("rlstr").await;
    let org = e.org_id.clone();
    let path = format!("/api/v1/organizations/{org}/trust-roots");

    let body = ok("list trust roots (empty)", e.get(&path).await);
    assert_eq!(body["trustRoots"], json!([]), "fresh org trust roots: {body}");

    // Wrong key length is rejected before any insert.
    client_err(
        "create trust root with short key",
        e.post(&path, json!({ "publicKeyB64": "AAAA", "name": "bad" })).await,
    );

    let key = random_ed25519_pubkey_b64();
    let body = ok(
        "create trust root",
        e.post(&path, json!({ "publicKeyB64": key, "name": "release signer" })).await,
    );
    let root = str_field("create trust root", &body, "id");
    assert_eq!(body["orgId"], org.as_str());
    assert_eq!(body["publicKeyB64"], key.as_str());
    assert_eq!(body["active"], true);

    let body = ok("list trust roots", e.get(&path).await);
    let listed = arr("trustRoots", &body["trustRoots"]);
    let row = listed
        .iter()
        .find(|r| r["id"] == root.as_str())
        .expect("root missing from list");
    assert_eq!(row["active"], true);

    let body = ok(
        "revoke trust root",
        e.post(&format!("{path}/{root}/revoke"), json!({ "reason": "e2e rotation" }))
            .await,
    );
    assert_eq!(body["active"], false);
    assert_eq!(body["revokedReason"], "e2e rotation");

    expect_status(
        "revoke trust root twice",
        StatusCode::CONFLICT,
        e.post(&format!("{path}/{root}/revoke"), json!({})).await,
    );

    let body = ok("list trust roots after revoke", e.get(&path).await);
    let row = arr("trustRoots", &body["trustRoots"])
        .iter()
        .find(|r| r["id"] == root.as_str())
        .cloned()
        .expect("revoked root should stay listed");
    assert_eq!(row["active"], false);
}

// ─── 4. Tunnels + cross-org subdomain uniqueness ─────────────────────────────

#[tokio::test]
#[ignore]
async fn tunnels_round_trip_and_global_subdomain_uniqueness() {
    let a = setup_team_org("rlstu").await;
    let b = setup_free_org("rlstb").await;
    let a_list = format!("/api/v1/organizations/{}/tunnels", a.org_id);
    let b_list = format!("/api/v1/organizations/{}/tunnels", b.org_id);
    let sub = unique_subdomain();

    let body = ok(
        "A create tunnel",
        a.post(&a_list, json!({ "name": "dev", "subdomain": sub })).await,
    );
    let t = str_field("A create tunnel", &body, "id");
    assert_eq!(body["org_id"], a.org_id.as_str());
    assert_eq!(body["subdomain"], sub.as_str());
    assert_eq!(body["custom_domain_verified"], false);

    assert_contains("A list tunnels", &list_ids(&a, "A list tunnels", &a_list).await, &t);
    let body = ok("A get tunnel", a.get(&format!("/api/v1/tunnels/{t}")).await);
    assert_eq!(body["subdomain"], sub.as_str());

    let body = ok(
        "A update tunnel",
        a.patch(&format!("/api/v1/tunnels/{t}"), json!({ "name": "dev-renamed" })).await,
    );
    assert_eq!(body["name"], "dev-renamed");

    // Same subdomain from a different org is rejected. Subdomains are globally
    // unique, so the pre-check must see org A's row even though it's another
    // tenant's (it runs on the owner pool, outside RLS).
    let body = expect_status(
        "B reserve A's subdomain",
        StatusCode::BAD_REQUEST,
        b.post(&b_list, json!({ "name": "steal", "subdomain": sub })).await,
    );
    assert!(body.to_string().contains("already taken"), "B duplicate subdomain body: {body}");

    // B itself may reserve a tunnel (Free allows 1) — so the rejection above
    // was the uniqueness check, not a plan limit.
    let b_sub = unique_subdomain();
    let body = ok(
        "B create own tunnel",
        b.post(&b_list, json!({ "name": "b", "subdomain": b_sub })).await,
    );
    let bt = str_field("B create tunnel", &body, "id");
    assert_eq!(body["org_id"], b.org_id.as_str());
    let b_ids = list_ids(&b, "B list tunnels", &b_list).await;
    assert_contains("B list tunnels", &b_ids, &bt);
    assert_absent("B list tunnels excludes A", &b_ids, &t);
    ok("B delete own tunnel", b.delete(&format!("/api/v1/tunnels/{bt}")).await);

    // A deletes; list and get no longer see it.
    let body = ok("A delete tunnel", a.delete(&format!("/api/v1/tunnels/{t}")).await);
    assert_eq!(body["deleted"], true);
    assert_absent("A list after delete", &list_ids(&a, "A list tunnels", &a_list).await, &t);
    client_err("A get deleted tunnel", a.get(&format!("/api/v1/tunnels/{t}")).await);

    // The freed subdomain is claimable by another org again.
    let body = ok(
        "B reserve freed subdomain",
        b.post(&b_list, json!({ "name": "reuse", "subdomain": sub })).await,
    );
    let bt2 = str_field("B reserve freed subdomain", &body, "id");
    ok("B cleanup", b.delete(&format!("/api/v1/tunnels/{bt2}")).await);
}

// ─── 5. Org templates ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn org_templates_round_trip() {
    let e = setup_team_org("rlsot").await;
    let base = format!("/api/v1/organizations/{}/templates", e.org_id);

    let body = ok("list templates (empty)", e.get(&base).await);
    assert_eq!(body["templates"], json!([]));

    let body = ok(
        "create template",
        e.post(
            &base,
            json!({
                "name": "Default blueprint",
                "description": "e2e",
                "blueprint_config": { "personas": ["admin"] },
                "security_baseline": { "rbac": "strict" },
                "is_default": false,
            }),
        )
        .await,
    );
    let t = str_field("create template", &body, "id");
    assert_eq!(body["org_id"], e.org_id.as_str());
    assert_eq!(body["blueprint_config"], json!({ "personas": ["admin"] }));

    let body = ok("list templates", e.get(&base).await);
    assert_contains("list templates", &ids("templates", &body["templates"]), &t);

    let body = ok("get template", e.get(&format!("{base}/{t}")).await);
    assert_eq!(body["name"], "Default blueprint");

    let body = ok(
        "update template",
        e.patch(&format!("{base}/{t}"), json!({ "description": "updated" })).await,
    );
    assert_eq!(body["description"], "updated");

    let body = ok("delete template", e.delete(&format!("{base}/{t}")).await);
    assert_eq!(body["success"], true);
    let body = ok("list templates after delete", e.get(&base).await);
    assert_absent("list templates after delete", &ids("templates", &body["templates"]), &t);
    client_err("get deleted template", e.get(&format!("{base}/{t}")).await);
}

// ─── 6. Federations ──────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn federations_round_trip() {
    let e = setup_team_org("rlsfe").await;

    let services = json!([
        {
            "name": "auth",
            "workspace_id": uuid::Uuid::new_v4().to_string(),
            "base_path": "/auth",
            "reality_level": "real",
            "dependencies": [],
        },
        {
            "name": "orders",
            "workspace_id": uuid::Uuid::new_v4().to_string(),
            "base_path": "/orders",
            "reality_level": "real",
            "dependencies": ["auth"],
        }
    ]);
    let body = ok(
        "create federation",
        e.post(
            "/api/v1/federation",
            json!({ "name": "shop", "description": "e2e", "services": services }),
        )
        .await,
    );
    let f = str_field("create federation", &body, "id");
    assert_eq!(body["org_id"], e.org_id.as_str());
    assert_eq!(arr("services", &body["services"]).len(), 2);

    assert_contains(
        "list federations",
        &list_ids(&e, "list federations", "/api/v1/federation").await,
        &f,
    );

    let body = ok("get federation", e.get(&format!("/api/v1/federation/{f}")).await);
    assert_eq!(body["name"], "shop");

    let body = ok(
        "update federation",
        e.patch(&format!("/api/v1/federation/{f}"), json!({ "description": "updated" }))
            .await,
    );
    assert_eq!(body["description"], "updated");

    let body = ok("delete federation", e.delete(&format!("/api/v1/federation/{f}")).await);
    assert_eq!(body["success"], true);
    assert_absent(
        "list federations after delete",
        &list_ids(&e, "list federations", "/api/v1/federation").await,
        &f,
    );
    client_err("get deleted federation", e.get(&format!("/api/v1/federation/{f}")).await);
}

// ─── 7. Cloud workspaces + services + fixtures ───────────────────────────────

#[tokio::test]
#[ignore]
async fn cloud_workspaces_services_fixtures_round_trip() {
    let e = setup_team_org("rlsws").await;

    // Workspace.
    let body = ok(
        "create workspace",
        e.post("/api/v1/workspaces", json!({ "name": "rls-ws", "description": "e2e" }))
            .await,
    );
    let ws = str_field("create workspace", &body, "id");
    assert_eq!(body["name"], "rls-ws");
    assert_contains(
        "list workspaces",
        &list_ids(&e, "list workspaces", "/api/v1/workspaces").await,
        &ws,
    );

    let body = ok("get workspace", e.get(&format!("/api/v1/workspaces/{ws}")).await);
    assert_eq!(body["workspace"]["summary"]["id"], ws.as_str());

    let body = ok(
        "update workspace",
        e.patch(&format!("/api/v1/workspaces/{ws}"), json!({ "name": "rls-ws-renamed" }))
            .await,
    );
    assert_eq!(body["name"], "rls-ws-renamed");

    // Service in that workspace.
    let body = ok(
        "create service",
        e.post(
            "/api/v1/services",
            json!({
                "name": "users-api",
                "description": "e2e",
                "base_url": "http://users.e2e-test.local",
                "workspace_id": ws,
            }),
        )
        .await,
    );
    let svc = str_field("create service", &body, "id");
    assert_eq!(body["org_id"], e.org_id.as_str());
    assert_eq!(body["workspace_id"], ws.as_str());

    assert_contains(
        "list services",
        &list_ids(&e, "list services", "/api/v1/services").await,
        &svc,
    );
    assert_contains(
        "list services by workspace",
        &list_ids(&e, "list services ws", &format!("/api/v1/services?workspace_id={ws}")).await,
        &svc,
    );
    let body = ok("get service", e.get(&format!("/api/v1/services/{svc}")).await);
    assert_eq!(body["name"], "users-api");

    let body = ok(
        "update service",
        e.patch(&format!("/api/v1/services/{svc}"), json!({ "enabled": false })).await,
    );
    assert_eq!(body["enabled"], false);

    // Fixture in that workspace.
    let body = ok(
        "create fixture",
        e.post(
            "/api/v1/fixtures",
            json!({
                "name": "list-users",
                "path": "/users",
                "method": "GET",
                "content": { "users": [] },
                "workspace_id": ws,
            }),
        )
        .await,
    );
    let fx = str_field("create fixture", &body, "id");
    assert_eq!(body["org_id"], e.org_id.as_str());
    assert_eq!(body["workspace_id"], ws.as_str());

    assert_contains("list fixtures", &list_ids(&e, "list fixtures", "/api/v1/fixtures").await, &fx);
    assert_contains(
        "list fixtures by workspace",
        &list_ids(&e, "list fixtures ws", &format!("/api/v1/fixtures?workspace_id={ws}")).await,
        &fx,
    );
    let body = ok("get fixture", e.get(&format!("/api/v1/fixtures/{fx}")).await);
    assert_eq!(body["path"], "/users");

    let body = ok(
        "update fixture",
        e.patch(&format!("/api/v1/fixtures/{fx}"), json!({ "description": "updated" }))
            .await,
    );
    assert_eq!(body["description"], "updated");

    // Tear down in dependency order.
    ok("delete fixture", e.delete(&format!("/api/v1/fixtures/{fx}")).await);
    assert_absent(
        "list fixtures after delete",
        &list_ids(&e, "list fixtures", "/api/v1/fixtures").await,
        &fx,
    );
    client_err("get deleted fixture", e.get(&format!("/api/v1/fixtures/{fx}")).await);

    ok("delete service", e.delete(&format!("/api/v1/services/{svc}")).await);
    assert_absent(
        "list services after delete",
        &list_ids(&e, "list services", "/api/v1/services").await,
        &svc,
    );
    client_err("get deleted service", e.get(&format!("/api/v1/services/{svc}")).await);

    let body = ok("delete workspace", e.delete(&format!("/api/v1/workspaces/{ws}")).await);
    assert_eq!(body["success"], true);
    assert_absent(
        "list workspaces after delete",
        &list_ids(&e, "list workspaces", "/api/v1/workspaces").await,
        &ws,
    );
    client_err("get deleted workspace", e.get(&format!("/api/v1/workspaces/{ws}")).await);
}

// ─── 8. Org settings / AI settings / quota / usage / billing ─────────────────

#[tokio::test]
#[ignore]
async fn org_settings_quota_usage_and_billing() {
    let e = setup_team_org("rlsos").await;
    let org = e.org_id.clone();
    let base = format!("/api/v1/organizations/{org}");

    let body = ok("get organization", e.get(&base).await);
    assert_eq!(body["plan"], "team");

    let body = ok("get settings", e.get(&format!("{base}/settings")).await);
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["plan"], "team");
    assert_eq!(body["byok_enabled"], false);

    // AI settings live in org_settings — a write + read-back round trip.
    // Unset settings come back as `OrgAiSettings::default()`; only the shape is
    // asserted here, the value comes from the write below.
    let body = ok("get ai settings (default)", e.get(&format!("{base}/settings/ai")).await);
    assert!(
        body["max_ai_calls_per_workspace_per_day"].is_u64(),
        "default ai settings: {body}"
    );
    assert_ne!(body["max_ai_calls_per_workspace_per_day"], 1234, "default ai settings: {body}");
    let body = ok(
        "update ai settings",
        e.patch(
            &format!("{base}/settings/ai"),
            json!({
                "max_ai_calls_per_workspace_per_day": 1234,
                "max_ai_calls_per_workspace_per_month": 5678,
            }),
        )
        .await,
    );
    assert_eq!(body["max_ai_calls_per_workspace_per_day"], 1234);
    let body = ok("get ai settings (updated)", e.get(&format!("{base}/settings/ai")).await);
    assert_eq!(body["max_ai_calls_per_workspace_per_day"], 1234);
    assert_eq!(body["max_ai_calls_per_workspace_per_month"], 5678);

    // Quota overrides (also org_settings): members read, only platform admins
    // write (an org raising its own quota would lift its plan limits).
    let body = ok("get quota (empty)", e.get(&format!("{base}/quota")).await);
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["quota"], json!({}));
    client_err(
        "owner set own quota",
        e.put(&format!("{base}/quota"), json!({ "e2e_marker": 7 })).await,
    );
    let body = ok("get quota (unchanged)", e.get(&format!("{base}/quota")).await);
    assert_eq!(body["quota"], json!({}));

    let body = ok("get org usage", e.get(&format!("{base}/usage")).await);
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["api_tokens_count"], 0);
    assert_eq!(body["hosted_mocks_count"], 0);

    let body = ok("get org billing", e.get(&format!("{base}/billing")).await);
    assert_eq!(body["plan"], "team");
    assert_eq!(
        body["subscription"]["status"], "active",
        "billing should show the webhook-created subscription: {body}"
    );
}

// ─── 9. API tokens + token auth + suspicious activity ────────────────────────

#[tokio::test]
#[ignore]
async fn api_tokens_round_trip_and_token_authentication() {
    let e = setup_team_org("rlsat").await;
    let org = e.org_id.clone();

    let body = ok(
        "create token",
        e.post(
            "/api/v1/tokens",
            json!({ "name": "ci", "scopes": ["read:packages", "read:usage"] }),
        )
        .await,
    );
    let token = str_field("create token", &body, "token");
    let token_id = str_field("create token", &body, "token_id");
    assert!(token.starts_with("mfx_"), "token prefix: {token}");
    assert_eq!(body["scopes"], json!(["read:packages", "read:usage"]));

    assert_contains("list tokens", &list_ids(&e, "list tokens", "/api/v1/tokens").await, &token_id);

    // Authenticate with the API token itself (pre-auth token lookup path).
    let t = E2e {
        access_token: token.clone(),
        ..e.clone()
    };
    assert_contains(
        "list tokens via API token",
        &list_ids(&t, "list tokens via API token", "/api/v1/tokens").await,
        &token_id,
    );
    let body = ok(
        "org usage via API token",
        t.get(&format!("/api/v1/organizations/{org}/usage")).await,
    );
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["api_tokens_count"], 1);

    // Five more in the same hour trips RapidApiTokenCreation on the sixth.
    let mut all = vec![token_id.clone()];
    for i in 0..5 {
        let body = ok(
            "create extra token",
            e.post(
                "/api/v1/tokens",
                json!({ "name": format!("burst-{i}"), "scopes": ["read:packages"] }),
            )
            .await,
        );
        all.push(str_field("create extra token", &body, "token_id"));
    }
    let listed = list_ids(&e, "list tokens", "/api/v1/tokens").await;
    for id in &all {
        assert_contains("list tokens after burst", &listed, id);
    }

    let body = ok(
        "list suspicious activities",
        e.get("/api/v1/security/suspicious-activities").await,
    );
    let acts = arr("activities", &body["activities"]);
    assert!(
        acts.iter().any(|a| a["activity_type"]
            .as_str()
            .unwrap_or("")
            .contains("RapidApiTokenCreation")
            && a["org_id"] == org.as_str()),
        "expected a RapidApiTokenCreation activity for org {org}: {body}"
    );
    assert!(body["total"].as_i64().unwrap_or(0) >= 1, "suspicious total: {body}");

    // Delete everything; the deleted API token stops authenticating.
    for id in &all {
        let body = ok("delete token", e.delete(&format!("/api/v1/tokens/{id}")).await);
        assert_eq!(body["success"], true);
    }
    let listed = list_ids(&e, "list tokens after delete", "/api/v1/tokens").await;
    for id in &all {
        assert_absent("list tokens after delete", &listed, id);
    }
    expect_status(
        "deleted API token no longer authenticates",
        StatusCode::UNAUTHORIZED,
        t.get("/api/v1/tokens").await,
    );
}

// ─── 10. Incidents, usage, test runs, billing subscription ───────────────────

#[tokio::test]
#[ignore]
async fn incidents_usage_test_runs_and_subscription() {
    let e = setup_team_org("rlsin").await;
    let org = e.org_id.clone();
    let incidents = format!("/api/v1/organizations/{org}/incidents");

    assert!(list_ids(&e, "list incidents (empty)", &incidents).await.is_empty());

    let body = ok(
        "raise incident",
        e.post(
            &incidents,
            json!({
                "source": "e2e",
                "dedupe_key": format!("rlsc-{}", unique()),
                "severity": "high",
                "title": "Checkout 5xx spike",
            }),
        )
        .await,
    );
    let inc = str_field("raise incident", &body, "id");
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["status"], "open");

    assert_contains("list incidents", &list_ids(&e, "list incidents", &incidents).await, &inc);
    assert_contains(
        "list open incidents",
        &list_ids(&e, "list incidents", &format!("{incidents}?status=open")).await,
        &inc,
    );
    let body = ok("get incident", e.get(&format!("/api/v1/incidents/{inc}")).await);
    assert_eq!(body["severity"], "high");

    let body = ok("incident stats (open)", e.get(&format!("{incidents}/stats")).await);
    assert_eq!(body["open"]["high"], 1, "stats: {body}");
    assert_eq!(body["open"]["total"], 1, "stats: {body}");

    let body = ok(
        "acknowledge",
        e.post(&format!("/api/v1/incidents/{inc}/acknowledge"), json!({})).await,
    );
    assert_eq!(body["status"], "acknowledged");
    let body = ok("resolve", e.post(&format!("/api/v1/incidents/{inc}/resolve"), json!({})).await);
    assert_eq!(body["status"], "resolved");
    let resolved = list_ids(&e, "list resolved", &format!("{incidents}?status=resolved")).await;
    assert_contains("list resolved incidents", &resolved, &inc);
    // Stats once a resolved incident exists: see incident_stats_with_resolved_incident.

    let body = ok("incident events", e.get(&format!("/api/v1/incidents/{inc}/events")).await);
    assert!(
        !arr("events", &body).is_empty(),
        "incident should have lifecycle events: {body}"
    );

    // Usage: /usage get-or-creates this period's usage_counters row (a write
    // under RLS), then history must show it.
    let body = ok("get usage", e.get("/api/v1/usage").await);
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["plan"], "team");
    assert_eq!(body["usage"]["requests"]["limit"], 1_000_000);
    let body = ok("usage history", e.get("/api/v1/usage/history").await);
    assert_eq!(body["org_id"], org.as_str());
    assert!(!arr("history", &body["history"]).is_empty(), "usage history: {body}");
    let body = ok("usage alerts", e.get("/api/v1/usage/alerts").await);
    assert_eq!(body["org_id"], org.as_str());
    arr("alerts", &body["alerts"]);

    // Test runs for the org (none triggered — the list must still succeed).
    let body = ok(
        "list org test runs",
        e.get(&format!("/api/v1/organizations/{org}/test-runs")).await,
    );
    assert!(arr("test runs", &body).is_empty(), "test runs: {body}");

    // Billing subscription reflects the Team subscription the webhook created.
    let body = ok("billing subscription", e.get("/api/v1/billing/subscription").await);
    assert_eq!(body["org_id"], org.as_str());
    assert_eq!(body["plan"], "team");
    assert_eq!(body["status"], "active", "subscription: {body}");
    assert!(body["current_period_end"].is_string(), "subscription period: {body}");
}

/// `GET /organizations/{org}/incidents/stats` once the org has a resolved
/// incident (the MTTR aggregate only has rows to average then). Separate from
/// the round trip above so a failure here names the stats endpoint directly.
#[tokio::test]
#[ignore]
async fn incident_stats_with_resolved_incident() {
    let e = setup_team_org("rlsis").await;
    let incidents = format!("/api/v1/organizations/{}/incidents", e.org_id);
    let body = ok(
        "raise incident",
        e.post(
            &incidents,
            json!({ "source": "e2e", "dedupe_key": unique(), "severity": "critical", "title": "x" }),
        )
        .await,
    );
    let inc = str_field("raise incident", &body, "id");
    ok("resolve", e.post(&format!("/api/v1/incidents/{inc}/resolve"), json!({})).await);

    let body = ok("incident stats (resolved)", e.get(&format!("{incidents}/stats")).await);
    assert_eq!(body["open"]["total"], 0, "stats: {body}");
    assert_eq!(body["resolved_30d"]["critical"], 1, "stats: {body}");
    assert!(body["mttr_seconds_30d"].is_i64(), "stats: {body}");
}

// ─── 11. SSO config (SAML, no IdP needed to store it) ────────────────────────

#[tokio::test]
#[ignore]
async fn sso_saml_config_round_trip() {
    let e = setup_team_org("rlssso").await;
    let b = setup_free_org("rlsssb").await;

    let body = ok("get sso config (none)", e.get("/api/v1/sso/config").await);
    assert!(body.is_null(), "fresh org sso config: {body}");

    let body = ok(
        "create saml sso config",
        e.post(
            "/api/v1/sso/config",
            json!({
                "provider": "saml",
                "saml_entity_id": "https://idp.e2e-test.invalid/entity",
                "saml_sso_url": "https://idp.e2e-test.invalid/sso",
                "saml_x509_cert": "MIIBdummyE2eCertificate",
                "email_domain": format!("{}.e2e-test.invalid", unique()),
            }),
        )
        .await,
    );
    assert_eq!(body["org_id"], e.org_id.as_str());
    assert_eq!(body["provider"], "saml");
    let cfg = str_field("create sso", &body, "id");

    let body = ok("get sso config", e.get("/api/v1/sso/config").await);
    assert_eq!(body["id"], cfg.as_str());
    assert_eq!(body["saml_entity_id"], "https://idp.e2e-test.invalid/entity");

    // Another tenant does not see it.
    let body = ok("B get sso config", b.get("/api/v1/sso/config").await);
    assert!(body.is_null(), "B must not see A's SSO config: {body}");

    let body = ok("delete sso config", e.delete("/api/v1/sso/config").await);
    assert_eq!(body["success"], true);
    let body = ok("get sso config after delete", e.get("/api/v1/sso/config").await);
    assert!(body.is_null(), "sso config after delete: {body}");
}

// ─── 12. Org members + invitations ───────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn org_members_and_invitations() {
    let e = setup_team_org("rlsom").await;
    let org = e.org_id.clone();
    let (c, _) = register_user("rlsomc").await;
    let c = c.as_org(&org);

    // Before joining, C cannot read the org.
    client_err("non-member get org", c.get(&format!("/api/v1/organizations/{org}")).await);

    let body = ok(
        "add member",
        e.post(
            &format!("/api/v1/organizations/{org}/members"),
            json!({ "user_id": c.user_id, "role": "member" }),
        )
        .await,
    );
    assert_eq!(body["user_id"], c.user_id.as_str());
    assert_eq!(body["role"], "member");

    let body = ok("list members", e.get(&format!("/api/v1/organizations/{org}/members")).await);
    let members = arr("members", &body);
    assert!(
        members
            .iter()
            .any(|m| m["user_id"] == e.user_id.as_str() && m["role"] == "owner"),
        "owner missing from members: {body}"
    );
    assert!(
        members
            .iter()
            .any(|m| m["user_id"] == c.user_id.as_str() && m["role"] == "member"),
        "new member missing from members: {body}"
    );

    // As a member (membership comes from org_members, not ownership), C can
    // read the org and its org-scoped resources.
    ok("member get org", c.get(&format!("/api/v1/organizations/{org}")).await);
    let body = ok(
        "member get settings",
        c.get(&format!("/api/v1/organizations/{org}/settings")).await,
    );
    assert_eq!(body["org_id"], org.as_str());
    ok(
        "member list channels",
        c.get(&format!("/api/v1/organizations/{org}/notification-channels")).await,
    );

    // Invitations (stored as org_settings rows).
    let invites = format!("/api/v1/organizations/{org}/invitations");
    let body = ok(
        "create invitation",
        e.post(
            &invites,
            json!({ "email": format!("inv{}@e2e-test.local", unique()), "role": "member" }),
        )
        .await,
    );
    let nonce = str_field("create invitation", &body, "nonce");
    assert_eq!(body["org_id"], org.as_str());
    let body = ok("list invitations", e.get(&invites).await);
    let nonces: Vec<String> = arr("invitations", &body["invitations"])
        .iter()
        .filter_map(|i| i["nonce"].as_str().map(str::to_string))
        .collect();
    assert_contains("list invitations", &nonces, &nonce);

    // A plain member may not list invitations.
    client_err("member list invitations", c.get(&invites).await);

    let body = ok("revoke invitation", e.delete(&format!("{invites}/{nonce}")).await);
    assert_eq!(body["success"], true);
    let body = ok("list invitations after revoke", e.get(&invites).await);
    assert!(
        !body["invitations"].to_string().contains(&nonce),
        "revoked invitation still listed: {body}"
    );

    // Remove C; membership is gone.
    ok(
        "remove member",
        e.delete(&format!("/api/v1/organizations/{org}/members/{}", c.user_id)).await,
    );
    let body = ok(
        "list members after remove",
        e.get(&format!("/api/v1/organizations/{org}/members")).await,
    );
    assert!(
        !arr("members", &body).iter().any(|m| m["user_id"] == c.user_id.as_str()),
        "removed member still listed: {body}"
    );
    client_err("removed member get org", c.get(&format!("/api/v1/organizations/{org}")).await);
}

// ─── 13. MULTI-ORG: handlers bind the PATH org ───────────────────────────────

/// User A owns two orgs: A1 (Team) and A2 (Free, created second so it is A's
/// newest-owned org — the server's default when no `X-Organization-Id` is sent).
///
/// Two handler families:
///   * path-bound (trust roots, settings, quota, usage, billing, members,
///     invitations): authorize the PATH org directly. Called with the header
///     pointing at the OTHER org, they must still read/write the path org.
///   * header-must-match (`authorize_org`: channels, routing rules, tunnels,
///     incidents, observability, test runs): reject a header/path mismatch with
///     400 by design; with the header set to the path org they must see it.
#[tokio::test]
#[ignore]
async fn multi_org_handlers_bind_the_path_org() {
    let a1 = setup_team_org("rlsmo").await;
    let a2_id = create_org(&a1, "rlsmo2").await;
    let a1_id = a1.org_id.clone();
    let a2 = a1.as_org(&a2_id);
    let bare = a1.without_org_header();

    // ── Path-bound handlers, header = A1, path = A2 ──
    let tr = format!("/api/v1/organizations/{a2_id}/trust-roots");
    let body = ok(
        "A1-header create trust root in A2",
        a1.post(&tr, json!({ "publicKeyB64": random_ed25519_pubkey_b64(), "name": "a2-root" }))
            .await,
    );
    let root = str_field("create a2 root", &body, "id");
    assert_eq!(body["orgId"], a2_id.as_str());
    let body = ok("A1-header list A2 trust roots", a1.get(&tr).await);
    assert_contains("A2 trust roots", &ids("trustRoots", &body["trustRoots"]), &root);
    let body = ok(
        "A1 trust roots",
        a1.get(&format!("/api/v1/organizations/{a1_id}/trust-roots")).await,
    );
    assert_absent("A1 trust roots exclude A2's", &ids("trustRoots", &body["trustRoots"]), &root);

    let a1_ai = format!("/api/v1/organizations/{a1_id}/settings/ai");
    let a1_ai_before = ok("A1 ai settings before", a1.get(&a1_ai).await);
    let ai = format!("/api/v1/organizations/{a2_id}/settings/ai");
    ok(
        "A1-header update A2 ai settings",
        a1.patch(&ai, json!({ "max_ai_calls_per_workspace_per_day": 4242 })).await,
    );
    let body = ok("A1-header get A2 ai settings", a1.get(&ai).await);
    assert_eq!(body["max_ai_calls_per_workspace_per_day"], 4242);
    let body = ok("A1 ai settings untouched", a1.get(&a1_ai).await);
    assert_eq!(body, a1_ai_before, "writing A2's ai settings changed A1's");

    let quota = format!("/api/v1/organizations/{a2_id}/quota");
    let body = ok("A1-header get A2 quota", a1.get(&quota).await);
    assert_eq!(body["org_id"], a2_id.as_str());

    let body = ok(
        "A1-header A2 usage",
        a1.get(&format!("/api/v1/organizations/{a2_id}/usage")).await,
    );
    assert_eq!(body["org_id"], a2_id.as_str());
    let body = ok(
        "A1-header A2 billing",
        a1.get(&format!("/api/v1/organizations/{a2_id}/billing")).await,
    );
    assert_eq!(body["plan"], "free");
    assert!(body["subscription"].is_null(), "A2 has no subscription: {body}");
    let body = ok(
        "A1-header A2 settings",
        a1.get(&format!("/api/v1/organizations/{a2_id}/settings")).await,
    );
    assert_eq!(body["org_id"], a2_id.as_str());
    let body = ok(
        "A1-header A2 members",
        a1.get(&format!("/api/v1/organizations/{a2_id}/members")).await,
    );
    assert!(
        arr("members", &body).iter().any(|m| m["user_id"] == a1.user_id.as_str()),
        "A2 members should list owner: {body}"
    );
    let inv = format!("/api/v1/organizations/{a2_id}/invitations");
    let body = ok(
        "A1-header create A2 invitation",
        a1.post(&inv, json!({ "email": format!("mo{}@e2e-test.local", unique()) }))
            .await,
    );
    let nonce = str_field("A2 invite", &body, "nonce");
    let body = ok("A1-header list A2 invitations", a1.get(&inv).await);
    assert!(body["invitations"].to_string().contains(&nonce), "A2 invitations: {body}");

    // No header at all (default org = A2), path = A1: must return A1's data.
    let body = ok(
        "no-header A1 usage",
        bare.get(&format!("/api/v1/organizations/{a1_id}/usage")).await,
    );
    assert_eq!(body["org_id"], a1_id.as_str());
    let body = ok(
        "no-header A1 billing",
        bare.get(&format!("/api/v1/organizations/{a1_id}/billing")).await,
    );
    assert_eq!(body["plan"], "team");
    assert_eq!(body["subscription"]["status"], "active", "A1 billing: {body}");

    // ── Header-must-match handlers ──
    // Mismatch is rejected (deliberate 400, not a 500 / not a leak).
    let a2_channels = format!("/api/v1/organizations/{a2_id}/notification-channels");
    expect_status(
        "A1-header list A2 channels (mismatch)",
        StatusCode::BAD_REQUEST,
        a1.get(&a2_channels).await,
    );

    // With the header on A2, A2's rows are created and visible.
    let ch = create_channel(&a2, &a2_id, "a2-channel").await;
    let rule = create_rule(&a2, &a2_id, &ch).await;
    assert_contains("A2 channels", &list_ids(&a2, "A2 channels", &a2_channels).await, &ch);
    assert_contains(
        "A2 rules",
        &list_ids(&a2, "A2 rules", &format!("/api/v1/organizations/{a2_id}/routing-rules")).await,
        &rule,
    );

    // No header: default org is A2 (newest owned), so the A2 path is authorized.
    assert_contains(
        "no-header A2 channels",
        &list_ids(&bare, "no-header A2 channels", &a2_channels).await,
        &ch,
    );

    // A1 sees none of A2's rows.
    assert_absent(
        "A1 channels exclude A2's",
        &list_ids(
            &a1,
            "A1 channels",
            &format!("/api/v1/organizations/{a1_id}/notification-channels"),
        )
        .await,
        &ch,
    );
    assert_absent(
        "A1 rules exclude A2's",
        &list_ids(&a1, "A1 rules", &format!("/api/v1/organizations/{a1_id}/routing-rules")).await,
        &rule,
    );
    // A1-header on A1's path cannot touch A2's rows by id.
    client_err(
        "A1 patch A2 rule via A1 path",
        a1.patch(
            &format!("/api/v1/organizations/{a1_id}/routing-rules/{rule}"),
            json!({ "priority": 1 }),
        )
        .await,
    );

    // Header-bound org-default endpoints follow the header.
    let body = ok("A2 usage (header)", a2.get("/api/v1/usage").await);
    assert_eq!(body["org_id"], a2_id.as_str());
    let body = ok("A1 usage (header)", a1.get("/api/v1/usage").await);
    assert_eq!(body["org_id"], a1_id.as_str());

    // Cleanup A2 rows.
    ok(
        "delete A2 rule",
        a2.delete(&format!("/api/v1/organizations/{a2_id}/routing-rules/{rule}")).await,
    );
    ok("delete A2 channel", a2.delete(&format!("{a2_channels}/{ch}")).await);
}

// ─── 14. CROSS-TENANT: user B cannot see or mutate A's rows ──────────────────

#[tokio::test]
#[ignore]
async fn cross_tenant_isolation() {
    let a = setup_team_org("rlsxa").await;
    let b = setup_team_org("rlsxb").await;
    let ao = a.org_id.clone();
    let bo = b.org_id.clone();
    let a_org = format!("/api/v1/organizations/{ao}");
    let b_org = format!("/api/v1/organizations/{bo}");

    // ── A's fixtures ──
    let ch = create_channel(&a, &ao, "a-secret-channel").await;
    let rule = create_rule(&a, &ao, &ch).await;
    let q = str_field(
        "A saved query",
        &ok(
            "A create saved query",
            a.post(
                &format!("{a_org}/observability/saved-queries"),
                json!({ "name": "a-q", "kind": "logs", "filters": {} }),
            )
            .await,
        ),
        "id",
    );
    let d = str_field(
        "A dashboard",
        &ok(
            "A create dashboard",
            a.post(
                &format!("{a_org}/observability/dashboards"),
                json!({ "name": "a-d", "layout": {}, "queries": [] }),
            )
            .await,
        ),
        "id",
    );
    let tun = str_field(
        "A tunnel",
        &ok(
            "A create tunnel",
            a.post(
                &format!("{a_org}/tunnels"),
                json!({ "name": "a-t", "subdomain": unique_subdomain() }),
            )
            .await,
        ),
        "id",
    );
    let tpl = str_field(
        "A template",
        &ok(
            "A create template",
            a.post(&format!("{a_org}/templates"), json!({ "name": "a-tpl" })).await,
        ),
        "id",
    );
    let fed = str_field(
        "A federation",
        &ok(
            "A create federation",
            a.post("/api/v1/federation", json!({ "name": "a-fed", "description": "" }))
                .await,
        ),
        "id",
    );
    let ws = str_field(
        "A workspace",
        &ok(
            "A create workspace",
            a.post("/api/v1/workspaces", json!({ "name": "a-ws" })).await,
        ),
        "id",
    );
    let svc = str_field(
        "A service",
        &ok("A create service", a.post("/api/v1/services", json!({ "name": "a-svc" })).await),
        "id",
    );
    let fx = str_field(
        "A fixture",
        &ok("A create fixture", a.post("/api/v1/fixtures", json!({ "name": "a-fx" })).await),
        "id",
    );
    let inc = str_field(
        "A incident",
        &ok(
            "A raise incident",
            a.post(
                &format!("{a_org}/incidents"),
                json!({ "source": "e2e", "dedupe_key": unique(), "severity": "low", "title": "a-inc" }),
            )
            .await,
        ),
        "id",
    );
    let tok = str_field(
        "A token",
        &ok(
            "A create token",
            a.post("/api/v1/tokens", json!({ "name": "a-tok", "scopes": ["read:packages"] }))
                .await,
        ),
        "token_id",
    );
    let root = str_field(
        "A trust root",
        &ok(
            "A create trust root",
            a.post(
                &format!("{a_org}/trust-roots"),
                json!({ "publicKeyB64": random_ed25519_pubkey_b64(), "name": "a-root" }),
            )
            .await,
        ),
        "id",
    );

    // ── B's own lists never include A's rows ──
    let lists: Vec<(String, &str)> = vec![
        (format!("{b_org}/notification-channels"), ch.as_str()),
        (format!("{b_org}/routing-rules"), rule.as_str()),
        (format!("{b_org}/observability/saved-queries"), q.as_str()),
        (format!("{b_org}/observability/dashboards"), d.as_str()),
        (format!("{b_org}/tunnels"), tun.as_str()),
        (format!("{b_org}/incidents"), inc.as_str()),
        ("/api/v1/federation".to_string(), fed.as_str()),
        ("/api/v1/workspaces".to_string(), ws.as_str()),
        ("/api/v1/services".to_string(), svc.as_str()),
        ("/api/v1/fixtures".to_string(), fx.as_str()),
        ("/api/v1/tokens".to_string(), tok.as_str()),
    ];
    for (path, id) in &lists {
        assert_absent(
            &format!("B list {path}"),
            &list_ids(&b, &format!("B list {path}"), path).await,
            id,
        );
    }
    let body = ok("B list templates", b.get(&format!("{b_org}/templates")).await);
    assert_absent("B templates", &ids("templates", &body["templates"]), &tpl);
    let body = ok("B list trust roots", b.get(&format!("{b_org}/trust-roots")).await);
    assert_absent("B trust roots", &ids("trustRoots", &body["trustRoots"]), &root);

    // ── B cannot read/mutate A's rows by id (own-org path or A's path) ──
    let patch = json!({ "name": "pwned" });
    for path in [
        format!("{b_org}/notification-channels/{ch}"),
        format!("{a_org}/notification-channels/{ch}"),
    ] {
        client_err(&format!("B patch {path}"), b.patch(&path, patch.clone()).await);
        client_err(&format!("B delete {path}"), b.delete(&path).await);
    }
    for path in [
        format!("{b_org}/routing-rules/{rule}"),
        format!("{a_org}/routing-rules/{rule}"),
    ] {
        client_err(&format!("B patch {path}"), b.patch(&path, json!({ "priority": 1 })).await);
        client_err(&format!("B delete {path}"), b.delete(&path).await);
    }
    for path in [
        format!("/api/v1/observability/saved-queries/{q}"),
        format!("/api/v1/observability/dashboards/{d}"),
        format!("/api/v1/tunnels/{tun}"),
        format!("/api/v1/federation/{fed}"),
        format!("/api/v1/workspaces/{ws}"),
        format!("/api/v1/services/{svc}"),
        format!("/api/v1/fixtures/{fx}"),
        format!("{a_org}/templates/{tpl}"),
        format!("{b_org}/templates/{tpl}"),
    ] {
        client_err(&format!("B patch {path}"), b.patch(&path, patch.clone()).await);
    }
    for path in [
        format!("/api/v1/tunnels/{tun}"),
        format!("/api/v1/federation/{fed}"),
        format!("/api/v1/workspaces/{ws}"),
        format!("/api/v1/services/{svc}"),
        format!("/api/v1/fixtures/{fx}"),
        format!("/api/v1/incidents/{inc}"),
        format!("/api/v1/incidents/{inc}/events"),
        format!("{a_org}/templates/{tpl}"),
        format!("{b_org}/templates/{tpl}"),
    ] {
        client_err(&format!("B get {path}"), b.get(&path).await);
    }
    for path in [
        format!("/api/v1/observability/saved-queries/{q}"),
        format!("/api/v1/observability/dashboards/{d}"),
        format!("/api/v1/tunnels/{tun}"),
        format!("/api/v1/federation/{fed}"),
        format!("/api/v1/workspaces/{ws}"),
        format!("/api/v1/services/{svc}"),
        format!("/api/v1/fixtures/{fx}"),
        format!("/api/v1/tokens/{tok}"),
        format!("{a_org}/templates/{tpl}"),
        format!("{b_org}/templates/{tpl}"),
    ] {
        client_err(&format!("B delete {path}"), b.delete(&path).await);
    }
    for path in [
        format!("/api/v1/incidents/{inc}/acknowledge"),
        format!("/api/v1/incidents/{inc}/resolve"),
        format!("{a_org}/trust-roots/{root}/revoke"),
        format!("{b_org}/trust-roots/{root}/revoke"),
    ] {
        client_err(&format!("B post {path}"), b.post(&path, json!({})).await);
    }

    // ── B on A's org paths: every read and write is refused ──
    for sub in [
        "",
        "/members",
        "/invitations",
        "/settings",
        "/settings/ai",
        "/usage",
        "/billing",
        "/trust-roots",
        "/notification-channels",
        "/routing-rules",
        "/observability/saved-queries",
        "/observability/dashboards",
        "/tunnels",
        "/incidents",
        "/incidents/stats",
        "/test-runs",
        "/templates",
    ] {
        client_err(&format!("B get {a_org}{sub}"), b.get(&format!("{a_org}{sub}")).await);
    }
    client_err(
        "B create channel in A",
        b.post(
            &format!("{a_org}/notification-channels"),
            json!({ "name": "x", "kind": "email", "config": {} }),
        )
        .await,
    );
    client_err(
        "B create tunnel in A",
        b.post(
            &format!("{a_org}/tunnels"),
            json!({ "name": "x", "subdomain": unique_subdomain() }),
        )
        .await,
    );
    client_err(
        "B create trust root in A",
        b.post(
            &format!("{a_org}/trust-roots"),
            json!({ "publicKeyB64": random_ed25519_pubkey_b64(), "name": "x" }),
        )
        .await,
    );
    client_err(
        "B create invitation in A",
        b.post(&format!("{a_org}/invitations"), json!({ "email": "x@e2e-test.local" }))
            .await,
    );
    client_err(
        "B update A ai settings",
        b.patch(
            &format!("{a_org}/settings/ai"),
            json!({ "max_ai_calls_per_workspace_per_day": 1 }),
        )
        .await,
    );
    client_err(
        "B add self to A",
        b.post(&format!("{a_org}/members"), json!({ "user_id": b.user_id })).await,
    );
    client_err(
        "B raise incident in A",
        b.post(
            &format!("{a_org}/incidents"),
            json!({ "source": "x", "dedupe_key": "x", "severity": "low", "title": "x" }),
        )
        .await,
    );
    // B sending A's org as its header is refused too (not a member).
    client_err("B with A header list workspaces", b.as_org(&ao).get("/api/v1/workspaces").await);

    // ── A's rows survived every attempt, unmodified ──
    let listed = ok("A list channels", a.get(&format!("{a_org}/notification-channels")).await);
    let row = arr("channels", &listed).iter().find(|c| c["id"] == ch.as_str()).cloned();
    assert_eq!(row.expect("A channel gone")["name"], "a-secret-channel");
    let listed = ok("A list rules", a.get(&format!("{a_org}/routing-rules")).await);
    let row = arr("rules", &listed).iter().find(|r| r["id"] == rule.as_str()).cloned();
    assert_eq!(row.expect("A rule gone")["priority"], 10);
    assert_eq!(
        ok("A get tunnel", a.get(&format!("/api/v1/tunnels/{tun}")).await)["name"],
        "a-t"
    );
    assert_eq!(
        ok("A get federation", a.get(&format!("/api/v1/federation/{fed}")).await)["name"],
        "a-fed"
    );
    assert_eq!(
        ok("A get workspace", a.get(&format!("/api/v1/workspaces/{ws}")).await)["workspace"]
            ["summary"]["name"],
        "a-ws"
    );
    assert_eq!(
        ok("A get service", a.get(&format!("/api/v1/services/{svc}")).await)["name"],
        "a-svc"
    );
    assert_eq!(
        ok("A get fixture", a.get(&format!("/api/v1/fixtures/{fx}")).await)["name"],
        "a-fx"
    );
    assert_eq!(
        ok("A get template", a.get(&format!("{a_org}/templates/{tpl}")).await)["name"],
        "a-tpl"
    );
    assert_eq!(
        ok("A get incident", a.get(&format!("/api/v1/incidents/{inc}")).await)["status"],
        "open"
    );
    assert_contains(
        "A saved queries",
        &list_ids(&a, "A queries", &format!("{a_org}/observability/saved-queries")).await,
        &q,
    );
    assert_contains(
        "A dashboards",
        &list_ids(&a, "A dashboards", &format!("{a_org}/observability/dashboards")).await,
        &d,
    );
    assert_contains("A tokens", &list_ids(&a, "A tokens", "/api/v1/tokens").await, &tok);
    let body = ok("A trust roots", a.get(&format!("{a_org}/trust-roots")).await);
    let row = arr("trustRoots", &body["trustRoots"])
        .iter()
        .find(|r| r["id"] == root.as_str())
        .cloned();
    assert_eq!(row.expect("A trust root gone")["active"], true);
    let body = ok("A members", a.get(&format!("{a_org}/members")).await);
    assert!(
        !arr("members", &body).iter().any(|m| m["user_id"] == b.user_id.as_str()),
        "B must not have joined A: {body}"
    );
}

/// `GET /api/v1/organizations/{org_id}/quota` is restricted to the org's
/// members (and platform admins); `PUT` to platform admins only. Before #1087
/// any authenticated user could read or overwrite any org's quota overrides.
/// Kept separate from `cross_tenant_isolation` so a failure names the endpoint.
#[tokio::test]
#[ignore]
async fn cross_tenant_quota_is_not_readable_or_writable() {
    let a = setup_team_org("rlsqa").await;
    let b = setup_free_org("rlsqb").await;
    let quota = format!("/api/v1/organizations/{}/quota", a.org_id);

    client_err("A (owner) write own quota", a.put(&quota, json!({ "e2e_marker": 1 })).await);

    client_err("B read A quota", b.get(&quota).await);
    client_err("B write A quota", b.put(&quota, json!({ "e2e_marker": 999 })).await);

    let body = ok("A read quota", a.get(&quota).await);
    assert_eq!(body["quota"], json!({}), "A's quota was modified: {body}");
}

/// `POST /api/v1/organizations/{org_id}/mockai/generate-openapi-from-traffic`
/// used to take the path org with no membership check, so any user could
/// summarise another tenant's traffic. It must now reject a foreign org.
#[tokio::test]
#[ignore]
async fn cross_tenant_mockai_traffic_is_rejected() {
    let a = setup_free_org("rlsma").await;
    let b = setup_free_org("rlsmb").await;
    let path = format!("/api/v1/organizations/{}/mockai/generate-openapi-from-traffic", a.org_id);
    client_err("B generate from A's traffic", b.post(&path, json!({})).await);
}

/// The GDPR export must not include `invite:{nonce}` org_settings rows: they
/// carry everything `accept_invitation` needs, so exporting them let a plain
/// member redeem a pending (e.g. admin) invite.
#[tokio::test]
#[ignore]
async fn gdpr_export_omits_invitation_payloads() {
    let e = setup_team_org("rlsgd").await;
    let org = e.org_id.clone();
    ok(
        "create invitation",
        e.post(
            &format!("/api/v1/organizations/{org}/invitations"),
            json!({ "email": format!("inv{}@e2e-test.local", unique()), "role": "admin" }),
        )
        .await,
    );
    let body = ok("gdpr export", e.get("/api/v1/gdpr/export").await);
    let text = body.to_string();
    assert!(!text.contains("\"invite:"), "export leaked an invite payload: {text}");
}
