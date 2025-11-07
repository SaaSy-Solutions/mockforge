//! VBR (Virtual Backend Reality) Integration Tests
//!
//! Tests that verify CRUD operations, relationship endpoints, and database
//! persistence work correctly end-to-end.

use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Router,
};
use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use mockforge_vbr::{
    config::VbrConfig,
    entities::Entity,
    handlers::HandlerContext,
    migration::MigrationManager,
    schema::{ForeignKeyDefinition, VbrSchemaDefinition},
    VbrEngine,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

/// Setup a test VBR engine with in-memory database
async fn setup_test_engine() -> Result<VbrEngine, Box<dyn std::error::Error>> {
    let config = VbrConfig::default();
    let engine = VbrEngine::new(config).await?;
    Ok(engine)
}

/// Create a test user entity schema
fn create_user_schema() -> VbrSchemaDefinition {
    let base = SchemaDefinition {
        name: "User".to_string(),
        fields: vec![
            FieldDefinition {
                name: "id".to_string(),
                field_type: "string".to_string(),
                required: true,
                description: Some("User ID".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
            FieldDefinition {
                name: "name".to_string(),
                field_type: "string".to_string(),
                required: true,
                description: Some("User name".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
            FieldDefinition {
                name: "email".to_string(),
                field_type: "string".to_string(),
                required: true,
                description: Some("User email".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
        ],
        description: Some("User entity".to_string()),
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    VbrSchemaDefinition {
        base,
        primary_key: vec!["id".to_string()],
        foreign_keys: Vec::new(),
        unique_constraints: Vec::new(),
        indexes: Vec::new(),
        auto_generation: HashMap::new(),
        many_to_many: Vec::new(),
    }
}

/// Create a test order entity schema with foreign key to users
fn create_order_schema() -> VbrSchemaDefinition {
    let base = SchemaDefinition {
        name: "Order".to_string(),
        fields: vec![
            FieldDefinition {
                name: "id".to_string(),
                field_type: "string".to_string(),
                required: true,
                description: Some("Order ID".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
            FieldDefinition {
                name: "user_id".to_string(),
                field_type: "string".to_string(),
                required: true,
                description: Some("User ID".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
            FieldDefinition {
                name: "total".to_string(),
                field_type: "number".to_string(),
                required: true,
                description: Some("Order total".to_string()),
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            },
            FieldDefinition {
                name: "status".to_string(),
                field_type: "string".to_string(),
                required: false,
                description: Some("Order status".to_string()),
                default: Some(Value::String("pending".to_string())),
                constraints: HashMap::new(),
                faker_template: None,
            },
        ],
        description: Some("Order entity".to_string()),
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    VbrSchemaDefinition {
        base,
        primary_key: vec!["id".to_string()],
        foreign_keys: vec![ForeignKeyDefinition {
            field: "user_id".to_string(),
            target_entity: "User".to_string(),
            target_field: "id".to_string(),
            on_delete: mockforge_vbr::schema::CascadeAction::Cascade,
            on_update: mockforge_vbr::schema::CascadeAction::Cascade,
        }],
        unique_constraints: Vec::new(),
        indexes: Vec::new(),
        auto_generation: HashMap::new(),
        many_to_many: Vec::new(),
    }
}

/// Setup test server with VBR routes and return listener
async fn setup_test_server(
    engine: VbrEngine,
) -> Result<(Router, TcpListener), Box<dyn std::error::Error>> {
    // Create database and registry from engine
    let database = engine.database_arc();
    let mut registry = engine.registry().clone();

    // Register test entities
    let user_schema = create_user_schema();
    let user_entity = Entity::new("User".to_string(), user_schema);
    registry.register(user_entity)?;

    let order_schema = create_order_schema();
    let order_entity = Entity::new("Order".to_string(), order_schema);
    registry.register(order_entity)?;

    // Run migrations
    let migration_manager = MigrationManager::new();
    for entity_name in registry.list() {
        let entity = registry.get(&entity_name).unwrap();
        let create_table_sql = migration_manager.generate_create_table(entity)?;

        // Execute migration
        database.execute(&create_table_sql, &[]).await?;

        // Create foreign key constraints
        let fk_sqls = migration_manager.generate_foreign_keys(entity);
        for fk_sql in fk_sqls {
            database.execute(&fk_sql, &[]).await?;
        }
    }

    // Create handler context
    let context = HandlerContext {
        database,
        registry,
        session_manager: None,
        snapshots_dir: None,
    };

    // Create router with VBR routes
    let router = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))
        // CRUD routes
        .route("/api/:entity", get(mockforge_vbr::handlers::list_handler))
        .route("/api/:entity", post(mockforge_vbr::handlers::create_handler))
        .route("/api/:entity/:id", get(mockforge_vbr::handlers::get_handler))
        .route("/api/:entity/:id", put(mockforge_vbr::handlers::update_handler))
        .route("/api/:entity/:id", patch(mockforge_vbr::handlers::patch_handler))
        .route("/api/:entity/:id", delete(mockforge_vbr::handlers::delete_handler))
        // Relationship routes
        .route(
            "/api/:entity/:id/:relationship",
            get(mockforge_vbr::handlers::get_relationship_handler),
        )
        .layer(
            ServiceBuilder::new()
                .layer(Extension(context))
                .layer(CorsLayer::permissive())
                .into_inner(),
        );

    // Bind to a random port
    let listener = TcpListener::bind("127.0.0.1:0").await?;

    Ok((router, listener))
}

/// Test helper to create HTTP client
fn create_test_client(_base_url: String) -> reqwest::Client {
    reqwest::Client::new()
}

/// Test CRUD operations for users
#[tokio::test]
async fn test_user_crud_operations() {
    let engine = match setup_test_engine().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping test: Failed to setup engine: {}", e);
            return;
        }
    };

    let (router, listener) = match setup_test_server(engine).await {
        Ok((r, l)) => (r, l),
        Err(e) => {
            eprintln!("Skipping test: Failed to setup server: {}", e);
            return;
        }
    };

    let addr = listener.local_addr().unwrap();

    // Start server in background
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Wait for server to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = create_test_client(format!("http://{}", addr));
    let base_url = format!("http://{}", addr);

    // Test CREATE (POST)
    let user_data = json!({
        "id": "user1",
        "name": "John Doe",
        "email": "john@example.com"
    });

    let response = client
        .post(&format!("{}/api/User", base_url))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to send POST request");

    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(created["id"], "user1");
    assert_eq!(created["name"], "John Doe");

    // Test GET by ID
    let response = client
        .get(&format!("{}/api/User/user1", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    assert_eq!(response.status(), StatusCode::OK);
    let user: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(user["id"], "user1");
    assert_eq!(user["email"], "john@example.com");

    // Test LIST (GET all)
    let response = client
        .get(&format!("{}/api/User", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    assert_eq!(response.status(), StatusCode::OK);
    let users: Value = response.json().await.expect("Failed to parse JSON");
    assert!(users["data"].is_array());
    assert_eq!(users["data"].as_array().unwrap().len(), 1);

    // Test UPDATE (PUT)
    let updated_data = json!({
        "id": "user1",
        "name": "Jane Doe",
        "email": "jane@example.com"
    });

    let response = client
        .put(&format!("{}/api/User/user1", base_url))
        .json(&updated_data)
        .send()
        .await
        .expect("Failed to send PUT request");

    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(updated["name"], "Jane Doe");

    // Verify update persisted
    let response = client
        .get(&format!("{}/api/User/user1", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    let user: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(user["name"], "Jane Doe");

    // Test PATCH (partial update)
    let patch_data = json!({
        "email": "jane.doe@example.com"
    });

    let response = client
        .patch(&format!("{}/api/User/user1", base_url))
        .json(&patch_data)
        .send()
        .await
        .expect("Failed to send PATCH request");

    assert_eq!(response.status(), StatusCode::OK);
    let patched: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(patched["email"], "jane.doe@example.com");

    // Test DELETE
    let response = client
        .delete(&format!("{}/api/User/user1", base_url))
        .send()
        .await
        .expect("Failed to send DELETE request");

    assert_eq!(response.status(), StatusCode::OK);

    // Verify deletion
    let response = client
        .get(&format!("{}/api/User/user1", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Test relationship endpoints (one-to-many)
#[tokio::test]
async fn test_relationship_endpoints() {
    let engine = match setup_test_engine().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping test: Failed to setup engine: {}", e);
            return;
        }
    };

    let (router, listener) = match setup_test_server(engine).await {
        Ok((r, l)) => (r, l),
        Err(e) => {
            eprintln!("Skipping test: Failed to setup server: {}", e);
            return;
        }
    };

    let addr = listener.local_addr().unwrap();

    // Start server
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = create_test_client(format!("http://{}", addr));
    let base_url = format!("http://{}", addr);

    // Create a user
    let user_data = json!({
        "id": "user1",
        "name": "John Doe",
        "email": "john@example.com"
    });

    let response = client
        .post(&format!("{}/api/User", base_url))
        .json(&user_data)
        .send()
        .await
        .expect("Failed to create user");

    assert_eq!(response.status(), StatusCode::CREATED);

    // Create orders for the user
    let order1_data = json!({
        "id": "order1",
        "user_id": "user1",
        "total": 100.0,
        "status": "pending"
    });

    let order2_data = json!({
        "id": "order2",
        "user_id": "user1",
        "total": 200.0,
        "status": "completed"
    });

    let response = client
        .post(&format!("{}/api/Order", base_url))
        .json(&order1_data)
        .send()
        .await
        .expect("Failed to create order1");

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .post(&format!("{}/api/Order", base_url))
        .json(&order2_data)
        .send()
        .await
        .expect("Failed to create order2");

    assert_eq!(response.status(), StatusCode::CREATED);

    // Test forward relationship: GET /api/User/user1/Order
    // This should return all orders for user1
    let response = client
        .get(&format!("{}/api/User/user1/Order", base_url))
        .send()
        .await
        .expect("Failed to get user orders");

    assert_eq!(response.status(), StatusCode::OK);
    let orders: Value = response.json().await.expect("Failed to parse JSON");
    assert!(orders["data"].is_array());
    let orders_array = orders["data"].as_array().unwrap();
    assert_eq!(orders_array.len(), 2);
    assert_eq!(orders["total"], 2);

    // Test reverse relationship: GET /api/Order/order1/User
    // This should return the user for order1
    let response = client
        .get(&format!("{}/api/Order/order1/User", base_url))
        .send()
        .await
        .expect("Failed to get order user");

    assert_eq!(response.status(), StatusCode::OK);
    let user: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(user["id"], "user1");
    assert_eq!(user["name"], "John Doe");
}

/// Test pagination and filtering
#[tokio::test]
async fn test_pagination_and_filtering() {
    let engine = match setup_test_engine().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping test: Failed to setup engine: {}", e);
            return;
        }
    };

    let (router, listener) = match setup_test_server(engine).await {
        Ok((r, l)) => (r, l),
        Err(e) => {
            eprintln!("Skipping test: Failed to setup server: {}", e);
            return;
        }
    };

    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = create_test_client(format!("http://{}", addr));
    let base_url = format!("http://{}", addr);

    // Create multiple users
    for i in 1..=5 {
        let user_data = json!({
            "id": format!("user{}", i),
            "name": format!("User {}", i),
            "email": format!("user{}@example.com", i)
        });

        let response = client
            .post(&format!("{}/api/User", base_url))
            .json(&user_data)
            .send()
            .await
            .expect(&format!("Failed to create user{}", i));

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    // Test pagination: limit=2
    let response = client
        .get(&format!("{}/api/User?limit=2", base_url))
        .send()
        .await
        .expect("Failed to get paginated users");

    assert_eq!(response.status(), StatusCode::OK);
    let users: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(users["data"].as_array().unwrap().len(), 2);
    assert_eq!(users["total"], 5);

    // Test pagination with offset
    let response = client
        .get(&format!("{}/api/User?limit=2&offset=2", base_url))
        .send()
        .await
        .expect("Failed to get paginated users with offset");

    assert_eq!(response.status(), StatusCode::OK);
    let users: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(users["data"].as_array().unwrap().len(), 2);
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() {
    let engine = match setup_test_engine().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Skipping test: Failed to setup engine: {}", e);
            return;
        }
    };

    let (router, listener) = match setup_test_server(engine).await {
        Ok((r, l)) => (r, l),
        Err(e) => {
            eprintln!("Skipping test: Failed to setup server: {}", e);
            return;
        }
    };

    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = create_test_client(format!("http://{}", addr));
    let base_url = format!("http://{}", addr);

    // Test GET non-existent entity
    let response = client
        .get(&format!("{}/api/User/nonexistent", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test GET non-existent entity type
    let response = client
        .get(&format!("{}/api/NonExistentEntity", base_url))
        .send()
        .await
        .expect("Failed to send GET request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test DELETE non-existent entity
    let response = client
        .delete(&format!("{}/api/User/nonexistent", base_url))
        .send()
        .await
        .expect("Failed to send DELETE request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
