use super::{organization, serve, state};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use mockforge_registry_server::{
    ai::{check_ai_quota, reservation::call_with_budget, LlmCall, ProviderSelection},
    handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
    middleware::AuthUser,
    models::Plan,
    store::CURRENT_ORG,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test]
#[ignore = "requires isolated PostgreSQL and owned provider"]
async fn unverified_free_owner_cannot_fund_resources_and_verification_enables_admission() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    sqlx::query("UPDATE users SET is_verified=FALSE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let provider = serve(Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Json(json!({"choices":[{"message":{"content":"verified"}}],
                "usage":{"prompt_tokens":2,"completion_tokens":3}}))
            }
        }),
    ))
    .await;
    let call = || LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-identity-key".into(),
        base_url: Some(provider.url.clone()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 100,
    };
    assert!(call_with_budget(&state, &org, ProviderSelection::Platform, call())
        .await
        .is_err());
    let quota = check_ai_quota(&state, &org, ProviderSelection::Platform).await.unwrap();
    assert!(!quota.allowed);
    assert!(quota.deny_reason.unwrap().contains("Verify"));
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let actor = org.owner_id;
    // Exercise the real handler's HTTP serialization with an owned synthetic
    // principal; authentication middleware has separate coverage.
    let quota_server = serve(
        Router::new()
            .route(
                "/quota",
                get(
                    move |State(state): State<mockforge_registry_server::AppState>,
                          headers: HeaderMap| async move {
                        mockforge_registry_server::handlers::ai_studio::quota(
                            State(state),
                            AuthUser(actor),
                            headers,
                        )
                        .await
                    },
                ),
            )
            .with_state(state.clone()),
    )
    .await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/quota", quota_server.url))
        .header("X-Organization-Id", org.id.to_string())
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let snapshot: serde_json::Value = response.json().await.unwrap();
    assert_eq!(snapshot["call_allowed"], false);
    assert!(snapshot["deny_reason"].as_str().unwrap().contains("Verify"));
    let input = || {
        serde_json::from_value::<CreateDeploymentRequest>(json!({
            "name":"Verified owner fixture","config_json":{}
        }))
        .unwrap()
    };
    assert!(CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(input())
            )
        )
        .await
        .is_err());
    let deployments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hosted_mocks WHERE org_id=$1")
        .bind(org.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    let held: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
            .bind(org.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!((deployments, held, calls.load(Ordering::SeqCst)), (0, 0, 0));
    sqlx::query("UPDATE users SET is_verified=TRUE WHERE id=$1")
        .bind(org.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    let snapshot: serde_json::Value = client
        .get(format!("{}/quota", quota_server.url))
        .header("X-Organization-Id", org.id.to_string())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snapshot["call_allowed"], true);
    assert!(snapshot.get("deny_reason").is_none());
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(State(state.clone()), AuthUser(org.owner_id), headers, Json(input())),
        )
        .await
        .unwrap();
    assert_eq!(created.0.name, "Verified owner fixture");
    call_with_budget(&state, &org, ProviderSelection::Platform, call())
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
