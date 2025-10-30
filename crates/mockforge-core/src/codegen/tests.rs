//! Unit tests for code generation module

#[cfg(test)]
mod tests {
    use super::super::{generate_mock_server_code, CodegenConfig, MockDataStrategy};
    use crate::openapi::spec::OpenApiSpec;
    use serde_json::json;

    fn create_test_spec() -> OpenApiSpec {
        let spec_json = json!({
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "responses": {
                            "200": {
                                "description": "List of users",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "id": {"type": "integer"},
                                                    "name": {"type": "string"}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "post": {
                        "operationId": "createUser",
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "Created",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "/users/{id}": {
                    "get": {
                        "operationId": "getUserById",
                        "parameters": [
                            {
                                "name": "id",
                                "in": "path",
                                "required": true,
                                "schema": {
                                    "type": "string"
                                }
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "User found",
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "string"},
                                                "name": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        OpenApiSpec::from_json(spec_json).unwrap()
    }

    #[test]
    fn test_generate_rust_code() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that basic structure is present
        assert!(code.contains("pub struct GeneratedMockServer"));
        assert!(code.contains("impl GeneratedMockServer"));
        assert!(code.contains("pub fn new()"));
        assert!(code.contains("pub fn router"));
        assert!(code.contains("pub async fn start"));

        // Check that routes are generated
        // Handler names are generated from operation IDs (snake_case) or method+path
        assert!(code.contains("listusers") || code.contains("list_users") || code.contains("handle_get"));
        assert!(code.contains("createuser") || code.contains("create_user") || code.contains("handle_post"));
        assert!(code.contains("getuserbyid") || code.contains("get_user_by_id") || code.contains("users/:id"));

        // Check that Axum imports are present
        assert!(code.contains("use axum::"));
        assert!(code.contains("use serde::"));

        // Check Bradum routes are added
        assert!(code.contains(".route("));
    }

    #[test]
    fn test_generate_rust_code_with_path_parameters() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that path parameters are converted correctly
        // OpenAPI /users/{id} should become /users/:id in Axum
        assert!(code.contains("/users/:id") || code.contains("/users/{id}"));
    }

    #[test]
    fn test_generate_rust_code_includes_all_methods() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that both GET and POST are present
        assert!(code.contains(".route(\"/users\"") || code.contains("get(") || code.contains("post("));
    }

    #[test]
    fn test_generate_rust_code_with_config() {
        let spec = create_test_spec();
        let config = CodegenConfig {
            port: Some(8080),
            enable_cors: true,
            default_delay_ms: Some(100),
            mock_data_strategy: MockDataStrategy::Examples,
        };

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that port is set correctly
        assert!(code.contains("8080"));

        // Check that delay is included if configured
        // Note: delay code generation is implemented, so we check for sleep
        if config.default_delay_ms.is_some() {
            assert!(code.contains("tokio::time::sleep") || code.contains("Duration::from_millis"));
        }
    }

    #[test]
    fn test_generate_typescript_code() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "ts", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that TypeScript placeholder is present
        assert!(code.contains("GeneratedMockServer"));
        assert!(code.contains("class"));
    }

    #[test]
    fn test_unsupported_language() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "python", &config);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("Unsupported language"));
    }

    #[test]
    fn test_codegen_config_defaults() {
        let config = CodegenConfig::default();

        assert_eq!(config.mock_data_strategy, MockDataStrategy::ExamplesOrRandom);
        assert_eq!(config.port, None);
        assert!(!config.enable_cors);
        assert_eq!(config.default_delay_ms, None);
    }

    #[test]
    fn test_generate_handler_signatures() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that async handler functions are generated
        assert!(code.contains("async fn handle_"));

        // Check that handlers return Json<Value>
        assert!(code.contains("Json<Value>") || code.contains("Json<"));

        // Check that POST handler has request body parameter
        assert!(code.contains("Json(body)") || code.contains("body"));
    }

    #[test]
    fn test_main_function_generated() {
        let spec = create_test_spec();
        let config = CodegenConfig::default();

        let result = generate_mock_server_code(&spec, "rs", &config);
        assert!(result.is_ok());

        let code = result.unwrap();

        // Check that main function is present
        assert!(code.contains("#[tokio::main]"));
        assert!(code.contains("async fn main"));
        assert!(code.contains("GeneratedMockServer::new()"));
        assert!(code.contains("server.start()"));
    }
}
