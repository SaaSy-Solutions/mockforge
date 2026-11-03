//! End-to-end tests for the #865 pre-enqueue quota + pending-cap gate on
//! `POST /api/v1/workspaces/{id}/test-generation/jobs`.
//!
//! Before #865, `create_job` enqueued AI test-generation jobs with NO quota
//! check — quota was only consulted per-job in the worker. A user could
//! enqueue unlimited jobs, each burning platform tokens up to the moment the
//! counter crossed the limit, with no cap on queue depth. The gate now
//! replicates the worker's `pick_provider` → `check_ai_quota` decision before
//! persisting, and caps in-flight jobs per org.
//!
//! Same e2e shape as the other `*_e2e.rs` suites: `#[ignore]`-gated, runs
//! against a live registry server with Postgres reachable on `DATABASE_URL`.
//!
//! Scenarios:
//!   1. `free_org_without_byok_is_rejected_before_enqueue`
//!      — A Free org (default: `ai_tokens_per_month = 0`, no BYOK) →
//!      provider is Disabled → 403 and NO row is persisted.
//!   2. `pending_cap_rejects_when_queue_is_full`
//!      — Pre-seed the cap's worth of queued jobs directly in the DB, then
//!      a paid-plan org with platform quota gets a 429 (queue full).
//!   3. `under_quota_paid_org_succeeds`
//!      — A Team org with platform quota and an empty queue enqueues OK.
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   DATABASE_URL=postgres://postgres:password@localhost:55432/mockforge_registry \
//!   cargo test -p mockforge-registry-server --test test_generation_quota_gate_e2e -- --ignored --nocapture

use chrono::Utc;
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
}

async fn register_and_setup(base_url: &str) -> E2e {
    let client = Client::new();
    let ts = Utc::now().timestamp_micros();
    let username = format!("testgen_{}", ts);
    let email = format!("testgen_{}@e2e-test.local", ts);
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

    let org_slug = format!("testgen-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("TestGen Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let body: Value = res.json().await.expect("org not JSON");
    let org_id = body["id"].as_str().expect("no org id").to_string();

    let res = client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Organization-Id", &org_id)
        .json(&json!({ "name": "testgen-e2e", "description": "e2e fixture" }))
        .send()
        .await
        .expect("create workspace failed");
    let body: Value = res.json().await.expect("ws not JSON");
    let workspace_id = body["id"].as_str().expect("no ws id").to_string();

    E2e {
        client,
        base_url: base_url.to_string(),
        access_token,
        org_id,
        workspace_id,
    }
}

async fn pool_or_skip() -> Option<(String, PgPool)> {
    let base_url = std::env::var("REGISTRY_URL").ok()?;
    let database_url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    Some((base_url, pool))
}

/// Force an org onto the Team plan + a generous platform AI quota, bypassing
/// the Stripe webhook path (we just need the plan + limits for gating).
async fn set_team_plan_with_ai_quota(pool: &PgPool, org_id: Uuid) {
    sqlx::query(
        r#"
        UPDATE organizations
        SET plan = 'team',
            limits_json = jsonb_set(limits_json, '{ai_tokens_per_month}', '1000000')
        WHERE id = $1
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("force team plan");
}

fn create_job_url(e: &E2e) -> String {
    format!("{}/api/v1/workspaces/{}/test-generation/jobs", e.base_url, e.workspace_id)
}

#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn free_org_without_byok_is_rejected_before_enqueue() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();

    // Default org is Free with ai_tokens_per_month = 0 and no BYOK →
    // pick_provider(false, None) = Disabled → quota not allowed.
    let res = e
        .client
        .post(create_job_url(&e))
        .header("Authorization", format!("Bearer {}", e.access_token))
        .header("X-Organization-Id", &e.org_id)
        .json(&json!({ "prompt": "gen tests", "captures_filter": {} }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "Free-without-BYOK org must be rejected before enqueue"
    );

    // No row should have been persisted.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cloud_test_generation_jobs WHERE org_id = $1")
            .bind(org_uuid)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 0, "no job row should be persisted on a rejected enqueue");
}

#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn under_quota_paid_org_succeeds() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    set_team_plan_with_ai_quota(&pool, org_uuid).await;

    let res = e
        .client
        .post(create_job_url(&e))
        .header("Authorization", format!("Bearer {}", e.access_token))
        .header("X-Organization-Id", &e.org_id)
        .json(&json!({ "prompt": "gen tests", "captures_filter": {} }))
        .send()
        .await
        .expect("post failed");
    assert!(
        res.status().is_success(),
        "Team org under quota with an empty queue should enqueue OK, got {}",
        res.status()
    );
}

#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn pending_cap_rejects_when_queue_is_full() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    let ws_uuid: Uuid = e.workspace_id.parse().unwrap();
    set_team_plan_with_ai_quota(&pool, org_uuid).await;

    // Pre-seed MAX_PENDING_JOBS_PER_ORG (20) queued jobs directly, so the
    // cap is already saturated before the request under test.
    for _ in 0..20 {
        sqlx::query(
            r#"
            INSERT INTO cloud_test_generation_jobs
                (workspace_id, org_id, prompt, captures_filter, status)
            VALUES ($1, $2, '', '{}'::jsonb, 'queued')
            "#,
        )
        .bind(ws_uuid)
        .bind(org_uuid)
        .execute(&pool)
        .await
        .expect("seed queued job");
    }

    let res = e
        .client
        .post(create_job_url(&e))
        .header("Authorization", format!("Bearer {}", e.access_token))
        .header("X-Organization-Id", &e.org_id)
        .json(&json!({ "prompt": "one too many", "captures_filter": {} }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(
        res.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "21st pending job must be rejected with 429 (queue full)"
    );
}
