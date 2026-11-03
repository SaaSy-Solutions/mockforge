//! End-to-end test for the AI-assisted contract drift second pass
//! added in #348.
//!
//! Covers the runner-facing internal endpoint contract:
//! `POST /api/v1/internal/contract-diff/score`.
//!
//! What we *can* exercise in CI:
//!   - Internal-auth gate (missing/wrong bearer token → "Not found").
//!   - Empty `endpoints` → no LLM call, returns `no_traffic=true`.
//!   - Endpoints set but no captured exchanges in `runtime_captures`
//!     → no LLM call, returns `no_traffic=true`.
//!   - Free-tier org (no BYOK, no platform credits) → LLM is gated by
//!     `check_ai_quota` and the endpoint surfaces a 403 with the same
//!     "AI features are not available" copy AI Studio uses.
//!
//! What we deliberately do NOT exercise:
//!   - A real LLM call. CI doesn't have a valid
//!     `MOCKFORGE_PLATFORM_LLM_API_KEY` and we don't want this test
//!     silently burning real credits when it does. The pure
//!     prompt-building + finding-parsing is unit-tested in
//!     `mockforge-registry-server::ai::contract_diff::tests`.
//!
//! Requires:
//!   - PostgreSQL (docker-compose up db)
//!   - Registry server running with `MOCKFORGE_INTERNAL_API_TOKEN` set
//!     (the e2e workflow already wires this).
//!
//! Run with:
//!   REGISTRY_URL=http://localhost:8080 \
//!   DATABASE_URL=postgres://postgres:password@localhost:55432/mockforge_registry \
//!   MOCKFORGE_INTERNAL_API_TOKEN=test-internal-token \
//!   cargo test --test cloud_ai_contract_diff_e2e -- --ignored --nocapture

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

struct E2e {
    client: Client,
    // Carried for completeness/symmetry with other e2e tests; the
    // contract-diff tests build URLs from `base_url` arg directly.
    #[allow(dead_code)]
    base_url: String,
    access_token: String,
    org_id: String,
    workspace_id: String,
    pool: PgPool,
}

async fn register_and_setup(base_url: &str, pool: &PgPool) -> E2e {
    let client = Client::new();
    let ts = Utc::now().timestamp_micros();
    let username = format!("aidiff_{}", ts);
    let email = format!("aidiff_{}@e2e-test.local", ts);
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

    let org_slug = format!("aidiff-{}", ts);
    let res = client
        .post(format!("{}/api/v1/organizations", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&json!({ "name": format!("AI Diff Test Org {}", ts), "slug": org_slug }))
        .send()
        .await
        .expect("create org failed");
    let body: Value = res.json().await.expect("org not JSON");
    let org_id = body["id"].as_str().expect("no org id").to_string();

    let res = client
        .post(format!("{}/api/v1/workspaces", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("X-Organization-Id", &org_id)
        .json(&json!({ "name": "ai-diff-e2e", "description": "e2e fixture" }))
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
        pool: pool.clone(),
    }
}

fn internal_token() -> String {
    std::env::var("MOCKFORGE_INTERNAL_API_TOKEN").expect("MOCKFORGE_INTERNAL_API_TOKEN must be set")
}

#[tokio::test]
#[ignore]
async fn rejects_missing_internal_auth() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    let e = register_and_setup(&base_url, &pool).await;

    // No bearer at all — handler should reject before doing any DB
    // work, returning the same opaque "Not found" the other internal
    // endpoints use.
    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0",
            "endpoints": [{"method": "GET", "path": "/x"}],
        }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Wrong bearer.
    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .header("Authorization", "Bearer wrong-token")
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0",
            "endpoints": [{"method": "GET", "path": "/x"}],
        }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Use the user's session token (also wrong — internal endpoints
    // don't accept user tokens).
    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .header("Authorization", format!("Bearer {}", e.access_token))
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0",
            "endpoints": [{"method": "GET", "path": "/x"}],
        }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn empty_endpoints_short_circuits_no_llm_call() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    let e = register_and_setup(&base_url, &pool).await;

    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .header("Authorization", format!("Bearer {}", internal_token()))
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0\npaths: {}",
            "endpoints": [],
        }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "empty endpoints should short-circuit OK: {:?}",
        res.text().await.unwrap_or_default()
    );
    let body: Value = res.json().await.expect("not JSON");
    assert_eq!(body["no_traffic"], json!(true));
    assert_eq!(body["tokens_used"], json!(0));
    assert!(body["findings"].as_array().expect("findings array").is_empty());
}

#[tokio::test]
#[ignore]
async fn no_captured_exchanges_short_circuits_no_llm_call() {
    // Ask to score endpoints that no `runtime_captures` rows exist for.
    // Should return no_traffic=true without invoking the LLM (which
    // would otherwise 403 on a free-tier org without BYOK).
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    let e = register_and_setup(&base_url, &pool).await;

    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .header("Authorization", format!("Bearer {}", internal_token()))
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0\npaths:\n  /api/users:\n    get: {}",
            "endpoints": [{"method": "GET", "path": "/api/users"}],
        }))
        .send()
        .await
        .expect("post failed");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.expect("not JSON");
    assert_eq!(body["no_traffic"], json!(true));
    assert_eq!(body["tokens_used"], json!(0));
    assert!(body["findings"].as_array().expect("findings array").is_empty());
}

#[tokio::test]
#[ignore]
async fn free_plan_without_byok_is_quota_blocked_when_traffic_exists() {
    // Seed a runtime_capture row so the endpoint reaches the LLM
    // gate. Then the free-tier org without BYOK should be denied
    // with the standard "AI features are not available" copy.
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    let e = register_and_setup(&base_url, &pool).await;

    // Insert a hosted_mock so runtime_captures.deployment_id FK is
    // satisfied (CASCADE on delete from hosted_mocks).
    let org_uuid = Uuid::parse_str(&e.org_id).expect("org id not UUID");
    let workspace_uuid = Uuid::parse_str(&e.workspace_id).expect("ws id not UUID");
    let deployment_id = Uuid::new_v4();
    let ts = Utc::now().timestamp_micros();
    sqlx::query(
        r#"
        INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status)
        VALUES ($1, $2, $3, $4, '{}'::jsonb, 'active')
        "#,
    )
    .bind(deployment_id)
    .bind(org_uuid)
    .bind("ai-diff-fixture")
    .bind(format!("aifix-{}", ts))
    .execute(&e.pool)
    .await
    .expect("insert hosted_mock failed");

    let capture_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO runtime_captures (
            deployment_id, capture_id, protocol, occurred_at, method, path,
            request_headers, request_body_encoding, status_code,
            workspace_id, source
        )
        VALUES (
            $1, $2, 'http', NOW(), 'POST', '/api/checkout',
            '{}', 'utf8', 200,
            $3, 'hosted'
        )
        "#,
    )
    .bind(deployment_id)
    .bind(&capture_id)
    .bind(workspace_uuid)
    .execute(&e.pool)
    .await
    .expect("insert runtime_capture failed");

    let res = e
        .client
        .post(format!("{}/api/v1/internal/contract-diff/score", base_url))
        .header("Authorization", format!("Bearer {}", internal_token()))
        .json(&json!({
            "org_id": e.org_id,
            "workspace_id": e.workspace_id,
            "spec_excerpt": "openapi: 3.0.0",
            "endpoints": [{"method": "POST", "path": "/api/checkout"}],
        }))
        .send()
        .await
        .expect("post failed");
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    // Free + no BYOK → ProviderSelection::Disabled → 403
    // ResourceLimitExceeded with the "AI features are not available"
    // message. (Same code path AI Studio's chat endpoint hits.)
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "expected 403 quota block, got {}: {}",
        status,
        body
    );
    assert!(
        body.to_lowercase().contains("ai")
            && (body.to_lowercase().contains("byok") || body.to_lowercase().contains("plan")),
        "expected quota error mentioning AI/BYOK/plan, got: {body}"
    );
}
