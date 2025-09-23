use mockforge_core::openapi_routes::*;
use serde_json::json;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Router,
};
use tower::Service;
use std::net::SocketAddr;

#[tokio::test]
async fn test_mock_route_generation() {
    // Create a simple OpenAPI spec
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "summary": "Get users",
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create user",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"}
                                    },
                                    "required": ["name"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"},
                                            "name": {"type": "string"}
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

    // Create registry from JSON
    let registry = create_registry_from_json(spec_json).unwrap();

    // Verify routes were generated
    assert_eq!(registry.routes().len(), 2);

    // Check GET route
    let get_route = registry.get_route("/users", "GET").unwrap();
    assert_eq!(get_route.method, "GET");
    assert_eq!(get_route.path, "/users");

    // Check POST route
    let post_route = registry.get_route("/users", "POST").unwrap();
    assert_eq!(post_route.method, "POST");
    assert!(post_route.operation.request_body.is_some());

    // Test mock response generation
    let (status, response) = get_route.mock_response_with_status();
    assert_eq!(status, 200);
    // Should generate schema-based response: array of user objects
    assert!(response.is_array());

    let response_array = response.as_array().unwrap();
    assert_eq!(response_array.len(), 1); // Our implementation generates 1 item

    let user = &response_array[0];
    assert!(user.is_object());
    let user_obj = user.as_object().unwrap();
    assert!(user_obj.contains_key("id"));
    assert!(user_obj.contains_key("name"));

    println!("✅ Mock route generation test passed!");
    println!("Generated {} routes", registry.routes().len());
    println!("GET /users route: {}", get_route.axum_path());
    println!("POST /users route: {}", post_route.axum_path());
}

#[tokio::test]
async fn test_request_logger_middleware() {
    use mockforge_core::openapi_routes::builder::request_logger;

    // Create a simple request
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Create a mock next middleware that returns a response
    let next = Next::new(|_| async {
        Ok(Response::builder()
            .status(200)
            .body(Body::empty())
            .unwrap())
    });

    // Call the middleware
    let result = request_logger(request, next).await;

    // Should succeed and return the response
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_error_handler_middleware() {
    use mockforge_core::openapi_routes::builder::error_handler;

    // Create a simple request
    let request = Request::builder()
        .method("GET")
        .uri("/test")
        .body(Body::empty())
        .unwrap();

    // Create a mock next middleware that returns a server error response
    let next = Next::new(|_| async {
        Ok(Response::builder()
            .status(500)
            .body(Body::empty())
            .unwrap())
    });

    // Call the middleware
    let response = error_handler(request, next).await;

    // Should return the error response unchanged
    assert_eq!(response.status(), 500);
}

#[tokio::test]
async fn test_validate_request_middleware() {
    use mockforge_core::openapi_routes::builder::validate_request;
    use axum::extract::State;

    // Create a simple OpenAPI spec for testing
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/users": {
                "get": {
                    "responses": {
                        "200": {"description": "OK"}
                    }
                }
            }
        }
    });

    let registry = create_registry_from_json(spec_json).unwrap();

    // Create a valid request
    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .body(Body::empty())
        .unwrap();

    // Create a mock next middleware
    let next = Next::new(|_| async {
        Ok(Response::builder()
            .status(200)
            .body(Body::empty())
            .unwrap())
    });

    // Call the middleware with the registry as state
    let result = validate_request(State(registry), request, next).await;

    // Should succeed for valid request
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_middleware_integration() {
    use axum::routing::get;
    use std::time::Duration;
    use reqwest::Client;

    // Create a simple OpenAPI spec
    let spec_json = json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/test": {
                "get": {
                    "responses": {
                        "200": {"description": "OK"}
                    }
                }
            }
        }
    });

    let registry = create_registry_from_json(spec_json).unwrap();

    // Create router with middleware
    let app = Router::new()
        .route("/test", get(|| async { "Hello, World!" }))
        .layer(create_router_with_validation(Router::new(), registry))
        .layer(create_router_with_logging(Router::new()))
        .layer(create_router_with_error_handling(Router::new()));

    // Start server on a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Make HTTP request
    let client = Client::new();
    let response = client
        .get(format!("http://{}/test", addr))
        .send()
        .await
        .unwrap();

    // Check response
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Hello, World!");
}