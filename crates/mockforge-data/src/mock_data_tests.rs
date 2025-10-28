//! Comprehensive tests for mock data generation functionality
//!
//! This module contains tests for the enhanced mock data generation system,
//! including OpenAPI specification processing, schema validation, and mock server functionality.

use crate::mock_generator::{MockDataGenerator, MockGeneratorConfig, MockDataResult};
use crate::mock_server::{MockServer, MockServerConfig, MockServerBuilder};
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Test OpenAPI specification for testing
fn create_test_openapi_spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Test API",
            "version": "1.0.0",
            "description": "A test API for mock data generation"
        },
        "paths": {
            "/api/users": {
                "get": {
                    "summary": "List users",
                    "responses": {
                        "200": {
                            "description": "List of users",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "users": {
                                                "type": "array",
                                                "items": {
                                                    "$ref": "#/components/schemas/User"
                                                }
                                            },
                                            "total": {
                                                "type": "integer",
                                                "minimum": 0
                                            }
                                        },
                                        "required": ["users", "total"]
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create user",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/CreateUserRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "User created",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/api/users/{id}": {
                "get": {
                    "summary": "Get user by ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string",
                                "format": "uuid"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "User details",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        }
                    }
                }
            }
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
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 100
                        },
                        "email": {
                            "type": "string",
                            "format": "email"
                        },
                        "age": {
                            "type": "integer",
                            "minimum": 18,
                            "maximum": 120
                        },
                        "active": {
                            "type": "boolean"
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "profile": {
                            "$ref": "#/components/schemas/UserProfile"
                        }
                    },
                    "required": ["id", "name", "email", "age", "active", "created_at"]
                },
                "UserProfile": {
                    "type": "object",
                    "properties": {
                        "bio": {
                            "type": "string",
                            "maxLength": 500
                        },
                        "avatar_url": {
                            "type": "string",
                            "format": "uri"
                        },
                        "location": {
                            "type": "string"
                        },
                        "website": {
                            "type": "string",
                            "format": "uri"
                        }
                    }
                },
                "CreateUserRequest": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 100
                        },
                        "email": {
                            "type": "string",
                            "format": "email"
                        },
                        "age": {
                            "type": "integer",
                            "minimum": 18,
                            "maximum": 120
                        }
                    },
                    "required": ["name", "email", "age"]
                }
            }
        }
    })
}

/// Test JSON Schema for testing
fn create_test_json_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {
                "type": "string",
                "format": "uuid"
            },
            "name": {
                "type": "string",
                "minLength": 1,
                "maxLength": 50
            },
            "email": {
                "type": "string",
                "format": "email"
            },
            "age": {
                "type": "integer",
                "minimum": 18,
                "maximum": 100
            },
            "active": {
                "type": "boolean"
            },
            "tags": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "minItems": 1,
                "maxItems": 5
            },
            "metadata": {
                "type": "object",
                "properties": {
                    "created_at": {
                        "type": "string",
                        "format": "date-time"
                    },
                    "last_login": {
                        "type": "string",
                        "format": "date-time"
                    }
                }
            }
        },
        "required": ["id", "name", "email", "age", "active"]
    })
}

#[cfg(test)]
mod mock_generator_tests {
    use super::*;

    #[test]
    fn test_mock_generator_config_default() {
        let config = MockGeneratorConfig::default();

        assert!(config.realistic_mode);
        assert_eq!(config.default_array_size, 3);
        assert_eq!(config.max_array_size, 10);
        assert!(config.include_optional_fields);
        assert!(config.validate_generated_data);
        assert!(config.field_mappings.is_empty());
    }

    #[test]
    fn test_mock_generator_config_builder() {
        let config = MockGeneratorConfig::new()
            .realistic_mode(false)
            .default_array_size(5)
            .max_array_size(20)
            .include_optional_fields(false)
            .field_mapping("email".to_string(), "email".to_string())
            .validate_generated_data(false);

        assert!(!config.realistic_mode);
        assert_eq!(config.default_array_size, 5);
        assert_eq!(config.max_array_size, 20);
        assert!(!config.include_optional_fields);
        assert!(!config.validate_generated_data);
        assert!(config.field_mappings.contains_key("email"));
    }

    #[test]
    fn test_mock_data_generator_new() {
        let generator = MockDataGenerator::new();

        assert!(generator.config.realistic_mode);
        assert!(!generator.field_patterns.is_empty());
        assert!(generator.schema_registry.is_empty());
    }

    #[test]
    fn test_mock_data_generator_with_config() {
        let config = MockGeneratorConfig::new()
            .realistic_mode(false)
            .default_array_size(10);

        let generator = MockDataGenerator::with_config(config);

        assert!(!generator.config.realistic_mode);
        assert_eq!(generator.config.default_array_size, 10);
    }

    #[test]
    fn test_generate_from_json_schema_simple() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" },
                "email": { "type": "string" }
            },
            "required": ["name", "age"]
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("age"));
        assert!(obj.contains_key("email"));
    }

    #[test]
    fn test_generate_from_json_schema_with_constraints() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "age": {
                    "type": "integer",
                    "minimum": 18,
                    "maximum": 65
                },
                "name": {
                    "type": "string",
                    "minLength": 5,
                    "maxLength": 20
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(age) = obj.get("age") {
            if let Some(age_num) = age.as_i64() {
                assert!(age_num >= 18);
                assert!(age_num <= 65);
            }
        }
    }

    #[test]
    fn test_generate_from_json_schema_with_enum() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(status) = obj.get("status") {
            if let Some(status_str) = status.as_str() {
                assert!(["active", "inactive", "pending"].contains(&status_str));
            }
        }
    }

    #[test]
    fn test_generate_from_json_schema_with_array() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "minItems": 2,
                    "maxItems": 5
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(tags) = obj.get("tags") {
            if let Some(tags_array) = tags.as_array() {
                assert!(tags_array.len() >= 2);
                assert!(tags_array.len() <= 5);
            }
        }
    }

    #[test]
    fn test_generate_from_json_schema_nested_object() {
        let mut generator = MockDataGenerator::new();

        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "email": { "type": "string", "format": "email" }
                    },
                    "required": ["name", "email"]
                }
            }
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(user) = obj.get("user") {
            assert!(user.is_object());
            let user_obj = user.as_object().unwrap();
            assert!(user_obj.contains_key("name"));
            assert!(user_obj.contains_key("email"));
        }
    }

    #[test]
    fn test_generate_from_openapi_spec() {
        let mut generator = MockDataGenerator::new();
        let spec = create_test_openapi_spec();

        let result = generator.generate_from_openapi_spec(&spec).unwrap();

        assert!(!result.schemas.is_empty());
        assert!(!result.responses.is_empty());
        assert_eq!(result.spec_info.title, "Test API");
        assert_eq!(result.spec_info.version, "1.0.0");
    }

    #[test]
    fn test_generate_from_openapi_spec_with_validation() {
        let config = MockGeneratorConfig::new()
            .validate_generated_data(true)
            .realistic_mode(true);

        let mut generator = MockDataGenerator::with_config(config);
        let spec = create_test_openapi_spec();

        let result = generator.generate_from_openapi_spec(&spec).unwrap();

        // Check that schemas were generated
        assert!(!result.schemas.is_empty());

        // Check that responses were generated
        assert!(!result.responses.is_empty());

        // Verify that User schema was generated
        assert!(result.schemas.contains_key("User"));

        // Verify that UserProfile schema was generated
        assert!(result.schemas.contains_key("UserProfile"));
    }

    #[test]
    fn test_field_pattern_matching() {
        let generator = MockDataGenerator::new();

        // Test email pattern matching
        let email_field = crate::schema::FieldDefinition::new("email_address".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&email_field);
        assert_eq!(faker_type, "email");

        // Test name pattern matching
        let name_field = crate::schema::FieldDefinition::new("user_name".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&name_field);
        assert_eq!(faker_type, "name");

        // Test phone pattern matching
        let phone_field = crate::schema::FieldDefinition::new("phone_number".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&phone_field);
        assert_eq!(faker_type, "phone");
    }

    #[test]
    fn test_custom_field_mapping() {
        let config = MockGeneratorConfig::new()
            .field_mapping("custom_field".to_string(), "email".to_string());

        let generator = MockDataGenerator::with_config(config);

        let field = crate::schema::FieldDefinition::new("custom_field".to_string(), "string".to_string());
        let faker_type = generator.determine_faker_type(&field);

        assert_eq!(faker_type, "email");
    }

    #[test]
    fn test_optional_fields_exclusion() {
        let config = MockGeneratorConfig::new()
            .include_optional_fields(false);

        let mut generator = MockDataGenerator::with_config(config);

        let schema = json!({
            "type": "object",
            "properties": {
                "required_field": { "type": "string" },
                "optional_field": { "type": "string" }
            },
            "required": ["required_field"]
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("required_field"));
        assert!(!obj.contains_key("optional_field"));
    }

    #[test]
    fn test_optional_fields_inclusion() {
        let config = MockGeneratorConfig::new()
            .include_optional_fields(true);

        let mut generator = MockDataGenerator::with_config(config);

        let schema = json!({
            "type": "object",
            "properties": {
                "required_field": { "type": "string" },
                "optional_field": { "type": "string" }
            },
            "required": ["required_field"]
        });

        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();
        assert!(obj.contains_key("required_field"));
        assert!(obj.contains_key("optional_field"));
    }
}

#[cfg(test)]
mod mock_server_tests {
    use super::*;

    #[test]
    fn test_mock_server_config_default() {
        let spec = create_test_openapi_spec();
        let config = MockServerConfig::new(spec);

        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.enable_cors);
        assert!(config.log_requests);
        assert!(config.response_delays.is_empty());
    }

    #[test]
    fn test_mock_server_config_builder() {
        let spec = create_test_openapi_spec();
        let config = MockServerConfig::new(spec)
            .port(8080)
            .host("0.0.0.0".to_string())
            .enable_cors(false)
            .response_delay("/api/users".to_string(), 100)
            .log_requests(false);

        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
        assert!(!config.enable_cors);
        assert!(!config.log_requests);
        assert!(config.response_delays.contains_key("/api/users"));
        assert_eq!(config.response_delays.get("/api/users"), Some(&100));
    }

    #[test]
    fn test_mock_server_builder() {
        let spec = create_test_openapi_spec();
        let builder = MockServerBuilder::new(spec)
            .port(8080)
            .host("0.0.0.0".to_string())
            .enable_cors(false);

        assert_eq!(builder.config.port, 8080);
        assert_eq!(builder.config.host, "0.0.0.0");
        assert!(!builder.config.enable_cors);
    }

    #[test]
    fn test_mock_server_creation() {
        let spec = create_test_openapi_spec();
        let config = MockServerConfig::new(spec);
        let server = MockServer::new(config);

        assert!(server.is_ok());
    }

    #[test]
    fn test_endpoints_match_exact() {
        assert!(MockServer::endpoints_match("GET /api/users", "GET /api/users"));
        assert!(!MockServer::endpoints_match("GET /api/users", "POST /api/users"));
        assert!(!MockServer::endpoints_match("GET /api/users", "GET /api/products"));
    }

    #[test]
    fn test_endpoints_match_with_params() {
        // This is a simplified test - real path parameter matching would be more complex
        assert!(MockServer::endpoints_match("GET /api/users/:id", "GET /api/users/123"));
        assert!(MockServer::endpoints_match("GET /api/users/:id", "GET /api/users/abc"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_openapi_generation_workflow() {
        let spec = create_test_openapi_spec();

        // Test with realistic mode enabled
        let config = MockGeneratorConfig::new()
            .realistic_mode(true)
            .include_optional_fields(true)
            .validate_generated_data(true);

        let mut generator = MockDataGenerator::with_config(config);
        let result = generator.generate_from_openapi_spec(&spec).unwrap();

        // Verify schemas were generated
        assert!(!result.schemas.is_empty());
        assert!(result.schemas.contains_key("User"));
        assert!(result.schemas.contains_key("UserProfile"));
        assert!(result.schemas.contains_key("CreateUserRequest"));

        // Verify responses were generated
        assert!(!result.responses.is_empty());

        // Check that User schema has realistic data
        if let Some(user_data) = result.schemas.get("User") {
            assert!(user_data.is_object());
            let user_obj = user_data.as_object().unwrap();

            // Check that email field contains @ symbol
            if let Some(email) = user_obj.get("email") {
                if let Some(email_str) = email.as_str() {
                    assert!(email_str.contains('@'));
                }
            }

            // Check that age is within bounds
            if let Some(age) = user_obj.get("age") {
                if let Some(age_num) = age.as_i64() {
                    assert!(age_num >= 18);
                    assert!(age_num <= 120);
                }
            }
        }
    }

    #[test]
    fn test_complex_nested_schema_generation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "profile": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "contact": {
                                    "type": "object",
                                    "properties": {
                                        "email": { "type": "string", "format": "email" },
                                        "phone": { "type": "string" }
                                    }
                                }
                            }
                        }
                    }
                },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "created_at": { "type": "string", "format": "date-time" },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                }
            }
        });

        let mut generator = MockDataGenerator::new();
        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        // Check nested structure
        if let Some(user) = obj.get("user") {
            assert!(user.is_object());
            let user_obj = user.as_object().unwrap();

            if let Some(profile) = user_obj.get("profile") {
                assert!(profile.is_object());
                let profile_obj = profile.as_object().unwrap();

                if let Some(contact) = profile_obj.get("contact") {
                    assert!(contact.is_object());
                    let contact_obj = contact.as_object().unwrap();

                    if let Some(email) = contact_obj.get("email") {
                        if let Some(email_str) = email.as_str() {
                            assert!(email_str.contains('@'));
                        }
                    }
                }
            }
        }

        // Check metadata
        if let Some(metadata) = obj.get("metadata") {
            assert!(metadata.is_object());
            let metadata_obj = metadata.as_object().unwrap();

            if let Some(tags) = metadata_obj.get("tags") {
                assert!(tags.is_array());
            }
        }
    }

    #[test]
    fn test_schema_validation_with_constraints() {
        let schema = json!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 100.0
                },
                "name": {
                    "type": "string",
                    "minLength": 3,
                    "maxLength": 20
                },
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive", "pending"]
                }
            },
            "required": ["score", "name", "status"]
        });

        let config = MockGeneratorConfig::new()
            .validate_generated_data(true);

        let mut generator = MockDataGenerator::with_config(config);
        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        // Validate score constraint
        if let Some(score) = obj.get("score") {
            if let Some(score_num) = score.as_f64() {
                assert!(score_num >= 0.0);
                assert!(score_num <= 100.0);
            }
        }

        // Validate name length constraint
        if let Some(name) = obj.get("name") {
            if let Some(name_str) = name.as_str() {
                assert!(name_str.len() >= 3);
                assert!(name_str.len() <= 20);
            }
        }

        // Validate enum constraint
        if let Some(status) = obj.get("status") {
            if let Some(status_str) = status.as_str() {
                assert!(["active", "inactive", "pending"].contains(&status_str));
            }
        }
    }

    #[test]
    fn test_array_generation_with_constraints() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "value": { "type": "number" }
                        }
                    },
                    "minItems": 2,
                    "maxItems": 5
                }
            }
        });

        let mut generator = MockDataGenerator::new();
        let result = generator.generate_from_json_schema(&schema).unwrap();

        assert!(result.is_object());
        let obj = result.as_object().unwrap();

        if let Some(items) = obj.get("items") {
            if let Some(items_array) = items.as_array() {
                assert!(items_array.len() >= 2);
                assert!(items_array.len() <= 5);

                // Check that each item has the expected structure
                for item in items_array {
                    assert!(item.is_object());
                    let item_obj = item.as_object().unwrap();
                    assert!(item_obj.contains_key("id"));
                    assert!(item_obj.contains_key("value"));
                }
            }
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_generation_performance() {
        let schema = create_test_json_schema();
        let mut generator = MockDataGenerator::new();

        let start = Instant::now();
        let result = generator.generate_from_json_schema(&schema).unwrap();
        let duration = start.elapsed();

        // Should complete in reasonable time (< 100ms for simple schema)
        assert!(duration.as_millis() < 100);
        assert!(result.is_object());
    }

    #[test]
    fn test_openapi_generation_performance() {
        let spec = create_test_openapi_spec();
        let mut generator = MockDataGenerator::new();

        let start = Instant::now();
        let result = generator.generate_from_openapi_spec(&spec).unwrap();
        let duration = start.elapsed();

        // Should complete in reasonable time (< 500ms for complex spec)
        assert!(duration.as_millis() < 500);
        assert!(!result.schemas.is_empty());
        assert!(!result.responses.is_empty());
    }

    #[test]
    fn test_batch_generation_performance() {
        let schema = create_test_json_schema();
        let mut generator = MockDataGenerator::new();

        let start = Instant::now();
        let mut results = Vec::new();

        // Generate 100 instances
        for _ in 0..100 {
            let result = generator.generate_from_json_schema(&schema).unwrap();
            results.push(result);
        }

        let duration = start.elapsed();

        // Should complete in reasonable time (< 1s for 100 instances)
        assert!(duration.as_millis() < 1000);
        assert_eq!(results.len(), 100);

        // All results should be valid objects
        for result in results {
            assert!(result.is_object());
        }
    }
}
