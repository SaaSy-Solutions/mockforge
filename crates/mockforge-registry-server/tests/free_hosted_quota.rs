//! Real Postgres and HTTP boundary tests for the Free hosted request cap.
//! Run explicitly against an isolated DB with TEST_DATABASE_URL, DATABASE_URL,
//! and JWT_SECRET configured; the test accounts have unusable login hashes.

use std::sync::Arc;

use axum::{routing::get, Router};
use mockforge_registry_server::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    deployment::router::{custom_domain_fallback, MultitenantRouter},
    models::{Organization, Plan, UsageCounter},
    storage::PluginStorage,
    store::{with_org_context, PgRegistryStore, RegistryStore, CURRENT_ORG},
    AppState,
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

#[path = "free_hosted_quota/ai_attribution.rs"]
mod attribution;

#[path = "free_hosted_quota/verified_identity.rs"]
mod verified_identity;

#[path = "free_hosted_quota/credit_reconciliation.rs"]
mod credit_reconciliation;

#[path = "free_hosted_quota/account_allowance.rs"]
mod account_allowance;

#[path = "free_hosted_quota/request_allowance.rs"]
mod request_allowance;

#[path = "free_hosted_quota/owner_capacity.rs"]
mod owner_capacity;

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(router: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Server { url, task }
}

async fn state() -> AppState {
    let url = std::env::var("TEST_DATABASE_URL").expect("isolated TEST_DATABASE_URL required");
    let db = Database::connect(&url).await.unwrap();
    db.migrate().await.unwrap();
    let config = Config::load().unwrap();
    let storage = PluginStorage::new(&config).await.unwrap();
    let store = Arc::new(
        PgRegistryStore::new(db.runtime_pool().clone()).with_owner_pool(db.pool().clone()),
    );
    AppState {
        db,
        config,
        storage,
        store,
        metrics: Arc::new(mockforge_observability::prometheus::MetricsRegistry::new()),
        analytics_db: None,
        redis: None,
        circuit_breakers: CircuitBreakerRegistry::new(),
        platform_signing: None,
    }
}

async fn organization(state: &AppState, plan: Plan) -> Organization {
    let store = PgRegistryStore::new(state.db.pool().clone());
    let tag = Uuid::new_v4().simple().to_string();
    let owner = store
        .create_user(
            &format!("quota-{tag}"),
            &format!("{tag}@example.invalid"),
            "disabled-test-account",
        )
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(owner.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    store.create_organization("Quota fixture", &tag, owner.id, plan).await.unwrap()
}

async fn deployment(state: &AppState, org: &Organization, target: &str) -> String {
    let slug = format!("quota-{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO hosted_mocks (org_id, name, slug, config_json, status, internal_url) VALUES ($1, 'Quota fixture', $2, '{}'::jsonb, 'active', $3)",
    )
    .bind(org.id)
    .bind(&slug)
    .bind(target)
    .execute(state.db.pool())
    .await
    .unwrap();
    slug
}

async fn set_limit(state: &AppState, org: &Organization, limit: i64) {
    sqlx::query("UPDATE organizations SET limits_json = jsonb_set(limits_json, '{requests_per_30d}', to_jsonb($2::bigint)) WHERE id = $1")
        .bind(org.id)
        .bind(limit)
        .execute(state.db.pool())
        .await
        .unwrap();
}

async fn requests(state: &AppState, org_id: Uuid) -> i64 {
    with_org_context(state.db.runtime_pool(), org_id, |tx| {
        Box::pin(async move {
            Ok(UsageCounter::get_or_create_current(&mut **tx, org_id).await?.requests)
        })
    })
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local provider HTTP listener"]
async fn platform_ai_reserves_before_call_and_settles_after_caller_cancellation() {
    use mockforge_registry_server::ai::{
        reservation::call_with_budget, LlmCall, ProviderSelection,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    sqlx::query("UPDATE organizations SET limits_json=jsonb_set(limits_json,'{ai_tokens_per_month}','1800') WHERE id=$1").bind(org.id).execute(state.db.pool()).await.unwrap();
    let org = Organization::find_by_id(state.db.pool(), org.id).await.unwrap().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let observed = Arc::new(tokio::sync::Notify::new());
    let provider = serve(Router::new().route("/v1/chat/completions", axum::routing::post({
        let calls = calls.clone(); let gate = gate.clone(); let observed = observed.clone();
        move || { let calls = calls.clone(); let gate = gate.clone(); let observed = observed.clone(); async move {
            calls.fetch_add(1, Ordering::SeqCst);
            observed.notify_one();
            gate.acquire().await.unwrap().forget();
            axum::Json(json!({"choices":[{"message":{"content":"done"}}],"usage":{"prompt_tokens":10,"completion_tokens":20}}))
        }}
    }))).await;
    let call = || LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(provider.url.clone()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 1000,
    };
    let first = tokio::spawn({
        let state = state.clone();
        let org = org.clone();
        let call = call();
        async move { call_with_budget(&state, &org, ProviderSelection::Platform, call).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.notified())
        .await
        .unwrap();
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call())
        .await
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1, "denied request must not contact provider");
    sqlx::query("UPDATE users SET is_admin=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let reservation: Uuid = sqlx::query_scalar(
        "SELECT id FROM ai_token_reservations WHERE org_id=$1 AND state='pending'",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    let recover =
        || mockforge_registry_server::handlers::ai_credit_reconciliation::ReconcileRequest {
            actual_tokens: 0,
            evidence_reference: "fixture-premature-evidence".into(),
        };
    let rejected = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        mockforge_registry_server::handlers::ai_credit_reconciliation::reconcile(
            axum::extract::State(state.clone()),
            mockforge_registry_server::middleware::AuthUser(org.owner_id),
            axum::extract::Path((org.id, reservation)),
            axum::Json(recover()),
        ),
    )
    .await
    .expect("live execution recovery must fail without waiting for provider");
    assert!(rejected.is_err(), "active provider call cannot be reconciled");
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    gate.add_permits(1);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let used: i64 =
            sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE org_id=$1")
                .bind(org.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        if used == 30 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "canceled caller lost provider settlement"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    gate.add_permits(1);
    assert_eq!(
        call_with_budget(&state, &org, ProviderSelection::Platform, call())
            .await
            .unwrap()
            .total_tokens(),
        30
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let completed: i64=sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1 AND state='completed' AND actual_tokens=30").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(completed, 2);
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local provider HTTP listener"]
async fn platform_ai_retains_unknown_usage_and_settles_the_admitted_period() {
    use mockforge_registry_server::ai::{
        reservation::call_with_budget, LlmCall, ProviderSelection,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let observed = Arc::new(tokio::sync::Notify::new());
    let provider=serve(Router::new()
        .route("/missing/v1/chat/completions",axum::routing::post(||async { axum::Json(json!({"choices":[{"message":{"content":"unmetered"}}]})) }))
        .route("/valid/v1/chat/completions",axum::routing::post({ let gate=gate.clone();let observed=observed.clone();move ||{ let gate=gate.clone();let observed=observed.clone();async move {
            observed.notify_one();gate.acquire().await.unwrap().forget();
            axum::Json(json!({"choices":[{"message":{"content":"done"}}],"usage":{"prompt_tokens":10,"completion_tokens":20}}))
        }}}))).await;
    let call = |path| LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(format!("{}/{path}", provider.url)),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 1000,
    };
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call("missing"))
        .await
        .is_err());
    let uncertain:i64=sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1 AND state='uncertain' AND actual_tokens IS NULL").bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(uncertain, 1);
    let other = organization(&state, Plan::Free).await;
    let visible: i64 = with_org_context(state.db.runtime_pool(), other.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
                .bind(org.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(visible, 0, "another tenant cannot see held credits");
    let deletion = sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(org.id)
        .execute(state.db.pool())
        .await
        .unwrap_err();
    assert_eq!(deletion.as_database_error().unwrap().code().as_deref(), Some("23514"));
    let counter: Uuid = sqlx::query_scalar("SELECT id FROM usage_counters WHERE org_id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    let first = tokio::spawn({
        let state = state.clone();
        let org = org.clone();
        let call = call("valid");
        async move { call_with_budget(&state, &org, ProviderSelection::Platform, call).await }
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), observed.notified())
        .await
        .unwrap();
    sqlx::query("UPDATE usage_counters SET period_start=(date_trunc('month',NOW())-INTERVAL '1 month')::date WHERE id=$1").bind(counter).execute(state.db.pool()).await.unwrap();
    let shared: Uuid =
        sqlx::query_scalar("SELECT id FROM free_ai_credit_periods WHERE owner_user_id=$1")
            .bind(org.owner_id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    sqlx::query("UPDATE free_ai_credit_periods SET period_start=(date_trunc('month',NOW())-INTERVAL '1 month')::date WHERE id=$1")
        .bind(shared).execute(state.db.pool()).await.unwrap();
    let current_shared: Uuid = sqlx::query_scalar("INSERT INTO free_ai_credit_periods(owner_user_id,period_start,granted_tokens) VALUES($1,date_trunc('month',NOW())::date,5000) RETURNING id")
        .bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap();
    with_org_context(state.db.runtime_pool(), org.id, |tx| {
        Box::pin(async move { Ok(UsageCounter::get_or_create_current(&mut **tx, org.id).await?) })
    })
    .await
    .unwrap();
    gate.add_permits(1);
    assert_eq!(first.await.unwrap().unwrap().total_tokens(), 30);
    let old: i64 = sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE id=$1")
        .bind(counter)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(old, 30);
    let current: i64 =
        sqlx::query_scalar("SELECT ai_tokens_used FROM usage_counters WHERE org_id=$1 AND id<>$2")
            .bind(org.id)
            .bind(counter)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(current, 0);
    let original_shared_used: i64 =
        sqlx::query_scalar("SELECT used_tokens FROM free_ai_credit_periods WHERE id=$1")
            .bind(shared)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(original_shared_used, 30);
    let current_shared_used: (i64, i64) = sqlx::query_as(
        "SELECT used_tokens,reserved_tokens FROM free_ai_credit_periods WHERE id=$1",
    )
    .bind(current_shared)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(current_shared_used, (0, 0));
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn hosted_project_reference_cannot_cross_tenants_or_delete_cleanup_records() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let other = organization(&state, Plan::Free).await;
    let foreign: Uuid = sqlx::query_scalar("INSERT INTO projects(org_id,slug,name) VALUES ($1,'foreign','Foreign project') RETURNING id").bind(other.id).fetch_one(state.db.pool()).await.unwrap();
    let own: Uuid = sqlx::query_scalar(
        "INSERT INTO projects(org_id,slug,name) VALUES ($1,'own','Own project') RETURNING id",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let input = |project| {
        serde_json::from_value::<CreateDeploymentRequest>(
            json!({"name":"Project fixture","project_id":project,"config_json":{}}),
        )
        .unwrap()
    };
    let denied = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(input(foreign)),
            ),
        )
        .await
        .unwrap_err();
    assert!(matches!(denied, mockforge_registry_server::error::ApiError::InvalidRequest(_)));
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers,
                Json(input(own)),
            ),
        )
        .await
        .unwrap()
        .0;
    // The database also rejects an elevated writer that bypasses request RLS.
    let error = sqlx::query("UPDATE hosted_mocks SET project_id=$2 WHERE id=$1")
        .bind(created.id)
        .bind(foreign)
        .execute(state.db.pool())
        .await
        .unwrap_err();
    assert_eq!(error.as_database_error().unwrap().code().as_deref(), Some("23503"));
    sqlx::query("DELETE FROM projects WHERE id=$1")
        .bind(own)
        .execute(state.db.pool())
        .await
        .unwrap();
    let remaining: (Uuid, Option<Uuid>, bool) =
        sqlx::query_as("SELECT org_id,project_id,capacity_reserved FROM hosted_mocks WHERE id=$1")
            .bind(created.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(remaining, (org.id, None, true));
}

#[tokio::test]
async fn hosted_openapi_ignores_process_and_constructor_validation_overrides() {
    use mockforge_openapi::{
        openapi_routes::{OpenApiRouteRegistry, ValidationMode, ValidationOptions},
        OpenApiSpec,
    };
    let spec = OpenApiSpec::from_json(json!({
        "openapi":"3.0.0", "info":{"title":"Isolation","version":"1"},
        "paths":{"/items":{"post":{
            "x-mockforge-ai":{"enabled":true,"prompt":"Generate a dynamic response"},
            "requestBody":{"required":true,"content":{"application/json":{
                "schema":{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}
            }}},
            "responses":{"201":{"description":"created","content":{"application/json":{
                "schema":{"type":"object","required":["created"],"properties":{"created":{"type":"boolean"},"literal":{"type":"string"}}},"example":{"created":true,"literal":"{{uuid}}"}
            }}}}
        }}}
    })).unwrap();
    let options = ValidationOptions {
        request_mode: ValidationMode::Disabled,
        ..Default::default()
    };
    let server =
        serve(OpenApiRouteRegistry::new_with_options(spec, options).build_hosted_router()).await;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(format!("{}/items", server.url))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let response = client
        .post(format!("{}/items", server.url))
        .header("X-Mockforge-Response-Status", "499")
        .header("X-Mockforge-AI", "true")
        .json(&json!({"name":"widget"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"created":true,"literal":"{{uuid}}"})
    );
    for path in [
        "/ai/generate-spec",
        "/api/ai/generate-spec",
        "/__mockforge/ai/generate-spec",
    ] {
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client
                .post(format!("{}{path}", server.url))
                .json(&json!({"prompt":"Generate a spec"}))
                .send(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), 404, "global AI API must not be mounted in hosted responses");
    }
    assert_eq!(
        client
            .get(format!("{}/openapi.json", server.url))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_uploaded_openapi_serves_durable_examples_and_validates_requests() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        deployment::orchestrator::DeploymentOrchestrator,
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    for use_yaml in [false, true] {
        let state = state().await;
        let org = organization(&state, Plan::Free).await;
        set_limit(&state, &org, 4).await;
        let document = json!({
            "openapi":"3.0.0", "info":{"title":"Owned fixture","version":"1"},
            "paths":{"/widgets": {
                "get":{"responses":{"200":{"description":"ok","content":{
                    "application/json":{"schema":{"type":"object"},"example":{"owner":"Ada"}}
                }}}},
                "post":{"requestBody":{"required":true,"content":{"application/json":{
                    "schema":{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}
                }}},"responses":{"201":{"description":"created","content":{
                    "application/json":{"schema":{"type":"object"},"example":{"created":true}}
                }}}}
            }}
        });
        let source = state
            .storage
            .upload_spec(
                &org.id.to_string(),
                "runtime",
                if use_yaml {
                    serde_yaml::to_string(&document).unwrap().into_bytes()
                } else {
                    serde_json::to_vec(&document).unwrap()
                },
            )
            .await
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
        let input: CreateDeploymentRequest = serde_json::from_value(json!({
            "name":"Owned OpenAPI fixture", "openapi_spec_url":source, "config_json":{}
        }))
        .unwrap();
        let created = CURRENT_ORG
            .scope(
                org.id,
                create_deployment(
                    State(state.clone()),
                    AuthUser(org.owner_id),
                    headers,
                    Json(input),
                ),
            )
            .await
            .unwrap()
            .0;
        let worker = Arc::new(
            DeploymentOrchestrator::new(Arc::new(state.db.pool().clone()), None, None)
                .with_storage(state.storage.clone()),
        )
        .start();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let status: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id=$1")
                .bind(created.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
            if status == "active" {
                break;
            }
            assert!(status != "failed", "uploaded specification provisioning failed");
            assert!(tokio::time::Instant::now() < deadline, "provisioning timed out: {status}");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        let snapshot: Value = sqlx::query_scalar(
            "SELECT metadata_json->'runtime_openapi' FROM hosted_mocks WHERE id=$1",
        )
        .bind(created.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        assert_eq!(snapshot, document);
        // Corrupt the source after provisioning: requests must use the validated snapshot.
        state
            .storage
            .upload_spec(&org.id.to_string(), "runtime", b"invalid document".to_vec())
            .await
            .unwrap();
        let server = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
        let base = format!("{}/mocks/{}/{}", server.url, org.id, created.slug);
        let client = reqwest::Client::new();
        let response = client.get(format!("{base}/widgets")).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.json::<Value>().await.unwrap(), json!({"owner":"Ada"}));
        assert_eq!(
            client
                .post(format!("{base}/widgets"))
                .json(&json!({}))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
        let response = client
            .post(format!("{base}/widgets"))
            .json(&json!({"name":"widget"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        assert_eq!(response.json::<Value>().await.unwrap(), json!({"created":true}));
        assert_eq!(client.get(format!("{base}/openapi.json")).send().await.unwrap().status(), 404);
        assert_eq!(client.get(format!("{base}/widgets")).send().await.unwrap().status(), 429);
        assert_eq!(requests(&state, org.id).await, 4);
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_cap_is_atomic_across_path_and_custom_domain_without_double_counting() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    set_limit(&state, &org, 2).await;
    let upstream = serve(Router::new().route("/", get(|| async { "quota fixture" }))).await;
    let slug = deployment(&state, &org, &upstream.url).await;
    let proxy = serve(
        MultitenantRouter::create_router()
            .fallback(custom_domain_fallback)
            .with_state(state.clone()),
    )
    .await;
    let domain = std::env::var("MOCKFORGE_MOCKS_DOMAIN").expect("fixture mocks domain required");
    let mut calls = tokio::task::JoinSet::new();
    for index in 0..16 {
        let url = if index % 2 == 0 {
            format!("{}/mocks/{}/{}", proxy.url, org.id, slug)
        } else {
            format!("{}/", proxy.url)
        };
        let host = format!("{slug}.{domain}");
        calls.spawn(async move {
            reqwest::Client::new()
                .get(url)
                .header("Host", host)
                .send()
                .await
                .unwrap()
                .status()
        });
    }
    let mut accepted = 0;
    let mut rejected = 0;
    while let Some(result) = calls.join_next().await {
        match result.unwrap() {
            StatusCode::OK => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => rejected += 1,
            unexpected => panic!("unexpected proxy status {unexpected}"),
        }
    }
    assert_eq!((accepted, rejected), (2, 14));
    // Wait for successful-response egress writes, then confirm they did not
    // increment the admission counter a second time.
    for _ in 0..100 {
        let egress: i64 =
            sqlx::query_scalar("SELECT egress_bytes FROM usage_counters WHERE org_id = $1")
                .bind(org.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        if egress == 26 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(requests(&state, org.id).await, 2);
    let egress: i64 =
        sqlx::query_scalar("SELECT egress_bytes FROM usage_counters WHERE org_id = $1")
            .bind(org.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(egress, 26);
    let response = reqwest::get(format!("{}/mocks/{}/{}", proxy.url, org.id, slug)).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["current"], 2);
    assert_eq!(body["max"], 2);
}

#[tokio::test]
#[ignore = "requires isolated Postgres and an unconfigured Fly provider"]
async fn deleting_a_registry_local_deployment_releases_its_slot_for_a_new_mock() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
        Json,
    };
    use mockforge_registry_server::{
        deployment::orchestrator::DeploymentOrchestrator,
        error::ApiError,
        handlers::hosted_mocks::{create_deployment, delete_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    assert!(std::env::var_os("FLYIO_API_TOKEN").is_none());
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let request: CreateDeploymentRequest =
        serde_json::from_value(json!({"name":"Local cleanup", "config_json":{}})).unwrap();
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(request),
            ),
        )
        .await
        .unwrap()
        .0;
    let worker =
        Arc::new(DeploymentOrchestrator::new(Arc::new(state.db.pool().clone()), None, None))
            .start();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let recorded: (String, Option<String>) = sqlx::query_as(
            "SELECT status, metadata_json->>'hosting_backend' FROM hosted_mocks WHERE id = $1",
        )
        .bind(created.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        if recorded == ("active".into(), Some("multitenant".into())) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "local worker did not record its backend intent"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    loop {
        let result = CURRENT_ORG
            .scope(
                org.id,
                delete_deployment(
                    State(state.clone()),
                    AuthUser(org.owner_id),
                    headers.clone(),
                    Path(created.id),
                ),
            )
            .await;
        match result {
            Ok(_) => break,
            Err(ApiError::Conflict(_)) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await
            }
            _ => panic!("registry-local cleanup must succeed without a remote provider"),
        }
    }
    let deleted: (bool, bool) = sqlx::query_as(
        "SELECT deleted_at IS NOT NULL, capacity_reserved FROM hosted_mocks WHERE id = $1",
    )
    .bind(created.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(deleted, (true, false));
    let request: CreateDeploymentRequest =
        serde_json::from_value(json!({"name":"Replacement local mock", "config_json":{}})).unwrap();
    let replacement = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(State(state.clone()), AuthUser(org.owner_id), headers, Json(request)),
        )
        .await
        .unwrap()
        .0;
    assert_ne!(replacement.id, created.id);
}

#[tokio::test]
#[ignore = "requires isolated Postgres and an unconfigured Fly provider"]
async fn lifecycle_actions_without_a_provider_cannot_claim_success_or_release_capacity() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
    };
    use mockforge_registry_server::{
        error::ApiError,
        handlers::hosted_mocks::{delete_deployment, start_deployment, stop_deployment},
        middleware::AuthUser,
    };
    assert!(
        std::env::var_os("FLYIO_API_TOKEN").is_none(),
        "isolated runner must disable Fly"
    );
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    for (status, metadata, expected) in [
        ("active", json!({}), "cannot stop"),
        (
            "active",
            json!({"flyio_machine_id":"fixture-machine"}),
            "provider is not configured",
        ),
        (
            "stopped",
            json!({"flyio_machine_id":"fixture-machine"}),
            "provider is not configured",
        ),
        (
            "pending",
            json!({"flyio_machine_id":"fixture-machine"}),
            "provider is not configured",
        ),
    ] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, deployment_url, metadata_json, capacity_reserved) VALUES ($1, $2, 'Lifecycle fixture', $3, '{}'::jsonb, $4, 'https://fixture.invalid', $5, TRUE)")
            .bind(id).bind(org.id).bind(id.simple().to_string()).bind(status).bind(metadata)
            .execute(state.db.pool()).await.unwrap();
        let result = if status == "active" {
            CURRENT_ORG
                .scope(
                    org.id,
                    stop_deployment(
                        State(state.clone()),
                        AuthUser(org.owner_id),
                        headers.clone(),
                        Path(id),
                    ),
                )
                .await
        } else {
            CURRENT_ORG
                .scope(
                    org.id,
                    start_deployment(
                        State(state.clone()),
                        AuthUser(org.owner_id),
                        headers.clone(),
                        Path(id),
                    ),
                )
                .await
        };
        match result {
            Err(ApiError::InvalidRequest(message)) => {
                assert!(message.contains(expected), "{message}")
            }
            _ => panic!("an unconfigured lifecycle operation must fail explicitly"),
        }
        let deletion = CURRENT_ORG
            .scope(
                org.id,
                delete_deployment(
                    State(state.clone()),
                    AuthUser(org.owner_id),
                    headers.clone(),
                    Path(id),
                ),
            )
            .await;
        match deletion {
            Err(ApiError::InvalidRequest(message)) => {
                assert!(message.contains("cannot confirm cleanup"))
            }
            _ => panic!("missing provider must not hide an unverified deployment"),
        }
        let deleted: bool =
            sqlx::query_scalar("SELECT deleted_at IS NOT NULL FROM hosted_mocks WHERE id = $1")
                .bind(id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert!(!deleted);
        let recorded: (String, bool) =
            sqlx::query_as("SELECT status, capacity_reserved FROM hosted_mocks WHERE id = $1")
                .bind(id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert_eq!(recorded, (status.into(), true));
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_upstream_errors_count_but_paid_overages_keep_responding() {
    let state = state().await;
    let free = organization(&state, Plan::Free).await;
    let paid = organization(&state, Plan::Pro).await;
    set_limit(&state, &free, 1).await;
    set_limit(&state, &paid, 1).await;
    let upstream =
        serve(Router::new().route("/", get(|| async { StatusCode::SERVICE_UNAVAILABLE }))).await;
    let free_slug = deployment(&state, &free, &upstream.url).await;
    let paid_slug = deployment(&state, &paid, &upstream.url).await;
    let proxy = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
    let free_url = format!("{}/mocks/{}/{}", proxy.url, free.id, free_slug);
    assert_eq!(reqwest::get(&free_url).await.unwrap().status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(reqwest::get(&free_url).await.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(requests(&state, free.id).await, 1);
    with_org_context(state.db.runtime_pool(), paid.id, |tx| {
        Box::pin(async move {
            UsageCounter::increment_requests(tx, paid.id, 3).await?;
            Ok(())
        })
    })
    .await
    .unwrap();
    let paid_url = format!("{}/mocks/{}/{}", proxy.url, paid.id, paid_slug);
    for _ in 0..3 {
        assert_eq!(
            reqwest::get(&paid_url).await.unwrap().status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    assert_eq!(requests(&state, paid.id).await, 3);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn free_defaults_and_migration_preserve_custom_grants_and_paid_limits() {
    let state = state().await;
    let new_free = organization(&state, Plan::Free).await;
    assert_eq!(new_free.limits_json["max_hosted_mocks"], 1);
    assert_eq!(new_free.limits_json["ai_tokens_per_month"], 5000);
    let legacy = organization(&state, Plan::Free).await;
    let custom = organization(&state, Plan::Free).await;
    let paid = organization(&state, Plan::Pro).await;
    for (org, limits) in [
        (
            &legacy,
            json!({"hosted_mocks":false,"max_hosted_mocks":0,"ai_tokens_per_month":0,"requests_per_30d":10000,"marker":"preserve"}),
        ),
        (
            &custom,
            json!({"max_hosted_mocks":3,"ai_tokens_per_month":-1,"requests_per_30d":25000}),
        ),
    ] {
        sqlx::query("UPDATE organizations SET limits_json = $2 WHERE id = $1")
            .bind(org.id)
            .bind(limits)
            .execute(state.db.pool())
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!("../migrations/20250101000090_free_hosted_tier.sql"))
        .execute(state.db.pool())
        .await
        .unwrap();
    let legacy = Organization::find_by_id(state.db.pool(), legacy.id).await.unwrap().unwrap();
    assert_eq!(legacy.limits_json["max_hosted_mocks"], 1);
    assert_eq!(legacy.limits_json["ai_tokens_per_month"], 5000);
    assert_eq!(legacy.limits_json["requests_per_30d"], 10000);
    assert_eq!(legacy.limits_json["marker"], "preserve");
    let custom = Organization::find_by_id(state.db.pool(), custom.id).await.unwrap().unwrap();
    assert_eq!(custom.limits_json["max_hosted_mocks"], 3);
    assert_eq!(custom.limits_json["ai_tokens_per_month"], -1);
    let after = Organization::find_by_id(state.db.pool(), paid.id).await.unwrap().unwrap();
    assert_eq!(after.limits_json, paid.limits_json);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn pending_creation_reserves_the_only_free_slot_under_concurrency() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        error::ApiError,
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let mut calls = tokio::task::JoinSet::new();
    for index in 0..16 {
        let state = state.clone();
        let org = org.clone();
        calls.spawn(async move {
            let mut headers = HeaderMap::new();
            headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
            let request: CreateDeploymentRequest = serde_json::from_value(json!({
                "name": format!("Free pending {index}"),
                "slug": format!("pending-{}-{index}", org.id.simple()),
                "config_json": {},
            }))
            .unwrap();
            CURRENT_ORG
                .scope(
                    org.id,
                    create_deployment(State(state), AuthUser(org.owner_id), headers, Json(request)),
                )
                .await
        });
    }
    let mut admitted = 0;
    let mut denied = 0;
    while let Some(result) = calls.join_next().await {
        match result.unwrap() {
            Ok(deployment) => {
                assert_eq!(deployment.0.status, "pending");
                admitted += 1;
            }
            Err(ApiError::ResourceLimitExceeded(_)) => denied += 1,
            Err(error) => panic!("unexpected admission error: {error}"),
        }
    }
    assert_eq!((admitted, denied), (1, 15));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hosted_mocks WHERE org_id = $1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    // Another tenant still receives its own slot.
    let other = organization(&state, Plan::Free).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", other.id.to_string().parse().unwrap());
    let request: CreateDeploymentRequest = serde_json::from_value(json!({
        "name": "Other free tenant", "config_json": {},
    }))
    .unwrap();
    assert!(CURRENT_ORG
        .scope(
            other.id,
            create_deployment(State(state), AuthUser(other.owner_id), headers, Json(request)),
        )
        .await
        .is_ok());
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn zero_request_override_denies_free_traffic_without_using_quota() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    state
        .store
        .set_org_setting(org.id, "quota", json!({"requests_per_30d":0}))
        .await
        .unwrap();
    let upstream = serve(Router::new().route("/", get(|| async { "unreachable" }))).await;
    let slug = deployment(&state, &org, &upstream.url).await;
    let proxy = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
    let response = reqwest::get(format!("{}/mocks/{}/{}", proxy.url, org.id, slug)).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["max"], 0);
    assert_eq!(requests(&state, org.id).await, 0);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn free_ai_allowance_and_canceled_paid_fallback_match_the_quota_response() {
    use axum::{extract::State, http::HeaderMap};
    use mockforge_registry_server::{
        handlers::ai_studio::quota,
        middleware::AuthUser,
        models::{Subscription, SubscriptionStatus},
    };
    let state = state().await;
    let free = organization(&state, Plan::Free).await;
    let paid = organization(&state, Plan::Pro).await;
    let tag = Uuid::new_v4().simple().to_string();
    let now = chrono::Utc::now();
    Subscription::upsert_from_stripe(
        state.db.pool(),
        paid.id,
        &format!("sub_fixture_{tag}"),
        &format!("cus_fixture_{tag}"),
        "price_fixture",
        Plan::Pro,
        SubscriptionStatus::Canceled,
        now,
        now,
        true,
        Some(now),
    )
    .await
    .unwrap();
    for org in [free, paid] {
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
        let before = quota(State(state.clone()), AuthUser(org.owner_id), headers.clone())
            .await
            .unwrap()
            .0;
        assert_eq!(before.provider, "platform");
        assert_eq!(before.tokens_limit, 5000);
        assert!(before.call_allowed);
        with_org_context(state.db.runtime_pool(), org.id, |tx| {
            Box::pin(async move {
                UsageCounter::increment_ai_tokens(tx, org.id, 5000).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
        let after = quota(State(state.clone()), AuthUser(org.owner_id), headers).await.unwrap().0;
        assert_eq!(after.tokens_used_this_period, 5000);
        assert!(!after.call_allowed);
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres with APP_DATABASE_URL set to a NOBYPASSRLS role"]
async fn usage_admission_runtime_role_cannot_see_another_tenants_counter() {
    let state = state().await;
    let bypasses: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(state.db.runtime_pool())
    .await
    .unwrap();
    assert!(!bypasses, "this boundary test must exercise enforced RLS");
    let first = organization(&state, Plan::Free).await;
    let second = organization(&state, Plan::Free).await;
    assert_eq!(requests(&state, second.id).await, 0);
    let visible: i64 = with_org_context(state.db.runtime_pool(), first.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM usage_counters WHERE org_id = $1")
                .bind(second.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(visible, 0);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn owner_status_updates_cannot_release_the_reserved_free_slot() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
        Json,
    };
    use mockforge_registry_server::{
        error::ApiError,
        handlers::hosted_mocks::{
            create_deployment, update_deployment_status, CreateDeploymentRequest,
            UpdateStatusRequest,
        },
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let request: CreateDeploymentRequest = serde_json::from_value(json!({
        "name":"Reserved fixture", "config_json":{},
    }))
    .unwrap();
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(request),
            ),
        )
        .await
        .unwrap()
        .0;
    for display_status in ["stopped", "failed", "deleting"] {
        let request = UpdateStatusRequest {
            status: display_status.into(),
            error_message: None,
            deployment_url: None,
            internal_url: None,
        };
        let changed = CURRENT_ORG
            .scope(
                org.id,
                update_deployment_status(
                    State(state.clone()),
                    AuthUser(org.owner_id),
                    headers.clone(),
                    Path(created.id),
                    Json(request),
                ),
            )
            .await
            .unwrap()
            .0;
        assert_eq!(changed.status, display_status);
        let request: CreateDeploymentRequest = serde_json::from_value(json!({
            "name":format!("Another {display_status}"), "config_json":{},
        }))
        .unwrap();
        let result = CURRENT_ORG
            .scope(
                org.id,
                create_deployment(
                    State(state.clone()),
                    AuthUser(org.owner_id),
                    headers.clone(),
                    Json(request),
                ),
            )
            .await;
        assert!(matches!(result, Err(ApiError::ResourceLimitExceeded(_))));
        let reserved: bool =
            sqlx::query_scalar("SELECT capacity_reserved FROM hosted_mocks WHERE id = $1")
                .bind(created.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
        assert!(reserved);
        let requested: bool = sqlx::query_scalar(
            "SELECT deletion_requested_at IS NOT NULL FROM hosted_mocks WHERE id = $1",
        )
        .bind(created.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        assert!(!requested, "customer-editable status cannot authorize destructive cleanup");
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn legacy_status_flags_do_not_clear_unverified_provider_reservations() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let mut tx = state.db.pool().begin().await.unwrap();
    let mut ids = Vec::new();
    for status in ["active", "stopped", "failed", "deleting"] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, deleted_at) VALUES ($1, $2, 'Legacy fixture', $3, '{}'::jsonb, $4, CASE WHEN $4 = 'deleting' THEN NOW() ELSE NULL END)")
            .bind(id).bind(org.id).bind(id.simple().to_string()).bind(status)
            .execute(&mut *tx).await.unwrap();
        ids.push(id);
    }
    // Recreate the pre-migration table shape in this isolated transaction.
    // Rollback restores its schema and SQLx tracking after the assertions.
    // Main's deferred workspace FK must validate pending inserts before DDL.
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut *tx).await.unwrap();
    sqlx::raw_sql(
        "ALTER TABLE hosted_mocks DROP COLUMN capacity_reserved, DROP COLUMN deletion_requested_at",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../migrations/20250101000091_hosted_capacity_reservations.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    for id in ids {
        let (reserved, requested, deleted): (bool, bool, bool) =
            sqlx::query_as("SELECT capacity_reserved, deletion_requested_at IS NOT NULL, deleted_at IS NOT NULL FROM hosted_mocks WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert!(reserved, "display/deletion flags alone must not certify provider cleanup");
        assert_eq!(requested, deleted, "only legacy soft deletion carries authorized intent");
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn organization_deletion_preserves_unfinished_cleanup_and_serializes_with_admission() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
    };
    use mockforge_registry_server::{
        error::ApiError, handlers::organizations::delete_organization, middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let slug = deployment(&state, &org, "http://fixture.invalid").await;
    let rejected = CURRENT_ORG
        .scope(
            org.id,
            delete_organization(
                State(state.clone()),
                AuthUser(org.owner_id),
                Path(org.id),
                HeaderMap::new(),
            ),
        )
        .await;
    assert!(matches!(rejected, Err(ApiError::Conflict(_))));

    // Neither owner-editable status nor a legacy soft-deletion flag certifies
    // cleanup. The trigger also protects direct elevated database callers.
    for status in ["stopped", "failed", "deleting"] {
        sqlx::query("UPDATE hosted_mocks SET status = $2, deleted_at = NOW() WHERE slug = $1")
            .bind(&slug)
            .bind(status)
            .execute(state.db.pool())
            .await
            .unwrap();
        let error = sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org.id)
            .execute(state.db.pool())
            .await
            .unwrap_err();
        assert_eq!(error.as_database_error().unwrap().code().as_deref(), Some("23514"));
    }
    let retained: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM hosted_mocks WHERE slug = $1)")
            .bind(&slug)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert!(retained);
    sqlx::query("UPDATE hosted_mocks SET capacity_reserved = FALSE WHERE slug = $1")
        .bind(&slug)
        .execute(state.db.pool())
        .await
        .unwrap();
    let deleted = CURRENT_ORG
        .scope(
            org.id,
            delete_organization(
                State(state.clone()),
                AuthUser(org.owner_id),
                Path(org.id),
                HeaderMap::new(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(deleted.0["success"], true);

    // Creation's parent-row lock must prevent a concurrent organization
    // deletion from passing its check before the new reservation commits.
    let concurrent = organization(&state, Plan::Free).await;
    let mut admission = state.db.pool().begin().await.unwrap();
    sqlx::query("SELECT id FROM organizations WHERE id = $1 FOR UPDATE")
        .bind(concurrent.id)
        .execute(&mut *admission)
        .await
        .unwrap();
    let pool = state.db.pool().clone();
    let id = concurrent.id;
    let mut deletion = tokio::spawn(async move {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
    });
    assert!(tokio::time::timeout(std::time::Duration::from_millis(100), &mut deletion)
        .await
        .is_err());
    sqlx::query("INSERT INTO hosted_mocks (org_id, name, slug, config_json, status) VALUES ($1, 'Concurrent fixture', $2, '{}'::jsonb, 'pending')")
        .bind(id).bind(Uuid::new_v4().simple().to_string()).execute(&mut *admission).await.unwrap();
    admission.commit().await.unwrap();
    let error = tokio::time::timeout(std::time::Duration::from_secs(5), deletion)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.as_database_error().unwrap().code().as_deref(), Some("23514"));
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_local_runtime_serves_configured_routes_and_caps_all_public_paths() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
        Json,
    };
    use mockforge_registry_server::{
        deployment::orchestrator::DeploymentOrchestrator,
        handlers::hosted_mocks::{
            create_deployment, start_deployment, stop_deployment, CreateDeploymentRequest,
        },
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    set_limit(&state, &org, 3).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let request: CreateDeploymentRequest = serde_json::from_value(json!({"name":"Hosted runtime", "config_json":{"routes":[
        {"path":"/users/{id}","method":"GET","response":{"status":207,"headers":{"x-mock":"tenant-alpha","content-length":"99999"},"body":{"name":"Ada"}}},
        {"path":"/users/{id}","method":"POST","response":{"status":201,"body":{"created":true}}}
    ]}})).unwrap();
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(request),
            ),
        )
        .await
        .unwrap()
        .0;
    let worker =
        Arc::new(DeploymentOrchestrator::new(Arc::new(state.db.pool().clone()), None, None))
            .start();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id = $1")
            .bind(created.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        if status == "active" {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "local runtime was not provisioned: {status}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let backend: (Option<String>, String) = sqlx::query_as(
        "SELECT internal_url, metadata_json->>'hosting_backend' FROM hosted_mocks WHERE id=$1",
    )
    .bind(created.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(backend, (None, "multitenant".into()));
    let server = serve(
        MultitenantRouter::create_router()
            .fallback(custom_domain_fallback)
            .with_state(state.clone()),
    )
    .await;
    let client = reqwest::Client::new();
    let path = format!("{}/mocks/{}/{}/users/42", server.url, org.id, created.slug);
    let other = organization(&state, Plan::Free).await;
    let other_slug = format!("runtime-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO hosted_mocks (org_id,name,slug,config_json,status,metadata_json) VALUES ($1,'Other tenant',$2,$3,'active','{\"hosting_backend\":\"multitenant\"}'::jsonb)")
        .bind(other.id).bind(&other_slug).bind(json!({"routes":[{"path":"/users/{id}","method":"GET","response":{"status":200,"body":{"name":"Grace"}}}]}))
        .execute(state.db.pool()).await.unwrap();
    let other_path = format!("{}/mocks/{}/{}/users/42", server.url, other.id, other_slug);
    assert_eq!(
        client.get(other_path).send().await.unwrap().json::<Value>().await.unwrap()["name"],
        "Grace"
    );
    assert_eq!(
        client
            .get(format!(
                "{}/mocks/{}/{}/__mockforge/api/config",
                server.url, other.id, other_slug
            ))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let first = client.get(&path).send().await.unwrap();
    assert_eq!(first.status(), 207);
    assert_eq!(first.headers()["x-mock"], "tenant-alpha");
    assert_eq!(first.headers()["x-content-type-options"], "nosniff");
    assert!(first.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .starts_with("sandbox;"));
    assert_eq!(first.json::<Value>().await.unwrap()["name"], "Ada");
    let domain = format!("{}.fixture.invalid", created.slug);
    let second = client
        .post(format!("{}/users/42", server.url))
        .header("Host", &domain)
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), 201);
    assert_eq!(second.json::<Value>().await.unwrap()["created"], true);
    let stopped = CURRENT_ORG
        .scope(
            org.id,
            stop_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Path(created.id),
            ),
        )
        .await
        .unwrap();
    assert_eq!(stopped.0.status, "stopped");
    assert_eq!(client.get(&path).send().await.unwrap().status(), 503);
    let restarted = CURRENT_ORG
        .scope(
            org.id,
            start_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers,
                Path(created.id),
            ),
        )
        .await
        .unwrap();
    assert_eq!(restarted.0.status, "active");
    assert_eq!(client.get(&path).send().await.unwrap().status(), 207);
    assert_eq!(
        client
            .get(format!("{}/users/42", server.url))
            .header("Host", domain)
            .send()
            .await
            .unwrap()
            .status(),
        429
    );
    let used: i64 = sqlx::query_scalar(
        "SELECT requests FROM usage_counters WHERE org_id=$1 ORDER BY period_start DESC LIMIT 1",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(used, 3);
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn hosted_subdomain_names_cannot_collide_across_tenants_or_select_legacy_duplicates() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        error::ApiError,
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
        models::HostedMock,
    };
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let second = organization(&state, Plan::Free).await;
    let slug = format!("shared-{}", Uuid::new_v4().simple());
    let mut handles = Vec::new();
    for org in [first.clone(), second.clone()] {
        let state = state.clone();
        let slug = slug.clone();
        handles.push(tokio::spawn(async move {
            let mut headers = HeaderMap::new();
            headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
            let request: CreateDeploymentRequest = serde_json::from_value(
                json!({"name":"Shared name", "slug":slug, "config_json":{}}),
            )
            .unwrap();
            CURRENT_ORG
                .scope(
                    org.id,
                    create_deployment(State(state), AuthUser(org.owner_id), headers, Json(request)),
                )
                .await
        }));
    }
    let mut admitted = 0;
    let mut conflicts = 0;
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => admitted += 1,
            Err(ApiError::Conflict(_)) => conflicts += 1,
            other => panic!("unexpected namespace result: {other:?}"),
        }
    }
    assert_eq!((admitted, conflicts), (1, 1));
    // Recreate ambiguous pre-migration records only inside a rolled-back
    // transaction. Production routing must fail closed even for legacy rows.
    let mut tx = state.db.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE hosted_mocks DISABLE TRIGGER reserve_global_hosted_slug")
        .execute(&mut *tx)
        .await
        .unwrap();
    let legacy_slug = format!("legacy-{}", Uuid::new_v4().simple());
    for org in [first, second] {
        sqlx::query("INSERT INTO hosted_mocks(org_id,name,slug,config_json,status) VALUES ($1,'Legacy duplicate',$2,'{}'::jsonb,'active')")
            .bind(org.id).bind(&legacy_slug).execute(&mut *tx).await.unwrap();
    }
    assert!(HostedMock::find_active_by_slug(&mut *tx, &legacy_slug).await.unwrap().is_none());
    tx.rollback().await.unwrap();
}
