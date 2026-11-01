//! E2E for the hosted-mock routes whose deployment lookup moved onto the
//! request-path (RLS) pool in #1087.
//!
//! Each route loads the deployment with the caller's org bound
//! (`with_org_context(runtime_pool, ctx.org_id, ..)`). A wrong binding there
//! does not error loudly: under the `NOBYPASSRLS` runtime role the row simply
//! disappears and the customer sees "Deployment not found". So for every route
//! this suite checks both directions:
//!
//! * the owner org gets PAST the deployment lookup (whatever the route does
//!   next; routes that proxy to the running mock fail later with a
//!   runtime/proxy error because the fixture has no machine, which is fine);
//! * another org is rejected with the lookup's not-found / no-access error.
//!
//! Redeploy additionally checks that the background task's status write lands:
//! with no `FLYIO_API_TOKEN` the task skips Fly and marks the deployment
//! `active` through the org-bound runtime pool, so a status stuck at
//! `deploying` means that write was silently filtered out.
//!
//! Needs `REGISTRY_URL` and `DATABASE_URL` (the owner/superuser URL, used only
//! to insert the fixture row and read the status back). Meant to run under
//! `scripts/rls-e2e-gate.sh`, where the server's request path is NOBYPASSRLS.

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
    org_id: String,
}

impl Tenant {
    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("X-Organization-Id", &self.org_id)
            .timeout(Duration::from_secs(20))
    }
}

async fn register(base_url: &str, tag: &str) -> Tenant {
    let client = Client::new();
    let ts = format!("{}{}", tag, Utc::now().timestamp_micros());
    let res = client
        .post(format!("{base_url}/api/v1/auth/register"))
        .json(&json!({
            "username": format!("hmr_{ts}"),
            "email": format!("hmr_{ts}@e2e-test.local"),
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
        .json(&json!({ "name": format!("HM RLS Org {ts}"), "slug": format!("hmr-{ts}") }))
        .send()
        .await
        .expect("create org failed");
    let status = res.status();
    let body: Value = res.json().await.expect("org not JSON");
    assert!(status.is_success(), "create org {status}: {body}");
    let org_id = body["id"].as_str().expect("no org id").to_string();

    Tenant {
        client,
        base_url: base_url.to_string(),
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

async fn insert_deployment(pool: &PgPool, org_id: &str, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, metadata_json) \
         VALUES ($1, $2, 'rls-routes-fixture', $3, '{}'::jsonb, $4, '{}'::jsonb)",
    )
    .bind(id)
    .bind(Uuid::parse_str(org_id).unwrap())
    .bind(format!("hmr-{}", id.simple()))
    .bind(status)
    .execute(pool)
    .await
    .expect("insert hosted_mock failed");
    id
}

/// The deployment-lookup rejection every rerouted route returns for a
/// deployment that is not in the caller's org (or does not exist).
fn is_lookup_rejection(text: &str) -> bool {
    text.contains("Deployment not found") || text.contains("don't have access to this deployment")
}

async fn call(t: &Tenant, method: Method, path: &str) -> (StatusCode, String) {
    let mut rb = t.request(method.clone(), path);
    if method == Method::POST {
        rb = rb.json(&json!({}));
    }
    match rb.send().await {
        Ok(res) => {
            let status = res.status();
            // SSE endpoints stream forever; the status line is all we need.
            if res
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|ct| ct.starts_with("text/event-stream"))
            {
                return (status, String::new());
            }
            let text = tokio::time::timeout(Duration::from_secs(10), res.text())
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            (status, text)
        }
        Err(e) => panic!("{method} {path}: request failed: {e}"),
    }
}

/// Every read-ish route whose handler resolves the deployment through the
/// rerouted lookup. `{id}` is the deployment id.
fn routes() -> Vec<(Method, &'static str)> {
    vec![
        // handlers/hosted_mocks.rs
        (Method::GET, "/api/v1/hosted-mocks/{id}/logs"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/runtime-logs"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/runtime-logs/stream"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/runtime-requests"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/captures"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/captures/status"),
        (
            Method::GET,
            "/api/v1/hosted-mocks/{id}/captures/00000000-0000-0000-0000-000000000001",
        ),
        (
            Method::GET,
            "/api/v1/hosted-mocks/{id}/captures/00000000-0000-0000-0000-000000000001/response",
        ),
        (Method::GET, "/api/v1/hosted-mocks/{id}/captures/export/har"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/metrics"),
        // handlers/otlp.rs
        (Method::GET, "/api/v1/hosted-mocks/{id}/traces"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/traces/0123456789abcdef0123456789abcdef"),
        // authorize_deployment in resilience.rs (also gates /overrides),
        // world_state.rs and time_travel.rs
        (Method::GET, "/api/v1/hosted-mocks/{id}/resilience/summary"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/world-state/snapshot"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/time-travel/status"),
        (Method::GET, "/api/v1/hosted-mocks/{id}/overrides"),
    ]
}

#[tokio::test]
#[ignore]
async fn rerouted_hosted_mock_routes_reach_own_and_reject_foreign_deployment() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let pool = owner_pool().await;

    let owner = register(&base_url, "o").await;
    let stranger = register(&base_url, "s").await;
    let deployment = insert_deployment(&pool, &owner.org_id, "active").await;

    let mut failures = Vec::new();
    for (method, template) in routes() {
        let path = template.replace("{id}", &deployment.to_string());

        let (status, text) = call(&owner, method.clone(), &path).await;
        if is_lookup_rejection(&text) {
            failures.push(format!(
                "OWNER could not see its own deployment: {method} {path} -> {status}: {text}"
            ));
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            failures.push(format!("OWNER denied: {method} {path} -> {status}: {text}"));
        }

        let (status, text) = call(&stranger, method.clone(), &path).await;
        if status.is_success() || !status.is_client_error() || !is_lookup_rejection(&text) {
            failures.push(format!(
                "STRANGER was not rejected by the deployment lookup: {method} {path} -> {status}: {text}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} route check(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[tokio::test]
#[ignore]
async fn custom_domain_routes_reach_own_and_reject_foreign_deployment() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let pool = owner_pool().await;

    let owner = register(&base_url, "do").await;
    let stranger = register(&base_url, "ds").await;
    let deployment = insert_deployment(&pool, &owner.org_id, "active").await;
    let set_path = format!("/api/v1/hosted-mocks/{deployment}/set-domain");
    let clear_path = format!("/api/v1/hosted-mocks/{deployment}/custom-domain");
    let domain = json!({ "domain": format!("rls-{}.e2e-test.local", deployment.simple()) });

    // Stranger first, so a successful write by the owner cannot mask it.
    let res = stranger.request(Method::POST, &set_path).json(&domain).send().await.unwrap();
    let (status, text) = (res.status(), res.text().await.unwrap_or_default());
    assert!(
        status.is_client_error() && is_lookup_rejection(&text),
        "stranger set-domain not rejected by the lookup: {status}: {text}"
    );
    let (status, text) = call(&stranger, Method::DELETE, &clear_path).await;
    assert!(
        status.is_client_error() && is_lookup_rejection(&text),
        "stranger clear-domain not rejected by the lookup: {status}: {text}"
    );

    // Owner gets past the lookup. set-domain may still refuse for plan or
    // DNS reasons in this environment; what must not happen is not-found.
    let res = owner.request(Method::POST, &set_path).json(&domain).send().await.unwrap();
    let (status, text) = (res.status(), res.text().await.unwrap_or_default());
    assert!(!is_lookup_rejection(&text), "owner set-domain hit the lookup: {status}: {text}");
    let (status, text) = call(&owner, Method::DELETE, &clear_path).await;
    assert!(
        !is_lookup_rejection(&text),
        "owner clear-domain hit the lookup: {status}: {text}"
    );

    // Nothing the stranger did touched the row's org.
    let org: Uuid = sqlx::query_scalar("SELECT org_id FROM hosted_mocks WHERE id = $1")
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(org.to_string(), owner.org_id);
}

#[tokio::test]
#[ignore]
async fn redeploy_background_status_write_lands_and_foreign_redeploy_rejected() {
    let base_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let pool = owner_pool().await;

    let owner = register(&base_url, "ro").await;
    let stranger = register(&base_url, "rs").await;
    // `failed` is a redeployable state and makes the transition observable:
    // failed -> deploying (request path) -> active (background task).
    let deployment = insert_deployment(&pool, &owner.org_id, "failed").await;
    // A previously deployed mock has a URL. It also keeps the deployment
    // orchestrator's sweep (which only picks up rows with no URL) from flipping
    // the row to active on its own and masking a broken background write.
    sqlx::query("UPDATE hosted_mocks SET deployment_url = $2 WHERE id = $1")
        .bind(deployment)
        .bind(format!("https://hmr-{}.e2e-test.local", deployment.simple()))
        .execute(&pool)
        .await
        .expect("set deployment_url");
    let path = format!("/api/v1/hosted-mocks/{deployment}/redeploy");

    let (status, text) = call(&stranger, Method::POST, &path).await;
    assert!(
        status.is_client_error() && is_lookup_rejection(&text),
        "stranger redeploy not rejected by the lookup: {status}: {text}"
    );
    let untouched: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id = $1")
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(untouched, "failed", "stranger's redeploy changed the deployment");

    let (status, text) = call(&owner, Method::POST, &path).await;
    assert!(status.is_success(), "owner redeploy {status}: {text}");

    // Poll the owner-side row until the background task marks it active.
    let mut last = String::new();
    for _ in 0..40 {
        last = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id = $1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap();
        if last == "active" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(
        last, "active",
        "background redeploy status write did not land (stuck at '{last}'); under RLS this \
         means the task's runtime-pool write was not bound to the deployment's org"
    );

    // The background task's own completion log, so the transition above is
    // attributable to it and not to another writer.
    let mut completed: i64 = 0;
    for _ in 0..20 {
        completed = sqlx::query_scalar(
            "SELECT count(*) FROM deployment_logs \
             WHERE hosted_mock_id = $1 AND message = 'Redeployment completed successfully'",
        )
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .expect("read deployment_logs");
        if completed > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(completed, 1, "background redeploy task did not complete");

    // And the owner still sees it through the API afterwards.
    let res = owner
        .request(Method::GET, &format!("/api/v1/hosted-mocks/{deployment}"))
        .send()
        .await
        .unwrap();
    let status = res.status();
    let body: Value = res.json().await.unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "owner GET after redeploy: {body}");
    assert_eq!(body["status"], "active", "{body}");
}
