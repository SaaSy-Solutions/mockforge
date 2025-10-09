///! Mock Coverage Demo
///!
///! This example demonstrates the mock coverage feature, which tracks which
///! API endpoints have been exercised during testing.
///!
///! Run this example with:
///! ```bash
///! cargo run --example coverage_demo
///! ```
///!
///! Then in another terminal, make some requests:
///! ```bash
///! # Get coverage report
///! curl http://localhost:3000/__mockforge/coverage | jq
///!
///! # Call some endpoints
///! curl http://localhost:3000/api/users
///! curl http://localhost:3000/api/users/123
///! curl -X POST http://localhost:3000/api/users -H "Content-Type: application/json" -d '{"name":"Alice"}'
///!
///! # Check coverage again
///! curl http://localhost:3000/__mockforge/coverage | jq
///!
///! # View coverage UI in browser
///! open http://localhost:3000/__mockforge/coverage.html
///! ```

use axum::Router;
use mockforge_http::build_router;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a sample OpenAPI spec
    let spec_json = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Sample API",
            "version": "1.0.0"
        },
        "paths": {
            "/api/users": {
                "get": {
                    "summary": "List users",
                    "operationId": "listUsers",
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "id": {"type": "integer"},
                                                "name": {"type": "string"},
                                                "email": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create user",
                    "operationId": "createUser",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["name", "email"],
                                    "properties": {
                                        "name": {"type": "string"},
                                        "email": {"type": "string", "format": "email"}
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
                                            "name": {"type": "string"},
                                            "email": {"type": "string"}
                                        }
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
                    "operationId": "getUserById",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "integer"}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Success",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"},
                                            "name": {"type": "string"},
                                            "email": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "put": {
                    "summary": "Update user",
                    "operationId": "updateUser",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "integer"}
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"},
                                        "email": {"type": "string", "format": "email"}
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Updated",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"},
                                            "name": {"type": "string"},
                                            "email": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                "delete": {
                    "summary": "Delete user",
                    "operationId": "deleteUser",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "integer"}
                        }
                    ],
                    "responses": {
                        "204": {
                            "description": "Deleted"
                        }
                    }
                }
            },
            "/api/products": {
                "get": {
                    "summary": "List products",
                    "operationId": "listProducts",
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            },
            "/api/orders": {
                "get": {
                    "summary": "List orders",
                    "operationId": "listOrders",
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    // Write spec to a temporary file
    let spec_path = "/tmp/coverage_demo_spec.json";
    std::fs::write(spec_path, serde_json::to_string_pretty(&spec_json)?)?;

    println!("📝 Mock Coverage Demo");
    println!("===================\n");
    println!("Starting MockForge server with {} routes...", 8);
    println!("\n🌐 Server running at: http://localhost:3000");
    println!("\n📊 Coverage Endpoints:");
    println!("  • Coverage API:  http://localhost:3000/__mockforge/coverage");
    println!("  • Coverage UI:   http://localhost:3000/__mockforge/coverage.html");
    println!("  • Routes list:   http://localhost:3000/__mockforge/routes");
    println!("\n🧪 Try calling some endpoints:");
    println!("  curl http://localhost:3000/api/users");
    println!("  curl http://localhost:3000/api/users/123");
    println!("  curl -X POST http://localhost:3000/api/users -H 'Content-Type: application/json' -d '{{\"name\":\"Alice\",\"email\":\"alice@example.com\"}}'");
    println!("\n📈 Then check coverage:");
    println!("  curl http://localhost:3000/__mockforge/coverage | jq");
    println!("  open http://localhost:3000/__mockforge/coverage.html");
    println!("\nPress Ctrl+C to stop\n");

    // Build router from the spec
    let app = build_router(Some(spec_path.to_string()), None, None).await;

    // Serve the static coverage UI
    let app = app.nest_service(
        "/__mockforge/coverage.html",
        tower_http::services::ServeFile::new("crates/mockforge-http/static/coverage.html"),
    );

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("✅ Server started successfully!\n");

    axum::serve(listener, app).await?;

    Ok(())
}
