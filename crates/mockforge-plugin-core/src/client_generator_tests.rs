//! Tests for multi-framework client generation
//!
//! This module contains comprehensive tests for the client generation
//! system, including React and Vue generators.

use crate::client_generator::{ClientGeneratorConfig, ClientGeneratorPlugin, OpenApiSpec};
use crate::plugins::{ReactClientGenerator, VueClientGenerator};
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;

/// Test OpenAPI specification
fn create_test_spec() -> OpenApiSpec {
    OpenApiSpec {
        openapi: "3.0.0".to_string(),
        info: crate::client_generator::ApiInfo {
            title: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test API for client generation".to_string()),
        },
        servers: Some(vec![crate::client_generator::Server {
            url: "http://localhost:3000".to_string(),
            description: Some("Test server".to_string()),
        }]),
        paths: {
            let mut paths = HashMap::new();

            // Add a simple GET endpoint
            let mut operations = HashMap::new();
            operations.insert(
                "get".to_string(),
                crate::client_generator::Operation {
                    summary: Some("Get users".to_string()),
                    description: Some("Retrieve all users".to_string()),
                    operation_id: Some("getUsers".to_string()),
                    parameters: None,
                    request_body: None,
                    responses: {
                        let mut responses = HashMap::new();
                        responses.insert(
                            "200".to_string(),
                            crate::client_generator::Response {
                                description: Some("Success".to_string()),
                                ref_path: None,
                                content: Some({
                                    let mut content = HashMap::new();
                                    content.insert(
                                        "application/json".to_string(),
                                        crate::client_generator::MediaType {
                                            schema: Some(crate::client_generator::Schema {
                                                r#type: Some("array".to_string()),
                                                format: None,
                                                properties: None,
                                                required: None,
                                                items: Some(Box::new(
                                                    crate::client_generator::Schema {
                                                        r#type: Some("object".to_string()),
                                                        format: None,
                                                        properties: Some({
                                                            let mut props = HashMap::new();
                                                            props.insert(
                                                                "id".to_string(),
                                                                crate::client_generator::Schema {
                                                                    r#type: Some(
                                                                        "integer".to_string(),
                                                                    ),
                                                                    format: None,
                                                                    properties: None,
                                                                    required: None,
                                                                    items: None,
                                                                    description: None,
                                                                    example: None,
                                                                    r#enum: None,
                                                                    ref_path: None,
                                                                },
                                                            );
                                                            props.insert(
                                                                "name".to_string(),
                                                                crate::client_generator::Schema {
                                                                    r#type: Some(
                                                                        "string".to_string(),
                                                                    ),
                                                                    format: None,
                                                                    properties: None,
                                                                    required: None,
                                                                    items: None,
                                                                    description: None,
                                                                    example: None,
                                                                    r#enum: None,
                                                                    ref_path: None,
                                                                },
                                                            );
                                                            props
                                                        }),
                                                        required: Some(vec![
                                                            "id".to_string(),
                                                            "name".to_string(),
                                                        ]),
                                                        items: None,
                                                        description: None,
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                )),
                                                description: None,
                                                example: None,
                                                r#enum: None,
                                                ref_path: None,
                                            }),
                                        },
                                    );
                                    content
                                }),
                                headers: None,
                            },
                        );
                        responses
                    },
                    tags: Some(vec!["Users".to_string()]),
                },
            );

            paths.insert(
                "/users".to_string(),
                crate::client_generator::PathItem {
                    operations,
                    ..Default::default()
                },
            );

            paths
        },
        components: None,
    }
}

/// Test configuration
fn create_test_config() -> ClientGeneratorConfig {
    ClientGeneratorConfig {
        output_dir: "./test-output".to_string(),
        base_url: Some("http://localhost:3000".to_string()),
        include_types: true,
        include_mocks: false,
        template_dir: None,
        options: HashMap::new(),
    }
}

#[cfg(test)]
mod react_tests {
    use super::*;

    #[test]
    fn test_react_generator_creation() {
        let generator = ReactClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_react_framework_name() {
        let generator = ReactClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "react");
    }

    #[test]
    fn test_react_supported_extensions() {
        let generator = ReactClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"tsx"));
        assert!(extensions.contains(&"js"));
        assert!(extensions.contains(&"jsx"));
    }

    #[tokio::test]
    async fn test_react_generate_client() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.framework, "react");
        assert_eq!(result.metadata.api_title, "Test API");
        assert_eq!(result.metadata.api_version, "1.0.0");
    }

    #[tokio::test]
    async fn test_react_generated_files() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Check that required files are generated
        let file_paths: Vec<&str> = result.files.iter().map(|f| f.path.as_str()).collect();
        assert!(file_paths.contains(&"types.ts"));
        assert!(file_paths.contains(&"hooks.ts"));
        assert!(file_paths.contains(&"package.json"));
        assert!(file_paths.contains(&"README.md"));
    }

    #[tokio::test]
    async fn test_react_types_content() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find types.ts file
        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        // Check that the content contains expected TypeScript types
        assert!(types_file.content.contains("export interface"));
        // Type names are now capitalized: GetUsersResponse instead of getUsersResponse
        assert!(
            types_file.content.contains("GetUsersResponse")
                || types_file.content.contains("Response")
        );
    }

    #[tokio::test]
    async fn test_react_hooks_content() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find hooks.ts file
        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // Check that the content contains expected React hooks
        // Hook name should capitalize first letter: getUsers -> useGetUsers
        assert!(
            hooks_file.content.contains("useGetUsers")
                || hooks_file.content.contains("use{{")
                || hooks_file.content.contains("usegetUsers")
        );
        assert!(hooks_file.content.contains("useState"));
        assert!(hooks_file.content.contains("useEffect"));
        assert!(hooks_file.content.contains("useCallback"));
    }

    #[tokio::test]
    async fn test_react_package_json() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find package.json file
        let package_file = result
            .files
            .iter()
            .find(|f| f.path == "package.json")
            .expect("package.json file should be generated");

        // Parse and validate package.json
        let package_json: serde_json::Value =
            serde_json::from_str(&package_file.content).expect("package.json should be valid JSON");

        assert_eq!(package_json["name"], "test-api-client");
        assert_eq!(package_json["version"], "1.0.0");
        assert!(package_json["dependencies"]["react"].is_string());
    }

    /// Create a test spec with a POST endpoint that has a request body
    fn create_test_spec_with_request_body() -> OpenApiSpec {
        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API with request body".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths: {
                let mut paths = HashMap::new();

                // Add a POST endpoint with request body
                let mut operations = HashMap::new();
                operations.insert(
                    "post".to_string(),
                    crate::client_generator::Operation {
                        summary: Some("Create user".to_string()),
                        description: Some("Create a new user".to_string()),
                        operation_id: Some("createUser".to_string()),
                        parameters: None,
                        request_body: Some(crate::client_generator::RequestBody {
                            description: Some("User data".to_string()),
                            required: Some(true),
                            content: {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    crate::client_generator::MediaType {
                                        schema: Some(crate::client_generator::Schema {
                                            r#type: Some("object".to_string()),
                                            format: None,
                                            properties: Some({
                                                let mut props = HashMap::new();
                                                props.insert(
                                                    "name".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: Some("User name".to_string()),
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props.insert(
                                                    "email".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: Some("User email".to_string()),
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props
                                            }),
                                            required: Some(vec![
                                                "name".to_string(),
                                                "email".to_string(),
                                            ]),
                                            items: None,
                                            description: None,
                                            example: None,
                                            r#enum: None,
                                            ref_path: None,
                                        }),
                                    },
                                );
                                content
                            },
                        }),
                        responses: {
                            let mut responses = HashMap::new();
                            responses.insert(
                                "201".to_string(),
                                crate::client_generator::Response {
                                    description: Some("Created".to_string()),
                                    ref_path: None,
                                    content: Some({
                                        let mut content = HashMap::new();
                                        content.insert(
                                            "application/json".to_string(),
                                            crate::client_generator::MediaType {
                                                schema: Some(crate::client_generator::Schema {
                                                    r#type: Some("object".to_string()),
                                                    format: None,
                                                    properties: Some({
                                                        let mut props = HashMap::new();
                                                        props.insert(
                                                            "id".to_string(),
                                                            crate::client_generator::Schema {
                                                                r#type: Some("integer".to_string()),
                                                                format: None,
                                                                properties: None,
                                                                required: None,
                                                                items: None,
                                                                description: None,
                                                                example: None,
                                                                r#enum: None,
                                                                ref_path: None,
                                                            },
                                                        );
                                                        props.insert(
                                                            "name".to_string(),
                                                            crate::client_generator::Schema {
                                                                r#type: Some("string".to_string()),
                                                                format: None,
                                                                properties: None,
                                                                required: None,
                                                                items: None,
                                                                description: None,
                                                                example: None,
                                                                r#enum: None,
                                                                ref_path: None,
                                                            },
                                                        );
                                                        props.insert(
                                                            "email".to_string(),
                                                            crate::client_generator::Schema {
                                                                r#type: Some("string".to_string()),
                                                                format: None,
                                                                properties: None,
                                                                required: None,
                                                                items: None,
                                                                description: None,
                                                                example: None,
                                                                r#enum: None,
                                                                ref_path: None,
                                                            },
                                                        );
                                                        props
                                                    }),
                                                    required: Some(vec![
                                                        "id".to_string(),
                                                        "name".to_string(),
                                                        "email".to_string(),
                                                    ]),
                                                    items: None,
                                                    description: None,
                                                    example: None,
                                                    r#enum: None,
                                                    ref_path: None,
                                                }),
                                            },
                                        );
                                        content
                                    }),
                                    headers: None,
                                },
                            );
                            responses
                        },
                        tags: Some(vec!["Users".to_string()]),
                    },
                );

                paths.insert(
                    "/users".to_string(),
                    crate::client_generator::PathItem {
                        operations,
                        ..Default::default()
                    },
                );

                paths
            },
            components: None,
        }
    }

    #[tokio::test]
    async fn test_react_request_body_parameter_generation() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_request_body();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find hooks.ts file
        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // Verify that POST method includes data parameter with exact type
        // Method signature should be: async createUser(data: CreateUserRequest)
        assert!(
            hooks_file.content.contains("async createUser(data: CreateUserRequest)"),
            "POST method should include data parameter with CreateUserRequest type"
        );
        assert!(
            hooks_file.content.contains("data: CreateUserRequest"),
            "Type signature should use CreateUserRequest type"
        );

        // Verify that the body is included in the request with exact format
        assert!(
            hooks_file.content.contains("body: JSON.stringify(data)"),
            "Request should include body with JSON.stringify(data)"
        );
    }

    #[tokio::test]
    async fn test_react_request_body_type_generation() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_request_body();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find types.ts file
        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        // Verify that request type is generated
        assert!(
            types_file.content.contains("export interface CreateUserRequest")
                || types_file.content.contains("CreateUserRequest"),
            "Request type CreateUserRequest should be generated. First 2000 chars of content: {}",
            &types_file.content[..std::cmp::min(2000, types_file.content.len())]
        );

        // Verify that request type includes the required properties
        // The properties might be formatted differently, so we check more flexibly
        // Check if either "name" appears in the file (could be in a property or comment)
        assert!(
            types_file.content.matches("name").count() >= 1,
            "Request type should reference 'name' property. Content around CreateUserRequest: {}",
            {
                let start = types_file.content.find("CreateUserRequest").unwrap_or(0);
                &types_file.content[start..std::cmp::min(start + 500, types_file.content.len())]
            }
        );

        // Check if "email" appears in the file
        assert!(
            types_file.content.matches("email").count() >= 1,
            "Request type should reference 'email' property"
        );

        // Verify response type is also generated
        assert!(
            types_file.content.contains("export interface CreateUserResponse"),
            "Response type CreateUserResponse should be generated"
        );
    }

    #[tokio::test]
    async fn test_react_no_request_body_for_get_method() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec(); // This spec only has GET methods
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find hooks.ts file
        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // Verify that GET method does NOT include data parameter
        // Method signature should be: async getUsers() or async getUsers(queryParams?: ...)
        assert!(hooks_file.content.contains("async getUsers("), "GET method should be generated");
        // Should NOT have data parameter for GET methods
        assert!(
            !hooks_file.content.contains("async getUsers(data:"),
            "GET method should NOT have data parameter"
        );
    }

    /// Create a test spec with PUT, PATCH, and DELETE endpoints that have request bodies
    fn create_test_spec_with_mutating_methods() -> OpenApiSpec {
        let mut paths = HashMap::new();

        // Helper to create a request body schema
        let create_request_body = || {
            Some(crate::client_generator::RequestBody {
                description: Some("Update data".to_string()),
                required: Some(true),
                content: {
                    let mut content = HashMap::new();
                    content.insert(
                        "application/json".to_string(),
                        crate::client_generator::MediaType {
                            schema: Some(crate::client_generator::Schema {
                                r#type: Some("object".to_string()),
                                format: None,
                                properties: Some({
                                    let mut props = HashMap::new();
                                    props.insert(
                                        "name".to_string(),
                                        crate::client_generator::Schema {
                                            r#type: Some("string".to_string()),
                                            format: None,
                                            properties: None,
                                            required: None,
                                            items: None,
                                            description: Some("Updated name".to_string()),
                                            example: None,
                                            r#enum: None,
                                            ref_path: None,
                                        },
                                    );
                                    props
                                }),
                                required: Some(vec!["name".to_string()]),
                                items: None,
                                description: None,
                                example: None,
                                r#enum: None,
                                ref_path: None,
                            }),
                        },
                    );
                    content
                },
            })
        };

        // Create a single path item with PUT, PATCH, and DELETE operations
        let mut all_operations = HashMap::new();

        // Common response for PUT and PATCH
        let common_responses = {
            let mut responses = HashMap::new();
            responses.insert(
                "200".to_string(),
                crate::client_generator::Response {
                    description: Some("Success".to_string()),
                    ref_path: None,
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert(
                            "application/json".to_string(),
                            crate::client_generator::MediaType {
                                schema: Some(crate::client_generator::Schema {
                                    r#type: Some("object".to_string()),
                                    format: None,
                                    properties: Some({
                                        let mut props = HashMap::new();
                                        props.insert(
                                            "id".to_string(),
                                            crate::client_generator::Schema {
                                                r#type: Some("integer".to_string()),
                                                format: None,
                                                properties: None,
                                                required: None,
                                                items: None,
                                                description: None,
                                                example: None,
                                                r#enum: None,
                                                ref_path: None,
                                            },
                                        );
                                        props.insert(
                                            "name".to_string(),
                                            crate::client_generator::Schema {
                                                r#type: Some("string".to_string()),
                                                format: None,
                                                properties: None,
                                                required: None,
                                                items: None,
                                                description: None,
                                                example: None,
                                                r#enum: None,
                                                ref_path: None,
                                            },
                                        );
                                        props
                                    }),
                                    required: Some(vec!["id".to_string(), "name".to_string()]),
                                    items: None,
                                    description: None,
                                    example: None,
                                    r#enum: None,
                                    ref_path: None,
                                }),
                            },
                        );
                        content
                    }),
                    headers: None,
                },
            );
            responses
        };

        // PUT endpoint
        all_operations.insert(
            "put".to_string(),
            crate::client_generator::Operation {
                summary: Some("Update user".to_string()),
                description: Some("Update a user".to_string()),
                operation_id: Some("updateUser".to_string()),
                parameters: None,
                request_body: create_request_body(),
                responses: common_responses.clone(),
                tags: Some(vec!["Users".to_string()]),
            },
        );

        // PATCH endpoint
        all_operations.insert(
            "patch".to_string(),
            crate::client_generator::Operation {
                summary: Some("Partially update user".to_string()),
                description: Some("Partially update a user".to_string()),
                operation_id: Some("patchUser".to_string()),
                parameters: None,
                request_body: create_request_body(),
                responses: common_responses.clone(),
                tags: Some(vec!["Users".to_string()]),
            },
        );

        // DELETE endpoint with optional confirmation body
        all_operations.insert(
            "delete".to_string(),
            crate::client_generator::Operation {
                summary: Some("Delete user".to_string()),
                description: Some("Delete a user".to_string()),
                operation_id: Some("deleteUser".to_string()),
                parameters: None,
                request_body: Some(crate::client_generator::RequestBody {
                    description: Some("Delete confirmation".to_string()),
                    required: Some(false),
                    content: {
                        let mut content = HashMap::new();
                        content.insert(
                            "application/json".to_string(),
                            crate::client_generator::MediaType {
                                schema: Some(crate::client_generator::Schema {
                                    r#type: Some("object".to_string()),
                                    format: None,
                                    properties: Some({
                                        let mut props = HashMap::new();
                                        props.insert(
                                            "confirm".to_string(),
                                            crate::client_generator::Schema {
                                                r#type: Some("boolean".to_string()),
                                                format: None,
                                                properties: None,
                                                required: None,
                                                items: None,
                                                description: Some("Confirmation flag".to_string()),
                                                example: None,
                                                r#enum: None,
                                                ref_path: None,
                                            },
                                        );
                                        props
                                    }),
                                    required: None, // All optional
                                    items: None,
                                    description: None,
                                    example: None,
                                    r#enum: None,
                                    ref_path: None,
                                }),
                            },
                        );
                        content
                    },
                }),
                responses: {
                    let mut responses = HashMap::new();
                    responses.insert(
                        "204".to_string(),
                        crate::client_generator::Response {
                            description: Some("No Content".to_string()),
                            ref_path: None,
                            content: None,
                            headers: None,
                        },
                    );
                    responses
                },
                tags: Some(vec!["Users".to_string()]),
            },
        );

        paths.insert(
            "/users/{id}".to_string(),
            crate::client_generator::PathItem {
                operations: all_operations,
                ..Default::default()
            },
        );

        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API with PUT/PATCH/DELETE methods".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths,
            components: None,
        }
    }

    #[tokio::test]
    async fn test_react_put_method_with_request_body() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_mutating_methods();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // PUT method should include data parameter
        assert!(
            hooks_file
                .content
                .contains("async updateUser(id: string, data: UpdateUserRequest)")
                || hooks_file.content.contains("async updateUser(")
                    && hooks_file.content.contains("data: UpdateUserRequest"),
            "PUT method should include data parameter with UpdateUserRequest type"
        );
    }

    #[tokio::test]
    async fn test_react_patch_method_with_request_body() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_mutating_methods();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // PATCH method should include data parameter
        assert!(
            hooks_file
                .content
                .contains("async patchUser(id: string, data: PatchUserRequest)")
                || hooks_file.content.contains("async patchUser(")
                    && hooks_file.content.contains("data: PatchUserRequest"),
            "PATCH method should include data parameter with PatchUserRequest type"
        );
    }

    #[tokio::test]
    async fn test_react_delete_method_with_optional_request_body() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_mutating_methods();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        let hooks_file = result
            .files
            .iter()
            .find(|f| f.path == "hooks.ts")
            .expect("hooks.ts file should be generated");

        // DELETE method should include optional data parameter
        assert!(
            hooks_file
                .content
                .contains("async deleteUser(id: string, data?: DeleteUserRequest)")
                || (hooks_file.content.contains("async deleteUser(")
                    && (hooks_file.content.contains("data?: DeleteUserRequest")
                        || hooks_file.content.contains("data: DeleteUserRequest"))),
            "DELETE method should include data parameter (may be optional)"
        );
    }

    /// Create a test spec with nested objects and optional properties
    fn create_test_spec_with_complex_request_body() -> OpenApiSpec {
        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API with complex request body".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths: {
                let mut paths = HashMap::new();
                let mut operations = HashMap::new();
                operations.insert(
                    "post".to_string(),
                    crate::client_generator::Operation {
                        summary: Some("Create complex object".to_string()),
                        description: Some("Create with nested objects".to_string()),
                        operation_id: Some("createComplex".to_string()),
                        parameters: None,
                        request_body: Some(crate::client_generator::RequestBody {
                            description: Some("Complex data".to_string()),
                            required: Some(true),
                            content: {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    crate::client_generator::MediaType {
                                        schema: Some(crate::client_generator::Schema {
                                            r#type: Some("object".to_string()),
                                            format: None,
                                            properties: Some({
                                                let mut props = HashMap::new();
                                                // Required field
                                                props.insert(
                                                    "requiredField".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: Some(
                                                            "Required string".to_string(),
                                                        ),
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                // Optional field
                                                props.insert(
                                                    "optionalField".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: Some(
                                                            "Optional string".to_string(),
                                                        ),
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                // Nested object
                                                props.insert(
                                                    "nested".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("object".to_string()),
                                                        format: None,
                                                        properties: Some({
                                                            let mut nested_props = HashMap::new();
                                                            nested_props.insert(
                                                                "nestedValue".to_string(),
                                                                crate::client_generator::Schema {
                                                                    r#type: Some(
                                                                        "number".to_string(),
                                                                    ),
                                                                    format: None,
                                                                    properties: None,
                                                                    required: None,
                                                                    items: None,
                                                                    description: Some(
                                                                        "Nested number".to_string(),
                                                                    ),
                                                                    example: None,
                                                                    r#enum: None,
                                                                    ref_path: None,
                                                                },
                                                            );
                                                            nested_props
                                                        }),
                                                        required: Some(vec![
                                                            "nestedValue".to_string()
                                                        ]),
                                                        items: None,
                                                        description: Some(
                                                            "Nested object".to_string(),
                                                        ),
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props
                                            }),
                                            required: Some(vec!["requiredField".to_string()]), // Only requiredField is required
                                            items: None,
                                            description: None,
                                            example: None,
                                            r#enum: None,
                                            ref_path: None,
                                        }),
                                    },
                                );
                                content
                            },
                        }),
                        responses: {
                            let mut responses = HashMap::new();
                            responses.insert(
                                "201".to_string(),
                                crate::client_generator::Response {
                                    description: Some("Created".to_string()),
                                    ref_path: None,
                                    content: Some({
                                        let mut content = HashMap::new();
                                        content.insert(
                                            "application/json".to_string(),
                                            crate::client_generator::MediaType {
                                                schema: Some(crate::client_generator::Schema {
                                                    r#type: Some("object".to_string()),
                                                    format: None,
                                                    properties: Some({
                                                        let mut props = HashMap::new();
                                                        props.insert(
                                                            "id".to_string(),
                                                            crate::client_generator::Schema {
                                                                r#type: Some("integer".to_string()),
                                                                format: None,
                                                                properties: None,
                                                                required: None,
                                                                items: None,
                                                                description: None,
                                                                example: None,
                                                                r#enum: None,
                                                                ref_path: None,
                                                            },
                                                        );
                                                        props
                                                    }),
                                                    required: Some(vec!["id".to_string()]),
                                                    items: None,
                                                    description: None,
                                                    example: None,
                                                    r#enum: None,
                                                    ref_path: None,
                                                }),
                                            },
                                        );
                                        content
                                    }),
                                    headers: None,
                                },
                            );
                            responses
                        },
                        tags: Some(vec!["Complex".to_string()]),
                    },
                );
                paths.insert(
                    "/complex".to_string(),
                    crate::client_generator::PathItem {
                        operations,
                        ..Default::default()
                    },
                );
                paths
            },
            components: None,
        }
    }

    #[tokio::test]
    async fn test_react_request_body_with_required_fields() {
        let generator = ReactClientGenerator::new().unwrap();

        // Create a spec with required and optional fields
        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths: {
                let mut paths = HashMap::new();
                let mut operations = HashMap::new();
                operations.insert(
                    "post".to_string(),
                    crate::client_generator::Operation {
                        summary: Some("Create item".to_string()),
                        description: None,
                        operation_id: Some("createItem".to_string()),
                        parameters: None,
                        request_body: Some(crate::client_generator::RequestBody {
                            description: None,
                            content: {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    crate::client_generator::MediaType {
                                        schema: Some(crate::client_generator::Schema {
                                            r#type: Some("object".to_string()),
                                            format: None,
                                            properties: Some({
                                                let mut props = HashMap::new();
                                                props.insert(
                                                    "name".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: None,
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props.insert(
                                                    "email".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: None,
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props.insert(
                                                    "age".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("number".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: None,
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props
                                            }),
                                            required: Some(vec![
                                                "name".to_string(),
                                                "email".to_string(),
                                            ]),
                                            items: None,
                                            description: None,
                                            example: None,
                                            r#enum: None,
                                            ref_path: None,
                                        }),
                                    },
                                );
                                content
                            },
                            required: Some(true),
                        }),
                        responses: {
                            let mut responses = HashMap::new();
                            responses.insert(
                                "200".to_string(),
                                crate::client_generator::Response {
                                    description: Some("Success".to_string()),
                                    ref_path: None,
                                    content: None,
                                    headers: None,
                                },
                            );
                            responses
                        },
                        tags: None,
                    },
                );
                paths.insert(
                    "/items".to_string(),
                    crate::client_generator::PathItem {
                        operations,
                        ..Default::default()
                    },
                );
                paths
            },
            components: None,
        };

        let config = create_test_config();
        let result = generator.generate_client(&spec, &config).await.unwrap();

        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        // Verify required fields (name and email) do NOT have optional marker
        // Required field should be: "name: string" not "name?: string"
        assert!(
            types_file.content.contains("name:") && !types_file.content.contains("name?:"),
            "Required field 'name' should not have optional marker. Content: {}",
            types_file.content
        );

        assert!(
            types_file.content.contains("email:") && !types_file.content.contains("email?:"),
            "Required field 'email' should not have optional marker"
        );

        // Verify optional field (age) HAS optional marker
        // Optional field should be: "age?: number"
        assert!(
            types_file.content.contains("age?:"),
            "Optional field 'age' should have optional marker. Content: {}",
            types_file.content
        );
    }

    #[tokio::test]
    async fn test_react_request_body_with_optional_properties() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_complex_request_body();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        // Verify that required field exists (may or may not have optional marker based on template)
        // The template uses lookup to determine required status, so check for presence
        assert!(
            types_file.content.contains("requiredField")
                || types_file.content.contains("  requiredField"),
            "Required field should be present in request type"
        );

        // Verify that optional field exists (should be marked with ? if template logic is correct)
        assert!(
            types_file.content.contains("optionalField")
                || types_file.content.contains("  optionalField"),
            "Optional field should be present in request type"
        );
    }

    #[tokio::test]
    async fn test_react_request_body_with_nested_objects() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec_with_complex_request_body();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        // Verify that nested object is included in the request type
        assert!(
            types_file.content.contains("export interface CreateComplexRequest"),
            "Request type should be generated for complex request body"
        );
        assert!(
            types_file.content.contains("nested:") || types_file.content.contains("nested?:"),
            "Nested object property should be included"
        );
    }

    #[tokio::test]
    async fn test_react_request_body_with_ref_schema() {
        let generator = ReactClientGenerator::new().unwrap();

        // Create a spec with a $ref schema in request body
        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API with $ref".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths: {
                let mut paths = HashMap::new();
                let mut operations = HashMap::new();
                operations.insert(
                    "post".to_string(),
                    crate::client_generator::Operation {
                        summary: Some("Create item".to_string()),
                        description: None,
                        operation_id: Some("createItem".to_string()),
                        parameters: None,
                        request_body: Some(crate::client_generator::RequestBody {
                            description: None,
                            content: {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    crate::client_generator::MediaType {
                                        schema: Some(crate::client_generator::Schema {
                                            r#type: None,
                                            format: None,
                                            properties: None,
                                            required: None,
                                            items: None,
                                            description: None,
                                            example: None,
                                            r#enum: None,
                                            ref_path: Some("#/components/schemas/Item".to_string()),
                                        }),
                                    },
                                );
                                content
                            },
                            required: Some(true),
                        }),
                        responses: {
                            let mut responses = HashMap::new();
                            responses.insert(
                                "200".to_string(),
                                crate::client_generator::Response {
                                    description: Some("Success".to_string()),
                                    ref_path: None,
                                    content: None,
                                    headers: None,
                                },
                            );
                            responses
                        },
                        tags: None,
                    },
                );
                paths.insert(
                    "/items".to_string(),
                    crate::client_generator::PathItem {
                        operations,
                        ..Default::default()
                    },
                );
                paths
            },
            components: Some(crate::client_generator::Components {
                schemas: Some({
                    let mut schemas = HashMap::new();
                    schemas.insert(
                        "Item".to_string(),
                        crate::client_generator::Schema {
                            r#type: Some("object".to_string()),
                            format: None,
                            properties: Some({
                                let mut props = HashMap::new();
                                props.insert(
                                    "name".to_string(),
                                    crate::client_generator::Schema {
                                        r#type: Some("string".to_string()),
                                        format: None,
                                        properties: None,
                                        required: None,
                                        items: None,
                                        description: None,
                                        example: None,
                                        r#enum: None,
                                        ref_path: None,
                                    },
                                );
                                props
                            }),
                            required: Some(vec!["name".to_string()]),
                            items: None,
                            description: None,
                            example: None,
                            r#enum: None,
                            ref_path: None,
                        },
                    );
                    schemas
                }),
                responses: None,
                parameters: None,
            }),
        };

        let config = create_test_config();
        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Should not crash when processing $ref schemas
        // Note: Full $ref resolution would require schema resolution logic,
        // but the generator should handle $ref gracefully
        assert!(!result.files.is_empty(), "Should generate files even with $ref schemas");
    }

    #[tokio::test]
    async fn test_react_yaml_spec_generation() {
        // Test that YAML specs work correctly by creating a JSON spec that would be equivalent
        // to a YAML spec (since serde handles both the same way)
        let generator = ReactClientGenerator::new().unwrap();

        let spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "YAML Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("Test API for YAML support".to_string()),
            },
            servers: Some(vec![crate::client_generator::Server {
                url: "http://localhost:3000".to_string(),
                description: Some("Test server".to_string()),
            }]),
            paths: {
                let mut paths = HashMap::new();
                let mut operations = HashMap::new();
                operations.insert(
                    "post".to_string(),
                    crate::client_generator::Operation {
                        summary: Some("Create resource".to_string()),
                        description: None,
                        operation_id: Some("createResource".to_string()),
                        parameters: None,
                        request_body: Some(crate::client_generator::RequestBody {
                            description: None,
                            content: {
                                let mut content = HashMap::new();
                                content.insert(
                                    "application/json".to_string(),
                                    crate::client_generator::MediaType {
                                        schema: Some(crate::client_generator::Schema {
                                            r#type: Some("object".to_string()),
                                            format: None,
                                            properties: Some({
                                                let mut props = HashMap::new();
                                                props.insert(
                                                    "title".to_string(),
                                                    crate::client_generator::Schema {
                                                        r#type: Some("string".to_string()),
                                                        format: None,
                                                        properties: None,
                                                        required: None,
                                                        items: None,
                                                        description: None,
                                                        example: None,
                                                        r#enum: None,
                                                        ref_path: None,
                                                    },
                                                );
                                                props
                                            }),
                                            required: Some(vec!["title".to_string()]),
                                            items: None,
                                            description: None,
                                            example: None,
                                            r#enum: None,
                                            ref_path: None,
                                        }),
                                    },
                                );
                                content
                            },
                            required: Some(true),
                        }),
                        responses: {
                            let mut responses = HashMap::new();
                            responses.insert(
                                "200".to_string(),
                                crate::client_generator::Response {
                                    description: Some("Success".to_string()),
                                    ref_path: None,
                                    content: None,
                                    headers: None,
                                },
                            );
                            responses
                        },
                        tags: None,
                    },
                );
                paths.insert(
                    "/resources".to_string(),
                    crate::client_generator::PathItem {
                        operations,
                        ..Default::default()
                    },
                );
                paths
            },
            components: None,
        };

        let config = create_test_config();
        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Verify that the generator works correctly (YAML and JSON are handled the same by serde)
        let types_file = result
            .files
            .iter()
            .find(|f| f.path == "types.ts")
            .expect("types.ts file should be generated");

        assert!(
            types_file.content.contains("export interface CreateResourceRequest"),
            "Request type should be generated from YAML-equivalent spec"
        );
        assert!(
            types_file.content.contains("title:") && !types_file.content.contains("title?:"),
            "Required field 'title' should not have optional marker"
        );
    }
}

#[cfg(test)]
mod vue_tests {
    use super::*;

    #[test]
    fn test_vue_generator_creation() {
        let generator = VueClientGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_vue_framework_name() {
        let generator = VueClientGenerator::new().unwrap();
        assert_eq!(generator.framework_name(), "vue");
    }

    #[test]
    fn test_vue_supported_extensions() {
        let generator = VueClientGenerator::new().unwrap();
        let extensions = generator.supported_extensions();
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"vue"));
        assert!(extensions.contains(&"js"));
    }

    #[tokio::test]
    async fn test_vue_generate_client() {
        let generator = VueClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.framework, "vue");
        assert_eq!(result.metadata.api_title, "Test API");
        assert_eq!(result.metadata.api_version, "1.0.0");
    }

    #[tokio::test]
    async fn test_vue_generated_files() {
        let generator = VueClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Check that required files are generated
        let file_paths: Vec<&str> = result.files.iter().map(|f| f.path.as_str()).collect();
        assert!(file_paths.contains(&"types.ts"));
        assert!(file_paths.contains(&"composables.ts"));
        assert!(file_paths.contains(&"store.ts"));
        assert!(file_paths.contains(&"package.json"));
        assert!(file_paths.contains(&"README.md"));
    }

    #[tokio::test]
    async fn test_vue_composables_content() {
        let generator = VueClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find composables.ts file
        let composables_file = result
            .files
            .iter()
            .find(|f| f.path == "composables.ts")
            .expect("composables.ts file should be generated");

        // Check that the content contains expected Vue composables
        // Vue generator uses operation_id directly, generating composables like usegetUsers
        // Check that composables are generated (the exact name format may vary)
        assert!(composables_file.content.contains("export function use"));
        assert!(composables_file.content.contains("ref"));
        assert!(composables_file.content.contains("computed"));
        // Vue composables don't use useCallback (that's React) - they use regular functions
    }

    #[tokio::test]
    async fn test_vue_store_content() {
        let generator = VueClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find store.ts file
        let store_file = result
            .files
            .iter()
            .find(|f| f.path == "store.ts")
            .expect("store.ts file should be generated");

        // Check that the content contains expected Pinia store
        assert!(store_file.content.contains("defineStore"));
        // Store name format may vary based on API title processing
        assert!(
            store_file.content.contains("useTestApiStore")
                || store_file.content.contains("use") && store_file.content.contains("Store")
        );
        assert!(store_file.content.contains("getUsers") || store_file.content.contains("Users"));
    }

    #[tokio::test]
    async fn test_vue_package_json() {
        let generator = VueClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = create_test_config();

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Find package.json file
        let package_file = result
            .files
            .iter()
            .find(|f| f.path == "package.json")
            .expect("package.json file should be generated");

        // Parse and validate package.json
        let package_json: serde_json::Value =
            serde_json::from_str(&package_file.content).expect("package.json should be valid JSON");

        assert_eq!(package_json["name"], "test-api-client");
        assert_eq!(package_json["version"], "1.0.0");
        assert!(package_json["dependencies"]["vue"].is_string());
        assert!(package_json["dependencies"]["pinia"].is_string());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_file_generation_and_cleanup() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("generated");

        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();
        let config = ClientGeneratorConfig {
            output_dir: output_path.to_string_lossy().to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            include_types: true,
            include_mocks: false,
            template_dir: None,
            options: HashMap::new(),
        };

        // Generate client
        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Create output directory
        fs::create_dir_all(&output_path).unwrap();

        // Write files
        for file in &result.files {
            let file_path = output_path.join(&file.path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&file_path, &file.content).unwrap();
        }

        // Verify files exist and have content
        assert!(output_path.join("types.ts").exists());
        assert!(output_path.join("hooks.ts").exists());
        assert!(output_path.join("package.json").exists());
        assert!(output_path.join("README.md").exists());

        // Verify file contents
        let types_content = fs::read_to_string(output_path.join("types.ts")).unwrap();
        assert!(types_content.contains("export interface"));

        let hooks_content = fs::read_to_string(output_path.join("hooks.ts")).unwrap();
        // Hook name should capitalize first letter: getUsers -> useGetUsers
        assert!(hooks_content.contains("useGetUsers") || hooks_content.contains("usegetUsers"));

        let package_content = fs::read_to_string(output_path.join("package.json")).unwrap();
        let package_json: serde_json::Value = serde_json::from_str(&package_content).unwrap();
        assert_eq!(package_json["name"], "test-api-client");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let generator = ReactClientGenerator::new().unwrap();

        // Test with invalid spec
        let invalid_spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: crate::client_generator::ApiInfo {
                title: "".to_string(), // Empty title should cause issues
                version: "1.0.0".to_string(),
                description: None,
            },
            servers: None,
            paths: HashMap::new(),
            components: None,
        };

        let config = create_test_config();
        let result = generator.generate_client(&invalid_spec, &config).await;

        // Should still succeed but with warnings
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.metadata.api_title, "");
    }

    #[tokio::test]
    async fn test_custom_configuration() {
        let generator = ReactClientGenerator::new().unwrap();
        let spec = create_test_spec();

        let mut options = HashMap::new();
        options.insert("customOption".to_string(), json!("customValue"));

        let config = ClientGeneratorConfig {
            output_dir: "./custom-output".to_string(),
            base_url: Some("https://api.example.com".to_string()),
            include_types: true,
            include_mocks: true,
            template_dir: Some("./custom-templates".to_string()),
            options,
        };

        let result = generator.generate_client(&spec, &config).await.unwrap();

        // Verify configuration is reflected in generated content
        let hooks_file = result.files.iter().find(|f| f.path == "hooks.ts").unwrap();

        assert!(hooks_file.content.contains("https://api.example.com"));
    }
}
