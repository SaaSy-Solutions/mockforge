//! Real authenticated API and deployment-JWT capture attribution regression.
//! Run against a disposable migrated PostgreSQL registry with JWT_SECRET set
//! and deployment providers disabled. Never use a production account.
use crate::fixture;
use chrono::Utc;
use fixture::{register_and_setup, E2e};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

fn patch(e: &E2e, id: Uuid, body: Value) -> reqwest::RequestBuilder {
    e.client
        .patch(format!("{}/api/v1/hosted-mocks/{id}/workspace", e.base_url))
        .header("Authorization", e.auth())
        .header("X-Organization-Id", &e.org_id)
        .json(&body)
}

fn exchange(capture: Uuid, claimed_workspace: &str) -> Value {
    json!({"exchanges":[{"request":{"id":capture,"protocol":"http","timestamp":Utc::now(),
        "method":"GET","path":"/workspace-proof","headers":"{}","body_encoding":"utf8",
        "duration_ms":25,"status_code":500,"workspace_id":claimed_workspace},
        "response":{"status_code":500,"headers":"{}","body_encoding":"utf8","size_bytes":0,"timestamp":Utc::now()}}]})
}

async fn attribution(pool: &PgPool, deployment: Uuid, capture: Uuid) -> Option<Uuid> {
    sqlx::query_scalar(
        "SELECT workspace_id FROM runtime_captures WHERE deployment_id=$1 AND capture_id=$2",
    )
    .bind(deployment)
    .bind(capture.to_string())
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires disposable native registry and PostgreSQL"]
async fn hosted_capture_workspace_http_lifecycle() {
    assert!(
        std::env::var("FLYIO_API_TOKEN").is_err(),
        "disable deployment providers for this fixture"
    );
    let base = std::env::var("REGISTRY_URL").expect("registry fixture");
    let secret = std::env::var("JWT_SECRET").expect("fixture JWT signing key");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&std::env::var("DATABASE_URL").expect("database fixture"))
        .await
        .unwrap();
    let own = register_and_setup(&base, &pool).await;
    let foreign = register_and_setup(&base, &pool).await;
    let pool = own.pool.clone();
    let org = Uuid::parse_str(&own.org_id).unwrap();
    let foreign_org = Uuid::parse_str(&foreign.org_id).unwrap();
    let workspace = Uuid::parse_str(&own.workspace_id).unwrap();
    let foreign_workspace = Uuid::parse_str(&foreign.workspace_id).unwrap();
    sqlx::query("UPDATE organizations SET limits_json=jsonb_set(limits_json,'{max_hosted_mocks}','-1'::jsonb,true) WHERE id=$1")
        .bind(org).execute(&pool).await.unwrap();
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hosted_mocks WHERE org_id=$1")
        .bind(org)
        .fetch_one(&pool)
        .await
        .unwrap();
    let denied = own.post("/api/v1/hosted-mocks").json(&json!({"name":"foreign workspace fixture","config_json":{},"workspace_id":foreign_workspace})).send().await.unwrap();
    assert_eq!(denied.status(), StatusCode::BAD_REQUEST);
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hosted_mocks WHERE org_id=$1")
        .bind(org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, before, "invalid scope must not persist a deployment");

    let created = own
        .post("/api/v1/hosted-mocks")
        .json(&json!({"name":"owned scope fixture","config_json":{},"workspace_id":workspace}))
        .send()
        .await
        .unwrap();
    let status = created.status();
    let created: Value = created.json().await.unwrap();
    assert!(status.is_success(), "{status}: {created}");
    assert_eq!(created["workspace_id"], json!(workspace));
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let detail: Value = own
        .get(&format!("/api/v1/hosted-mocks/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail["workspace_id"], json!(workspace));
    let list: Vec<Value> =
        own.get("/api/v1/hosted-mocks").send().await.unwrap().json().await.unwrap();
    assert_eq!(
        list.iter().find(|row| row["id"] == json!(id)).unwrap()["workspace_id"],
        json!(workspace)
    );

    let token =
        mockforge_registry_core::auth::create_deployment_ingest_token(id, &secret, 1).unwrap();
    let ingest_url = format!("{base}/api/v1/hosted-mocks/{id}/captures/ingest");
    let capture = Uuid::new_v4();
    let ingested = own
        .client
        .post(&ingest_url)
        .bearer_auth(&token)
        .json(&exchange(capture, &foreign.workspace_id))
        .send()
        .await
        .unwrap();
    assert!(ingested.status().is_success());
    assert_eq!(
        attribution(&pool, id, capture).await,
        Some(workspace),
        "ignore client-supplied attribution"
    );
    let retried = own
        .client
        .post(&ingest_url)
        .bearer_auth(&token)
        .json(&exchange(capture, &foreign.workspace_id))
        .send()
        .await
        .unwrap();
    assert!(retried.status().is_success());
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM runtime_captures WHERE deployment_id=$1 AND capture_id=$2",
    )
    .bind(id)
    .bind(capture.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "ingest retries must stay deduplicated");
    let wrong_token = mockforge_registry_core::auth::create_deployment_ingest_token(
        foreign.deployment_id,
        &secret,
        1,
    )
    .unwrap();
    let refused = own
        .client
        .post(&ingest_url)
        .bearer_auth(&wrong_token)
        .json(&exchange(Uuid::new_v4(), &own.workspace_id))
        .send()
        .await
        .unwrap();
    assert!(refused.status().is_client_error());

    for body in [
        json!({}),
        json!({"workspace_id":"invalid"}),
        json!({"workspace_id":foreign_workspace}),
    ] {
        assert!(patch(&own, id, body).send().await.unwrap().status().is_client_error());
    }
    assert!(patch(&foreign, id, json!({"workspace_id":foreign_workspace}))
        .send()
        .await
        .unwrap()
        .status()
        .is_client_error());
    let foreign_user: Uuid = sqlx::query_scalar("SELECT created_by FROM workspaces WHERE id=$1")
        .bind(foreign_workspace)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO org_members(org_id,user_id,role) VALUES($1,$2,'member')")
        .bind(org)
        .bind(foreign_user)
        .execute(&pool)
        .await
        .unwrap();
    let viewer = foreign
        .client
        .patch(format!("{base}/api/v1/hosted-mocks/{id}/workspace"))
        .header("Authorization", foreign.auth())
        .header("X-Organization-Id", &own.org_id)
        .json(&json!({"workspace_id":null}))
        .send()
        .await
        .unwrap();
    assert_eq!(viewer.status(), StatusCode::FORBIDDEN);
    let current: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM hosted_mocks WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(current, Some(workspace), "all rejected edits must leave attribution intact");
    let mut forged = pool.begin().await.unwrap();
    sqlx::query("UPDATE hosted_mocks SET workspace_id=$1 WHERE id=$2")
        .bind(foreign_workspace)
        .bind(id)
        .execute(&mut *forged)
        .await
        .unwrap();
    assert!(
        forged.commit().await.is_err(),
        "database must reject cross-org assignment even from owner pool"
    );

    let second = own
        .post("/api/v1/workspaces")
        .json(&json!({"name":"second scope fixture","description":"synthetic"}))
        .send()
        .await
        .unwrap();
    assert!(second.status().is_success());
    let second: Value = second.json().await.unwrap();
    let second = Uuid::parse_str(second["id"].as_str().unwrap()).unwrap();
    assert!(patch(&own, id, json!({"workspace_id":second}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let next = Uuid::new_v4();
    assert!(own
        .client
        .post(&ingest_url)
        .bearer_auth(&token)
        .json(&exchange(next, &own.workspace_id))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(attribution(&pool, id, next).await, Some(second));
    assert_eq!(
        attribution(&pool, id, capture).await,
        Some(workspace),
        "reassignment must not rewrite historical attribution"
    );

    // Hold a real row update while the actual ingest request is executing.
    // It must wait, then read the committed attribution rather than a stale one.
    let mut change = pool.begin().await.unwrap();
    sqlx::query("UPDATE hosted_mocks SET workspace_id=$1 WHERE id=$2")
        .bind(workspace)
        .bind(id)
        .execute(&mut *change)
        .await
        .unwrap();
    let serialized = Uuid::new_v4();
    let client = own.client.clone();
    let url = ingest_url.clone();
    let auth = token.clone();
    let body = exchange(serialized, &foreign.workspace_id);
    let request =
        tokio::spawn(async move { client.post(url).bearer_auth(auth).json(&body).send().await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT workspace_id FROM hosted_mocks WHERE id=%FOR SHARE')")
                .fetch_one(&pool).await.unwrap();
            if waiting { break; }
            assert!(!request.is_finished(), "ingest must wait for attribution transaction");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }).await.expect("actual ingest SELECT must acquire a shared row lock");
    change.commit().await.unwrap();
    assert!(tokio::time::timeout(std::time::Duration::from_secs(5), request)
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .status()
        .is_success());
    assert_eq!(attribution(&pool, id, serialized).await, Some(workspace));

    assert!(patch(&own, id, json!({"workspace_id":null}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let unassigned = Uuid::new_v4();
    assert!(own
        .client
        .post(&ingest_url)
        .bearer_auth(&token)
        .json(&exchange(unassigned, &own.workspace_id))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    assert_eq!(
        attribution(&pool, id, unassigned).await,
        None,
        "never guess a default workspace"
    );
    let legacy=own.post("/api/v1/hosted-mocks").json(&json!({"name":"legacy scope fixture","config_json":{"workspace_id":foreign_workspace}})).send().await.unwrap();
    assert!(legacy.status().is_success());
    let legacy: Value = legacy.json().await.unwrap();
    assert!(
        legacy["workspace_id"].is_null(),
        "untrusted configuration must not assign scope"
    );

    assert!(patch(&own, id, json!({"workspace_id":workspace}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let deleted = own
        .client
        .delete(format!("{base}/api/v1/workspaces/{workspace}"))
        .header("Authorization", own.auth())
        .header("X-Organization-Id", &own.org_id)
        .send()
        .await
        .unwrap();
    assert!(deleted.status().is_success(), "workspace deletion must detach its deployments");
    let detached: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM hosted_mocks WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(detached, None);
    sqlx::query("UPDATE hosted_mocks SET deleted_at=NOW() WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(own
        .client
        .post(&ingest_url)
        .bearer_auth(&token)
        .json(&exchange(Uuid::new_v4(), &own.workspace_id))
        .send()
        .await
        .unwrap()
        .status()
        .is_client_error());
    for org in [org, foreign_org] {
        // Synthetic deployments have no provider resources, so they can be
        // removed directly to satisfy the unfinished-cleanup guard (#1229).
        sqlx::query("DELETE FROM hosted_mocks WHERE org_id=$1")
            .bind(org)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM organizations WHERE id=$1")
            .bind(org)
            .execute(&pool)
            .await
            .unwrap();
    }
}
