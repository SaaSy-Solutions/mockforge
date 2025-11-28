//! Example: Using VBR Engine with OpenAPI Specification
//!
//! This example demonstrates how to:
//! 1. Create a VBR engine from an OpenAPI specification
//! 2. Use the auto-generated CRUD endpoints
//! 3. Seed data and create snapshots

use mockforge_vbr::{VbrConfig, VbrEngine};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example OpenAPI specification
    let openapi_spec = r#"
{
  "openapi": "3.0.0",
  "info": {
    "title": "E-Commerce API",
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
          },
          "stock": {
            "type": "integer"
          }
        },
        "required": ["id", "name", "price"]
      },
      "Order": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string"
          },
          "user_id": {
            "type": "string"
          },
          "product_id": {
            "type": "string"
          },
          "quantity": {
            "type": "integer"
          },
          "total": {
            "type": "number"
          }
        },
        "required": ["id", "user_id", "product_id", "quantity", "total"]
      }
    }
  },
  "paths": {}
}
"#;

    // Create VBR engine from OpenAPI spec
    let config = VbrConfig::default();
    let (engine, conversion_result) = VbrEngine::from_openapi(config, openapi_spec)
        .await?;

    println!("✅ Created VBR engine from OpenAPI specification");
    println!("   Entities created: {}", conversion_result.entities.len());
    for (name, _) in &conversion_result.entities {
        println!("   - {}", name);
    }

    if !conversion_result.warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in &conversion_result.warnings {
            println!("   - {}", warning);
        }
    }

    // Seed some initial data
    let mut seed_data = HashMap::new();
    seed_data.insert(
        "User".to_string(),
        vec![
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), json!("user1"));
                record.insert("name".to_string(), json!("Alice"));
                record.insert("email".to_string(), json!("alice@example.com"));
                record
            },
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), json!("user2"));
                record.insert("name".to_string(), json!("Bob"));
                record.insert("email".to_string(), json!("bob@example.com"));
                record
            },
        ],
    );

    seed_data.insert(
        "Product".to_string(),
        vec![
            {
                let mut record = HashMap::new();
                record.insert("id".to_string(), json!("prod1"));
                record.insert("name".to_string(), json!("Laptop"));
                record.insert("price".to_string(), json!(999.99));
                record.insert("stock".to_string(), json!(10));
                record
            },
        ],
    );

    let results = engine.seed_all(&seed_data).await?;
    println!("\n✅ Seeded data:");
    for (entity, count) in results {
        println!("   - {}: {} records", entity, count);
    }

    // Create a snapshot
    let snapshots_dir = "./data/vbr/snapshots";
    let metadata = engine
        .create_snapshot("initial_state", Some("Initial seeded data".to_string()), snapshots_dir)
        .await?;

    println!("\n✅ Created snapshot: {}", metadata.name);
    println!("   Created at: {}", metadata.created_at);
    println!("   Entity counts:");
    for (entity, count) in &metadata.entity_counts {
        println!("     - {}: {}", entity, count);
    }

    println!("\n✅ VBR engine is ready!");
    println!("   You can now use the HTTP endpoints to interact with the entities.");

    Ok(())
}
