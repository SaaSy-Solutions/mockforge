use super::{organization, serve, state};
use axum::{routing::post, Json, Router};
use mockforge_registry_server::{
    ai::{
        reservation::{call_with_budget_attributed, CallOrigin},
        LlmCall, ProviderSelection,
    },
    models::Plan,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

fn call(endpoint: &str) -> LlmCall {
    LlmCall {
        provider: "openai".into(),
        model: "fixture".into(),
        api_key: "synthetic-fixture-key".into(),
        base_url: Some(endpoint.into()),
        system: "system".into(),
        user: "user".into(),
        temperature: 0.0,
        max_tokens: 100,
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres and owned HTTP provider"]
async fn uncertain_platform_call_retains_actor_and_rejects_foreign_actor_before_provider() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let foreign = organization(&state, Plan::Free).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let provider = serve(Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let observed = observed.clone();
            async move {
                observed.fetch_add(1, Ordering::SeqCst);
                Json(json!({"choices":[{"message":{"content":"unknown usage"}}]}))
            }
        }),
    ))
    .await;
    assert!(call_with_budget_attributed(
        &state,
        &org,
        ProviderSelection::Platform,
        call(&provider.url),
        CallOrigin::OrganizationCompletion {
            actor_user_id: Some(org.owner_id)
        }
    )
    .await
    .is_err());
    let row: (Option<Uuid>, String, String) = sqlx::query_as(
        "SELECT actor_user_id,call_origin,state FROM ai_token_reservations WHERE org_id=$1",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(row, (Some(org.owner_id), "organization_completion".into(), "uncertain".into()));
    assert!(call_with_budget_attributed(
        &state,
        &org,
        ProviderSelection::Platform,
        call(&provider.url),
        CallOrigin::OrganizationCompletion {
            actor_user_id: Some(foreign.owner_id)
        }
    )
    .await
    .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_token_reservations WHERE org_id=$1")
            .bind(org.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(count, 1, "denied actors must not create reservations");
}

#[tokio::test]
#[ignore = "requires isolated Postgres and owned HTTP provider"]
async fn platform_job_attribution_uses_persisted_actor_and_rejects_foreign_or_unclaimed_jobs() {
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let foreign = organization(&state, Plan::Free).await;
    let workspace: Uuid = sqlx::query_scalar("INSERT INTO workspaces(org_id,name,created_by) VALUES($1,'Attribution fixture',$2) RETURNING id")
        .bind(org.id).bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap();
    let job: Uuid = sqlx::query_scalar("INSERT INTO cloud_test_generation_jobs(org_id,workspace_id,created_by,status) VALUES($1,$2,$3,'running') RETURNING id")
        .bind(org.id).bind(workspace).bind(org.owner_id).fetch_one(state.db.pool()).await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let provider = serve(Router::new().route("/v1/chat/completions", post(move || {
        let observed = observed.clone();
        async move {
            observed.fetch_add(1, Ordering::SeqCst);
            Json(json!({"choices":[{"message":{"content":"complete"}}],"usage":{"prompt_tokens":2,"completion_tokens":3}}))
        }
    }))).await;
    call_with_budget_attributed(
        &state,
        &org,
        ProviderSelection::Platform,
        call(&provider.url),
        CallOrigin::TestGeneration { job_id: job },
    )
    .await
    .unwrap();
    let row: (Option<Uuid>, Option<Uuid>, String, i64) = sqlx::query_as(
        "SELECT actor_user_id,test_generation_job_id,state,actual_tokens FROM ai_token_reservations WHERE org_id=$1"
    ).bind(org.id).fetch_one(state.db.pool()).await.unwrap();
    assert_eq!(row, (Some(org.owner_id), Some(job), "completed".into(), 5));
    assert!(call_with_budget_attributed(
        &state,
        &foreign,
        ProviderSelection::Platform,
        call(&provider.url),
        CallOrigin::TestGeneration { job_id: job }
    )
    .await
    .is_err());
    sqlx::query("UPDATE cloud_test_generation_jobs SET status='cancelled' WHERE id=$1")
        .bind(job)
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(call_with_budget_attributed(
        &state,
        &org,
        ProviderSelection::Platform,
        call(&provider.url),
        CallOrigin::TestGeneration { job_id: job }
    )
    .await
    .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1, "denied jobs must not reach the provider");
}
