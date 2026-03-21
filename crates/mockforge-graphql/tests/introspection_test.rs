//! Introspection tests for GraphQL protocol.
//!
//! Tests GraphQL introspection query handling including __schema, __type,
//! type details, and full introspection query patterns.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mockforge_graphql::{create_router, GraphQLSchema};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Helper to execute a GraphQL query against the router and return parsed JSON.
async fn execute_query(router: axum::Router, query: &str) -> Value {
    let body = json!({"query": query});
    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_schema_introspection_query_type() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(router, "{ __schema { queryType { name } } }").await;

    let data = result.get("data").expect("should have data");
    let query_type_name = &data["__schema"]["queryType"]["name"];
    assert_eq!(query_type_name, "QueryRoot");
}

#[tokio::test]
async fn test_schema_introspection_mutation_type() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(router, "{ __schema { mutationType { name } } }").await;

    let data = result.get("data").expect("should have data");
    // The default schema has EmptyMutation, so mutationType should be null
    let mutation_type = &data["__schema"]["mutationType"];
    assert!(
        mutation_type.is_null(),
        "default schema has no mutations, mutationType should be null"
    );
}

#[tokio::test]
async fn test_schema_introspection_types_list() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(router, "{ __schema { types { name kind } } }").await;

    let data = result.get("data").expect("should have data");
    let types = data["__schema"]["types"].as_array().expect("types should be an array");

    // Should include built-in types and our custom types
    let type_names: Vec<&str> = types.iter().filter_map(|t| t["name"].as_str()).collect();

    // Check for built-in GraphQL types
    assert!(type_names.contains(&"String"), "should have String type");
    assert!(type_names.contains(&"Boolean"), "should have Boolean type");
    assert!(type_names.contains(&"Int"), "should have Int type");

    // Check for our custom types
    assert!(type_names.contains(&"User"), "should have User type");
    assert!(type_names.contains(&"Post"), "should have Post type");
}

#[tokio::test]
async fn test_type_introspection_user() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(
        router,
        r#"{ __type(name: "User") { name kind fields { name type { name kind } } } }"#,
    )
    .await;

    let data = result.get("data").expect("should have data");
    let user_type = &data["__type"];

    assert_eq!(user_type["name"], "User");
    assert_eq!(user_type["kind"], "OBJECT");

    let fields = user_type["fields"].as_array().expect("User should have fields");
    let field_names: Vec<&str> = fields.iter().filter_map(|f| f["name"].as_str()).collect();

    assert!(field_names.contains(&"id"), "User should have id field");
    assert!(field_names.contains(&"name"), "User should have name field");
    assert!(field_names.contains(&"email"), "User should have email field");
}

#[tokio::test]
async fn test_type_introspection_nonexistent() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(router, r#"{ __type(name: "NonExistentType") { name } }"#).await;

    let data = result.get("data").expect("should have data");
    assert!(data["__type"].is_null(), "__type for nonexistent type should be null");
}

#[tokio::test]
async fn test_introspection_query_fields() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(
        router,
        r#"{
            __schema {
                queryType {
                    fields {
                        name
                        args {
                            name
                            type { name kind }
                        }
                    }
                }
            }
        }"#,
    )
    .await;

    let data = result.get("data").expect("should have data");
    let fields = data["__schema"]["queryType"]["fields"]
        .as_array()
        .expect("queryType should have fields");

    let field_names: Vec<&str> = fields.iter().filter_map(|f| f["name"].as_str()).collect();

    // Our schema defines users, user, and posts queries
    assert!(field_names.contains(&"users"), "should have users query field");
    assert!(field_names.contains(&"user"), "should have user query field");
    assert!(field_names.contains(&"posts"), "should have posts query field");

    // Check that the `user` field has an `id` argument
    let user_field = fields
        .iter()
        .find(|f| f["name"].as_str() == Some("user"))
        .expect("should find user field");
    let args = user_field["args"].as_array().expect("user field should have args");
    let arg_names: Vec<&str> = args.iter().filter_map(|a| a["name"].as_str()).collect();
    assert!(arg_names.contains(&"id"), "user field should have id arg");
}

#[tokio::test]
async fn test_full_introspection_query() {
    // This mirrors the query that GraphQL clients like Apollo and GraphiQL send
    let router = create_router(None).await.unwrap();
    let result = execute_query(
        router,
        r#"
        query IntrospectionQuery {
            __schema {
                queryType { name }
                mutationType { name }
                subscriptionType { name }
                types {
                    kind
                    name
                    description
                    fields(includeDeprecated: true) {
                        name
                        description
                        args {
                            name
                            description
                            type {
                                kind
                                name
                                ofType {
                                    kind
                                    name
                                }
                            }
                            defaultValue
                        }
                        type {
                            kind
                            name
                            ofType {
                                kind
                                name
                            }
                        }
                        isDeprecated
                        deprecationReason
                    }
                    inputFields {
                        name
                        description
                        type {
                            kind
                            name
                            ofType {
                                kind
                                name
                            }
                        }
                        defaultValue
                    }
                    interfaces {
                        kind
                        name
                    }
                    enumValues(includeDeprecated: true) {
                        name
                        description
                        isDeprecated
                        deprecationReason
                    }
                    possibleTypes {
                        kind
                        name
                    }
                }
                directives {
                    name
                    description
                    locations
                    args {
                        name
                        description
                        type {
                            kind
                            name
                            ofType {
                                kind
                                name
                            }
                        }
                        defaultValue
                    }
                }
            }
        }
        "#,
    )
    .await;

    // Should succeed without errors
    assert!(
        result.get("errors").is_none()
            || result["errors"].as_array().map(|a| a.is_empty()).unwrap_or(true),
        "full introspection should not produce errors"
    );
    let data = result.get("data").expect("should have data");
    assert!(data.get("__schema").is_some());

    // Validate directives are present (standard GraphQL directives)
    let directives = data["__schema"]["directives"].as_array().expect("should have directives");
    let directive_names: Vec<&str> = directives.iter().filter_map(|d| d["name"].as_str()).collect();
    assert!(directive_names.contains(&"skip"), "should have @skip directive");
    assert!(directive_names.contains(&"include"), "should have @include directive");
}

#[tokio::test]
async fn test_typename_introspection() {
    let router = create_router(None).await.unwrap();
    let result = execute_query(router, r#"{ users(limit: 1) { __typename id } }"#).await;

    assert!(
        result.get("errors").is_none()
            || result["errors"].as_array().map(|a| a.is_empty()).unwrap_or(true),
        "__typename should be valid"
    );
    let data = result.get("data").expect("should have data");
    let users = data["users"].as_array().expect("users should be an array");
    if !users.is_empty() {
        assert_eq!(users[0]["__typename"], "User");
    }
}

#[test]
fn test_schema_sdl_via_direct_access() {
    let schema = GraphQLSchema::new();
    let sdl = schema.schema().sdl();

    // The SDL should be valid and contain our types
    assert!(sdl.contains("type QueryRoot"), "SDL should contain QueryRoot type");
    assert!(sdl.contains("type User"), "SDL should contain User type");
    assert!(sdl.contains("type Post"), "SDL should contain Post type");

    // Fields should be present
    assert!(sdl.contains("id: String!"), "User should have id field");
    assert!(sdl.contains("name: String!"), "User should have name field");
    assert!(sdl.contains("email: String!"), "User should have email field");
}

#[tokio::test]
async fn test_introspection_with_operation_name() {
    let router = create_router(None).await.unwrap();

    let body = json!({
        "query": "query MyIntrospection { __schema { queryType { name } } }",
        "operationName": "MyIntrospection"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(
        result.get("errors").is_none()
            || result["errors"].as_array().map(|a| a.is_empty()).unwrap_or(true),
    );
    let data = result.get("data").expect("should have data");
    assert!(data["__schema"]["queryType"]["name"].is_string());
}

#[tokio::test]
async fn test_schema_introspection_via_direct_execute() {
    let schema = GraphQLSchema::new();

    // Execute introspection directly on the schema (no router needed)
    let result = schema
        .schema()
        .execute("{ __schema { queryType { name } types { name } } }")
        .await;

    assert!(result.errors.is_empty(), "direct execution should succeed");
}
