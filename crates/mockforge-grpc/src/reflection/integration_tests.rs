//! Integration tests for advanced data synthesis features
//!
//! This module contains comprehensive tests that demonstrate the full
//! data synthesis pipeline including SmartMockGenerator, RAG synthesis,
//! schema graph extraction, and validation framework integration.

#[cfg(test)]
mod tests {
    use crate::reflection::{
        rag_synthesis::{RagDataSynthesizer, RagSynthesisConfig},
        schema_graph::SchemaGraph,
        smart_mock_generator::{SmartMockConfig, SmartMockGenerator},
        validation_framework::{
            CustomValidationRule, GeneratedEntity, ValidationConfig, ValidationFramework,
            ValidationRuleType,
        },
    };
    
    use std::collections::HashMap;
    use std::time::SystemTime;

    /// Test the complete data synthesis pipeline
    #[tokio::test]
    async fn test_complete_data_synthesis_pipeline() {
        // Step 1: Create a deterministic smart mock generator
        let config = SmartMockConfig {
            field_name_inference: true,
            use_faker: true,
            field_overrides: HashMap::new(),
            service_profiles: HashMap::new(),
            max_depth: 3,
            seed: Some(12345),
            deterministic: true,
        };

        let generator = SmartMockGenerator::new_with_seed(config, 12345);

        // Step 2: Create RAG synthesizer
        let rag_config = RagSynthesisConfig {
            enabled: false, // Disable for testing without external dependencies
            rag_config: None,
            context_sources: vec![],
            prompt_templates: HashMap::new(),
            max_context_length: 2000,
            cache_contexts: true,
        };

        let rag_synthesizer = RagDataSynthesizer::new(rag_config);

        // Step 3: Create validation framework
        let validation_config = ValidationConfig {
            enabled: true,
            strict_mode: false,
            max_validation_depth: 3,
            custom_rules: vec![CustomValidationRule {
                name: "email_format".to_string(),
                applies_to_entities: vec!["User".to_string()],
                validates_fields: vec!["email".to_string()],
                rule_type: ValidationRuleType::FieldFormat,
                parameters: HashMap::from([(
                    "pattern".to_string(),
                    r"^[^@\s]+@[^@\s]+\.[^@\s]+$".to_string(),
                )]),
                error_message: "Invalid email format".to_string(),
            }],
            cache_results: true,
        };

        let mut validator = ValidationFramework::new(validation_config);

        // Step 4: Generate multiple entities and validate them
        let entities = vec![
            GeneratedEntity {
                entity_type: "User".to_string(),
                primary_key: Some("user_1".to_string()),
                field_values: HashMap::from([
                    ("id".to_string(), "user_1".to_string()),
                    ("name".to_string(), "John Doe".to_string()),
                    ("email".to_string(), "john@example.com".to_string()),
                ]),
                endpoint: "/users".to_string(),
                generated_at: SystemTime::now(),
            },
            GeneratedEntity {
                entity_type: "Order".to_string(),
                primary_key: Some("order_1".to_string()),
                field_values: HashMap::from([
                    ("id".to_string(), "order_1".to_string()),
                    ("user_id".to_string(), "user_1".to_string()),
                    ("total".to_string(), "99.99".to_string()),
                ]),
                endpoint: "/orders".to_string(),
                generated_at: SystemTime::now(),
            },
        ];

        // Register entities with validator
        for entity in entities {
            validator.register_entity(entity).expect("Should register entity");
        }

        // Step 5: Run validation
        let validation_result = validator.validate_all_entities();

        // Verify validation results
        assert!(validation_result.is_valid, "Validation should pass");
        assert!(validation_result.errors.is_empty(), "Should have no validation errors");

        let stats = validator.get_statistics();
        assert_eq!(stats.total_entities, 2);
        assert_eq!(stats.entity_type_count, 2);
    }

    #[test]
    fn test_deterministic_data_generation() {
        // Create two identical generators with same seed
        let config = SmartMockConfig {
            seed: Some(999),
            deterministic: true,
            ..Default::default()
        };

        let mut gen1 = SmartMockGenerator::new(config.clone());
        let mut gen2 = SmartMockGenerator::new(config);

        // Generate UUIDs - should be identical
        let uuid1 = gen1.generate_uuid();
        let uuid2 = gen2.generate_uuid();
        assert_eq!(uuid1, uuid2, "Deterministic generators should produce identical UUIDs");

        // Generate strings - should be identical
        let str1 = gen1.generate_random_string(10);
        let str2 = gen2.generate_random_string(10);
        assert_eq!(str1, str2, "Deterministic generators should produce identical strings");
    }

    #[test]
    fn test_validation_with_foreign_key_violations() {
        let mut validator = ValidationFramework::new(ValidationConfig::default());

        // Create a mock schema graph with foreign key relationships
        let schema_graph = SchemaGraph {
            entities: HashMap::new(),
            relationships: vec![],
            foreign_keys: HashMap::from([(
                "Order".to_string(),
                vec![crate::reflection::schema_graph::ForeignKeyMapping {
                    field_name: "user_id".to_string(),
                    target_entity: "User".to_string(),
                    confidence: 0.9,
                    detection_method:
                        crate::reflection::schema_graph::ForeignKeyDetectionMethod::NamingConvention,
                }],
            )]),
        };

        validator.set_schema_graph(schema_graph);

        // Register an order without corresponding user
        let order_entity = GeneratedEntity {
            entity_type: "Order".to_string(),
            primary_key: Some("order_1".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "order_1".to_string()),
                ("user_id".to_string(), "nonexistent_user".to_string()),
            ]),
            endpoint: "/orders".to_string(),
            generated_at: SystemTime::now(),
        };

        validator.register_entity(order_entity).unwrap();

        let result = validator.validate_all_entities();

        // Should have validation errors due to missing foreign key
        assert!(!result.errors.is_empty(), "Should have foreign key validation errors");
        assert!(result.errors.iter().any(|e| matches!(
            e.error_type,
            crate::reflection::validation_framework::ValidationErrorType::ForeignKeyNotFound
        )));
    }

    #[test]
    fn test_rag_synthesizer_deterministic_behavior() {
        let config = RagSynthesisConfig::default();
        let synthesizer = RagDataSynthesizer::new(config);

        // Test deterministic field hashing
        let hash1 = synthesizer.hash_field_name("user_id");
        let hash2 = synthesizer.hash_field_name("user_id");
        assert_eq!(hash1, hash2, "Field name hashing should be deterministic");

        let hash3 = synthesizer.hash_field_name("different_field");
        assert_ne!(hash1, hash3, "Different field names should have different hashes");
    }

    #[cfg(feature = "data-faker")]
    #[test]
    fn test_faker_integration_with_deterministic_seeding() {
        

        let config = SmartMockConfig {
            use_faker: true,
            seed: Some(777),
            deterministic: true,
            ..Default::default()
        };

        let generator = SmartMockGenerator::new(config);

        // Verify faker is initialized
        assert!(generator.is_faker_enabled(), "Faker should be initialized");

        // Test that deterministic behavior works
        assert!(generator.config().deterministic);
        assert_eq!(generator.config().seed, Some(777));
    }

    #[test]
    fn test_validation_custom_rules_comprehensive() {
        let config = ValidationConfig {
            enabled: true,
            strict_mode: true,
            max_validation_depth: 3,
            custom_rules: vec![
                CustomValidationRule {
                    name: "age_range".to_string(),
                    applies_to_entities: vec!["User".to_string()],
                    validates_fields: vec!["age".to_string()],
                    rule_type: ValidationRuleType::Range,
                    parameters: HashMap::from([
                        ("min".to_string(), "0".to_string()),
                        ("max".to_string(), "120".to_string()),
                    ]),
                    error_message: "Age must be between 0 and 120".to_string(),
                },
                CustomValidationRule {
                    name: "unique_email".to_string(),
                    applies_to_entities: vec!["User".to_string()],
                    validates_fields: vec!["email".to_string()],
                    rule_type: ValidationRuleType::Unique,
                    parameters: HashMap::new(),
                    error_message: "Email must be unique".to_string(),
                },
            ],
            cache_results: true,
        };

        let mut validator = ValidationFramework::new(config);

        // Add entities with validation violations
        let entities = vec![
            GeneratedEntity {
                entity_type: "User".to_string(),
                primary_key: Some("user_1".to_string()),
                field_values: HashMap::from([
                    ("id".to_string(), "user_1".to_string()),
                    ("age".to_string(), "150".to_string()), // Invalid age
                    ("email".to_string(), "test@example.com".to_string()),
                ]),
                endpoint: "/users".to_string(),
                generated_at: SystemTime::now(),
            },
            GeneratedEntity {
                entity_type: "User".to_string(),
                primary_key: Some("user_2".to_string()),
                field_values: HashMap::from([
                    ("id".to_string(), "user_2".to_string()),
                    ("age".to_string(), "25".to_string()),
                    ("email".to_string(), "test@example.com".to_string()), // Duplicate email
                ]),
                endpoint: "/users".to_string(),
                generated_at: SystemTime::now(),
            },
        ];

        for entity in entities {
            validator.register_entity(entity).unwrap();
        }

        let result = validator.validate_all_entities();

        // Should have multiple validation errors
        assert!(!result.is_valid, "Validation should fail in strict mode");
        assert!(result.errors.len() >= 2, "Should have age range and duplicate email errors");

        // Verify specific error types
        assert!(result.errors.iter().any(|e| matches!(
            e.error_type,
            crate::reflection::validation_framework::ValidationErrorType::OutOfRange
        )));
        assert!(result.errors.iter().any(|e| matches!(
            e.error_type,
            crate::reflection::validation_framework::ValidationErrorType::DuplicateValue
        )));
    }

    #[test]
    fn test_generator_reset_functionality() {
        let mut config = SmartMockConfig::default();
        config.seed = Some(555);
        config.deterministic = true;

        let mut generator = SmartMockGenerator::new(config);

        // Generate some data
        let uuid1 = generator.generate_uuid();
        let seq1 = generator.next_sequence();
        let str1 = generator.generate_random_string(8);

        assert_eq!(seq1, 1);

        // Generate more data
        let seq2 = generator.next_sequence();
        assert_eq!(seq2, 2);

        // Reset generator
        generator.reset();

        // Should produce same initial results
        let uuid2 = generator.generate_uuid();
        let seq3 = generator.next_sequence();
        let str2 = generator.generate_random_string(8);

        assert_eq!(uuid1, uuid2, "UUID should be same after reset");
        assert_eq!(seq3, 1, "Sequence should reset to 1");
        assert_eq!(str1, str2, "Random string should be same after reset");
    }

    #[test]
    fn test_comprehensive_validation_statistics() {
        let config = ValidationConfig::default();
        let mut validator = ValidationFramework::new(config);

        // Add multiple entities of different types
        for i in 1..=10 {
            let user = GeneratedEntity {
                entity_type: "User".to_string(),
                primary_key: Some(format!("user_{}", i)),
                field_values: HashMap::from([
                    ("id".to_string(), format!("user_{}", i)),
                    ("name".to_string(), format!("User {}", i)),
                ]),
                endpoint: "/users".to_string(),
                generated_at: SystemTime::now(),
            };
            validator.register_entity(user).unwrap();
        }

        for i in 1..=5 {
            let order = GeneratedEntity {
                entity_type: "Order".to_string(),
                primary_key: Some(format!("order_{}", i)),
                field_values: HashMap::from([
                    ("id".to_string(), format!("order_{}", i)),
                    ("user_id".to_string(), format!("user_{}", i)),
                ]),
                endpoint: "/orders".to_string(),
                generated_at: SystemTime::now(),
            };
            validator.register_entity(order).unwrap();
        }

        let stats = validator.get_statistics();
        assert_eq!(stats.total_entities, 15);
        assert_eq!(stats.entity_type_count, 2);
        assert_eq!(stats.indexed_foreign_keys, 15); // 10 users + 5 orders

        // Clear and verify
        validator.clear();
        let stats_after_clear = validator.get_statistics();
        assert_eq!(stats_after_clear.total_entities, 0);
        assert_eq!(stats_after_clear.entity_type_count, 0);
        assert_eq!(stats_after_clear.indexed_foreign_keys, 0);
    }

    #[tokio::test]
    async fn test_end_to_end_synthesis_workflow() {
        // This test simulates a complete workflow from schema analysis to validation

        // Step 1: Setup deterministic generator
        let mut generator = SmartMockGenerator::new_with_seed(SmartMockConfig::default(), 99999);

        // Step 2: Generate consistent test data
        let users_data: Vec<_> = (1..=3)
            .map(|i| {
                HashMap::from([
                    ("id".to_string(), format!("user_{}", i)),
                    ("name".to_string(), format!("User {}", i)),
                    ("email".to_string(), format!("user{}@example.com", i)),
                ])
            })
            .collect();

        let orders_data: Vec<_> = (1..=5)
            .map(|i| {
                HashMap::from([
                    ("id".to_string(), format!("order_{}", i)),
                    ("user_id".to_string(), format!("user_{}", (i % 3) + 1)),
                    ("total".to_string(), format!("{}.99", 10 + i * 10)),
                ])
            })
            .collect();

        // Step 3: Create validation framework with comprehensive rules
        let validation_config = ValidationConfig {
            enabled: true,
            strict_mode: false,
            max_validation_depth: 5,
            custom_rules: vec![CustomValidationRule {
                name: "positive_total".to_string(),
                applies_to_entities: vec!["Order".to_string()],
                validates_fields: vec!["total".to_string()],
                rule_type: ValidationRuleType::Range,
                parameters: HashMap::from([
                    ("min".to_string(), "0.01".to_string()),
                    ("max".to_string(), "10000.00".to_string()),
                ]),
                error_message: "Order total must be positive".to_string(),
            }],
            cache_results: true,
        };

        let mut validator = ValidationFramework::new(validation_config);

        // Step 4: Register all generated entities
        for (_i, user_data) in users_data.iter().enumerate() {
            let entity = GeneratedEntity {
                entity_type: "User".to_string(),
                primary_key: Some(user_data.get("id").unwrap().clone()),
                field_values: user_data.clone(),
                endpoint: "/users".to_string(),
                generated_at: SystemTime::now(),
            };
            validator.register_entity(entity).unwrap();
        }

        for order_data in orders_data.iter() {
            let entity = GeneratedEntity {
                entity_type: "Order".to_string(),
                primary_key: Some(order_data.get("id").unwrap().clone()),
                field_values: order_data.clone(),
                endpoint: "/orders".to_string(),
                generated_at: SystemTime::now(),
            };
            validator.register_entity(entity).unwrap();
        }

        // Step 5: Run comprehensive validation
        let result = validator.validate_all_entities();

        // Step 6: Verify results
        assert!(result.is_valid, "End-to-end workflow should produce valid data");

        let stats = validator.get_statistics();
        assert_eq!(stats.total_entities, 8); // 3 users + 5 orders
        assert_eq!(stats.entity_type_count, 2);

        // Step 7: Test deterministic regeneration
        generator.reset();
        let uuid_after_reset = generator.generate_uuid();
        let string_after_reset = generator.generate_random_string(12);

        // Reset again and verify same results
        generator.reset();
        let uuid_again = generator.generate_uuid();
        let string_again = generator.generate_random_string(12);

        assert_eq!(uuid_after_reset, uuid_again);
        assert_eq!(string_after_reset, string_again);

        println!("✓ End-to-end synthesis workflow completed successfully");
        println!("  - Generated {} users and {} orders", users_data.len(), orders_data.len());
        println!(
            "  - Validation: {} errors, {} warnings",
            result.errors.len(),
            result.warnings.len()
        );
        println!("  - Deterministic generation verified");
    }
}
