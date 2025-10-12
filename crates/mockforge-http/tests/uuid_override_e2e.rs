use axum::Router;
use mockforge_core::openapi_routes::ValidationOptions;
use mockforge_http::build_router;
use regex::Regex;
use std::net::SocketAddr;

/// Test that UUID templating works in override files
#[tokio::test]
async fn test_uuid_override_templating() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"UUID Override Test","version":"1"},
        "paths": {
            "/user/{id}": {
                "get": {
                    "operationId": "getUser",
                    "parameters": [{
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": {"type": "string"}
                    }],
                    "responses": {
                        "200": {
                            "description":"User data",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    }
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let spec_path = dir.path().join("spec.json");
    tokio::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Create override file with UUID templating
    let override_dir = tempfile::tempdir().unwrap();
    let override_file = override_dir.path().join("uuid-override.yaml");

    let override_content = r#"
- targets: ["operation:getUser"]
  patch:
    - op: add
      path: /requestId
      value: "{{uuid}}"
    - op: add
      path: /correlationId
      value: "{{uuid}}"
    - op: add
      path: /traceId
      value: "{{uuid}}"
    - op: replace
      path: /user/sessionToken
      value: "{{uuid}}"
  mode: merge
"#;

    tokio::fs::write(&override_file, override_content).await.unwrap();

    // Set environment variable for overrides
    std::env::set_var("MOCKFORGE_HTTP_OVERRIDES_GLOB", override_file.to_string_lossy().to_string());

    let app: Router = build_router(
        Some(spec_path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        None,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/user/123", addr);

    let res = client.get(&url).send().await.unwrap();
    let status = res.status();
    if status != reqwest::StatusCode::OK {
        let body: serde_json::Value = res.json().await.unwrap();
        println!("Error response: {}", serde_json::to_string_pretty(&body).unwrap());
        assert_eq!(status, reqwest::StatusCode::OK);
    }

    let res = client.get(&url).send().await.unwrap();
    let body: serde_json::Value = res.json().await.unwrap();
    println!("Response body: {}", serde_json::to_string_pretty(&body).unwrap());

    // Validate that UUIDs were generated and are valid
    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Check requestId
    assert!(body["requestId"].is_string(), "requestId should be a string");
    let request_id = body["requestId"].as_str().unwrap();
    assert!(uuid_regex.is_match(request_id), "requestId should be a valid UUID format");

    // Check correlationId
    assert!(body["correlationId"].is_string(), "correlationId should be a string");
    let correlation_id = body["correlationId"].as_str().unwrap();
    assert!(
        uuid_regex.is_match(correlation_id),
        "correlationId should be a valid UUID format"
    );

    // Check traceId
    assert!(body["traceId"].is_string(), "traceId should be a string");
    let trace_id = body["traceId"].as_str().unwrap();
    assert!(uuid_regex.is_match(trace_id), "traceId should be a valid UUID format");

    // Check sessionToken in user object
    assert!(body["user"]["sessionToken"].is_string(), "sessionToken should be a string");
    let session_token = body["user"]["sessionToken"].as_str().unwrap();
    assert!(uuid_regex.is_match(session_token), "sessionToken should be a valid UUID format");

    // Verify all UUIDs are unique
    let uuids = [request_id, correlation_id, trace_id, session_token];
    let unique_uuids: std::collections::HashSet<_> = uuids.iter().collect();
    assert_eq!(unique_uuids.len(), 4, "All UUIDs should be unique");

    // Clean up environment variable
    std::env::remove_var("MOCKFORGE_HTTP_OVERRIDES_GLOB");

    drop(server);
}

/// Test UUID templating with different override modes
#[tokio::test]
async fn test_uuid_override_modes() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"UUID Override Modes Test","version":"1"},
        "paths": {
            "/test-replace": {
                "get": {
                    "operationId": "testReplace",
                    "responses": {
                        "200": {
                            "description":"Test replace mode",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    }
                }
            },
            "/test-merge": {
                "get": {
                    "operationId": "testMerge",
                    "responses": {
                        "200": {
                            "description":"Test merge mode",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    }
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let spec_path = dir.path().join("spec.json");
    tokio::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Create override files
    let override_dir = tempfile::tempdir().unwrap();

    // Replace mode override
    let replace_override = override_dir.path().join("replace-override.yaml");
    let replace_content = r#"
- targets: ["operation:testReplace"]
  patch:
    - op: replace
      path: ""
      value: {"id": "{{uuid}}", "type": "replaced"}
  mode: replace
"#;
    tokio::fs::write(&replace_override, replace_content).await.unwrap();

    // Merge mode override
    let merge_override = override_dir.path().join("merge-override.yaml");
    let merge_content = r#"
- targets: ["operation:testMerge"]
  patch:
    - op: add
      path: /id
      value: "{{uuid}}"
    - op: add
      path: /type
      value: "merged"
  mode: merge
"#;
    tokio::fs::write(&merge_override, merge_content).await.unwrap();

    // Test replace mode
    std::env::set_var(
        "MOCKFORGE_HTTP_OVERRIDES_GLOB",
        replace_override.to_string_lossy().to_string(),
    );

    let app: Router = build_router(
        Some(spec_path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        None,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();

    // Test replace mode
    let url_replace = format!("http://{}/test-replace", addr);
    let res_replace = client.get(&url_replace).send().await.unwrap();
    assert_eq!(res_replace.status(), reqwest::StatusCode::OK);

    let body_replace: serde_json::Value = res_replace.json().await.unwrap();

    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    assert!(body_replace["id"].is_string(), "id should be a string");
    assert!(uuid_regex.is_match(body_replace["id"].as_str().unwrap()));
    assert_eq!(body_replace["type"], "replaced");

    drop(server);

    // Test merge mode
    std::env::set_var(
        "MOCKFORGE_HTTP_OVERRIDES_GLOB",
        merge_override.to_string_lossy().to_string(),
    );

    let app2: Router = build_router(
        Some(spec_path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        None,
    )
    .await;

    let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr2 = listener2.local_addr().unwrap();
    let server2 = tokio::spawn(async move {
        axum::serve(listener2, app2.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let url_merge = format!("http://{}/test-merge", addr2);
    let res_merge = client.get(&url_merge).send().await.unwrap();
    assert_eq!(res_merge.status(), reqwest::StatusCode::OK);

    let body_merge: serde_json::Value = res_merge.json().await.unwrap();

    assert!(uuid_regex.is_match(body_merge["id"].as_str().unwrap()));
    assert_eq!(body_merge["type"], "merged");

    // Clean up
    std::env::remove_var("MOCKFORGE_HTTP_OVERRIDES_GLOB");

    drop(server2);
}

/// Test UUID templating with post-templating enabled
#[tokio::test]
async fn test_uuid_post_templating() {
    let spec = serde_json::json!({
        "openapi":"3.0.0",
        "info": {"title":"UUID Post-Templating Test","version":"1"},
        "paths": {
            "/post-template": {
                "get": {
                    "operationId": "postTemplateTest",
                    "responses": {
                        "200": {
                            "description":"Post templating test",
                            "content":{"application/json":{"schema":{"type":"object"}}}
                        }
                    }
                }
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let spec_path = dir.path().join("spec.json");
    tokio::fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).await.unwrap();

    // Create override file with post-templating
    let override_dir = tempfile::tempdir().unwrap();
    let override_file = override_dir.path().join("post-template-override.yaml");

    let override_content = r#"
- targets: ["operation:postTemplateTest"]
  patch:
    - op: add
      path: /staticField
      value: "not-a-uuid"
    - op: replace
      path: ""
      value: {
        "staticField": "not-a-uuid",
        "dynamicField": "{{uuid}}",
        "nested": {
          "uuid1": "{{uuid}}",
          "uuid2": "{{uuid}}",
          "array": ["{{uuid}}", "static", "{{uuid}}"]
        }
      }
  mode: replace
  post_templating: true
"#;

    tokio::fs::write(&override_file, override_content).await.unwrap();

    std::env::set_var("MOCKFORGE_HTTP_OVERRIDES_GLOB", override_file.to_string_lossy().to_string());

    let app: Router = build_router(
        Some(spec_path.to_string_lossy().to_string()),
        Some(ValidationOptions::default()),
        None,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap()
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/post-template", addr);

    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = res.json().await.unwrap();

    let uuid_regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();

    // Check that static field remains unchanged
    assert_eq!(body["staticField"], "not-a-uuid");

    // Check that templated fields were processed
    assert!(uuid_regex.is_match(body["dynamicField"].as_str().unwrap()));

    // Check nested UUIDs
    assert!(uuid_regex.is_match(body["nested"]["uuid1"].as_str().unwrap()));
    assert!(uuid_regex.is_match(body["nested"]["uuid2"].as_str().unwrap()));

    // Check array UUIDs
    assert!(uuid_regex.is_match(body["nested"]["array"][0].as_str().unwrap()));
    assert_eq!(body["nested"]["array"][1], "static");
    assert!(uuid_regex.is_match(body["nested"]["array"][2].as_str().unwrap()));

    // Verify nested UUIDs are unique
    let uuid1 = body["nested"]["uuid1"].as_str().unwrap();
    let uuid2 = body["nested"]["uuid2"].as_str().unwrap();
    let array_uuid1 = body["nested"]["array"][0].as_str().unwrap();
    let array_uuid2 = body["nested"]["array"][2].as_str().unwrap();

    assert_ne!(uuid1, uuid2, "Nested UUIDs should be unique");
    assert_ne!(array_uuid1, array_uuid2, "Array UUIDs should be unique");

    // Clean up
    std::env::remove_var("MOCKFORGE_HTTP_OVERRIDES_GLOB");

    drop(server);
}
