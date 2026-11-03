//! Cross-tenant boundaries for hosted project references and subdomain names.

use super::*;

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn hosted_project_reference_cannot_cross_tenants_or_delete_cleanup_records() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    let other = organization(&state, Plan::Free).await;
    let foreign: Uuid = sqlx::query_scalar("INSERT INTO projects(org_id,slug,name) VALUES ($1,'foreign','Foreign project') RETURNING id").bind(other.id).fetch_one(state.db.pool()).await.unwrap();
    let own: Uuid = sqlx::query_scalar(
        "INSERT INTO projects(org_id,slug,name) VALUES ($1,'own','Own project') RETURNING id",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let input = |project| {
        serde_json::from_value::<CreateDeploymentRequest>(
            json!({"name":"Project fixture","project_id":project,"config_json":{}}),
        )
        .unwrap()
    };
    let denied = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Json(input(foreign)),
            ),
        )
        .await
        .unwrap_err();
    assert!(matches!(denied, mockforge_registry_server::error::ApiError::InvalidRequest(_)));
    let created = CURRENT_ORG
        .scope(
            org.id,
            create_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers,
                Json(input(own)),
            ),
        )
        .await
        .unwrap()
        .0;
    // The database also rejects an elevated writer that bypasses request RLS.
    let error = sqlx::query("UPDATE hosted_mocks SET project_id=$2 WHERE id=$1")
        .bind(created.id)
        .bind(foreign)
        .execute(state.db.pool())
        .await
        .unwrap_err();
    assert_eq!(error.as_database_error().unwrap().code().as_deref(), Some("23503"));
    sqlx::query("DELETE FROM projects WHERE id=$1")
        .bind(own)
        .execute(state.db.pool())
        .await
        .unwrap();
    let remaining: (Uuid, Option<Uuid>, bool) =
        sqlx::query_as("SELECT org_id,project_id,capacity_reserved FROM hosted_mocks WHERE id=$1")
            .bind(created.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
    assert_eq!(remaining, (org.id, None, true));
}

#[tokio::test]
#[ignore = "requires isolated Postgres"]
async fn hosted_subdomain_names_cannot_collide_across_tenants_or_select_legacy_duplicates() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        error::ApiError,
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
        models::HostedMock,
    };
    let state = state().await;
    let first = organization(&state, Plan::Free).await;
    let second = organization(&state, Plan::Free).await;
    let slug = format!("shared-{}", Uuid::new_v4().simple());
    let mut handles = Vec::new();
    for org in [first.clone(), second.clone()] {
        let state = state.clone();
        let slug = slug.clone();
        handles.push(tokio::spawn(async move {
            let mut headers = HeaderMap::new();
            headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
            let request: CreateDeploymentRequest = serde_json::from_value(
                json!({"name":"Shared name", "slug":slug, "config_json":{}}),
            )
            .unwrap();
            CURRENT_ORG
                .scope(
                    org.id,
                    create_deployment(State(state), AuthUser(org.owner_id), headers, Json(request)),
                )
                .await
        }));
    }
    let mut admitted = 0;
    let mut conflicts = 0;
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => admitted += 1,
            Err(ApiError::Conflict(_)) => conflicts += 1,
            other => panic!("unexpected namespace result: {other:?}"),
        }
    }
    assert_eq!((admitted, conflicts), (1, 1));
    // Recreate ambiguous pre-migration records only inside a rolled-back
    // transaction. Production routing must fail closed even for legacy rows.
    let mut tx = state.db.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE hosted_mocks DISABLE TRIGGER reserve_global_hosted_slug")
        .execute(&mut *tx)
        .await
        .unwrap();
    let legacy_slug = format!("legacy-{}", Uuid::new_v4().simple());
    for org in [first, second] {
        sqlx::query("INSERT INTO hosted_mocks(org_id,name,slug,config_json,status) VALUES ($1,'Legacy duplicate',$2,'{}'::jsonb,'active')")
            .bind(org.id).bind(&legacy_slug).execute(&mut *tx).await.unwrap();
    }
    assert!(HostedMock::find_active_by_slug(&mut *tx, &legacy_slug).await.unwrap().is_none());
    tx.rollback().await.unwrap();
}
