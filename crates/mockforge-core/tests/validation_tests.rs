use mockforge_core::validation::Validator;
use mockforge_core::validation::*;
use mockforge_core::workspace::request::RequestProcessor;
use serde_json::json;
use std::fs;

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_openapi_basic() {
        // Valid OpenAPI spec
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {}
        });
        let result = validate_openapi(&json!({"test": "data"}), &spec);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_openapi_missing_required_fields() {
        // Invalid OpenAPI spec - missing info
        let spec = json!({
            "openapi": "3.0.0",
            "paths": {}
        });
        let result = validate_openapi(&json!({"test": "data"}), &spec);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("info")));
    }

    #[test]
    fn test_validate_openapi_wrong_version() {
        // Invalid OpenAPI spec - wrong version
        let spec = json!({
            "openapi": "2.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {}
        });
        let result = validate_openapi(&json!({"test": "data"}), &spec);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("version")));
    }

    #[test]
    fn test_validate_openapi_operation_security() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {
                "/users": {
                    "get": {
                        "responses": {"200": {"description": "OK"}},
                        "security": [{"bearerAuth": []}]
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        });

        let openapi_spec = mockforge_core::OpenApiSpec::from_json(spec).unwrap();

        // Test with valid Bearer token
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            Some("Bearer token123"),
            None,
        );
        assert!(result.valid);

        // Test without authentication
        let result =
            validate_openapi_operation_security(&openapi_spec, "/users", "GET", None, None);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Security validation failed")
            || e.contains("authentication")
            || e.contains("Bearer")));

        // Test with invalid Bearer token format
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            Some("token123"),
            None,
        );
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Security validation failed")
            || e.contains("Bearer")
            || e.contains("authentication")));
    }

    #[test]
    fn test_validate_openapi_api_key_security() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {
                "/users": {
                    "get": {
                        "responses": {"200": {"description": "OK"}},
                        "security": [{"apiKey": []}]
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "apiKey": {
                        "type": "apiKey",
                        "in": "header",
                        "name": "X-API-Key"
                    }
                }
            }
        });

        let openapi_spec = mockforge_core::OpenApiSpec::from_json(spec).unwrap();

        // Test with valid API key in header
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            Some("api-key-123"),
            None,
        );
        assert!(result.valid);

        // Test without API key
        let result =
            validate_openapi_operation_security(&openapi_spec, "/users", "GET", None, None);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Security validation failed")
            || e.contains("API key")
            || e.contains("authentication")));
    }

    #[test]
    fn test_validate_openapi_no_security() {
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {
                "/users": {
                    "get": {
                        "responses": {"200": {"description": "OK"}}
                    }
                }
            }
        });

        let openapi_spec = mockforge_core::OpenApiSpec::from_json(spec).unwrap();

        // No security requirements - should pass
        let result =
            validate_openapi_operation_security(&openapi_spec, "/users", "GET", None, None);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_protobuf_basic() {
        // This test is limited since we don't have actual protobuf data
        // In a real test environment, you'd have actual protobuf binary data and descriptors
        let result = validate_protobuf(&[], &[]);
        assert!(!result.valid); // Should fail with empty data
    }

    #[test]
    fn test_enhanced_schema_validation_integration() {
        use mockforge_core::validation;

        // Test the new enhanced schema validation functionality
        let expected_schema = json!({
            "type": "object",
            "properties": {
                "username": {"type": "string", "minLength": 3, "maxLength": 50},
                "email": {"type": "string", "format": "email"},
                "age": {"type": "integer", "minimum": 18, "maximum": 100}
            },
            "required": ["username", "email"]
        });

        // Valid request - should pass
        let valid_request = json!({
            "username": "john_doe",
            "email": "john.doe@example.com",
            "age": 25
        });

        let result = validation::validate_json_schema(&valid_request, &expected_schema);
        assert!(result.valid, "Valid request should not produce errors: {:?}", result.errors);

        // Invalid request with multiple issues
        let invalid_request = json!({
            "username": "a",  // Too short
            "age": 120        // Too old, missing required email
        });

        let result = validation::validate_json_schema(&invalid_request, &expected_schema);
        assert!(!result.valid, "Invalid request should produce errors");
        assert!(!result.errors.is_empty(), "Should have validation errors");

        // Check that we get detailed error information
        let error_text = result.errors.join(" ");
        assert!(
            error_text.contains("shorter than 3 characters") || error_text.contains("minLength"),
            "Should have username length error: {}",
            error_text
        );
        assert!(
            error_text.contains("email") && error_text.contains("required"),
            "Should have missing email error: {}",
            error_text
        );
        assert!(
            error_text.contains("greater than the maximum of 100")
                || error_text.contains("maximum"),
            "Should have age range error: {}",
            error_text
        );

        println!("✓ Enhanced schema validation integration test passed");
    }

    #[test]
    fn test_openapi31_multiple_of_validation() {
        let schema = json!({
            "type": "number",
            "multipleOf": 5.0
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: 10 is multiple of 5
        let result = validator.validate_openapi_ext(&json!(10), &schema);
        assert!(result.is_ok(), "10 should be valid multiple of 5");

        // Invalid: 12 is not multiple of 5
        let result = validator.validate_openapi_ext(&json!(12), &schema);
        assert!(result.is_err(), "12 should be invalid multiple of 5");
        assert!(result.unwrap_err().to_string().contains("not a multiple of"));
    }

    #[test]
    fn test_openapi31_exclusive_minimum_validation() {
        let schema = json!({
            "type": "number",
            "exclusiveMinimum": 10.0
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: 15 > 10
        let result = validator.validate_openapi_ext(&json!(15), &schema);
        assert!(result.is_ok(), "15 should be valid (greater than 10)");

        // Invalid: 10 is not greater than 10
        let result = validator.validate_openapi_ext(&json!(10), &schema);
        assert!(result.is_err(), "10 should be invalid (not greater than 10)");
        assert!(result.unwrap_err().to_string().contains("must be greater than"));

        // Invalid: 5 is not greater than 10
        let result = validator.validate_openapi_ext(&json!(5), &schema);
        assert!(result.is_err(), "5 should be invalid (not greater than 10)");
    }

    #[test]
    fn test_openapi31_exclusive_maximum_validation() {
        let schema = json!({
            "type": "number",
            "exclusiveMaximum": 100.0
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: 99 < 100
        let result = validator.validate_openapi_ext(&json!(99), &schema);
        assert!(result.is_ok(), "99 should be valid (less than 100)");

        // Invalid: 100 is not less than 100
        let result = validator.validate_openapi_ext(&json!(100), &schema);
        assert!(result.is_err(), "100 should be invalid (not less than 100)");
        assert!(result.unwrap_err().to_string().contains("must be less than"));

        // Invalid: 101 is not less than 100
        let result = validator.validate_openapi_ext(&json!(101), &schema);
        assert!(result.is_err(), "101 should be invalid (not less than 100)");
    }

    #[test]
    fn test_openapi31_array_min_max_items() {
        let schema = json!({
            "type": "array",
            "minItems": 2,
            "maxItems": 5
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: 3 items (between 2-5)
        let result = validator.validate_openapi_ext(&json!([1, 2, 3]), &schema);
        assert!(result.is_ok(), "Array with 3 items should be valid");

        // Invalid: 1 item (less than minItems 2)
        let result = validator.validate_openapi_ext(&json!([1]), &schema);
        assert!(result.is_err(), "Array with 1 item should be invalid (less than minItems)");
        assert!(result.unwrap_err().to_string().contains("minimum is 2"));

        // Invalid: 6 items (more than maxItems 5)
        let result = validator.validate_openapi_ext(&json!([1, 2, 3, 4, 5, 6]), &schema);
        assert!(result.is_err(), "Array with 6 items should be invalid (more than maxItems)");
        assert!(result.unwrap_err().to_string().contains("maximum is 5"));
    }

    #[test]
    fn test_openapi31_array_unique_items() {
        let schema = json!({
            "type": "array",
            "uniqueItems": true
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: unique items
        let result = validator.validate_openapi_ext(&json!([1, 2, 3, "a", "b"]), &schema);
        assert!(result.is_ok(), "Array with unique items should be valid");

        // Invalid: duplicate items
        let result = validator.validate_openapi_ext(&json!([1, 2, 2, 3]), &schema);
        assert!(result.is_err(), "Array with duplicate items should be invalid");
        assert!(result.unwrap_err().to_string().contains("must be unique"));

        // Invalid: duplicate strings
        let result = validator.validate_openapi_ext(&json!(["a", "b", "a"]), &schema);
        assert!(result.is_err(), "Array with duplicate strings should be invalid");
    }

    #[test]
    fn test_openapi31_content_encoding_validation() {
        // Test base64 content encoding
        let schema = json!({
            "type": "string",
            "contentEncoding": "base64"
        });

        let validator = Validator::from_openapi31_schema(&schema).unwrap();

        // Valid base64
        let result = validator.validate(&json!("SGVsbG8gV29ybGQ="));
        assert!(result.is_ok(), "Valid base64 should pass content encoding validation");

        // Invalid base64
        let result = validator.validate(&json!("not-valid-base64!@#"));
        assert!(result.is_err(), "Invalid base64 should fail content encoding validation");

        // Test base64url content encoding
        let schema = json!({
            "type": "string",
            "contentEncoding": "base64url"
        });

        let validator = Validator::from_openapi31_schema(&schema).unwrap();

        // Valid base64url (Hello World in base64url with padding)
        let result = validator.validate(&json!("SGVsbG8gV29ybGQ="));
        assert!(result.is_ok(), "Valid base64url should pass content encoding validation");

        // Test hex content encoding
        let schema = json!({
            "type": "string",
            "contentEncoding": "hex"
        });

        let validator = Validator::from_openapi31_schema(&schema).unwrap();

        // Valid hex
        let result = validator.validate(&json!("48656c6c6f"));
        assert!(result.is_ok(), "Valid hex should pass content encoding validation");

        // Invalid hex
        let result = validator.validate(&json!("not-hex"));
        assert!(result.is_err(), "Invalid hex should fail content encoding validation");
    }

    #[test]
    fn test_openapi31_composition_any_of() {
        let schema = json!({
            "anyOf": [
                {"type": "string", "minLength": 5},
                {"type": "number", "minimum": 10}
            ]
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: matches first subschema (string >= 5 chars)
        let result = validator.validate(&json!("hello"));
        assert!(result.is_ok(), "String with 5+ chars should match anyOf");

        // Valid: matches second subschema (number >= 10)
        let result = validator.validate(&json!(15));
        assert!(result.is_ok(), "Number >= 10 should match anyOf");

        // Invalid: doesn't match any subschema
        let result = validator.validate(&json!("hi"));
        assert!(result.is_err(), "Short string should not match anyOf");

        let result = validator.validate(&json!(5));
        assert!(result.is_err(), "Small number should not match anyOf");
    }

    #[test]
    fn test_openapi31_composition_one_of() {
        let schema = json!({
            "oneOf": [
                {"type": "string", "minLength": 5},
                {"type": "string", "maxLength": 3}
            ]
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: matches exactly one subschema (minLength: 5+ chars)
        let result = validator.validate(&json!("hello"));
        assert!(result.is_ok(), "String with 5+ chars should match exactly one in oneOf");

        // Valid: matches exactly one subschema (maxLength: 3 chars)
        let result = validator.validate(&json!("hi"));
        assert!(result.is_ok(), "String with 3 chars should match exactly one in oneOf");

        // Invalid: doesn't match any subschema
        let result = validator.validate(&json!(42));
        assert!(result.is_err(), "Number should not match oneOf expecting strings");

        // Invalid: matches both subschemas (4 chars)
        let result = validator.validate(&json!("test"));
        assert!(result.is_err(), "String with 4 chars should match both subschemas in oneOf");
    }

    #[test]
    fn test_openapi31_composition_all_of() {
        let schema = json!({
            "allOf": [
                {"type": "string", "minLength": 3},
                {"type": "string", "maxLength": 10}
            ]
        });

        let validator = Validator::from_json_schema(&schema).unwrap();

        // Valid: matches all subschemas
        let result = validator.validate(&json!("hello"));
        assert!(result.is_ok(), "String with 3-10 chars should match allOf");

        // Invalid: doesn't match first subschema (< 3 chars)
        let result = validator.validate(&json!("hi"));
        assert!(result.is_err(), "Short string should not match allOf");

        // Invalid: doesn't match second subschema (> 10 chars)
        let result = validator.validate(&json!("this-is-a-very-long-string"));
        assert!(result.is_err(), "Long string should not match allOf");
    }

    #[test]
    fn test_schema_diff_with_enhanced_error_reporting() {
        // Test complex nested object validation
        let expected_schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "pattern": "^[a-zA-Z ]+$"},
                        "profile": {
                            "type": "object",
                            "properties": {
                                "website": {"type": "string", "format": "uri"},
                                "phone": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    },
                    "required": ["name"]
                },
                "tags": {
                    "type": "array",
                    "items": {"type": "string", "minLength": 2},
                    "maxItems": 10
                }
            }
        });

        // Request with multiple validation issues
        let invalid_request = json!({
            "user": {
                "name": "John123",  // Invalid: contains numbers
                "profile": {
                    "website": "not-a-uri",
                    "phone": "+1234567890",
                    "extra_field": "should_not_be_here"  // Additional property not allowed
                }
            },
            "tags": ["", "go", "javascript", "rust", "typescript"],  // Error: empty tag, rest are good
            "unexpected_root_field": "not allowed"  // Additional property at root
        });

        use mockforge_core::validation;
        let result = validation::validate_json_schema(&invalid_request, &expected_schema);

        // Verify we get comprehensive error information
        assert!(!result.valid, "Invalid request should fail validation");
        assert!(result.errors.len() >= 1, "Should have validation errors");

        // Check that we get detailed error information
        let error_text = result.errors.join(" ");
        assert!(
            error_text.contains("pattern") || error_text.contains("John123"),
            "Should have pattern validation error: {}",
            error_text
        );
        assert!(
            error_text.contains("format")
                || error_text.contains("uri")
                || error_text.contains("not-a-uri"),
            "Should have format validation error: {}",
            error_text
        );
        assert!(
            error_text.contains("additional") || error_text.contains("extra_field"),
            "Should have additional property error: {}",
            error_text
        );
        assert!(
            error_text.contains("minLength") || error_text.contains("shorter than 2 characters"),
            "Should have array item validation error: {}",
            error_text
        );
    }

    #[test]
    fn test_url_pattern_matching() {
        let processor = RequestProcessor::new();

        // Test exact match
        assert!(processor.url_matches_pattern("/api/users", "/api/users"));
        assert!(!processor.url_matches_pattern("/api/users", "/api/posts"));

        // Test single wildcard (*)
        assert!(processor.url_matches_pattern("/api/users/*", "/api/users/123"));
        assert!(processor.url_matches_pattern("/api/users/*", "/api/users/abc"));
        assert!(!processor.url_matches_pattern("/api/users/*", "/api/users/123/profile"));
        assert!(!processor.url_matches_pattern("/api/users/*", "/api/posts/123"));

        // Test double wildcard (**)
        assert!(processor.url_matches_pattern("/api/users/**", "/api/users/123"));
        assert!(processor.url_matches_pattern("/api/users/**", "/api/users/123/profile"));
        assert!(processor.url_matches_pattern("/api/users/**", "/api/users/123/profile/settings"));
        assert!(!processor.url_matches_pattern("/api/users/**", "/api/posts/123"));

        // Test mixed patterns
        assert!(processor.url_matches_pattern("/api/*/posts", "/api/users/posts"));
        assert!(processor.url_matches_pattern("/api/*/posts", "/api/blogs/posts"));
        assert!(!processor.url_matches_pattern("/api/*/posts", "/api/users/comments"));

        // Test root wildcard
        assert!(processor.url_matches_pattern("*", "/api/users"));
        assert!(processor.url_matches_pattern("*", "/any/path"));

        // Test edge cases
        assert!(processor.url_matches_pattern("/api/users/**", "/api/users"));
        assert!(processor.url_matches_pattern("/api/users/**", "/api/users/"));
        assert!(!processor.url_matches_pattern("/api/users/*", "/api/users/123/456"));
    }

    #[test]
    fn test_openapi_validator_stores_spec() {
        // Valid OpenAPI spec
        let spec = json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {}
        });

        // Create validator - should now store the spec
        let validator = Validator::from_openapi(&spec).unwrap();

        // Verify it's the OpenApi variant with stored spec
        match &validator {
            Validator::OpenApi(stored_spec) => {
                // Verify the spec was stored correctly
                assert_eq!(stored_spec.spec.openapi, "3.0.0");
                assert_eq!(stored_spec.spec.info.title, "Test API");
                assert_eq!(stored_spec.spec.info.version, "1.0.0");
            }
            _ => panic!("Expected OpenApi validator variant"),
        }

        // Test validation still works
        let result = validator.validate(&json!({"test": "data"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_protobuf_valid_descriptor() {
        use std::process::Command;

        // Generate the descriptor file if it doesn't exist
        let descriptor_path = "/tmp/test_descriptor.bin";
        if !std::path::Path::new(descriptor_path).exists() {
            // Try to generate the descriptor file using protoc
            let proto_file = "../../proto/gretter.proto";
            if std::path::Path::new(proto_file).exists() {
                let output = Command::new("protoc")
                    .args(&[
                        "--proto_path=../../proto",
                        &format!("--descriptor_set_out={}", descriptor_path),
                        "gretter.proto",
                    ])
                    .current_dir("../../proto")
                    .output();

                if output.is_err() || !output.unwrap().status.success() {
                    // If protoc fails, skip the test
                    println!(
                        "Skipping protobuf test - protoc not available or proto file not found"
                    );
                    return;
                }
            } else {
                println!("Skipping protobuf test - proto file not found");
                return;
            }
        }

        // Read the test descriptor file generated from greeter.proto
        let descriptor_bytes = match fs::read(descriptor_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("Skipping protobuf test - descriptor file not found");
                return;
            }
        };

        // Create validator from protobuf descriptor
        let validator = Validator::from_protobuf(&descriptor_bytes).unwrap();

        // Verify it's the Protobuf variant
        match validator {
            Validator::Protobuf(_) => {
                // Successfully created protobuf validator
            }
            _ => panic!("Expected Protobuf validator variant"),
        }

        // Test that is_implemented returns true
        assert!(validator.is_implemented());
    }

    #[test]
    fn test_from_protobuf_invalid_descriptor() {
        // Test with invalid descriptor bytes
        let invalid_descriptor = b"invalid protobuf descriptor";

        // Should fail to create validator
        let result = Validator::from_protobuf(invalid_descriptor);
        assert!(result.is_err());
    }
}
