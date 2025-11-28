//! Example: Data Seeding with VBR
//!
//! This example demonstrates how to seed VBR database from JSON/YAML files

use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use mockforge_vbr::{
    config::VbrConfig, entities::Entity, schema::VbrSchemaDefinition, VbrEngine,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create VBR engine
    let config = VbrConfig::default();
    let mut engine = VbrEngine::new(config).await?;

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
    engine.registry_mut().register(user_entity)?;

    // Create table
    let entity = engine.registry().get("User").unwrap();
    mockforge_vbr::migration::create_table_for_entity(engine.database.as_ref(), entity)
        .await?;

    // Example: Seed from a JSON file
    // First, create a sample seed file
    let seed_file_content = r#"
{
  "users": [
    {
      "id": "user1",
      "name": "Alice Johnson",
      "email": "alice@example.com"
    },
    {
      "id": "user2",
      "name": "Bob Smith",
      "email": "bob@example.com"
    },
    {
      "id": "user3",
      "name": "Charlie Brown",
      "email": "charlie@example.com"
    }
  ]
}
"#;

    // Write seed file
    let seed_file_path = "./seed_data.json";
    tokio::fs::write(seed_file_path, seed_file_content).await?;

    // Load and seed from file
    let results = engine.seed_from_file(seed_file_path).await?;

    println!("✅ Seeded data from file:");
    for (entity, count) in results {
        println!("   - {}: {} records", entity, count);
    }

    // Verify data was inserted
    let query = "SELECT COUNT(*) as count FROM users";
    let results = engine.database().query(query, &[]).await?;
    let count = results[0]
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    println!("\n✅ Verified: {} users in database", count);

    // Clean up
    tokio::fs::remove_file(seed_file_path).await.ok();

    Ok(())
}
