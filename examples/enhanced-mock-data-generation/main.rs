//! Example demonstrating enhanced mock data generation
//!
//! This example shows how to use the enhanced mock data generation
//! features to generate realistic mock data from OpenAPI specifications
//! and start a mock server.

use mockforge_data::{
    MockDataGenerator, MockGeneratorConfig, MockServer, MockServerConfig,
    MockServerBuilder, start_mock_server,
};
use serde_json::json;
use std::path::PathBuf;

/// Example OpenAPI specification for a user management API
fn create_example_openapi_spec() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "User Management API",
            "version": "1.0.0",
            "description": "A comprehensive user management API with realistic data patterns"
        },
        "servers": [
            {
                "url": "https://api.example.com/v1",
                "description": "Production server"
            }
        ],
        "paths": {
            "/api/users": {
                "get": {
                    "summary": "List all users",
                    "description": "Retrieve a paginated list of all users",
                    "parameters": [
                        {
                            "name": "page",
                            "in": "query",
                            "schema": {
                                "type": "integer",
                                "minimum": 1,
                                "default": 1
                            }
                        },
                        {
                            "name": "limit",
                            "in": "query",
                            "schema": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "default": 20
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Successful response",
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
                                            "pagination": {
                                                "$ref": "#/components/schemas/Pagination"
                                            }
                                        },
                                        "required": ["users", "pagination"]
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a new user",
                    "description": "Create a new user account",
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
                            "description": "User created successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Error"
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
                    "description": "Retrieve a specific user by their ID",
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
                            "description": "User found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "User not found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Error"
                                    }
                                }
                            }
                        }
                    }
                },
                "put": {
                    "summary": "Update user",
                    "description": "Update an existing user",
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
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/UpdateUserRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "User updated successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/User"
                                    }
                                }
                            }
                        }
                    }
                },
                "delete": {
                    "summary": "Delete user",
                    "description": "Delete a user account",
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
                        "204": {
                            "description": "User deleted successfully"
                        }
                    }
                }
            },
            "/api/users/{id}/profile": {
                "get": {
                    "summary": "Get user profile",
                    "description": "Retrieve detailed profile information for a user",
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
                            "description": "Profile found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/UserProfile"
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
                            "format": "uuid",
                            "description": "Unique identifier for the user"
                        },
                        "email": {
                            "type": "string",
                            "format": "email",
                            "description": "User's email address"
                        },
                        "username": {
                            "type": "string",
                            "minLength": 3,
                            "maxLength": 30,
                            "description": "Unique username"
                        },
                        "first_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50,
                            "description": "User's first name"
                        },
                        "last_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50,
                            "description": "User's last name"
                        },
                        "phone_number": {
                            "type": "string",
                            "description": "User's phone number"
                        },
                        "date_of_birth": {
                            "type": "string",
                            "format": "date",
                            "description": "User's date of birth"
                        },
                        "is_active": {
                            "type": "boolean",
                            "description": "Whether the user account is active"
                        },
                        "is_verified": {
                            "type": "boolean",
                            "description": "Whether the user's email is verified"
                        },
                        "created_at": {
                            "type": "string",
                            "format": "date-time",
                            "description": "When the user account was created"
                        },
                        "updated_at": {
                            "type": "string",
                            "format": "date-time",
                            "description": "When the user account was last updated"
                        },
                        "last_login_at": {
                            "type": "string",
                            "format": "date-time",
                            "description": "When the user last logged in"
                        },
                        "profile": {
                            "$ref": "#/components/schemas/UserProfile"
                        }
                    },
                    "required": [
                        "id", "email", "username", "first_name", "last_name",
                        "is_active", "is_verified", "created_at", "updated_at"
                    ]
                },
                "UserProfile": {
                    "type": "object",
                    "properties": {
                        "bio": {
                            "type": "string",
                            "maxLength": 500,
                            "description": "User's biography"
                        },
                        "avatar_url": {
                            "type": "string",
                            "format": "uri",
                            "description": "URL to user's avatar image"
                        },
                        "website": {
                            "type": "string",
                            "format": "uri",
                            "description": "User's personal website"
                        },
                        "location": {
                            "type": "string",
                            "description": "User's location"
                        },
                        "company": {
                            "type": "string",
                            "description": "User's company"
                        },
                        "job_title": {
                            "type": "string",
                            "description": "User's job title"
                        },
                        "interests": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "maxItems": 10,
                            "description": "User's interests"
                        },
                        "social_links": {
                            "type": "object",
                            "properties": {
                                "twitter": {
                                    "type": "string",
                                    "format": "uri"
                                },
                                "linkedin": {
                                    "type": "string",
                                    "format": "uri"
                                },
                                "github": {
                                    "type": "string",
                                    "format": "uri"
                                }
                            }
                        }
                    }
                },
                "CreateUserRequest": {
                    "type": "object",
                    "properties": {
                        "email": {
                            "type": "string",
                            "format": "email"
                        },
                        "username": {
                            "type": "string",
                            "minLength": 3,
                            "maxLength": 30
                        },
                        "first_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "last_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "phone_number": {
                            "type": "string"
                        },
                        "date_of_birth": {
                            "type": "string",
                            "format": "date"
                        }
                    },
                    "required": ["email", "username", "first_name", "last_name"]
                },
                "UpdateUserRequest": {
                    "type": "object",
                    "properties": {
                        "first_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "last_name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "phone_number": {
                            "type": "string"
                        },
                        "date_of_birth": {
                            "type": "string",
                            "format": "date"
                        }
                    }
                },
                "Pagination": {
                    "type": "object",
                    "properties": {
                        "page": {
                            "type": "integer",
                            "minimum": 1
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100
                        },
                        "total": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "total_pages": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "has_next": {
                            "type": "boolean"
                        },
                        "has_prev": {
                            "type": "boolean"
                        }
                    },
                    "required": ["page", "limit", "total", "total_pages", "has_next", "has_prev"]
                },
                "Error": {
                    "type": "object",
                    "properties": {
                        "error": {
                            "type": "string"
                        },
                        "message": {
                            "type": "string"
                        },
                        "code": {
                            "type": "integer"
                        },
                        "details": {
                            "type": "object"
                        }
                    },
                    "required": ["error", "message", "code"]
                }
            }
        }
    })
}

/// Example 1: Generate mock data from OpenAPI specification
async fn example_generate_mock_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Example 1: Generating mock data from OpenAPI specification");

    let spec = create_example_openapi_spec();

    // Create generator configuration
    let config = MockGeneratorConfig::new()
        .realistic_mode(true)
        .include_optional_fields(true)
        .validate_generated_data(true)
        .default_array_size(3)
        .max_array_size(10);

    // Generate mock data
    let mut generator = MockDataGenerator::with_config(config);
    let result = generator.generate_from_openapi_spec(&spec)?;

    println!("✅ Generated mock data:");
    println!("   📊 Schemas: {}", result.schemas.len());
    println!("   🔗 Responses: {}", result.responses.len());
    println!("   ⚠️  Warnings: {}", result.warnings.len());

    // Display some generated data
    if let Some(user_data) = result.schemas.get("User") {
        println!("\n📋 Sample User data:");
        println!("{}", serde_json::to_string_pretty(user_data)?);
    }

    // Display some responses
    if let Some(response) = result.responses.get("GET /api/users") {
        println!("\n🔗 Sample GET /api/users response:");
        println!("{}", serde_json::to_string_pretty(&response.body)?);
    }

    Ok(())
}

/// Example 2: Start a mock server
async fn example_start_mock_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌐 Example 2: Starting mock server");

    let spec = create_example_openapi_spec();

    // Create server configuration
    let config = MockServerConfig::new(spec)
        .port(3000)
        .host("127.0.0.1".to_string())
        .enable_cors(true)
        .log_requests(true)
        .response_delay("/api/users".to_string(), 100)
        .generator_config(
            MockGeneratorConfig::new()
                .realistic_mode(true)
                .validate_generated_data(true)
        );

    println!("🚀 Starting mock server on http://127.0.0.1:3000");
    println!("📋 OpenAPI spec loaded with {} paths",
        config.openapi_spec.get("paths")
            .and_then(|p| p.as_object())
            .map(|p| p.len())
            .unwrap_or(0));
    println!("⏱️  Response delay configured for /api/users: 100ms");
    println!("🛑 Press Ctrl+C to stop the server");

    // Start the server
    let server = MockServer::new(config)?;
    server.start().await?;

    Ok(())
}

/// Example 3: Using the builder pattern
async fn example_builder_pattern() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 Example 3: Using builder pattern");

    let spec = create_example_openapi_spec();

    // Use builder pattern for server configuration
    let server = MockServerBuilder::new(spec)
        .port(8080)
        .host("0.0.0.0".to_string())
        .enable_cors(true)
        .log_requests(true)
        .response_delay("/api/users".to_string(), 200)
        .response_delay("/api/users/{id}".to_string(), 150)
        .build()?;

    println!("🚀 Starting mock server with builder pattern");
    println!("📡 Server will be available at: http://0.0.0.0:8080");
    println!("⏱️  Response delays configured:");
    println!("   - /api/users: 200ms");
    println!("   - /api/users/{{id}}: 150ms");

    server.start().await?;

    Ok(())
}

/// Example 4: Quick start function
async fn example_quick_start() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Example 4: Quick start function");

    let spec = create_example_openapi_spec();

    println!("🚀 Starting mock server with quick start function");
    println!("📡 Server will be available at: http://127.0.0.1:3000");

    // Use the quick start function
    start_mock_server(spec, 3000).await?;

    Ok(())
}

/// Main function demonstrating all examples
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 MockForge Enhanced Mock Data Generation Examples");
    println!("==================================================");

    // Example 1: Generate mock data
    example_generate_mock_data().await?;

    // Example 2: Start mock server
    // Uncomment the line below to start the server
    // example_start_mock_server().await?;

    // Example 3: Builder pattern
    // Uncomment the line below to start the server with builder pattern
    // example_builder_pattern().await?;

    // Example 4: Quick start
    // Uncomment the line below to start the server with quick start
    // example_quick_start().await?;

    println!("\n✅ All examples completed successfully!");
    println!("\n📚 For more information, see:");
    println!("   - Documentation: docs/ENHANCED_MOCK_DATA_GENERATION.md");
    println!("   - CLI Commands: mockforge data --help");
    println!("   - API Reference: crates/mockforge-data/src/");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_openapi_spec() {
        let spec = create_example_openapi_spec();

        // Verify it's a valid OpenAPI spec
        assert!(spec.is_object());
        assert_eq!(spec.get("openapi").and_then(|v| v.as_str()), Some("3.0.3"));
        assert_eq!(spec.get("info").and_then(|i| i.get("title")).and_then(|t| t.as_str()), Some("User Management API"));

        // Verify paths exist
        assert!(spec.get("paths").is_some());
        let paths = spec.get("paths").unwrap().as_object().unwrap();
        assert!(paths.contains_key("/api/users"));
        assert!(paths.contains_key("/api/users/{id}"));

        // Verify components exist
        assert!(spec.get("components").is_some());
        let components = spec.get("components").unwrap().as_object().unwrap();
        assert!(components.contains_key("schemas"));

        let schemas = components.get("schemas").unwrap().as_object().unwrap();
        assert!(schemas.contains_key("User"));
        assert!(schemas.contains_key("UserProfile"));
        assert!(schemas.contains_key("CreateUserRequest"));
    }

    #[tokio::test]
    async fn test_generate_mock_data_example() {
        let result = example_generate_mock_data().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_data_generation_with_example_spec() {
        let spec = create_example_openapi_spec();

        let config = MockGeneratorConfig::new()
            .realistic_mode(true)
            .validate_generated_data(true);

        let mut generator = MockDataGenerator::with_config(config);
        let result = generator.generate_from_openapi_spec(&spec).unwrap();

        // Verify schemas were generated
        assert!(!result.schemas.is_empty());
        assert!(result.schemas.contains_key("User"));
        assert!(result.schemas.contains_key("UserProfile"));

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

            // Check that username has reasonable length
            if let Some(username) = user_obj.get("username") {
                if let Some(username_str) = username.as_str() {
                    assert!(username_str.len() >= 3);
                    assert!(username_str.len() <= 30);
                }
            }
        }
    }
}
