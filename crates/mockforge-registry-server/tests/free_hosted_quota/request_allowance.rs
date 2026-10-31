use super::{deployment, organization, requests, serve, state};
use axum::{routing::get, Router};
use mockforge_registry_server::{
    deployment::router::{custom_domain_fallback, MultitenantRouter},
    models::Plan,
    store::with_org_context,
};
use reqwest::StatusCode;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned HTTP receivers"]
async fn free_owner_request_pool_is_atomic_across_org_routes_and_survives_org_deletion() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let second = state
        .store
        .create_organization(
            "Shared HTTP owner",
            &Uuid::new_v4().simple().to_string(),
            first.owner_id,
            Plan::Free,
        )
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let upstream = serve(Router::new().route(
        "/",
        get(move || {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                "owned request fixture"
            }
        }),
    ))
    .await;
    let a = deployment(&state, &first, &upstream.url).await;
    let b = deployment(&state, &second, &upstream.url).await;
    sqlx::query("INSERT INTO free_hosted_request_periods(owner_user_id,period_start,granted_requests,requests) VALUES($1,date_trunc('month',NOW())::date,10000,9999)").bind(first.owner_id).execute(state.db.pool()).await.unwrap();
    let proxy = serve(
        MultitenantRouter::create_router()
            .fallback(custom_domain_fallback)
            .with_state(state.clone()),
    )
    .await;
    let domain = std::env::var("MOCKFORGE_MOCKS_DOMAIN").unwrap();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let (left, right) = tokio::join!(
        client.get(format!("{}/mocks/{}/{}", proxy.url, first.id, a)).send(),
        client
            .get(format!("{}/", proxy.url))
            .header("Host", format!("{b}.{domain}"))
            .send()
    );
    let left = left.unwrap();
    let right = right.unwrap();
    let statuses = [left.status(), right.status()];
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1);
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::TOO_MANY_REQUESTS).count(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(requests(&state, first.id).await + requests(&state, second.id).await, 1);
    let used: i64 = sqlx::query_scalar(
        "SELECT requests FROM free_hosted_request_periods WHERE owner_user_id=$1",
    )
    .bind(first.owner_id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(used, 10000);
    // Wait for the one response's egress write before deleting its synthetic row.
    tokio::time::timeout(std::time::Duration::from_secs(5),async {loop {
 let egress:i64=sqlx::query_scalar("SELECT COALESCE(SUM(egress_bytes),0)::bigint FROM usage_counters WHERE org_id IN ($1,$2)").bind(first.id).bind(second.id).fetch_one(state.db.pool()).await.unwrap();if egress>0 {break;}tokio::time::sleep(std::time::Duration::from_millis(10)).await;
 }}).await.unwrap();
    let spent = if left.status() == StatusCode::OK {
        first.id
    } else {
        second.id
    };
    // This owned fixture has no real deployment machine. Remove its synthetic
    // deployment record; real provider cleanup is covered by separate tests.
    sqlx::query("DELETE FROM hosted_mocks WHERE org_id=$1")
        .bind(spent)
        .execute(state.db.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM organizations WHERE id=$1")
        .bind(spent)
        .execute(state.db.pool())
        .await
        .unwrap();
    let fresh = state
        .store
        .create_organization(
            "Fresh HTTP org",
            &Uuid::new_v4().simple().to_string(),
            first.owner_id,
            Plan::Free,
        )
        .await
        .unwrap();
    let slug = deployment(&state, &fresh, &upstream.url).await;
    assert_eq!(
        client
            .get(format!("{}/mocks/{}/{}", proxy.url, fresh.id, slug))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let other = organization(&state, Plan::Free).await;
    let other_slug = deployment(&state, &other, &upstream.url).await;
    assert_eq!(
        client
            .get(format!("{}/mocks/{}/{}", proxy.url, other.id, other_slug))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let owner = first.owner_id;
    let hidden: i64 = with_org_context(state.db.runtime_pool(), other.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar(
                "SELECT COUNT(*) FROM free_hosted_request_periods WHERE owner_user_id=$1",
            )
            .bind(owner)
            .fetch_one(&mut **tx)
            .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(hidden, 0);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned HTTP receivers"]
async fn verified_account_recreation_preserves_request_limit_and_private_identity() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(first.owner_id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO free_hosted_request_periods(owner_user_id,period_start,granted_requests,requests) VALUES($1,date_trunc('month',NOW())::date,10000,10000)")
        .bind(first.owner_id).execute(state.db.pool()).await.unwrap();
    state.store.delete_user_data_cascade(first.owner_id).await.unwrap();
    let tag = Uuid::new_v4().simple().to_string();
    let user = state
        .store
        .create_user(&format!("recreated-{tag}"), &email, "disabled-test-account")
        .await
        .unwrap();
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(user.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let fresh = state
        .store
        .create_organization("Recreated HTTP owner", &tag, user.id, Plan::Free)
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let upstream = serve(Router::new().route(
        "/",
        get(move || {
            let counted = counted.clone();
            async move {
                counted.fetch_add(1, Ordering::SeqCst);
                "fixture"
            }
        }),
    ))
    .await;
    let slug = deployment(&state, &fresh, &upstream.url).await;
    let proxy = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
    let response = reqwest::Client::new()
        .get(format!("{}/mocks/{}/{}", proxy.url, fresh.id, slug))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let hidden: i64 = with_org_context(state.db.runtime_pool(), fresh.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM free_verified_identities")
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(hidden, 0, "request role cannot enumerate identity bindings");
    let secret: i64 = with_org_context(state.db.runtime_pool(), fresh.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT COUNT(*) FROM free_identity_secret")
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert_eq!(secret, 0, "request role cannot read identity salt");
}
