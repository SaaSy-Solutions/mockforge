use super::{organization, state};
use axum::{extract::State, http::HeaderMap, Json};
use mockforge_registry_server::{
    handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
    middleware::AuthUser,
    models::Plan,
    store::{with_org_context, CURRENT_ORG},
};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn one_free_owner_cannot_reserve_multiple_slots_through_different_orgs() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let second = state
        .store
        .create_organization(
            "Second capacity org",
            &Uuid::new_v4().simple().to_string(),
            first.owner_id,
            Plan::Free,
        )
        .await
        .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for org in [first.clone(), second.clone()] {
        let state = state.clone();
        tasks.spawn(async move {
            let mut headers = HeaderMap::new();
            headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
            let input: CreateDeploymentRequest =
                serde_json::from_value(json!({"name":"Shared capacity fixture","config_json":{}}))
                    .unwrap();
            CURRENT_ORG
                .scope(
                    org.id,
                    create_deployment(State(state), AuthUser(org.owner_id), headers, Json(input)),
                )
                .await
                .is_ok()
        });
    }
    let mut admitted = 0;
    while let Some(result) = tasks.join_next().await {
        admitted += usize::from(result.unwrap());
    }
    assert_eq!(admitted, 1);
    let occupied: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM hosted_mocks WHERE org_id IN ($1,$2) AND capacity_reserved",
    )
    .bind(first.id)
    .bind(second.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(occupied, 1);
    let deployment: (Uuid, Uuid) = sqlx::query_as(
        "SELECT id,org_id FROM hosted_mocks WHERE org_id IN ($1,$2) AND capacity_reserved",
    )
    .bind(first.id)
    .bind(second.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    let valid: bool = with_org_context(state.db.runtime_pool(), deployment.1, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,$2)")
                .bind(deployment.1)
                .bind(deployment.0)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert!(valid, "the deployment may retain its own already-held restart slot");
    let other = if deployment.1 == first.id {
        second.id
    } else {
        first.id
    };
    let invalid: bool = with_org_context(state.db.runtime_pool(), other, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,$2)")
                .bind(other)
                .bind(deployment.0)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert!(!invalid, "sibling deployment ID cannot be excluded as this org's restart");
    let mismatch: bool = with_org_context(state.db.runtime_pool(), first.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,NULL)")
                .bind(second.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert!(!mismatch);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn free_account_slot_admission_preserves_explicit_admin_grants_and_other_owners() {
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    sqlx::query("UPDATE organizations SET limits_json=jsonb_set(limits_json,'{max_hosted_mocks}','2') WHERE id=$1").bind(first.id).execute(state.db.pool()).await.unwrap();
    let second = state
        .store
        .create_organization(
            "Granted sibling",
            &Uuid::new_v4().simple().to_string(),
            first.owner_id,
            Plan::Free,
        )
        .await
        .unwrap();
    let mut admitted = Vec::new();
    for org in [first.clone(), second.clone()] {
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
        let input: CreateDeploymentRequest =
            serde_json::from_value(json!({"name":"Granted capacity fixture","config_json":{}}))
                .unwrap();
        admitted.push(
            CURRENT_ORG
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
                .0,
        );
    }
    assert_eq!(admitted.len(), 2);
    let third = state
        .store
        .create_organization(
            "Third sibling",
            &Uuid::new_v4().simple().to_string(),
            first.owner_id,
            Plan::Free,
        )
        .await
        .unwrap();
    let allowed: bool = with_org_context(state.db.runtime_pool(), third.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,NULL)")
                .bind(third.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert!(!allowed);
    let foreign = organization(&state, Plan::Free).await;
    let allowed: bool = with_org_context(state.db.runtime_pool(), foreign.id, |tx| {
        Box::pin(async move {
            Ok(sqlx::query_scalar("SELECT free_owner_hosted_capacity($1,NULL)")
                .bind(foreign.id)
                .fetch_one(&mut **tx)
                .await?)
        })
    })
    .await
    .unwrap();
    assert!(allowed);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn deleting_an_owner_cannot_transfer_a_reserved_slot_into_a_full_free_account() {
    let state = state().await;
    let source = organization(&state, Plan::Free).await;
    let destination = organization(&state, Plan::Free).await;
    for org in [&source, &destination] {
        sqlx::query("INSERT INTO hosted_mocks(org_id,name,slug,config_json,status,capacity_reserved) VALUES($1,'Owned transfer fixture',$2,'{}'::jsonb,'active',TRUE)")
            .bind(org.id).bind(format!("transfer-{}",Uuid::new_v4().simple())).execute(state.db.pool()).await.unwrap();
    }
    sqlx::query("INSERT INTO org_members(org_id,user_id,role) VALUES($1,$2,'member')")
        .bind(source.id)
        .bind(destination.owner_id)
        .execute(state.db.pool())
        .await
        .unwrap();
    assert!(
        state.store.delete_user_data_cascade(source.owner_id).await.is_err(),
        "transferring one occupied Free slot into another full account must roll back"
    );
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM organizations WHERE id=$1")
        .bind(source.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(owner, source.owner_id);
    // This fixture has no real provider machine. Model an already-confirmed
    // stop, then prove a transfer with no occupied incoming slot still works.
    sqlx::query("UPDATE hosted_mocks SET capacity_reserved=FALSE,status='stopped' WHERE org_id=$1")
        .bind(source.id)
        .execute(state.db.pool())
        .await
        .unwrap();
    state.store.delete_user_data_cascade(source.owner_id).await.unwrap();
    let owner: Uuid = sqlx::query_scalar("SELECT owner_id FROM organizations WHERE id=$1")
        .bind(source.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
    assert_eq!(owner, destination.owner_id);
}

#[tokio::test]
#[ignore = "requires isolated PostgreSQL"]
async fn concurrent_owner_transfer_and_new_admission_preserve_capacity_without_deadlocks() {
    for _ in 0..4 {
        let state = state().await;
        let source = organization(&state, Plan::Free).await;
        let destination = organization(&state, Plan::Free).await;
        sqlx::query("INSERT INTO hosted_mocks(org_id,name,slug,config_json,status,capacity_reserved) VALUES($1,'Owned concurrent transfer',$2,'{}'::jsonb,'active',TRUE)")
            .bind(source.id).bind(format!("transfer-{}",Uuid::new_v4().simple())).execute(state.db.pool()).await.unwrap();
        sqlx::query("INSERT INTO org_members(org_id,user_id,role) VALUES($1,$2,'member')")
            .bind(source.id)
            .bind(destination.owner_id)
            .execute(state.db.pool())
            .await
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", destination.id.to_string().parse().unwrap());
        let input: CreateDeploymentRequest =
            serde_json::from_value(json!({"name":"Concurrent admission fixture","config_json":{}}))
                .unwrap();
        let (transfer, admission) =
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                tokio::join!(
                    state.store.delete_user_data_cascade(source.owner_id),
                    CURRENT_ORG.scope(
                        destination.id,
                        create_deployment(
                            State(state.clone()),
                            AuthUser(destination.owner_id),
                            headers,
                            Json(input)
                        )
                    )
                )
            })
            .await
            .unwrap();
        assert_ne!(
            transfer.is_ok(),
            admission.is_ok(),
            "exactly one operation may consume the destination's sole slot"
        );
        if let Err(error) = transfer {
            assert!(
                !error.to_string().contains("deadlock"),
                "transfer rejection must be a capacity decision"
            );
        }
        if let Err(error) = admission {
            assert!(
                matches!(
                    error,
                    mockforge_registry_server::error::ApiError::ResourceLimitExceeded(_)
                ),
                "admission rejection must be a capacity decision"
            );
        }
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM hosted_mocks h JOIN organizations o ON o.id=h.org_id WHERE o.owner_id=$1 AND h.capacity_reserved")
            .bind(destination.owner_id).fetch_one(state.db.pool()).await.unwrap();
        assert_eq!(count, 1);
    }
}
