//! E2E regression suite for two cross-tenant holes in the chaos and capture
//! replay paths.
//!
//! 1. Chaos injection. A campaign's `target_ref` names the hosted mock the
//!    runner toggles chaos on. It was accepted without checking the
//!    deployment belongs to the caller's org, and the internal
//!    `/internal/hosted-mocks/{id}/chaos` proxy resolved ANY deployment on the
//!    owner pool. Org A could fault-inject org B's live mock.
//! 2. Capture-body exfiltration. Adding a member to a capture session took any
//!    `capture_id`, and the internal exchanges endpoint joined
//!    `runtime_captures` by capture id alone, so org A's replay run received
//!    org B's full request/response bodies. The same join cast
//!    `rc.capture_id::uuid` over every row, so one non-UUID id ingested by any
//!    deployment broke replay for everyone (22P02).
//!
//! Needs `REGISTRY_URL`, `DATABASE_URL` (owner URL, fixtures only),
//! `MOCKFORGE_INTERNAL_API_TOKEN` and `JWT_SECRET` (both shared with the
//! server). Runs in the plain Registry E2E job and under
//! `scripts/rls-e2e-gate.sh`.

use std::time::Duration;

use chrono::Utc;
use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

struct Tenant {
    client: Client,
    base_url: String,
    token: String,
    org_id: Uuid,
}

impl Tenant {
    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("X-Organization-Id", self.org_id.to_string())
            .timeout(Duration::from_secs(20))
    }

    async fn send(&self, method: Method, path: &str, body: Value) -> (StatusCode, Value) {
        let res = self
            .request(method.clone(), path)
            .json(&body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("{method} {path}: {e}"));
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    async fn create_workspace(&self) -> Uuid {
        let (status, body) = self
            .send(Method::POST, "/api/v1/workspaces", json!({ "name": "xtenant-ws" }))
            .await;
        assert!(status.is_success(), "create workspace {status}: {body}");
        Uuid::parse_str(body["id"].as_str().expect("no workspace id")).unwrap()
    }
}

fn base_url() -> String {
    std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set")
}

async fn register(tag: &str) -> Tenant {
    let base_url = base_url();
    let client = Client::new();
    let ts = format!("{}{}", tag, Utc::now().timestamp_micros());
    let res = client
        .post(format!("{base_url}/api/v1/auth/register"))
        .json(&json!({
            "username": format!("xt_{ts}"),
            "email": format!("xt_{ts}@e2e-test.local"),
            "password": "SecureP@ssw0rd!2024",
        }))
        .send()
        .await
        .expect("register failed");
    let status = res.status();
    let body: Value = res.json().await.expect("register not JSON");
    assert!(status.is_success(), "register {status}: {body}");
    let token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();

    let res = client
        .post(format!("{base_url}/api/v1/organizations"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "name": format!("XTenant Org {ts}"), "slug": format!("xt-{ts}") }))
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.expect("org not JSON");
    assert!(status.is_success(), "create org {status}: {body}");
    let org_id = Uuid::parse_str(body["id"].as_str().expect("no org id")).unwrap();

    Tenant {
        client,
        base_url,
        token,
        org_id,
    }
}

async fn owner_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("DB connect failed")
}

/// Insert a hosted mock for `org_id`. `deployment_url` points at a port
/// nothing listens on, so a chaos toggle that gets forwarded fails with a
/// recognizable "chaos proxy fetch failed" instead of hitting anything real.
async fn insert_deployment(pool: &PgPool, org_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO hosted_mocks \
           (id, org_id, name, slug, config_json, status, metadata_json, deployment_url) \
         VALUES ($1, $2, 'xtenant-fixture', $3, '{}'::jsonb, 'active', '{}'::jsonb, \
                 'http://127.0.0.1:9')",
    )
    .bind(id)
    .bind(org_id)
    .bind(format!("xt-{}", id.simple()))
    .execute(pool)
    .await
    .expect("insert hosted_mock failed");
    id
}

async fn insert_capture(pool: &PgPool, deployment_id: Uuid, capture_id: &str, secret: &str) {
    sqlx::query(
        "INSERT INTO runtime_captures \
           (deployment_id, capture_id, protocol, occurred_at, method, path, \
            request_headers, request_body, request_body_encoding, \
            response_status_code, response_headers, response_body, response_body_encoding) \
         VALUES ($1, $2, 'http', NOW(), 'GET', '/secret', '{}', $3, 'utf8', \
                 200, '{}', $3, 'utf8')",
    )
    .bind(deployment_id)
    .bind(capture_id)
    .bind(secret)
    .execute(pool)
    .await
    .expect("insert runtime_capture failed");
}

/// Let the org run chaos campaigns (free plans have `max_concurrent_runs: 0`).
async fn allow_runs(pool: &PgPool, org_id: Uuid) {
    sqlx::query(
        "UPDATE organizations \
            SET limits_json = limits_json || '{\"max_concurrent_runs\": 5}'::jsonb \
          WHERE id = $1",
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("raise max_concurrent_runs");
}

fn internal(method: Method, path: &str) -> reqwest::RequestBuilder {
    let token = std::env::var("MOCKFORGE_INTERNAL_API_TOKEN")
        .expect("MOCKFORGE_INTERNAL_API_TOKEN must be set (shared with the server)");
    Client::new()
        .request(method, format!("{}{}", base_url(), path))
        .bearer_auth(token)
        .timeout(Duration::from_secs(20))
}

fn campaign_body(target_ref: &str) -> Value {
    json!({
        "name": "xtenant-campaign",
        "target_kind": "hosted_mock",
        "target_ref": target_ref,
        "config": {},
        "safety_config": {},
    })
}

// ---------------------------------------------------------------------------
// Chaos
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn chaos_campaign_create_rejects_foreign_deployment_target() {
    let pool = owner_pool().await;
    let attacker = register("ca").await;
    let victim = register("cv").await;
    let ws = attacker.create_workspace().await;
    let own_dep = insert_deployment(&pool, attacker.org_id).await;
    let victim_dep = insert_deployment(&pool, victim.org_id).await;
    let path = format!("/api/v1/workspaces/{ws}/chaos-campaigns");

    let (status, body) =
        attacker.send(Method::POST, &path, campaign_body(&own_dep.to_string())).await;
    assert!(status.is_success(), "own deployment target must be accepted: {status}: {body}");

    let (status, body) =
        attacker.send(Method::POST, &path, campaign_body(&victim_dep.to_string())).await;
    assert!(
        status.is_client_error(),
        "campaign targeting ANOTHER org's deployment was accepted: {status}: {body}"
    );

    let (status, body) = attacker.send(Method::POST, &path, campaign_body("not-a-uuid")).await;
    assert!(
        status.is_client_error(),
        "non-UUID hosted_mock target accepted: {status}: {body}"
    );
}

#[tokio::test]
#[ignore]
async fn chaos_trigger_rejects_preexisting_foreign_target() {
    let pool = owner_pool().await;
    let attacker = register("ta").await;
    let victim = register("tv").await;
    allow_runs(&pool, attacker.org_id).await;
    let ws = attacker.create_workspace().await;
    let victim_dep = insert_deployment(&pool, victim.org_id).await;

    // A campaign row that predates create-time validation.
    let campaign_id: Uuid = sqlx::query_scalar(
        "INSERT INTO chaos_campaigns \
           (workspace_id, name, target_kind, target_ref, config, safety_config) \
         VALUES ($1, 'legacy-xtenant', 'hosted_mock', $2, '{}'::jsonb, '{}'::jsonb) \
         RETURNING id",
    )
    .bind(ws)
    .bind(victim_dep.to_string())
    .fetch_one(&pool)
    .await
    .expect("insert chaos_campaign");

    let (status, body) = attacker
        .send(Method::POST, &format!("/api/v1/chaos-campaigns/{campaign_id}/runs"), json!({}))
        .await;
    assert!(
        status.is_client_error(),
        "triggering a campaign aimed at ANOTHER org's deployment was accepted: {status}: {body}"
    );
    let queued: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs WHERE suite_id = $1")
        .bind(campaign_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(queued, 0, "a chaos run was queued against another org's deployment");
}

#[tokio::test]
#[ignore]
async fn internal_chaos_proxy_rejects_deployment_outside_run_org() {
    let pool = owner_pool().await;
    let attacker = register("pa").await;
    let victim = register("pv").await;
    let own_dep = insert_deployment(&pool, attacker.org_id).await;
    let victim_dep = insert_deployment(&pool, victim.org_id).await;
    let run_id: Uuid = sqlx::query_scalar(
        "INSERT INTO test_runs (suite_id, org_id, kind, triggered_by, status) \
         VALUES ($1, $2, 'chaos_campaign', 'manual', 'running') RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(attacker.org_id)
    .fetch_one(&pool)
    .await
    .expect("insert test_run");

    let toggle = |dep: Uuid| async move {
        let res = internal(Method::POST, &format!("/api/v1/internal/hosted-mocks/{dep}/chaos"))
            .json(&json!({ "enabled": true, "run_id": run_id }))
            .send()
            .await
            .expect("chaos proxy request");
        (res.status(), res.text().await.unwrap_or_default())
    };

    // Control: the run's own deployment is forwarded (and fails to connect).
    let (status, text) = toggle(own_dep).await;
    assert!(
        text.contains("chaos proxy fetch failed"),
        "own-org toggle should be forwarded to the deployment: {status}: {text}"
    );

    let (status, text) = toggle(victim_dep).await;
    assert!(
        !text.contains("chaos proxy fetch failed") && status.is_client_error(),
        "chaos toggle was FORWARDED to another org's deployment: {status}: {text}"
    );
}

// ---------------------------------------------------------------------------
// Captures
// ---------------------------------------------------------------------------

async fn create_session(t: &Tenant) -> Uuid {
    let ws = t.create_workspace().await;
    let (status, body) = t
        .send(
            Method::POST,
            &format!("/api/v1/workspaces/{ws}/capture-sessions"),
            json!({ "name": "xtenant-session" }),
        )
        .await;
    assert!(status.is_success(), "create capture session {status}: {body}");
    Uuid::parse_str(body["id"].as_str().expect("no session id")).unwrap()
}

#[tokio::test]
#[ignore]
async fn capture_session_add_member_rejects_foreign_capture() {
    let pool = owner_pool().await;
    let attacker = register("ma").await;
    let victim = register("mv").await;
    let session = create_session(&attacker).await;
    let own_dep = insert_deployment(&pool, attacker.org_id).await;
    let victim_dep = insert_deployment(&pool, victim.org_id).await;
    let own_capture = Uuid::new_v4();
    let victim_capture = Uuid::new_v4();
    insert_capture(&pool, own_dep, &own_capture.to_string(), "own-body").await;
    insert_capture(&pool, victim_dep, &victim_capture.to_string(), "victim-secret").await;
    let path = format!("/api/v1/capture-sessions/{session}/members");

    let (status, body) = attacker
        .send(Method::PATCH, &path, json!({ "op": "add", "capture_id": own_capture }))
        .await;
    assert!(status.is_success(), "own capture must be addable: {status}: {body}");

    let (status, body) = attacker
        .send(Method::PATCH, &path, json!({ "op": "add", "capture_id": victim_capture }))
        .await;
    assert!(
        status.is_client_error(),
        "ANOTHER org's capture was added to the session: {status}: {body}"
    );
    let member: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM capture_session_members \
                         WHERE session_id = $1 AND capture_id = $2)",
    )
    .bind(session)
    .bind(victim_capture)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!member, "foreign capture landed in capture_session_members");
}

#[tokio::test]
#[ignore]
async fn capture_exchanges_exclude_foreign_captures_and_tolerate_bad_ids() {
    let pool = owner_pool().await;
    let attacker = register("ea").await;
    let victim = register("ev").await;
    let session = create_session(&attacker).await;
    let own_dep = insert_deployment(&pool, attacker.org_id).await;
    let victim_dep = insert_deployment(&pool, victim.org_id).await;
    let own_capture = Uuid::new_v4();
    let victim_capture = Uuid::new_v4();
    insert_capture(&pool, own_dep, &own_capture.to_string(), "own-body").await;
    insert_capture(&pool, victim_dep, &victim_capture.to_string(), "victim-secret").await;
    // A row with a non-UUID capture_id (ingested before ids were validated).
    insert_capture(&pool, victim_dep, "not-a-uuid", "junk").await;

    // Membership rows that predate the add-member ownership check.
    for capture in [own_capture, victim_capture] {
        sqlx::query("INSERT INTO capture_session_members (session_id, capture_id) VALUES ($1, $2)")
            .bind(session)
            .bind(capture)
            .execute(&pool)
            .await
            .expect("insert member");
    }

    let res =
        internal(Method::GET, &format!("/api/v1/internal/capture-sessions/{session}/exchanges"))
            .send()
            .await
            .expect("exchanges request");
    let status = res.status();
    let text = res.text().await.unwrap_or_default();

    // Clean up the non-UUID row before asserting so a failure here can't
    // poison other suites sharing the database.
    sqlx::query("DELETE FROM hosted_mocks WHERE id = $1")
        .bind(victim_dep)
        .execute(&pool)
        .await
        .ok();

    assert!(
        status.is_success(),
        "exchanges endpoint failed (non-UUID row?): {status}: {text}"
    );
    assert!(text.contains("own-body"), "own capture missing from exchanges: {text}");
    assert!(
        !text.contains("victim-secret"),
        "ANOTHER org's capture body leaked into the replay exchanges: {text}"
    );
}

#[tokio::test]
#[ignore]
async fn capture_ingest_rejects_non_uuid_capture_id() {
    let pool = owner_pool().await;
    let tenant = register("ia").await;
    let dep = insert_deployment(&pool, tenant.org_id).await;
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set (shared with server)");
    let token =
        mockforge_registry_core::auth::create_deployment_ingest_token(dep, &secret, 1).unwrap();
    let good = Uuid::new_v4();
    let exchange = |id: &str| {
        json!({
            "request": {
                "id": id,
                "protocol": "http",
                "timestamp": Utc::now(),
                "method": "GET",
                "path": "/x",
                "headers": "{}",
                "body_encoding": "utf8",
            }
        })
    };

    let res = Client::new()
        .post(format!("{}/api/v1/hosted-mocks/{dep}/captures/ingest", base_url()))
        .bearer_auth(token)
        .json(&json!({ "exchanges": [exchange(&good.to_string()), exchange("not-a-uuid")] }))
        .send()
        .await
        .expect("ingest request");
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    assert!(status.is_success(), "ingest {status}: {text}");

    let ids: Vec<String> =
        sqlx::query_scalar("SELECT capture_id FROM runtime_captures WHERE deployment_id = $1")
            .bind(dep)
            .fetch_all(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM hosted_mocks WHERE id = $1")
        .bind(dep)
        .execute(&pool)
        .await
        .ok();

    assert_eq!(ids, vec![good.to_string()], "a non-UUID capture_id was stored");
}
