//! Integration tests for Spec Import API

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use mockforge_http::spec_import::{
    spec_import_router, ImportSpecRequest, SpecImportState, SpecType,
};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_import_openapi_spec() {
    let state = SpecImportState::new();
    let app = spec_import_router(state);

    let openapi_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let request_body = ImportSpecRequest {
        spec_content: serde_json::to_string(&openapi_spec).unwrap(),
        spec_type: Some(SpecType::OpenApi),
        name: Some("Test API".to_string()),
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("spec_id").is_some());
    assert_eq!(json["spec_type"], "openapi");
    assert_eq!(json["routes_generated"], 1);
}

#[tokio::test]
async fn test_import_asyncapi_spec() {
    let state = SpecImportState::new();
    let app = spec_import_router(state);

    let asyncapi_spec = r#"
    {
        "asyncapi": "2.6.0",
        "info": {
            "title": "Test MQTT API",
            "version": "1.0.0"
        },
        "servers": {
            "production": {
                "url": "mqtt://localhost:1883",
                "protocol": "mqtt"
            }
        },
        "channels": {
            "sensors/temperature": {
                "publish": {
                    "message": {
                        "payload": {
                            "type": "object",
                            "properties": {
                                "temperature": { "type": "number" }
                            }
                        }
                    }
                }
            }
        }
    }
    "#;

    let request_body = ImportSpecRequest {
        spec_content: asyncapi_spec.to_string(),
        spec_type: Some(SpecType::AsyncApi),
        name: Some("Test MQTT API".to_string()),
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("spec_id").is_some());
    assert_eq!(json["spec_type"], "asyncapi");
    assert_eq!(json["routes_generated"], 1);
}

#[tokio::test]
async fn test_list_specs() {
    let state = SpecImportState::new();
    let app = spec_import_router(state.clone());

    // First import a spec
    let openapi_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    let request_body = ImportSpecRequest {
        spec_content: serde_json::to_string(&openapi_spec).unwrap(),
        spec_type: Some(SpecType::OpenApi),
        name: Some("Test API".to_string()),
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(import_response.status(), StatusCode::OK);

    // Now list specs
    let list_response = app
        .oneshot(Request::builder().uri("/specs").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(list_response.into_body(), usize::MAX).await.unwrap();
    let specs: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["name"], "Test API");
}

#[tokio::test]
async fn test_get_spec_routes() {
    let state = SpecImportState::new();
    let app = spec_import_router(state);

    // Import a spec
    let openapi_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    let request_body = ImportSpecRequest {
        spec_content: serde_json::to_string(&openapi_spec).unwrap(),
        spec_type: Some(SpecType::OpenApi),
        name: Some("Test API".to_string()),
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(import_response.into_body(), usize::MAX).await.unwrap();
    let import_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let spec_id = import_json["spec_id"].as_str().unwrap();

    // Get routes for the spec
    let routes_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/specs/{}/routes", spec_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(routes_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(routes_response.into_body(), usize::MAX).await.unwrap();
    let routes: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["method"], "GET");
    assert_eq!(routes[0]["path"], "/users");
}

#[tokio::test]
async fn test_delete_spec() {
    let state = SpecImportState::new();
    let app = spec_import_router(state);

    // Import a spec
    let openapi_spec = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    let request_body = ImportSpecRequest {
        spec_content: serde_json::to_string(&openapi_spec).unwrap(),
        spec_type: Some(SpecType::OpenApi),
        name: Some("Test API".to_string()),
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(import_response.into_body(), usize::MAX).await.unwrap();
    let import_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let spec_id = import_json["spec_id"].as_str().unwrap();

    // Delete the spec
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/specs/{}", spec_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_import_yaml_spec() {
    let state = SpecImportState::new();
    let app = spec_import_router(state);

    let openapi_yaml = r#"
openapi: 3.0.0
info:
  title: YAML Test API
  version: 1.0.0
paths:
  /items:
    get:
      responses:
        '200':
          description: Success
    "#;

    let request_body = ImportSpecRequest {
        spec_content: openapi_yaml.to_string(),
        spec_type: None, // Auto-detect
        name: None,
        base_url: None,
        auto_generate_mocks: Some(true),
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/specs")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["spec_type"], "openapi");
    assert_eq!(json["routes_generated"], 1);
}
