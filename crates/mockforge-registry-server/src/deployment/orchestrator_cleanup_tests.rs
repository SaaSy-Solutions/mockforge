use super::*;
use crate::{
    database::Database,
    models::Plan,
    store::{PgRegistryStore, RegistryStore},
};
use axum::{
    extract::Request,
    http::{Method, StatusCode},
    routing::any,
    Router,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listener"]
async fn failed_cleanup_retries_legacy_tombstones_without_trusting_display_status() {
    let url = std::env::var("TEST_DATABASE_URL").expect("isolated Postgres required");
    let db = Database::connect(&url).await.unwrap();
    db.migrate().await.unwrap();
    let store = PgRegistryStore::new(db.pool().clone());
    let tag = Uuid::new_v4().simple().to_string();
    let user = store
        .create_user(&tag, &format!("{tag}@example.invalid"), "disabled-test-account")
        .await
        .unwrap();
    let org = store
        .create_organization("Cleanup fixture", &tag, user.id, Plan::Free)
        .await
        .unwrap();
    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("INSERT INTO hosted_mocks (id, org_id, name, slug, config_json, status, deleted_at, deletion_requested_at, metadata_json) VALUES ($1, $2, 'Cleanup fixture', $3, '{}'::jsonb, $4, CASE WHEN $5 THEN NOW() - INTERVAL '1 day' ELSE NULL END, CASE WHEN $5 THEN NOW() - INTERVAL '1 day' ELSE NULL END, '{}'::jsonb)")
            .bind(id).bind(org.id).bind(id.simple().to_string())
            .bind(if index == 0 { "active" } else { "deleting" })
            .bind(index == 1).execute(db.pool()).await.unwrap();
    }
    let legacy_deleted: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT deleted_at FROM hosted_mocks WHERE id = $1")
            .bind(ids[1])
            .fetch_one(db.pool())
            .await
            .unwrap();
    let fail = Arc::new(AtomicBool::new(true));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let response_fail = fail.clone();
    let captured = calls.clone();
    let router = Router::new().fallback(any(move |request: Request| {
        let fail = response_fail.clone();
        let calls = captured.clone();
        async move {
            let method = request.method().clone();
            calls.lock().unwrap().push((method.clone(), request.uri().to_string()));
            if method == Method::DELETE {
                StatusCode::ACCEPTED
            } else if fail.load(Ordering::SeqCst) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::NOT_FOUND
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let worker = DeploymentOrchestrator {
        cloudflare: None,
        db: Arc::new(db.pool().clone()),
        flyio_client: Some(FlyioClient::for_test_endpoint(endpoint)),
        storage: None,
        flyio_org_slug: None,
    };
    assert!(worker.delete_deployment(ids[0]).await.is_err());
    let reserved: bool =
        sqlx::query_scalar("SELECT capacity_reserved FROM hosted_mocks WHERE id = $1")
            .bind(ids[0])
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert!(reserved);
    fail.store(false, Ordering::SeqCst);
    worker.process_requested_deletions().await.unwrap();
    for id in &ids[..2] {
        let row: (bool, bool) = sqlx::query_as(
            "SELECT deleted_at IS NOT NULL, capacity_reserved FROM hosted_mocks WHERE id = $1",
        )
        .bind(id)
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(row, (true, false));
    }
    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT deleted_at FROM hosted_mocks WHERE id = $1")
            .bind(ids[1])
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(after, legacy_deleted);
    let untouched: (bool, bool, bool) = sqlx::query_as("SELECT deleted_at IS NULL, capacity_reserved, deletion_requested_at IS NULL FROM hosted_mocks WHERE id = $1")
        .bind(ids[2]).fetch_one(db.pool()).await.unwrap();
    assert_eq!(untouched, (true, true, true));
    let before = calls.lock().unwrap().len();
    worker.process_requested_deletions().await.unwrap();
    assert_eq!(calls.lock().unwrap().len(), before);
    assert!(calls
        .lock()
        .unwrap()
        .iter()
        .all(|(_, uri)| !uri.contains(&ids[2].simple().to_string())));
    server.abort();
}
