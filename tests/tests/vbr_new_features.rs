//! VBR New Features Integration Tests
//!
//! Tests for the newly implemented features:
//! - OpenAPI integration
//! - Many-to-many relationships
//! - Data seeding
//! - Enhanced ID generation
//! - State snapshots and resets

use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use mockforge_vbr::{
    config::{StorageBackend, VbrConfig}, entities::Entity, schema::{
        AutoGenerationRule, CascadeAction, ForeignKeyDefinition, ManyToManyDefinition,
        VbrSchemaDefinition,
    },
    VbrEngine,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test OpenAPI integration
#[tokio::test]
async fn test_openapi_integration() {
    let openapi_spec = r#"
{
  "openapi": "3.0.0",
  "info": {
    "title": "Test API",
    "version": "1.0.0"
  },
  "components": {
    "schemas": {
      "User": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string",
            "format": "uuid"
          },
          "name": {
            "type": "string"
          },
          "email": {
            "type": "string",
            "format": "email"
          }
        },
        "required": ["id", "name", "email"]
      },
      "Product": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "price": {
            "type": "number"
          }
        },
        "required": ["id", "name", "price"]
      }
    }
  },
  "paths": {
    "/users": {
      "get": {
        "summary": "List users",
        "responses": {
          "200": {
            "description": "Success"
          }
        }
      }
    }
  }
}
"#;

    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Memory);
    let (engine, result) = VbrEngine::from_openapi(config, openapi_spec)
        .await
        .expect("Failed to create engine from OpenAPI");

    // Check that entities were created
    assert_eq!(result.entities.len(), 2);
    assert!(result.entities.iter().any(|(name, _)| name == "User"));
    assert!(result.entities.iter().any(|(name, _)| name == "Product"));

    // Check that User entity is registered
    assert!(engine.registry().exists("User"));
    assert!(engine.registry().exists("Product"));

    // Check that primary key was auto-detected
    let user_entity = engine.registry().get("User").unwrap();
    assert_eq!(user_entity.schema.primary_key, vec!["id"]);
    assert!(user_entity.schema.auto_generation.contains_key("id"));
}

/// Test many-to-many relationships
#[tokio::test]
async fn test_many_to_many_relationships() {
    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Memory);
    let mut engine = VbrEngine::new(config).await.expect("Failed to create engine");

    // Create User entity
    let user_base = SchemaDefinition {
        name: "User".to_string(),
        fields: vec![
            FieldDefinition::new("id".to_string(), "string".to_string()),
            FieldDefinition::new("name".to_string(), "string".to_string()),
        ],
        description: None,
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    let user_schema = VbrSchemaDefinition::new(user_base)
        .with_primary_key(vec!["id".to_string()]);

    let user_entity = Entity::new("User".to_string(), user_schema);
    engine.registry_mut().register(user_entity).unwrap();

    // Create Role entity
    let role_base = SchemaDefinition {
        name: "Role".to_string(),
        fields: vec![
            FieldDefinition::new("id".to_string(), "string".to_string()),
            FieldDefinition::new("name".to_string(), "string".to_string()),
        ],
        description: None,
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    let role_schema = VbrSchemaDefinition::new(role_base)
        .with_primary_key(vec!["id".to_string()])
        .with_many_to_many(ManyToManyDefinition::new(
            "User".to_string(),
            "Role".to_string(),
        ));

    let role_entity = Entity::new("Role".to_string(), role_schema);
    engine.registry_mut().register(role_entity).unwrap();

    // Create tables
    for entity_name in engine.registry().list() {
        let entity = engine.registry().get(&entity_name).unwrap();
        mockforge_vbr::migration::create_table_for_entity(
            engine.database(),
            entity,
        )
        .await
        .unwrap();
    }

    // Create junction table
    mockforge_vbr::migration::create_junction_tables(
        engine.database(),
        engine.registry(),
    )
    .await
    .unwrap();

    // Verify junction table exists (auto-generated name is alphabetically sorted: role_user)
    let junction_exists = engine
        .database()
        .table_exists("role_user")
        .await
        .unwrap();
    assert!(junction_exists);
}

/// Test data seeding
#[tokio::test]
async fn test_data_seeding() {
    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Memory);
    let mut engine = VbrEngine::new(config).await.expect("Failed to create engine");

    // Create User entity
    let user_base = SchemaDefinition {
        name: "User".to_string(),
        fields: vec![
            FieldDefinition::new("id".to_string(), "string".to_string()),
            FieldDefinition::new("name".to_string(), "string".to_string()),
            FieldDefinition::new("email".to_string(), "string".to_string()),
        ],
        description: None,
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    let user_schema = VbrSchemaDefinition::new(user_base)
        .with_primary_key(vec!["id".to_string()]);

    let user_entity = Entity::new("User".to_string(), user_schema);
    engine.registry_mut().register(user_entity).unwrap();

    // Create table
    let entity = engine.registry().get("User").unwrap();
    mockforge_vbr::migration::create_table_for_entity(engine.database(), entity)
        .await
        .unwrap();

    // Create seed data
    let mut seed_data = HashMap::new();
    seed_data.insert(
        "User".to_string(),
        vec![
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), Value::String("user1".to_string()));
                record.insert("name".to_string(), Value::String("Alice".to_string()));
                record.insert("email".to_string(), Value::String("alice@example.com".to_string()));
                record
            },
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), Value::String("user2".to_string()));
                record.insert("name".to_string(), Value::String("Bob".to_string()));
                record.insert("email".to_string(), Value::String("bob@example.com".to_string()));
                record
            },
        ],
    );

    // Seed the data
    let results = engine.seed_all(&seed_data).await.expect("Failed to seed data");
    assert_eq!(results.get("User"), Some(&2));

    // Verify data was inserted
    let query = "SELECT * FROM users";
    let records = engine
        .database()
        .query(query, &[])
        .await
        .expect("Failed to query");
    assert_eq!(records.len(), 2);
}

/// Test enhanced ID generation - Pattern
#[tokio::test]
async fn test_pattern_id_generation() {
    use mockforge_vbr::id_generation::generate_id;

    // Test pattern with increment
    let rule = AutoGenerationRule::Pattern("USR-{increment:06}".to_string());
    let id1 = generate_id(&rule, "User", "id", Some(1)).expect("Failed to generate ID");
    assert_eq!(id1, "USR-000001");

    let id2 = generate_id(&rule, "User", "id", Some(42)).expect("Failed to generate ID");
    assert_eq!(id2, "USR-000042");

    // Test pattern with timestamp
    let rule = AutoGenerationRule::Pattern("ORD-{timestamp}".to_string());
    let id = generate_id(&rule, "Order", "id", None).expect("Failed to generate ID");
    assert!(id.starts_with("ORD-"));

    // Test pattern with random
    let rule = AutoGenerationRule::Pattern("TXN-{random:12}".to_string());
    let id = generate_id(&rule, "Transaction", "id", None).expect("Failed to generate ID");
    assert!(id.starts_with("TXN-"));
    assert_eq!(id.len(), 16); // "TXN-" (4) + 12 random chars
}

/// Test enhanced ID generation - Realistic
#[tokio::test]
async fn test_realistic_id_generation() {
    use mockforge_vbr::id_generation::generate_id;

    let rule = AutoGenerationRule::Realistic {
        prefix: "cus".to_string(),
        length: 14,
    };

    let id = generate_id(&rule, "Customer", "id", None).expect("Failed to generate ID");
    assert!(id.starts_with("cus_"));
    assert_eq!(id.len(), 18); // "cus_" (4) + 14 random chars
}

/// Test state snapshots
#[tokio::test]
async fn test_state_snapshots() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let snapshots_dir = temp_dir.path().join("snapshots");

    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Memory);
    let mut engine = VbrEngine::new(config).await.expect("Failed to create engine");

    // Create User entity
    let user_base = SchemaDefinition {
        name: "User".to_string(),
        fields: vec![
            FieldDefinition::new("id".to_string(), "string".to_string()),
            FieldDefinition::new("name".to_string(), "string".to_string()),
        ],
        description: None,
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    let user_schema = VbrSchemaDefinition::new(user_base)
        .with_primary_key(vec!["id".to_string()]);

    let user_entity = Entity::new("User".to_string(), user_schema);
    engine.registry_mut().register(user_entity).unwrap();

    // Create table
    let entity = engine.registry().get("User").unwrap();
    mockforge_vbr::migration::create_table_for_entity(engine.database(), entity)
        .await
        .unwrap();

    // Seed some data
    let mut seed_data = HashMap::new();
    seed_data.insert(
        "User".to_string(),
        vec![{
            let mut record = HashMap::new();
            record.insert("id".to_string(), Value::String("user1".to_string()));
            record.insert("name".to_string(), Value::String("Alice".to_string()));
            record
        }],
    );

    engine.seed_all(&seed_data).await.expect("Failed to seed");

    // Verify data was seeded before creating snapshot
    let query = "SELECT COUNT(*) as count FROM users";
    let results = engine.database().query(query, &[]).await.expect("Failed to query");
    let count_before_snapshot = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(count_before_snapshot, 1, "Data should be seeded before snapshot");

    // Create snapshot
    let metadata = engine
        .create_snapshot("test_snapshot", Some("Test snapshot".to_string()), &snapshots_dir)
        .await
        .expect("Failed to create snapshot");

    assert_eq!(metadata.name, "test_snapshot");
    assert_eq!(metadata.entity_counts.get("User"), Some(&1));

    // Reset database
    engine.reset().await.expect("Failed to reset");

    // Verify data is gone
    let query = "SELECT COUNT(*) as count FROM users";
    let results = engine.database().query(query, &[]).await.expect("Failed to query");
    let count = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(count, 0);

    // Restore snapshot
    engine
        .restore_snapshot("test_snapshot", &snapshots_dir)
        .await
        .expect("Failed to restore snapshot");

    // Verify data is restored
    let results = engine.database().query(query, &[]).await.expect("Failed to query");
    let count = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(count, 1);

    // List snapshots
    let snapshots = VbrEngine::list_snapshots(&snapshots_dir)
        .await
        .expect("Failed to list snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].name, "test_snapshot");

    // Delete snapshot
    mockforge_vbr::snapshots::SnapshotManager::new(&snapshots_dir)
        .delete_snapshot("test_snapshot")
        .await
        .expect("Failed to delete snapshot");

    let snapshots = VbrEngine::list_snapshots(&snapshots_dir)
        .await
        .expect("Failed to list snapshots");
    assert_eq!(snapshots.len(), 0);
}

/// Test database reset
#[tokio::test]
async fn test_database_reset() {
    let config = VbrConfig::default()
        .with_storage_backend(StorageBackend::Memory);
    let mut engine = VbrEngine::new(config).await.expect("Failed to create engine");

    // Create User entity
    let user_base = SchemaDefinition {
        name: "User".to_string(),
        fields: vec![
            FieldDefinition::new("id".to_string(), "string".to_string()),
            FieldDefinition::new("name".to_string(), "string".to_string()),
        ],
        description: None,
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    let user_schema = VbrSchemaDefinition::new(user_base)
        .with_primary_key(vec!["id".to_string()]);

    let user_entity = Entity::new("User".to_string(), user_schema);
    engine.registry_mut().register(user_entity).unwrap();

    // Create table
    let entity = engine.registry().get("User").unwrap();
    mockforge_vbr::migration::create_table_for_entity(engine.database(), entity)
        .await
        .unwrap();

    // Seed some data
    let mut seed_data = HashMap::new();
    seed_data.insert(
        "User".to_string(),
        vec![
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), Value::String("user1".to_string()));
                record.insert("name".to_string(), Value::String("Alice".to_string()));
                record
            },
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), Value::String("user2".to_string()));
                record.insert("name".to_string(), Value::String("Bob".to_string()));
                record
            },
        ],
    );

    engine.seed_all(&seed_data).await.expect("Failed to seed");

    // Verify data exists
    let query = "SELECT COUNT(*) as count FROM users";
    let results = engine.database().query(query, &[]).await.expect("Failed to query");
    let count = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(count, 2);

    // Reset database
    engine.reset().await.expect("Failed to reset");

    // Verify data is gone
    let results = engine.database().query(query, &[]).await.expect("Failed to query");
    let count = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert_eq!(count, 0);
}
