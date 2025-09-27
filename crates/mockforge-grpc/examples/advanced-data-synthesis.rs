//! Advanced Data Synthesis Example
//!
//! This example demonstrates MockForge's advanced data synthesis capabilities,
//! including deterministic generation, relationship awareness, and validation.

use mockforge_grpc::reflection::{
    smart_mock_generator::{SmartMockConfig, SmartMockGenerator},
    validation_framework::{GeneratedEntity, ValidationConfig, ValidationFramework},
};
use std::collections::HashMap;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 MockForge Advanced Data Synthesis Example\n");

    // 1. Create a deterministic smart mock generator
    println!("📊 Setting up deterministic data generation...");
    let config = SmartMockConfig {
        field_name_inference: true,
        use_faker: true,
        seed: Some(42),
        deterministic: true,
        ..Default::default()
    };

    let mut generator = SmartMockGenerator::new(config);

    // 2. Generate deterministic UUIDs and data
    println!("🎲 Generating deterministic data...");
    let user_id = generator.generate_uuid();
    let order_id = generator.generate_uuid();
    let session_token = generator.generate_random_string(32);

    println!("   User ID: {}", user_id);
    println!("   Order ID: {}", order_id);
    println!("   Session Token: {}", session_token);

    // 3. Reset and verify deterministic behavior
    println!("\n🔄 Resetting generator and verifying deterministic behavior...");
    generator.reset();
    let user_id_2 = generator.generate_uuid();
    let order_id_2 = generator.generate_uuid();
    let session_token_2 = generator.generate_random_string(32);

    assert_eq!(user_id, user_id_2);
    assert_eq!(order_id, order_id_2);
    assert_eq!(session_token, session_token_2);
    println!("   ✅ Deterministic generation verified!");

    // 4. Set up validation framework
    println!("\n🛡️  Setting up cross-endpoint validation...");
    let validation_config = ValidationConfig {
        enabled: true,
        strict_mode: false,
        ..Default::default()
    };

    let mut validator = ValidationFramework::new(validation_config);

    // 5. Generate coherent entities with relationships
    println!("🔗 Generating related entities...");

    // Create users
    let users = vec![
        GeneratedEntity {
            entity_type: "User".to_string(),
            primary_key: Some("user_001".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "user_001".to_string()),
                ("name".to_string(), "Alice Johnson".to_string()),
                ("email".to_string(), "alice@example.com".to_string()),
                ("age".to_string(), "28".to_string()),
            ]),
            endpoint: "/api/users".to_string(),
            generated_at: SystemTime::now(),
        },
        GeneratedEntity {
            entity_type: "User".to_string(),
            primary_key: Some("user_002".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "user_002".to_string()),
                ("name".to_string(), "Bob Smith".to_string()),
                ("email".to_string(), "bob@example.com".to_string()),
                ("age".to_string(), "34".to_string()),
            ]),
            endpoint: "/api/users".to_string(),
            generated_at: SystemTime::now(),
        },
    ];

    // Create orders that reference users
    let orders = vec![
        GeneratedEntity {
            entity_type: "Order".to_string(),
            primary_key: Some("order_001".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "order_001".to_string()),
                ("user_id".to_string(), "user_001".to_string()), // Valid reference
                ("total".to_string(), "99.99".to_string()),
                ("status".to_string(), "pending".to_string()),
            ]),
            endpoint: "/api/orders".to_string(),
            generated_at: SystemTime::now(),
        },
        GeneratedEntity {
            entity_type: "Order".to_string(),
            primary_key: Some("order_002".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "order_002".to_string()),
                ("user_id".to_string(), "user_002".to_string()), // Valid reference
                ("total".to_string(), "149.50".to_string()),
                ("status".to_string(), "completed".to_string()),
            ]),
            endpoint: "/api/orders".to_string(),
            generated_at: SystemTime::now(),
        },
        GeneratedEntity {
            entity_type: "Order".to_string(),
            primary_key: Some("order_003".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "order_003".to_string()),
                ("user_id".to_string(), "user_999".to_string()), // Invalid reference!
                ("total".to_string(), "75.25".to_string()),
                ("status".to_string(), "failed".to_string()),
            ]),
            endpoint: "/api/orders".to_string(),
            generated_at: SystemTime::now(),
        },
    ];

    // 6. Register all entities
    println!("📝 Registering entities for validation...");
    for user in users {
        if let Err(e) = validator.register_entity(user) {
            eprintln!("Failed to register user: {}", e);
            return Err(e);
        }
    }

    for order in orders {
        if let Err(e) = validator.register_entity(order) {
            eprintln!("Failed to register order: {}", e);
            return Err(e);
        }
    }

    // 7. Run validation
    println!("🔍 Running cross-endpoint validation...");
    let result = validator.validate_all_entities();

    // 8. Display results
    println!("\n📈 Validation Results:");
    println!("   Valid: {}", result.is_valid);
    println!("   Errors: {}", result.errors.len());
    println!("   Warnings: {}", result.warnings.len());

    if !result.errors.is_empty() {
        println!("\n❌ Validation Errors:");
        for error in &result.errors {
            println!("   • {}: {}", error.entity_name, error.message);
            if let Some(fix) = &error.suggested_fix {
                println!("     💡 Suggested fix: {}", fix);
            }
        }
    }

    if !result.warnings.is_empty() {
        println!("\n⚠️  Validation Warnings:");
        for warning in &result.warnings {
            println!("   • {}", warning.message);
        }
    }

    // 9. Display statistics
    let stats = validator.get_statistics();
    println!("\n📊 Statistics:");
    println!("   Total entities: {}", stats.total_entities);
    println!("   Entity types: {}", stats.entity_type_count);
    println!("   Indexed foreign keys: {}", stats.indexed_foreign_keys);

    // 10. Demonstrate field inference capabilities
    println!("\n🧠 Smart Field Inference Examples:");
    generator.reset();

    // Simulate different field types
    let field_examples = vec![
        ("email", "Email addresses"),
        ("phone_number", "Phone numbers"),
        ("user_id", "User IDs"),
        ("created_at", "Timestamps"),
        ("uuid", "UUIDs"),
        ("token", "Security tokens"),
        ("latitude", "Geographic coordinates"),
        ("price", "Monetary values"),
    ];

    for (field_name, description) in field_examples {
        let sample_data = generator.generate_random_string(10); // Simplified for example
        println!("   • {}: {} → Sample: {}", field_name, description, sample_data);
    }

    println!("\n✨ Advanced Data Synthesis Demo Complete!");
    println!("   This example demonstrated:");
    println!("   • Deterministic data generation with seeded randomness");
    println!("   • Cross-endpoint referential integrity validation");
    println!("   • Intelligent field type inference");
    println!("   • Comprehensive error reporting and suggestions");
    println!("   • Statistical analysis of generated data");

    Ok(())
}
