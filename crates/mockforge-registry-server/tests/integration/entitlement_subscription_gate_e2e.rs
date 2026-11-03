//! End-to-end tests for the #870 effective-plan entitlement gate.
//!
//! Before #870, feature gates trusted `org.plan()` alone. `plan` is only
//! flipped by Stripe webhooks, so a missed/dropped `customer.subscription.*`
//! webhook would leave a canceled or past-due org at its paid tier
//! indefinitely. The gates now resolve the *effective* plan
//! (`handlers::entitlements::effective_plan`): a Team org keeps Team only
//! while its subscription is `active`/`trialing` (or `past_due` within the
//! 24h grace); otherwise it's gated as Free.
//!
//! These tests drive the SSO-config gate (`POST /api/v1/sso/config`, Team-
//! only) end-to-end with the org's plan forced to Team in the DB and a
//! subscription row seeded at the status under test. They assert:
//!   (a) `trialing`  → Team kept → SSO config allowed.
//!   (b) `active`    → Team kept → SSO config allowed.
//!   (c) `canceled`  → gated as Free → SSO config denied.
//!   (d) `unpaid`    → gated as Free → SSO config denied.
//!
//! The pure decision core (`resolve_effective_plan`, incl. the past-due grace
//! window) is unit-tested without a DB in
//! `mockforge-registry-server::handlers::entitlements::tests`.
//!
//! Same `#[ignore]` + `REGISTRY_URL`/`DATABASE_URL` shape as the sibling
//! `*_e2e.rs` suites.
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   DATABASE_URL=postgres://postgres:password@localhost:55432/mockforge_registry \
//!   cargo test -p mockforge-registry-server --test entitlement_subscription_gate_e2e -- --ignored --nocapture

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

async fn register_and_setup(base_url: &str) -> E2e {
    let client = Client::new();
    let ts = Utc::now().timestamp_micros();
    let username = format!("entgate_{}", ts);
    let email = format!("entgate_{}@e2e-test.local", ts);
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

    let org_slug = format!("entgate-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("EntGate Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let body: Value = res.json().await.expect("org not JSON");
    let org_id = body["id"].as_str().expect("no org id").to_string();

    E2e {
        client,
        base_url: base_url.to_string(),
        access_token,
        org_id,
    }
}

/// Force an org onto the Team plan (bypassing Stripe) so the only variable
/// under test is the subscription status the effective-plan gate reads.
async fn force_team_plan(pool: &PgPool, org_id: Uuid) {
    sqlx::query("UPDATE organizations SET plan = 'team' WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .expect("force team plan");
}

/// Seed a subscription row for the org at the given status. `updated_at` is
/// NOW(), so a `past_due` row would still be inside its grace window; the
/// statuses exercised here are terminal (canceled/unpaid) or active/trialing,
/// where the grace window is irrelevant.
async fn seed_subscription(pool: &PgPool, org_id: Uuid, status: &str) {
    let ts = Utc::now().timestamp_micros();
    sqlx::query(
        r#"
        INSERT INTO subscriptions
            (org_id, stripe_subscription_id, stripe_customer_id, price_id,
             plan, status, current_period_start, current_period_end)
        VALUES ($1, $2, $3, 'price_test', 'team', $4, NOW(), NOW() + INTERVAL '30 days')
        "#,
    )
    .bind(org_id)
    .bind(format!("sub_entgate_{ts}"))
    .bind(format!("cus_entgate_{ts}"))
    .bind(status)
    .execute(pool)
    .await
    .expect("seed subscription");
}

/// Attempt to create an SSO config (Team-only gate). Returns the HTTP status.
async fn try_create_sso_config(e: &E2e) -> StatusCode {
    e.client
        .post(format!("{}/api/v1/sso/config", e.base_url))
        .header("Authorization", format!("Bearer {}", e.access_token))
        .header("X-Organization-Id", &e.org_id)
        .json(&json!({
            "provider": "oidc",
            "oidc_issuer_url": "https://idp.example.com",
            "oidc_client_id": "client-1",
            "oidc_client_secret": "secret-1",
        }))
        .send()
        .await
        .expect("sso config post failed")
        .status()
}

/// Active subscription → Team kept → SSO config gate passes (non-403/plan).
#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn active_subscription_keeps_team_sso_allowed() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    force_team_plan(&pool, org_uuid).await;
    seed_subscription(&pool, org_uuid, "active").await;

    let status = try_create_sso_config(&e).await;
    assert!(
        status.is_success(),
        "active Team subscription must keep SSO available, got {status}"
    );
}

/// Trialing subscription → Team kept → SSO config allowed. MUST not break the
/// 14-day trial.
#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn trialing_subscription_keeps_team_sso_allowed() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    force_team_plan(&pool, org_uuid).await;
    seed_subscription(&pool, org_uuid, "trialing").await;

    let status = try_create_sso_config(&e).await;
    assert!(
        status.is_success(),
        "trialing Team subscription MUST keep SSO available (do not break trials), got {status}"
    );
}

/// Canceled subscription → gated as Free → SSO config denied.
#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn canceled_subscription_blocks_team_sso() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    force_team_plan(&pool, org_uuid).await;
    seed_subscription(&pool, org_uuid, "canceled").await;

    // Stored plan is still Team, but the effective plan is Free → the
    // "SSO is only available for Team plans" gate must reject (400).
    let status = try_create_sso_config(&e).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "canceled subscription must downgrade gating to Free and block SSO"
    );
}

/// Unpaid subscription → gated as Free → SSO config denied.
#[tokio::test]
#[ignore = "requires Postgres + running registry (REGISTRY_URL / DATABASE_URL)"]
async fn unpaid_subscription_blocks_team_sso() {
    let Some((base_url, pool)) = pool_or_skip().await else {
        eprintln!("REGISTRY_URL/DATABASE_URL not set; skipping");
        return;
    };
    let e = register_and_setup(&base_url).await;
    let org_uuid: Uuid = e.org_id.parse().unwrap();
    force_team_plan(&pool, org_uuid).await;
    seed_subscription(&pool, org_uuid, "unpaid").await;

    let status = try_create_sso_config(&e).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "unpaid subscription must downgrade gating to Free and block SSO"
    );
}
