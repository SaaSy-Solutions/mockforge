//! End-to-end test for the cloud verification surface added in #390.
//!
//! Covers each endpoint registered under
//! `/api/v1/workspaces/{id}/request-log/*` and asserts behavior parity
//! with the local matcher in `mockforge-core`. Captures are seeded
//! directly into `runtime_captures` via SQL (with `workspace_id`
//! populated) — that side-steps the hosted-mock ingest path which is
//! its own deployment-scoped JWT flow, and exercises the matcher
//! pipeline end-to-end.
//!
//! Why direct SQL seeding: today the hosted-mock log shipper does not
//! populate `runtime_captures.workspace_id` (it inserts only the
//! `deployment_id`). Hosted captures therefore can't be queried by
//! workspace today — that's a pre-existing data-flow gap tracked
//! separately. The e2e test here proves the new endpoints work
//! correctly *given* a row with `workspace_id` set, which is the
//! contract `source='local'` (cloud-shipped) captures already meet
//! and the contract any future hosted-side fix will need to meet.
//!
//! Requires:
//!   - PostgreSQL running (docker-compose up db)
//!   - Registry server running (see signup_flow_e2e.rs header for env vars)
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   DATABASE_URL=postgres://mockforge:mockforge@localhost:5432/mockforge_registry \
//!   cargo test --test cloud_verification_e2e -- --ignored --nocapture

use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

struct E2e {
    client: Client,
    base_url: String,
    access_token: String,
    org_id: String,
    workspace_id: String,
    pool: PgPool,
    deployment_id: Uuid,
}

impl E2e {
    fn auth(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth())
            .header("X-Organization-Id", &self.org_id)
    }
}

async fn register_and_setup(base_url: &str, pool: &PgPool) -> E2e {
    let client = Client::new();
    let ts = Utc::now().timestamp_micros();
    let username = format!("ver_{}", ts);
    let email = format!("ver_{}@e2e-test.local", ts);
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
    let access_token = body["access_token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .expect("no access token")
        .to_string();

    let org_slug = format!("ver-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("Verification Test Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.expect("org not JSON");
    assert!(status.is_success(), "create org {}: {}", status, body);
    let org_id = body["id"].as_str().expect("no org id").to_string();

    // Lift the free-plan max_projects=1 cap so tests can create more than
    // one workspace under the same org (workspace_isolation needs two).
    // Mirrors the gate in handlers::cloud_workspaces::create_workspace,
    // which reads `org.limits_json.max_projects` and falls back to 1.
    // -1 means unlimited.
    let org_uuid_for_limits = Uuid::parse_str(&org_id).expect("org id not UUID");
    sqlx::query(
        r#"
        UPDATE organizations
        SET limits_json = jsonb_set(
            COALESCE(limits_json, '{}'::jsonb),
            '{max_projects}',
            '-1'::jsonb,
            true
        )
        WHERE id = $1
        "#,
    )
    .bind(org_uuid_for_limits)
    .execute(pool)
    .await
    .expect("relax org limits failed");

    // Workspace
    let res = client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Organization-Id", &org_id)
        .json(&json!({ "name": "verification-e2e", "description": "e2e fixture" }))
        .send()
        .await
        .expect("create workspace failed");
    let status = res.status();
    let body: Value = res.json().await.expect("ws not JSON");
    assert!(status.is_success(), "create ws {}: {}", status, body);
    let workspace_id = body["id"].as_str().expect("no ws id").to_string();

    // Insert a hosted_mock row so `runtime_captures.deployment_id` FK
    // is satisfied. We don't actually deploy anything — `status='active'`
    // is just a label here, no Fly machine is created.
    let org_uuid = Uuid::parse_str(&org_id).expect("org id not UUID");
    let deployment_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status)
        VALUES ($1, $2, $3, $4, $5::jsonb, 'active')
        "#,
    )
    .bind(deployment_id)
    .bind(org_uuid)
    .bind("verification-fixture")
    .bind(format!("vfix-{}", ts))
    .bind("{}")
    .execute(pool)
    .await
    .expect("insert hosted_mock failed");

    E2e {
        client,
        base_url: base_url.to_string(),
        access_token,
        org_id,
        workspace_id,
        pool: pool.clone(),
        deployment_id,
    }
}

/// Insert a single capture row, returning its `occurred_at` for sequencing.
#[allow(clippy::too_many_arguments)]
async fn seed_capture(
    e: &E2e,
    method: &str,
    path: &str,
    headers: &str,      // JSON-encoded `{"k":"v"}`
    query_params: &str, // JSON-encoded
    body: Option<&str>,
    status_code: i32,
    occurred_at: DateTime<Utc>,
) {
    let workspace_uuid = Uuid::parse_str(&e.workspace_id).expect("ws id not UUID");
    let capture_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO runtime_captures (
            deployment_id, capture_id, protocol, occurred_at, method, path,
            query_params, request_headers, request_body, request_body_encoding,
            status_code, workspace_id, source
        )
        VALUES (
            $1, $2, 'http', $3, $4, $5,
            $6, $7, $8, 'utf8',
            $9, $10, 'hosted'
        )
        "#,
    )
    .bind(e.deployment_id)
    .bind(&capture_id)
    .bind(occurred_at)
    .bind(method)
    .bind(path)
    .bind(query_params)
    .bind(headers)
    .bind(body)
    .bind(status_code)
    .bind(workspace_uuid)
    .execute(&e.pool)
    .await
    .expect("insert runtime_capture failed");
}

#[tokio::test]
#[ignore]
async fn cloud_verification_full_flow() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");

    let e = register_and_setup(&base_url, &pool).await;

    // ---- Status: empty ---------------------------------------------------
    let res = e
        .get(&format!("/api/v1/workspaces/{}/request-log/status", e.workspace_id))
        .send()
        .await
        .expect("status request failed");
    assert_eq!(res.status(), StatusCode::OK);
    let status: Value = res.json().await.expect("status not JSON");
    assert_eq!(status["has_captures"], json!(false));
    assert_eq!(status["recent_capture_count"], json!(0));

    // ---- Seed three captures in chronological order ---------------------
    let now = Utc::now();
    let common_headers = r#"{"content-type":"application/json","x-tenant":"alpha"}"#;
    seed_capture(
        &e,
        "POST",
        "/api/checkout",
        common_headers,
        r#"{"ref":"abc"}"#,
        Some(r#"{"item":"widget","qty":1}"#),
        201,
        now - Duration::minutes(10),
    )
    .await;
    seed_capture(
        &e,
        "GET",
        "/api/users/42",
        common_headers,
        r#"{}"#,
        None,
        200,
        now - Duration::minutes(5),
    )
    .await;
    seed_capture(
        &e,
        "POST",
        "/api/checkout",
        common_headers,
        r#"{"ref":"def"}"#,
        Some(r#"{"item":"sprocket","qty":2}"#),
        201,
        now - Duration::minutes(2),
    )
    .await;

    // ---- Status: now non-empty ------------------------------------------
    let res = e
        .get(&format!("/api/v1/workspaces/{}/request-log/status", e.workspace_id))
        .send()
        .await
        .expect("status (post-seed) failed");
    let status: Value = res.json().await.expect("status not JSON");
    assert_eq!(status["has_captures"], json!(true));
    assert_eq!(status["recent_capture_count"], json!(3));

    // ---- count: 2 POST /api/checkout ------------------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/count", e.workspace_id))
        .json(&json!({
            "pattern": { "method": "POST", "path": "/api/checkout" }
        }))
        .send()
        .await
        .expect("count failed");
    let body: Value = res.json().await.expect("count not JSON");
    assert_eq!(body["count"], json!(2), "count body: {}", body);

    // ---- verify exactly 2 ------------------------------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/verify", e.workspace_id))
        .json(&json!({
            "pattern": { "method": "POST", "path": "/api/checkout" },
            "expected": { "type": "exactly", "value": 2 }
        }))
        .send()
        .await
        .expect("verify exactly failed");
    let body: Value = res.json().await.expect("verify not JSON");
    assert_eq!(body["matched"], json!(true), "verify body: {}", body);
    assert_eq!(body["count"], json!(2));

    // ---- verify exactly 5 -> failure ------------------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/verify", e.workspace_id))
        .json(&json!({
            "pattern": { "method": "POST", "path": "/api/checkout" },
            "expected": { "type": "exactly", "value": 5 }
        }))
        .send()
        .await
        .expect("verify mismatch failed");
    let body: Value = res.json().await.expect("verify mismatch not JSON");
    assert_eq!(body["matched"], json!(false));
    assert_eq!(body["count"], json!(2));
    assert!(body["error_message"].is_string());

    // ---- never (no DELETE has been recorded) -----------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/never", e.workspace_id))
        .json(&json!({
            "pattern": { "method": "DELETE", "path": "/api/users/*" }
        }))
        .send()
        .await
        .expect("never failed");
    let body: Value = res.json().await.expect("never not JSON");
    assert_eq!(body["matched"], json!(true), "never body: {}", body);
    assert_eq!(body["count"], json!(0));

    // ---- at-least 1 with a body pattern (regex) --------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/at-least", e.workspace_id))
        .json(&json!({
            "pattern": {
                "method": "POST",
                "path": "/api/checkout",
                "body_pattern": "\"item\":\"widget\""
            },
            "min": 1
        }))
        .send()
        .await
        .expect("at-least failed");
    let body: Value = res.json().await.expect("at-least not JSON");
    assert_eq!(body["matched"], json!(true), "at-least body: {}", body);
    assert_eq!(body["count"], json!(1));

    // ---- header matching -------------------------------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/count", e.workspace_id))
        .json(&json!({
            "pattern": {
                "headers": { "X-Tenant": "alpha" }
            }
        }))
        .send()
        .await
        .expect("count headers failed");
    let body: Value = res.json().await.expect("count headers not JSON");
    assert_eq!(body["count"], json!(3), "count by header: {}", body);

    // ---- query-param matching --------------------------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/count", e.workspace_id))
        .json(&json!({
            "pattern": {
                "method": "POST",
                "query_params": { "ref": "abc" }
            }
        }))
        .send()
        .await
        .expect("count query failed");
    let body: Value = res.json().await.expect("count query not JSON");
    assert_eq!(body["count"], json!(1), "count by query: {}", body);

    // ---- sequence: POST /api/checkout -> GET /api/users/* -> POST /api/checkout
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/sequence", e.workspace_id))
        .json(&json!({
            "patterns": [
                { "method": "POST", "path": "/api/checkout" },
                { "method": "GET",  "path": "/api/users/*"  },
                { "method": "POST", "path": "/api/checkout" }
            ]
        }))
        .send()
        .await
        .expect("sequence failed");
    let body: Value = res.json().await.expect("sequence not JSON");
    assert_eq!(body["matched"], json!(true), "sequence body: {}", body);

    // ---- sequence: order reversed should fail ----------------------------
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/sequence", e.workspace_id))
        .json(&json!({
            "patterns": [
                { "method": "POST", "path": "/api/checkout" },
                { "method": "POST", "path": "/api/checkout" },
                { "method": "GET",  "path": "/api/users/*"  }
            ]
        }))
        .send()
        .await
        .expect("sequence reversed failed");
    let body: Value = res.json().await.expect("sequence reversed not JSON");
    assert_eq!(body["matched"], json!(false));

    // ---- window clamping: 48h lookback should 400 ------------------------
    let since = (Utc::now() - Duration::hours(48)).to_rfc3339();
    let until = Utc::now().to_rfc3339();
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/count", e.workspace_id))
        .json(&json!({
            "pattern": { "method": "GET" },
            "since": since,
            "until": until,
        }))
        .send()
        .await
        .expect("oversized window failed");
    assert_eq!(
        res.status(),
        StatusCode::BAD_REQUEST,
        "oversized window should reject: {:?}",
        res.text().await
    );
}

#[tokio::test]
#[ignore]
async fn cloud_verification_workspace_isolation() {
    // Two workspaces under the same org should not see each other's
    // captures. Guards against a dropped WHERE clause.
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");

    let e = register_and_setup(&base_url, &pool).await;

    // Make a second workspace.
    let res = e
        .post("/api/v1/workspaces")
        .json(&json!({ "name": "verification-e2e-other", "description": "isolation fixture" }))
        .send()
        .await
        .expect("create second ws failed");
    let body: Value = res.json().await.expect("ws2 not JSON");
    let other_workspace_id = body["id"].as_str().expect("no other ws id").to_string();

    // Seed only against the FIRST workspace.
    seed_capture(
        &e,
        "POST",
        "/api/orders",
        r#"{"content-type":"application/json"}"#,
        r#"{}"#,
        None,
        201,
        Utc::now() - Duration::minutes(1),
    )
    .await;

    // Other workspace should see zero matches.
    let res = e
        .post(&format!("/api/v1/workspaces/{}/request-log/count", other_workspace_id))
        .json(&json!({
            "pattern": { "method": "POST", "path": "/api/orders" }
        }))
        .send()
        .await
        .expect("isolation count failed");
    let body: Value = res.json().await.expect("isolation not JSON");
    assert_eq!(body["count"], json!(0), "captures leaked across workspaces: {}", body);
}
