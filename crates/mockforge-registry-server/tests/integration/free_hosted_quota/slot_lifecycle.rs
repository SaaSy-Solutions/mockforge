//! Free hosted slot reservation across creation, deletion, status changes and provider absence.

use super::*;

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
            Err(ApiError::InvalidRequest(message)) if status == "active" => {
                assert!(message.contains(expected), "{message}")
            }
            Err(ApiError::ResourceLimitExceeded(message)) if status != "active" => {
                assert!(message.contains("External hosting requires a paid plan"), "{message}");
            }
            _ => {
                panic!("external lifecycle must reject missing providers or Free plan eligibility")
            }
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
    sqlx::raw_sql(include_str!(
        "../../../migrations/20250101000091_hosted_capacity_reservations.sql"
    ))
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
