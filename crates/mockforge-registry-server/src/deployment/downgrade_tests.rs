use super::*;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[tokio::test]
#[ignore = "requires isolated PostgreSQL via TEST_DATABASE_URL and local Fly fixture"]
async fn downgraded_fly_stop_retries_without_releasing_capacity_before_confirmation() {
    let pool = PgPool::connect(&std::env::var("TEST_DATABASE_URL").unwrap()).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let user = Uuid::new_v4();
    let org = Uuid::new_v4();
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users(id,username,email,password_hash) VALUES($1,$2,$3,'unusable-fixture')",
    )
    .bind(user)
    .bind(format!("downgrade-{user}"))
    .bind(format!("{user}@example.invalid"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO organizations(id,name,slug,owner_id,plan) VALUES($1,'downgrade fixture',$2,$3,'pro')").bind(org).bind(format!("downgrade-{org}")).bind(user).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO hosted_mocks(id,org_id,name,slug,config_json,status,metadata_json,capacity_reserved) VALUES($1,$2,'fixture','downgrade','{}','active','{\"hosting_backend\":\"flyio\",\"flyio_machine_id\":\"owned-machine\"}',TRUE)").bind(id).bind(org).execute(&pool).await.unwrap();
    let deployment = HostedMock::find_by_id(&pool, id).await.unwrap().unwrap();
    let confirmed = Arc::new(AtomicBool::new(false));
    let stops = Arc::new(AtomicUsize::new(0));
    let stopped = Arc::new(AtomicBool::new(false));
    let path = format!("/v1/apps/{}/machines/owned-machine", deployment.fly_app_name());
    let router = Router::new().route(&path, get({ let stopped=stopped.clone(); move || { let stopped=stopped.clone(); async move { Json(json!({"id":"owned-machine","name":"fixture","state":if stopped.load(Ordering::SeqCst) {"stopped"} else {"started"},"instance_id":"owned-instance","region":"iad","image_ref":null,"config":{"image":"fixture","env":{},"services":[]}})) }}}))
        .route(&format!("{path}/stop"), post({let stops=stops.clone(); move || {let stops=stops.clone(); async move {stops.fetch_add(1,Ordering::SeqCst); StatusCode::OK}}}))
        .route(&format!("{path}/wait"), get({let confirmed=confirmed.clone(); let stopped=stopped.clone(); move || {let confirmed=confirmed.clone(); let stopped=stopped.clone(); async move {if confirmed.load(Ordering::SeqCst) {stopped.store(true,Ordering::SeqCst); (StatusCode::OK,Json(json!({"ok":true})))} else {(StatusCode::SERVICE_UNAVAILABLE,Json(json!({"ok":false})))}}}}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let orchestrator = DeploymentOrchestrator {
        db: Arc::new(pool.clone()),
        flyio_client: Some(FlyioClient::for_test_endpoint(url)),
        flyio_org_slug: None,
        storage: None,
        cloudflare: None,
    };
    orchestrator.stop_downgraded_backend(&deployment).await.unwrap();
    assert_eq!(stops.load(Ordering::SeqCst), 0, "paid deployment must not be touched");
    sqlx::query("UPDATE organizations SET plan='free' WHERE id=$1")
        .bind(org)
        .execute(&pool)
        .await
        .unwrap();
    // Owners can change display status; that must not hide a running backend.
    sqlx::query("UPDATE hosted_mocks SET status='stopped' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(orchestrator.stop_downgraded_backend(&deployment).await.is_err());
    let before: (String, bool) =
        sqlx::query_as("SELECT status,capacity_reserved FROM hosted_mocks WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, ("stopped".into(), true));
    confirmed.store(true, Ordering::SeqCst);
    orchestrator.stop_downgraded_backend(&deployment).await.unwrap();
    assert_eq!(HostedMock::find_by_id(&pool, id).await.unwrap().unwrap().status, "stopped");
    orchestrator.stop_downgraded_backend(&deployment).await.unwrap();
    assert_eq!(
        stops.load(Ordering::SeqCst),
        2,
        "stopped deployment must not repeat provider calls"
    );
    sqlx::query("UPDATE hosted_mocks SET capacity_reserved=FALSE WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    stopped.store(false, Ordering::SeqCst);
    orchestrator.stop_downgraded_backend(&deployment).await.unwrap();
    assert_eq!(
        stops.load(Ordering::SeqCst),
        2,
        "released capacity must not authorize further provider operations"
    );
    task.abort();
    sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(org)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
}
