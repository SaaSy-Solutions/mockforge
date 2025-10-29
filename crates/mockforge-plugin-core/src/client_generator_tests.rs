//! Tests for multi-framework client generation
//!
//! This module contains comprehensive tests for the client generation
//! system, including React and Vue generators.

use crate::client_generator::{
    ClientGeneratorConfig, ClientGeneratorPlugin, OpenApiSpec,
};
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
                                description: "Success".to_string(),
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

            paths.insert("/users".to_string(), crate::client_generator::PathItem { operations });

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
        assert!(types_file.content.contains("getUsersResponse"));
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
        assert!(hooks_file.content.contains("useGetUsers"));
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
        assert!(composables_file.content.contains("useGetUsers"));
        assert!(composables_file.content.contains("ref"));
        assert!(composables_file.content.contains("computed"));
        assert!(composables_file.content.contains("useCallback"));
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
        assert!(store_file.content.contains("useTestApiStore"));
        assert!(store_file.content.contains("getUsers"));
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
        assert!(hooks_content.contains("useGetUsers"));

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
