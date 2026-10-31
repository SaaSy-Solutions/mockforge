use super::*;
use crate::{
    circuit_breaker::CircuitBreakerRegistry,
    config::Config,
    database::Database,
    handlers::settings::encrypt_api_key,
    models::Plan,
    storage::PluginStorage,
    store::{PgRegistryStore, RegistryStore},
};
use axum::{routing::post, Json, Router};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn state() -> AppState {
    let db = Database::connect(&std::env::var("TEST_DATABASE_URL").expect("owned DB required"))
        .await
        .unwrap();
    db.migrate().await.unwrap();
    let config = Config::load().unwrap();
    let storage = PluginStorage::new(&config).await.unwrap();
    let store = Arc::new(PgRegistryStore::new(db.runtime_pool().clone()));
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

async fn org(state: &AppState) -> Organization {
    let store = PgRegistryStore::new(state.db.pool().clone());
    let tag = Uuid::new_v4().simple().to_string();
    let user = store
        .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-login")
        .await
        .unwrap();
    store
        .create_organization("Worker fixture", &tag, user.id, Plan::Free)
        .await
        .unwrap()
}

async fn workspace(state: &AppState, org: &Organization) -> Uuid {
    sqlx::query_scalar("INSERT INTO workspaces(org_id,name,created_by) VALUES($1,'Worker fixture',$2) RETURNING id")
        .bind(org.id).bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap()
}

async fn job(state: &AppState, org: &Organization, workspace: Uuid) -> TestGenerationJob {
    let id: Uuid = sqlx::query_scalar("INSERT INTO cloud_test_generation_jobs(org_id,workspace_id,created_by,status) VALUES($1,$2,$3,'running') RETURNING id")
        .bind(org.id).bind(workspace).bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap();
    TestGenerationJob::find_in_workspace(state.db.pool(), workspace, id)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL with NOBYPASSRLS request role and owned provider"]
async fn actual_byok_worker_serves_owned_corpus_and_rejects_invalid_job_context() {
    let state = state().await;
    let own = org(&state).await;
    let other = org(&state).await;
    let own_workspace = workspace(&state, &own).await;
    let foreign_workspace = workspace(&state, &other).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let router = Router::new().route(
        "/v1/chat/completions",
        post(move |Json(body): Json<Value>| {
            let seen = seen.clone();
            async move {
                let messages = body["messages"].to_string();
                assert!(messages.contains("/owned-worker-path"));
                assert!(messages.contains("3000000000"));
                assert!(!messages.contains("foreign-workspace-marker"));
                seen.fetch_add(1, Ordering::SeqCst);
                Json(json!({"choices":[{"message":{"content":"{\"scenarios\":[]}"}}],
                "usage":{"prompt_tokens":2,"completion_tokens":3}}))
            }
        }),
    );
    let _server = Server(tokio::spawn(async move { axum::serve(listener, router).await.unwrap() }));
    let config = BYOKConfig {
        provider: "openai".into(),
        api_key: encrypt_api_key("synthetic-worker-key").unwrap(),
        base_url: Some(endpoint),
        model: Some("fixture".into()),
        enabled: true,
    };
    sqlx::query("INSERT INTO org_settings(org_id,setting_key,setting_value) VALUES($1,'byok',$2)")
        .bind(own.id)
        .bind(serde_json::to_value(config).unwrap())
        .execute(state.db.pool())
        .await
        .unwrap();
    let deployment: Uuid = sqlx::query_scalar("INSERT INTO hosted_mocks(org_id,name,slug,config_json,status) VALUES($1,'Worker fixture',$2,'{}','active') RETURNING id")
        .bind(own.id).bind(Uuid::new_v4().simple().to_string())
        .fetch_one(state.db.pool()).await.unwrap();
    sqlx::query("INSERT INTO runtime_captures(deployment_id,workspace_id,capture_id,protocol,occurred_at,method,path,request_headers,request_body_encoding,status_code,duration_ms) VALUES($1,$2,$3,'http',NOW(),'GET','/owned-worker-path','{}','utf8',200,3000000000)")
        .bind(deployment).bind(own_workspace).bind(Uuid::new_v4().to_string())
        .execute(state.db.pool()).await.unwrap();
    let valid = job(&state, &own, own_workspace).await;
    let result = process_job(&state, valid.clone())
        .await
        .expect("owned BYOK worker must execute");
    assert_eq!(result["model_meta"]["billing"], "byok");
    assert_eq!(result["captures_sampled"], 1);
    let invalid = job(&state, &own, foreign_workspace).await;
    assert!(matches!(process_job(&state, invalid).await, Err(WorkerError::ContextDenied)));
    let mut forged = valid.clone();
    forged.created_by = Some(other.owner_id);
    assert!(matches!(process_job(&state, forged).await, Err(WorkerError::ContextDenied)));
    sqlx::query("UPDATE cloud_test_generation_jobs SET status='cancelled' WHERE id=$1")
        .bind(valid.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(matches!(process_job(&state, valid).await, Err(WorkerError::ContextDenied)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let held: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
            .bind(own.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(held, 0, "BYOK must not reserve platform credits");

    // Exercise real claims and persistence through two competing queue drains.
    let queued = job(&state, &own, own_workspace).await;
    let denied = job(&state, &own, foreign_workspace).await;
    sqlx::query("UPDATE cloud_test_generation_jobs SET status='queued' WHERE id=ANY($1)")
        .bind(vec![queued.id, denied.id])
        .execute(state.db.pool())
        .await
        .unwrap();
    let (first, second) = tokio::time::timeout(Duration::from_secs(8), async {
        tokio::join!(drain_queue(&state, 2), drain_queue(&state, 2))
    })
    .await
    .expect("owned competing queue drains timed out");
    first.unwrap();
    second.unwrap();
    let completed = TestGenerationJob::find_in_workspace(state.db.pool(), own_workspace, queued.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, "succeeded");
    assert_eq!(completed.result.unwrap()["model_meta"]["billing"], "byok");
    let rejected =
        TestGenerationJob::find_in_workspace(state.db.pool(), foreign_workspace, denied.id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(rejected.status, "failed");
    assert_eq!(
        rejected.error.as_deref(),
        Some("Test-generation job context is no longer authorized")
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "competing claims must execute the owned queued call exactly once"
    );
}
