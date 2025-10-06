use mockforge_core::import::{generate_commands_from_openapi, CommandGenerationOptions, CommandFormat};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test OpenAPI spec
    let openapi_spec = r#"{
        "openapi": "3.0.3",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "https://api.example.com"
            }
        ],
        "security": [
            {
                "bearerAuth": []
            }
        ],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer"
                },
                "apiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "X-API-Key"
                }
            }
        },
        "paths": {
            "/users": {
                "get": {
                    "operationId": "getUsers",
                    "summary": "Get all users",
                    "security": [
                        {
                            "bearerAuth": []
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "example": {"users": []}
                                }
                            }
                        }
                    }
                },
                "post": {
                    "operationId": "createUser",
                    "summary": "Create a user",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "example": {"name": "John", "email": "john@example.com"}
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created"
                        }
                    }
                }
            }
        }
    }"#;

    let options = CommandGenerationOptions {
        base_url: Some("https://api.example.com".to_string()),
        format: CommandFormat::Both,
        include_auth: true,
        all_operations: true,
        include_examples: true,
        custom_headers: std::collections::HashMap::new(),
        max_examples_per_operation: 1,
    };

    match generate_commands_from_openapi(openapi_spec, options) {
        Ok(result) => {
            println!("✅ Successfully generated {} commands", result.commands.len());
            println!("Spec: {} v{}", result.spec_info.title, result.spec_info.version);

            for command in result.commands {
                println!("\n--- {} {} ---", command.method, command.path_template);
                println!("Curl command:");
                println!("{}", command.curl_command);
                println!("\nHTTPie command:");
                println!("{}", command.httpie_command);
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
