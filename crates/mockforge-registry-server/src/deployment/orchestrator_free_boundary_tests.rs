use super::*;
use crate::{
    database::Database,
    models::Plan,
    store::{PgRegistryStore, RegistryStore},
};
use axum::{http::StatusCode, routing::any, Router};
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listener"]
async fn free_deployment_never_calls_configured_provider_and_invalid_routes_stay_inactive() {
    let db = Database::connect(&std::env::var("TEST_DATABASE_URL").unwrap()).await.unwrap();
    db.migrate().await.unwrap();
    let store = PgRegistryStore::new(db.pool().clone());
    let tag = Uuid::new_v4().simple().to_string();
    let user = store
        .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-test-account")
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let captured = calls.clone();
    let provider = Router::new().fallback(any(move || {
        let calls = captured.clone();
        async move {
            calls.fetch_add(1, Ordering::SeqCst);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, provider).await.unwrap();
    });
    let orchestrator = DeploymentOrchestrator {
        db: Arc::new(db.pool().clone()),
        flyio_client: Some(FlyioClient::for_test_endpoint(endpoint.clone())),
        cloudflare: CloudflareMocks::new(&endpoint, "fixture-edge-token"),
        storage: None,
        flyio_org_slug: Some("fixture".into()),
    };
    for (index, config) in [
        serde_json::json!({"routes":[{"path":"/users/{id}","method":"GET","response":{"status":200,"body":{"name":"Ada"}}}]}),
        serde_json::json!({"routes":[{"path":"/:id","method":"GET","response":{"status":200}}]}),
        serde_json::json!({"routes":[{"path":"/","method":"GET","response":{"status":200,"headers":{"Set-Cookie":"mock-cookie=fixture"}}}]}),
    ].into_iter().enumerate() {
        let label = Uuid::new_v4().simple().to_string();
        let org = store.create_organization("Provider boundary", &label, user.id, Plan::Free).await.unwrap();
        let id: Uuid = sqlx::query_scalar("INSERT INTO hosted_mocks (org_id,name,slug,config_json,status) VALUES ($1,'Provider boundary',$2,$3,'pending') RETURNING id")
            .bind(org.id).bind(&label).bind(config).fetch_one(db.pool()).await.unwrap();
        let deployment = HostedMock::find_by_id(db.pool(), id).await.unwrap().unwrap();
        let lifecycle = crate::handlers::hosted_capacity::LifecycleGuard::acquire(db.pool(), org.id, id).await.unwrap();
        orchestrator.stop_downgraded_backend(&deployment).await.expect("no external intent must skip cleanup without acquiring the held lifecycle lock");
        drop(lifecycle);
        assert_eq!(calls.load(Ordering::SeqCst), 0, "unprovisioned Free deployment has no external backend to stop");
        assert_eq!(HostedMock::find_by_id(db.pool(), id).await.unwrap().unwrap().status, "pending");
        let result = orchestrator.deploy(&deployment).await;
        if index == 0 { result.unwrap(); } else { assert!(result.is_err()); }
        let stored: (String, Option<String>, String, bool) = sqlx::query_as("SELECT status, internal_url, metadata_json->>'hosting_backend', capacity_reserved FROM hosted_mocks WHERE id=$1")
            .bind(id).fetch_one(db.pool()).await.unwrap();
        assert_eq!(stored.0 == "active", index == 0);
        assert_eq!(stored.1, None, "Free must not create a public provider origin");
        assert_eq!(stored.2, "multitenant");
        assert!(stored.3, "invalid config must not lose its pending cleanup reservation");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}
