//! Hosted OpenAPI and configured-route serving on the shared Free runtime.

use super::*;

#[tokio::test]
async fn hosted_openapi_ignores_process_and_constructor_validation_overrides() {
    use mockforge_openapi::{
        openapi_routes::{OpenApiRouteRegistry, ValidationMode, ValidationOptions},
        OpenApiSpec,
    };
    let spec = OpenApiSpec::from_json(json!({
        "openapi":"3.0.0", "info":{"title":"Isolation","version":"1"},
        "paths":{"/items":{"post":{
            "x-mockforge-ai":{"enabled":true,"prompt":"Generate a dynamic response"},
            "requestBody":{"required":true,"content":{"application/json":{
                "schema":{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}
            }}},
            "responses":{"201":{"description":"created","content":{"application/json":{
                "schema":{"type":"object","required":["created"],"properties":{"created":{"type":"boolean"},"literal":{"type":"string"}}},"example":{"created":true,"literal":"{{uuid}}"}
            }}}}
        }}}
    })).unwrap();
    let options = ValidationOptions {
        request_mode: ValidationMode::Disabled,
        ..Default::default()
    };
    let server =
        serve(OpenApiRouteRegistry::new_with_options(spec, options).build_hosted_router()).await;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(format!("{}/items", server.url))
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    let response = client
        .post(format!("{}/items", server.url))
        .header("X-Mockforge-Response-Status", "499")
        .header("X-Mockforge-AI", "true")
        .json(&json!({"name":"widget"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"created":true,"literal":"{{uuid}}"})
    );
    for path in [
        "/ai/generate-spec",
        "/api/ai/generate-spec",
        "/__mockforge/ai/generate-spec",
    ] {
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client
                .post(format!("{}{path}", server.url))
                .json(&json!({"prompt":"Generate a spec"}))
                .send(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.status(), 404, "global AI API must not be mounted in hosted responses");
    }
    assert_eq!(
        client
            .get(format!("{}/openapi.json", server.url))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_uploaded_openapi_serves_durable_examples_and_validates_requests() {
    use axum::{extract::State, http::HeaderMap, Json};
    use mockforge_registry_server::{
        deployment::orchestrator::DeploymentOrchestrator,
        handlers::hosted_mocks::{create_deployment, CreateDeploymentRequest},
        middleware::AuthUser,
    };
    for use_yaml in [false, true] {
        let state = state().await;
        let org = organization(&state, Plan::Free).await;
        set_limit(&state, &org, 4).await;
        let document = json!({
            "openapi":"3.0.0", "info":{"title":"Owned fixture","version":"1"},
            "paths":{"/widgets": {
                "get":{"responses":{"200":{"description":"ok","content":{
                    "application/json":{"schema":{"type":"object"},"example":{"owner":"Ada"}}
                }}}},
                "post":{"requestBody":{"required":true,"content":{"application/json":{
                    "schema":{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}
                }}},"responses":{"201":{"description":"created","content":{
                    "application/json":{"schema":{"type":"object"},"example":{"created":true}}
                }}}}
            }}
        });
        let source = state
            .storage
            .upload_spec(
                &org.id.to_string(),
                "runtime",
                if use_yaml {
                    serde_yaml::to_string(&document).unwrap().into_bytes()
                } else {
                    serde_json::to_vec(&document).unwrap()
                },
            )
            .await
            .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
        let input: CreateDeploymentRequest = serde_json::from_value(json!({
            "name":"Owned OpenAPI fixture", "openapi_spec_url":source, "config_json":{}
        }))
        .unwrap();
        let created = CURRENT_ORG
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
            .0;
        let worker = Arc::new(
            DeploymentOrchestrator::new(Arc::new(state.db.pool().clone()), None, None)
                .with_storage(state.storage.clone()),
        )
        .start();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let status: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id=$1")
                .bind(created.id)
                .fetch_one(state.db.pool())
                .await
                .unwrap();
            if status == "active" {
                break;
            }
            assert!(status != "failed", "uploaded specification provisioning failed");
            assert!(tokio::time::Instant::now() < deadline, "provisioning timed out: {status}");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
        let snapshot: Value = sqlx::query_scalar(
            "SELECT metadata_json->'runtime_openapi' FROM hosted_mocks WHERE id=$1",
        )
        .bind(created.id)
        .fetch_one(state.db.pool())
        .await
        .unwrap();
        assert_eq!(snapshot, document);
        // Corrupt the source after provisioning: requests must use the validated snapshot.
        state
            .storage
            .upload_spec(&org.id.to_string(), "runtime", b"invalid document".to_vec())
            .await
            .unwrap();
        let server = serve(MultitenantRouter::create_router().with_state(state.clone())).await;
        let base = format!("{}/mocks/{}/{}", server.url, org.id, created.slug);
        let client = reqwest::Client::new();
        let response = client.get(format!("{base}/widgets")).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.json::<Value>().await.unwrap(), json!({"owner":"Ada"}));
        assert_eq!(
            client
                .post(format!("{base}/widgets"))
                .json(&json!({}))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
        let response = client
            .post(format!("{base}/widgets"))
            .json(&json!({"name":"widget"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        assert_eq!(response.json::<Value>().await.unwrap(), json!({"created":true}));
        assert_eq!(client.get(format!("{base}/openapi.json")).send().await.unwrap().status(), 404);
        assert_eq!(client.get(format!("{base}/widgets")).send().await.unwrap().status(), 429);
        assert_eq!(requests(&state, org.id).await, 4);
    }
}

#[tokio::test]
#[ignore = "requires isolated Postgres and local HTTP listeners"]
async fn free_local_runtime_serves_configured_routes_and_caps_all_public_paths() {
    use axum::{
        extract::{Path, State},
        http::HeaderMap,
        Json,
    };
    use mockforge_registry_server::{
        deployment::orchestrator::DeploymentOrchestrator,
        handlers::hosted_mocks::{
            create_deployment, start_deployment, stop_deployment, CreateDeploymentRequest,
        },
        middleware::AuthUser,
    };
    let state = state().await;
    let org = organization(&state, Plan::Free).await;
    set_limit(&state, &org, 3).await;
    let mut headers = HeaderMap::new();
    headers.insert("X-Organization-Id", org.id.to_string().parse().unwrap());
    let request: CreateDeploymentRequest = serde_json::from_value(json!({"name":"Hosted runtime", "config_json":{"routes":[
        {"path":"/users/{id}","method":"GET","response":{"status":207,"headers":{"x-mock":"tenant-alpha","content-length":"99999"},"body":{"name":"Ada"}}},
        {"path":"/users/{id}","method":"POST","response":{"status":201,"body":{"created":true}}}
    ]}})).unwrap();
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
        let status: String = sqlx::query_scalar("SELECT status FROM hosted_mocks WHERE id = $1")
            .bind(created.id)
            .fetch_one(state.db.pool())
            .await
            .unwrap();
        if status == "active" {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "local runtime was not provisioned: {status}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    let backend: (Option<String>, String) = sqlx::query_as(
        "SELECT internal_url, metadata_json->>'hosting_backend' FROM hosted_mocks WHERE id=$1",
    )
    .bind(created.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(backend, (None, "multitenant".into()));
    let server = serve(
        MultitenantRouter::create_router()
            .fallback(custom_domain_fallback)
            .with_state(state.clone()),
    )
    .await;
    let client = reqwest::Client::new();
    let path = format!("{}/mocks/{}/{}/users/42", server.url, org.id, created.slug);
    let other = organization(&state, Plan::Free).await;
    let other_slug = format!("runtime-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO hosted_mocks (org_id,name,slug,config_json,status,metadata_json) VALUES ($1,'Other tenant',$2,$3,'active','{\"hosting_backend\":\"multitenant\"}'::jsonb)")
        .bind(other.id).bind(&other_slug).bind(json!({"routes":[{"path":"/users/{id}","method":"GET","response":{"status":200,"body":{"name":"Grace"}}}]}))
        .execute(state.db.pool()).await.unwrap();
    let other_path = format!("{}/mocks/{}/{}/users/42", server.url, other.id, other_slug);
    assert_eq!(
        client.get(other_path).send().await.unwrap().json::<Value>().await.unwrap()["name"],
        "Grace"
    );
    assert_eq!(
        client
            .get(format!(
                "{}/mocks/{}/{}/__mockforge/api/config",
                server.url, other.id, other_slug
            ))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let first = client.get(&path).send().await.unwrap();
    assert_eq!(first.status(), 207);
    assert_eq!(first.headers()["x-mock"], "tenant-alpha");
    assert_eq!(first.headers()["x-content-type-options"], "nosniff");
    assert!(first.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .starts_with("sandbox;"));
    assert_eq!(first.json::<Value>().await.unwrap()["name"], "Ada");
    let domain = format!(
        "{}.{}",
        created.slug,
        std::env::var("MOCKFORGE_MOCKS_DOMAIN").expect("fixture mocks domain required")
    );
    let second = client
        .post(format!("{}/users/42", server.url))
        .header("Host", &domain)
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), 201);
    assert_eq!(second.json::<Value>().await.unwrap()["created"], true);
    let stopped = CURRENT_ORG
        .scope(
            org.id,
            stop_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers.clone(),
                Path(created.id),
            ),
        )
        .await
        .unwrap();
    assert_eq!(stopped.0.status, "stopped");
    assert_eq!(client.get(&path).send().await.unwrap().status(), 503);
    let restarted = CURRENT_ORG
        .scope(
            org.id,
            start_deployment(
                State(state.clone()),
                AuthUser(org.owner_id),
                headers,
                Path(created.id),
            ),
        )
        .await
        .unwrap();
    assert_eq!(restarted.0.status, "active");
    assert_eq!(client.get(&path).send().await.unwrap().status(), 207);
    assert_eq!(
        client
            .get(format!("{}/users/42", server.url))
            .header("Host", domain)
            .send()
            .await
            .unwrap()
            .status(),
        429
    );
    let used: i64 = sqlx::query_scalar(
        "SELECT requests FROM usage_counters WHERE org_id=$1 ORDER BY period_start DESC LIMIT 1",
    )
    .bind(org.id)
    .fetch_one(state.db.pool())
    .await
    .unwrap();
    assert_eq!(used, 3);
}
