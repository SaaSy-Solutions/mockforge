//! Free monthly request cap: atomic admission, error accounting, overrides and tenant isolation.

use super::*;

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
    sqlx::raw_sql(include_str!("../../../migrations/20250101000090_free_hosted_tier.sql"))
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
#[ignore = "requires isolated Postgres and MOCKFORGE_INTERNAL_API_TOKEN fixture"]
async fn downgraded_cloudflare_mock_cannot_bypass_free_cap() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
    };
    use chrono::Datelike;
    use mockforge_registry_server::handlers::internal_hosted_mocks::{
        get_runtime, report_usage, UsageReport,
    };
    let state = state().await;
    let org = organization(&state, Plan::Pro).await;
    let slug = deployment(&state, &org, "https://fixture.invalid").await;
    let id: Uuid = sqlx::query_scalar("UPDATE hosted_mocks SET deployment_url='https://fixture.invalid', metadata_json='{\"backend\":\"cloudflare\",\"hosting_backend\":\"cloudflare\"}'::jsonb WHERE org_id=$1 AND slug=$2 RETURNING id")
        .bind(org.id).bind(&slug).fetch_one(state.db.pool()).await.unwrap();
    let mut headers = HeaderMap::new();
    let token =
        std::env::var("MOCKFORGE_INTERNAL_API_TOKEN").expect("local internal token required");
    headers.insert("authorization", format!("Bearer {token}").parse().unwrap());
    assert!(get_runtime(State(state.clone()), Path((org.id, slug.clone())), headers.clone())
        .await
        .is_ok());
    let proxy = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let url = format!("{}/mocks/{}/{}", proxy.url, org.id, slug);
    assert_eq!(client.get(&url).send().await.unwrap().status(), StatusCode::TEMPORARY_REDIRECT);
    sqlx::query("UPDATE organizations SET plan='free' WHERE id=$1")
        .bind(org.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(get_runtime(State(state.clone()), Path((org.id, slug)), headers.clone())
        .await
        .is_err());
    let response = client.get(&url).send().await.unwrap();
    assert!(!response.status().is_success() && !response.status().is_redirection());
    assert!(response.headers().get("location").is_none());
    let result = report_usage(
        State(state.clone()),
        Path(id),
        headers,
        axum::Json(UsageReport {
            batch_id: Uuid::new_v4(),
            period_start: chrono::Utc::now().date_naive().with_day(1).unwrap(),
            requests: 1,
            egress_bytes: 1,
        }),
    )
    .await
    .unwrap();
    assert!(result.blocked, "usage acknowledgement must stop an already cached edge runtime");
}
