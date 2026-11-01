//! End-to-end tests for the #449 usage-limit + past_due enforcement
//! acceptance criteria 7 and 8.
//!
//! Both tests use the same e2e shape as the other `*_e2e.rs` suites in this
//! crate: `#[ignore]`-gated, runs against a live registry server with a
//! Postgres reachable on `DATABASE_URL`. The two test scenarios:
//!
//! 1. `free_org_over_request_limit_returns_429_with_spec_body`
//!    — Exhausts the org's `requests_per_30d` counter against the proxy
//!    fast-path (`enforce_monthly_quota`) and asserts the response shape
//!    matches the criterion-1 spec exactly.
//!
//! 2. `past_due_beyond_grace_returns_402_payment_required`
//!    — Backdates the subscription's `updated_at` past the 24h grace
//!    window and asserts a deploy attempt returns 402 with the new
//!    `PAYMENT_REQUIRED` error code (criterion 8).
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   DATABASE_URL=postgres://postgres:password@localhost:55432/mockforge_registry \
//!   cargo test -p mockforge-registry-server --test usage_limits_e2e -- --ignored --nocapture

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    client: Client,
    base_url: String,
    access_token: String,
    org_id: Uuid,
    pool: PgPool,
}

async fn fixture() -> Fixture {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    let client = Client::new();

    // Unique per-test identifiers — each suite runs against the same DB and
    // we don't tear down between runs, so collision-resistance matters.
    let ts = Utc::now().timestamp_micros();
    let username = format!("usage_e2e_{}", ts);
    let email = format!("usage_e2e_{}@e2e-test.local", ts);
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

    let org_slug = format!("usage-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("Usage Test Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let body: Value = res.json().await.expect("org not JSON");
    let org_id: Uuid = body["id"].as_str().expect("no org id").parse().expect("org id not a UUID");

    Fixture {
        client,
        base_url,
        access_token,
        org_id,
        pool,
    }
}

/// Acceptance criterion 7: a free org that has burned through its monthly
/// request allotment must hit 429 on the very next proxy hit, with a body
/// matching the criterion-1 spec.
///
/// We seed the state directly:
///   1. Pin the org's `limits_json.requests_per_30d` to a tiny value (1) so
///      the test doesn't have to hammer the proxy 250k times.
///   2. Insert a fake `hosted_mocks` row with status='active'. Its
///      `internal_url`/`deployment_url` are nonsense — we never reach the
///      proxy_request step, the 429 fires earlier inside
///      `enforce_monthly_quota`.
///   3. Set `usage_counters.requests = 1` so we're already at the limit.
///   4. Hit `/mocks/{org_id}/{slug}` and check the response.
#[tokio::test]
#[ignore]
async fn free_org_over_request_limit_returns_429_with_spec_body() {
    let f = fixture().await;
    let slug = format!("usage-mock-{}", Utc::now().timestamp_micros());

    // (1) lower the monthly cap so we can saturate it deterministically.
    sqlx::query(
        r#"UPDATE organizations
           SET limits_json = jsonb_set(limits_json, '{requests_per_30d}', '1'::jsonb, true)
           WHERE id = $1"#,
    )
    .bind(f.org_id)
    .execute(&f.pool)
    .await
    .expect("lower monthly cap");

    // (2) insert a fake active deployment. config_json is NOT NULL in the
    // schema; an empty object is fine because we never reach the proxied
    // upstream — the 429 fires inside `enforce_monthly_quota`.
    let deployment_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO hosted_mocks
            (id, org_id, slug, name, status, config_json,
             deployment_url, internal_url)
           VALUES ($1, $2, $3, 'usage-e2e', 'active', '{}'::jsonb,
                   'http://127.0.0.1:1', 'http://127.0.0.1:1')"#,
    )
    .bind(deployment_id)
    .bind(f.org_id)
    .bind(&slug)
    .execute(&f.pool)
    .await
    .expect("insert hosted_mock");

    // (3) crank the counter above the cap. UPSERT on (org_id, period_start)
    // — the registry handler likely created the row already during signup.
    sqlx::query(
        r#"INSERT INTO usage_counters
            (org_id, period_start, requests)
           VALUES ($1, date_trunc('month', NOW())::date, 100)
           ON CONFLICT (org_id, period_start) DO UPDATE SET requests = 100"#,
    )
    .bind(f.org_id)
    .execute(&f.pool)
    .await
    .expect("seed usage counter");

    // (4) hit the proxy and verify spec response shape.
    let res = f
        .client
        .get(format!("{}/mocks/{}/{}", f.base_url, f.org_id, slug))
        .send()
        .await
        .expect("proxy request failed");

    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS, "expected 429");
    let body: Value = res.json().await.expect("429 response not JSON");

    // Criterion 1 spec: `{"error":"usage_limit_exceeded","limit":"requests","current":N,"max":M}`
    // Wrapped in our standard error envelope (request_id, error_code, status,
    // details) but the four required fields are all present at the top level.
    assert_eq!(body["error"], "usage_limit_exceeded", "wrong error label: {}", body);
    assert_eq!(body["limit"], "requests", "wrong limit type: {}", body);
    assert!(body["current"].as_i64().unwrap_or(0) >= 1, "current missing/zero: {}", body);
    assert_eq!(body["max"].as_i64().unwrap_or(0), 1, "max should be 1: {}", body);
    assert_eq!(
        body["error_code"], "USAGE_LIMIT_EXCEEDED",
        "envelope error_code wrong: {}",
        body
    );
    assert_eq!(body["status"].as_i64().unwrap_or(0), 429);
}

/// Acceptance criterion 8: an org whose subscription has been past_due for
/// more than the 24h grace window (PR #507) gets 402 PaymentRequired on the
/// next deploy attempt — *not* 400 InvalidRequest, which is what main shipped
/// before this PR.
///
/// We seed the state directly:
///   1. Insert a `subscriptions` row with status='past_due' and
///      `updated_at = NOW() - 25h` to push the org past the grace window.
///   2. Attempt POST /api/v1/hosted-mocks (the actual deploy gate, not just
///      a synthetic call into the middleware).
#[tokio::test]
#[ignore]
async fn past_due_beyond_grace_returns_402_payment_required() {
    let f = fixture().await;

    // (1) park the org in past_due, beyond the 24h grace. Each fixture()
    // makes a fresh org so we never collide on the unique stripe_subscription_id.
    let stripe_sub_id = format!("sub_e2e_pastdue_{}", Utc::now().timestamp_micros());
    sqlx::query(
        r#"INSERT INTO subscriptions
            (org_id, status, plan, price_id,
             stripe_customer_id, stripe_subscription_id,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at)
           VALUES ($1, 'past_due', 'pro', 'price_e2e_pro',
                   'cus_e2e_past_due', $2,
                   NOW() - INTERVAL '20 days', NOW() + INTERVAL '10 days',
                   false, NOW() - INTERVAL '25 hours', NOW() - INTERVAL '25 hours')"#,
    )
    .bind(f.org_id)
    .bind(&stripe_sub_id)
    .execute(&f.pool)
    .await
    .expect("seed past_due subscription");

    // (2) attempt to deploy. The handler runs the past_due check before any
    // permission/limit logic, so we don't need a real spec payload — the
    // request will short-circuit at the billing gate.
    let res = f
        .client
        .post(format!("{}/api/v1/hosted-mocks", f.base_url))
        .header("Authorization", format!("Bearer {}", f.access_token))
        .json(&json!({
            "name": "past-due-e2e",
            "slug": "past-due-e2e",
            "spec_url": "https://example.com/spec.json",
            "protocol": "http"
        }))
        .send()
        .await
        .expect("deploy request failed");

    let status = res.status();
    let body: Value = res.json().await.unwrap_or_else(|_| json!({}));
    assert_eq!(
        status,
        StatusCode::PAYMENT_REQUIRED,
        "expected 402 PaymentRequired, got {}: {}",
        status,
        body
    );
    assert_eq!(body["error_code"], "PAYMENT_REQUIRED", "wrong error_code: {}", body);
}

/// Companion to the 402 test: an org in past_due *within* the 24h grace
/// window must NOT be blocked. Seeds `updated_at = NOW() - 1 hour` and
/// asserts the deploy progresses past the billing gate (we don't care which
/// downstream error it ultimately surfaces — only that 402 is *not* returned).
#[tokio::test]
#[ignore]
async fn past_due_within_grace_does_not_return_402() {
    let f = fixture().await;

    let stripe_sub_id = format!("sub_e2e_grace_{}", Utc::now().timestamp_micros());
    sqlx::query(
        r#"INSERT INTO subscriptions
            (org_id, status, plan, price_id,
             stripe_customer_id, stripe_subscription_id,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at)
           VALUES ($1, 'past_due', 'pro', 'price_e2e_pro',
                   'cus_e2e_grace', $2,
                   NOW() - INTERVAL '5 days', NOW() + INTERVAL '25 days',
                   false, NOW() - INTERVAL '1 hour', NOW() - INTERVAL '1 hour')"#,
    )
    .bind(f.org_id)
    .bind(&stripe_sub_id)
    .execute(&f.pool)
    .await
    .expect("seed past_due-within-grace subscription");

    let res = f
        .client
        .post(format!("{}/api/v1/hosted-mocks", f.base_url))
        .header("Authorization", format!("Bearer {}", f.access_token))
        .json(&json!({
            "name": "grace-e2e",
            "slug": "grace-e2e",
            "spec_url": "https://example.com/spec.json",
            "protocol": "http"
        }))
        .send()
        .await
        .expect("deploy request failed");

    assert_ne!(
        res.status(),
        StatusCode::PAYMENT_REQUIRED,
        "deploy must NOT be 402 inside the 24h grace window"
    );
}

/// Companion to the 402 test for the route-wide read-only middleware:
/// a past_due-beyond-grace org must be blocked from *any* hot-write API
/// endpoint, not just the deploy handler. We assert a 402 on POST
/// /api/v1/workspaces (a different handler with its own gates) and confirm
/// GETs against the same org still succeed.
#[tokio::test]
#[ignore]
async fn past_due_beyond_grace_blocks_arbitrary_writes_but_not_reads() {
    let f = fixture().await;

    let stripe_sub_id = format!("sub_e2e_writes_{}", Utc::now().timestamp_micros());
    sqlx::query(
        r#"INSERT INTO subscriptions
            (org_id, status, plan, price_id,
             stripe_customer_id, stripe_subscription_id,
             current_period_start, current_period_end,
             cancel_at_period_end, created_at, updated_at)
           VALUES ($1, 'past_due', 'pro', 'price_e2e_pro',
                   'cus_e2e_writes', $2,
                   NOW() - INTERVAL '20 days', NOW() + INTERVAL '10 days',
                   false, NOW() - INTERVAL '25 hours', NOW() - INTERVAL '25 hours')"#,
    )
    .bind(f.org_id)
    .bind(&stripe_sub_id)
    .execute(&f.pool)
    .await
    .expect("seed past_due-beyond-grace subscription");

    // Write: must be blocked by the route-wide middleware.
    let write = f
        .client
        .post(format!("{}/api/v1/workspaces", f.base_url))
        .header("Authorization", format!("Bearer {}", f.access_token))
        .header("X-Organization-Id", f.org_id.to_string())
        .json(&json!({ "name": "should-be-blocked", "description": "past_due test" }))
        .send()
        .await
        .expect("workspace create failed");
    assert_eq!(
        write.status(),
        StatusCode::PAYMENT_REQUIRED,
        "POST /workspaces during past_due past grace must return 402"
    );

    // Read: must still pass through.
    let read = f
        .client
        .get(format!("{}/api/v1/organizations/{}", f.base_url, f.org_id))
        .header("Authorization", format!("Bearer {}", f.access_token))
        .send()
        .await
        .expect("org GET failed");
    assert!(
        read.status().is_success(),
        "GET /organizations must still succeed during past_due"
    );

    // Recovery path: billing endpoints must remain reachable so the user
    // can fix payment. (We don't assert 200 because Stripe may not be wired
    // in the test env; we only assert *not* 402.)
    let billing = f
        .client
        .get(format!("{}/api/v1/billing/subscription", f.base_url))
        .header("Authorization", format!("Bearer {}", f.access_token))
        .send()
        .await
        .expect("billing GET failed");
    assert_ne!(
        billing.status(),
        StatusCode::PAYMENT_REQUIRED,
        "billing endpoints must remain reachable during past_due"
    );
}
