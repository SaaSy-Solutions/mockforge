use mockforge_core::validation::*;
use serde_json::json;

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
            None
        );
        assert!(result.valid);

        // Test without authentication
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            None,
            None
        );
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("authentication header is required")));

        // Test with invalid Bearer token format
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            Some("token123"),
            None
        );
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Bearer token must start with")));
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
            None
        );
        assert!(result.valid);

        // Test without API key
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            None,
            None
        );
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("API key header")));
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
        let result = validate_openapi_operation_security(
            &openapi_spec,
            "/users",
            "GET",
            None,
            None
        );
        assert!(result.valid);
    }

    #[test]
    fn test_validate_protobuf_basic() {
        // This test is limited since we don't have actual protobuf data
        // In a real test environment, you'd have actual protobuf binary data and descriptors
        let result = validate_protobuf(&[], &[]);
        assert!(!result.valid); // Should fail with empty data
    }
}
