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

/// Actual saved-query HTTP configuration + native background evaluator + SQL
/// attribution/lifecycle. Telemetry is deliberately synthetic, like the
/// verification tests above; no hosted machine or external provider is used.
#[tokio::test]
#[ignore = "requires fixture PostgreSQL and native registry"]
async fn observability_alert_http_and_worker_lifecycle() {
    use mockforge_registry_server::workers::observability_alerts::run_tick;
    let base_url = std::env::var("REGISTRY_URL").expect("registry fixture");
    let database_url = std::env::var("DATABASE_URL").expect("database fixture");
    async fn tick(pool: &PgPool) {
        tokio::time::timeout(std::time::Duration::from_secs(5), run_tick(pool))
            .await
            .expect("alert tick must not deadlock")
            .unwrap();
    }
    // A single connection exercises the transaction-backed IncidentBus:
    // borrowing a second pool connection from the worker would deadlock.
    let pool = PgPoolOptions::new().max_connections(1).connect(&database_url).await.unwrap();
    let own = register_and_setup(&base_url, &pool).await;
    let foreign = register_and_setup(&base_url, &pool).await;
    let org = Uuid::parse_str(&own.org_id).unwrap();
    let foreign_org = Uuid::parse_str(&foreign.org_id).unwrap();
    let path = format!("/api/v1/organizations/{org}/observability/saved-queries");
    let rule =
        json!({"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5});
    for alert in [
        json!({"metric":"error_rate","threshold":1.1,"severity":"critical","window_minutes":5}),
        json!({"metric":"error_count","threshold":0,"severity":"critical","window_minutes":5}),
        json!({"metric":"error_count","threshold":1,"severity":"bogus","window_minutes":5}),
        json!({"metric":"error_count","threshold":1,"severity":"critical","window_minutes":0}),
    ] {
        let response = own
            .post(&path)
            .json(&json!({"name":"invalid","kind":"metrics","filters":{"alert":alert}}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    for body in [
        json!({"name":"foreign deployment","kind":"metrics","filters":{"alert":{"deployment_id":foreign.deployment_id,"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5}}}),
        json!({"name":"foreign workspace","kind":"metrics","workspace_id":foreign.workspace_id,"filters":{"alert":rule}}),
        json!({"name":"unsupported workspace","kind":"traces","workspace_id":own.workspace_id,"filters":{"alert":rule}}),
    ] {
        assert_eq!(
            own.post(&path).json(&body).send().await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    let rejected: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM observability_saved_queries WHERE org_id=$1")
            .bind(org)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rejected, 0, "invalid configuration must not persist");

    async fn seed(e: &E2e, healthy: bool) {
        for (status, latency) in [(if healthy { 200 } else { 500 }, 50), (200, 150)] {
            seed_capture(e, "GET", "/api", "{}", "{}", None, status, Utc::now()).await;
            sqlx::query("UPDATE runtime_captures SET duration_ms=$1 WHERE deployment_id=$2 AND duration_ms IS NULL").bind(i64::from(latency)).bind(e.deployment_id).execute(&e.pool).await.unwrap();
            sqlx::query("INSERT INTO runtime_request_logs(deployment_id,occurred_at,method,path,status,latency_ms) VALUES($1,NOW(),'GET','/api',$2,$3)").bind(e.deployment_id).bind(status as i16).bind(latency).execute(&e.pool).await.unwrap();
            sqlx::query("INSERT INTO runtime_traces(deployment_id,trace_id,span_id,name,service_name,start_unix_nano,end_unix_nano,occurred_at,status_code) VALUES($1,$2,$3,'request','orders',0,$4,NOW(),$5)").bind(e.deployment_id).bind(Uuid::new_v4().to_string()).bind(Uuid::new_v4().to_string()).bind(i64::from(latency)*1_000_000).bind(if status>=500 {2_i16} else {1_i16}).execute(&e.pool).await.unwrap();
        }
    }
    seed(&own, false).await;
    seed(&foreign, false).await;
    let mut ids = Vec::new();
    for kind in ["logs", "traces", "metrics"] {
        for (metric, threshold) in [
            ("sample_count", 2.0),
            ("error_count", 1.0),
            ("error_rate", 0.5),
            ("latency_p95_ms", 140.0),
        ] {
            let severity = match kind {
                "logs" => "warning",
                "traces" => "info",
                _ => "critical",
            };
            let mut alert = json!({"metric":metric,"threshold":threshold,"severity":severity,"window_minutes":5,"deployment_id":own.deployment_id});
            if kind == "traces" {
                alert["service_name"] = json!("orders");
            } else {
                alert["path_contains"] = json!("/api");
            }
            let response=own.post(&path).json(&json!({"name":format!("{kind}-{metric}"),"kind":kind,"workspace_id":if kind=="metrics" {Some(&own.workspace_id)} else {None},"filters":{"kind":"request_count","alert":alert}})).send().await.unwrap();
            assert!(response.status().is_success(), "create {kind}/{metric}");
            let body: Value = response.json().await.unwrap();
            ids.push((body["id"].as_str().unwrap().to_string(), kind.to_string()));
        }
    }
    for (name, alert) in [
        (
            "disabled",
            json!({"enabled":false,"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5}),
        ),
        (
            "insufficient",
            json!({"min_samples":5,"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5}),
        ),
        (
            "tenant isolation",
            json!({"metric":"error_count","threshold":2,"severity":"critical","window_minutes":5}),
        ),
        (
            "literal substring",
            json!({"path_contains":"%","metric":"sample_count","threshold":1,"severity":"critical","window_minutes":5}),
        ),
    ] {
        assert!(own
            .post(&path)
            .json(&json!({"name":name,"kind":"metrics","filters":{"alert":alert}}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    }
    // Wait for the native server worker: no first manual call can satisfy this.
    tokio::time::timeout(std::time::Duration::from_secs(45), async {
        loop {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability'",
            )
            .bind(org)
            .fetch_one(&pool)
            .await
            .unwrap();
            if count >= 12 {
                assert_eq!(count, 12);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("native observability worker deadline");
    for (id, kind) in &ids {
        let (severity,workspace,reference):(String,Option<Uuid>,String)=sqlx::query_as("SELECT severity,workspace_id,source_ref FROM incidents WHERE org_id=$1 AND source='observability' AND dedupe_key=$2").bind(org).bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(reference, *id);
        assert_eq!(
            severity,
            match kind.as_str() {
                "logs" => "medium",
                "traces" => "low",
                _ => "critical",
            }
        );
        assert_eq!(
            workspace,
            if kind == "metrics" {
                Some(Uuid::parse_str(&own.workspace_id).unwrap())
            } else {
                None
            }
        );
    }
    let alien: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability'",
    )
    .bind(foreign_org)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(alien, 0);
    // A second replica's own one-connection pool shares the coordination lock.
    let replica = PgPoolOptions::new().max_connections(1).connect(&database_url).await.unwrap();
    tokio::join!(tick(&pool), tick(&replica));
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 12, "concurrent/repeated ticks dedupe");
    // Incomplete responses and invalid latency are unknown, not healthy.
    sqlx::query("UPDATE runtime_captures SET status_code=NULL,response_status_code=NULL,duration_ms=NULL WHERE deployment_id=$1").bind(own.deployment_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE runtime_request_logs SET latency_ms=-1 WHERE deployment_id=$1")
        .bind(own.deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE runtime_traces SET status_code=NULL,end_unix_nano=-1 WHERE deployment_id=$1",
    )
    .bind(own.deployment_id)
    .execute(&pool)
    .await
    .unwrap();
    tick(&pool).await;
    let still_open:i64=sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability' AND status!='resolved'").bind(org).fetch_one(&pool).await.unwrap();
    assert_eq!(still_open, 12, "unknown observations must not prove recovery");
    // Missing data must not silently resolve real incidents.
    for table in ["runtime_captures", "runtime_request_logs", "runtime_traces"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE deployment_id=$1"))
            .bind(own.deployment_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    tick(&pool).await;
    let open:i64=sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability' AND status!='resolved'").bind(org).fetch_one(&pool).await.unwrap();
    assert_eq!(open, 12);
    seed(&own, true).await;
    sqlx::query("UPDATE runtime_captures SET duration_ms=0 WHERE deployment_id=$1")
        .bind(own.deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE runtime_request_logs SET latency_ms=0 WHERE deployment_id=$1")
        .bind(own.deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE runtime_traces SET end_unix_nano=start_unix_nano WHERE deployment_id=$1")
        .bind(own.deployment_id)
        .execute(&pool)
        .await
        .unwrap();
    tick(&pool).await;
    let resolved:i64=sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability' AND status='resolved'").bind(org).fetch_one(&pool).await.unwrap();
    assert_eq!(resolved, 9, "count rules stay breached; error and latency rules recover");
    for table in ["runtime_captures", "runtime_request_logs", "runtime_traces"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE deployment_id=$1"))
            .bind(own.deployment_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    seed(&own, false).await;
    tick(&pool).await;
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(total, 21, "rebreach opens fresh incidents after recovery");
    // Cross-tenant query edits remain inaccessible through the actual API.
    let forbidden = foreign
        .client
        .patch(format!("{base_url}/api/v1/observability/saved-queries/{}", ids[0].0))
        .header("Authorization", foreign.auth())
        .header("X-Organization-Id", &foreign.org_id)
        .json(&json!({"filters":{"alert":null}}))
        .send()
        .await
        .unwrap();
    assert!(forbidden.status().is_client_error());
    let invalid=own.client.patch(format!("{base_url}/api/v1/observability/saved-queries/{}",ids[0].0)).header("Authorization",own.auth()).header("X-Organization-Id",&own.org_id).json(&json!({"filters":{"alert":{"metric":"error_count","threshold":-1,"severity":"critical","window_minutes":5}}})).send().await.unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    async fn create_rule(e: &E2e, name: &str) -> Uuid {
        let response=e.post(&format!("/api/v1/organizations/{}/observability/saved-queries",e.org_id))
            .json(&json!({"name":name,"kind":"metrics","filters":{"alert":{"metric":"error_count","threshold":1,"severity":"critical","window_minutes":5}}}))
            .send().await.unwrap();
        assert!(response.status().is_success());
        Uuid::parse_str(response.json::<Value>().await.unwrap()["id"].as_str().unwrap()).unwrap()
    }
    async fn incident_count(pool: &PgPool, org: Uuid, query: Uuid) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE org_id=$1 AND source='observability' AND dedupe_key=$2")
            .bind(org).bind(query.to_string()).fetch_one(pool).await.unwrap()
    }
    // Causal coordination test: a held replica lock must prevent the first
    // raise, then releasing it must permit evaluation. Dedupe alone cannot
    // make this pass, and a blocking lock would fail the bounded tick.
    let mut lock = replica.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(0x4d46_4f42_5341_4c52_i64)
        .execute(&mut *lock)
        .await
        .unwrap();
    let locked_query = create_rule(&own, "held evaluator lock fixture").await;
    tick(&pool).await;
    assert_eq!(incident_count(&pool, org, locked_query).await, 0);
    lock.commit().await.unwrap();
    tick(&pool).await;
    assert_eq!(incident_count(&pool, org, locked_query).await, 1);

    // A genuine database failure in one raise must roll back that rule and
    // leave other rules runnable. All names/objects belong to this fixture.
    let suffix = Uuid::new_v4().simple().to_string();
    let function = format!("obs_alert_reject_{suffix}");
    let trigger = format!("obs_alert_fail_{suffix}");
    let failed_title = format!("failed observability fixture {suffix}");
    sqlx::query(&format!("CREATE FUNCTION {function}() RETURNS trigger LANGUAGE plpgsql AS $body$ BEGIN RAISE EXCEPTION 'fixture alert rejection'; END; $body$"))
        .execute(&pool).await.unwrap();
    sqlx::query(&format!("CREATE TRIGGER {trigger} BEFORE INSERT ON incidents FOR EACH ROW WHEN (NEW.title='{failed_title}') EXECUTE FUNCTION {function}()"))
        .execute(&pool).await.unwrap();
    let failed_query = create_rule(&own, &failed_title).await;
    let succeeding_query = create_rule(&own, "successful rule beside failure").await;
    tick(&pool).await;
    assert_eq!(incident_count(&pool, org, failed_query).await, 0);
    assert_eq!(incident_count(&pool, org, succeeding_query).await, 1);
    sqlx::query(&format!("DROP TRIGGER {trigger} ON incidents"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION {function}()"))
        .execute(&pool)
        .await
        .unwrap();
    tick(&pool).await;
    assert_eq!(
        incident_count(&pool, org, failed_query).await,
        1,
        "failed raise retries after rollback"
    );
    // Remove only this test's synthetic organizations; shared CI fixture lives.
    for id in [org, foreign_org] {
        sqlx::query("DELETE FROM organizations WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
