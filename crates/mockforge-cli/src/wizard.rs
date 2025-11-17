//! Interactive getting started wizard for MockForge
//!
//! This module provides an interactive first-run experience that:
//! - Auto-detects environment and suggests optimal config
//! - Creates sample mocks based on detected API patterns
//! - Guides users through setup interactively

use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Wizard configuration collected from user
#[derive(Debug, Clone)]
pub struct WizardConfig {
    pub project_name: String,
    pub project_dir: PathBuf,
    pub protocols: Vec<Protocol>,
    pub enable_admin: bool,
    pub enable_examples: bool,
    pub template: Option<Template>,
}

/// Supported protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    WebSocket,
    Grpc,
    GraphQL,
    Kafka,
    Mqtt,
    Amqp,
}

/// Available templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    RestApi,
    Grpc,
    WebSocket,
    GraphQL,
    Microservices,
}

impl Protocol {
    fn all() -> Vec<Protocol> {
        vec![
            Protocol::Http,
            Protocol::WebSocket,
            Protocol::Grpc,
            Protocol::GraphQL,
            Protocol::Kafka,
            Protocol::Mqtt,
            Protocol::Amqp,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Protocol::Http => "HTTP/REST",
            Protocol::WebSocket => "WebSocket",
            Protocol::Grpc => "gRPC",
            Protocol::GraphQL => "GraphQL",
            Protocol::Kafka => "Kafka",
            Protocol::Mqtt => "MQTT",
            Protocol::Amqp => "AMQP/RabbitMQ",
        }
    }

    fn port(&self) -> u16 {
        match self {
            Protocol::Http => 3000,
            Protocol::WebSocket => 3001,
            Protocol::Grpc => 50051,
            Protocol::GraphQL => 4000,
            Protocol::Kafka => 9092,
            Protocol::Mqtt => 1883,
            Protocol::Amqp => 5672,
        }
    }
}

impl Template {
    fn all() -> Vec<Template> {
        vec![
            Template::RestApi,
            Template::Grpc,
            Template::WebSocket,
            Template::GraphQL,
            Template::Microservices,
        ]
    }

    fn name(&self) -> &'static str {
        match self {
            Template::RestApi => "REST API",
            Template::Grpc => "gRPC Service",
            Template::WebSocket => "WebSocket Server",
            Template::GraphQL => "GraphQL API",
            Template::Microservices => "Microservices (Multi-Protocol)",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Template::RestApi => "Standard REST API with CRUD operations",
            Template::Grpc => "gRPC service with protobuf definitions",
            Template::WebSocket => "Real-time WebSocket server",
            Template::GraphQL => "GraphQL API with queries and mutations",
            Template::Microservices => "Multiple protocols for microservices architecture",
        }
    }
}

/// Run the interactive wizard
pub async fn run_wizard() -> Result<WizardConfig, Box<dyn std::error::Error + Send + Sync>> {
    let theme = ColorfulTheme::default();

    println!("\n{}", "🎯 MockForge Getting Started Wizard".bright_cyan().bold());
    println!("{}", "=".repeat(50).bright_cyan());
    println!(
        "\n{}",
        "This wizard will help you set up your first MockForge project.\n".bright_white()
    );

    // Show environment detection
    let suggestions = detect_environment();
    if !suggestions.is_empty() {
        println!("{}", "🔍 Environment Detection:".bright_yellow().bold());
        for suggestion in &suggestions {
            println!("  • {}", suggestion.bright_white());
        }
        println!();
    }

    // Step 1: Project name
    let project_name: String = Input::with_theme(&theme)
        .with_prompt("Project name")
        .default("my-mock-api".to_string())
        .interact()?;

    // Step 2: Project directory
    let current_dir = std::env::current_dir()?;
    let project_dir = if project_name == "." {
        current_dir.clone()
    } else {
        current_dir.join(&project_name)
    };

    // Step 3: Template selection
    let template_selection = Select::with_theme(&theme)
        .with_prompt("Choose a template to get started")
        .items(&[
            "REST API - Standard REST API with CRUD operations",
            "gRPC Service - gRPC service with protobuf definitions",
            "WebSocket Server - Real-time WebSocket server",
            "GraphQL API - GraphQL API with queries and mutations",
            "Microservices - Multiple protocols for microservices",
            "Custom - Start from scratch",
        ])
        .default(0)
        .interact()?;

    let template = match template_selection {
        0 => Some(Template::RestApi),
        1 => Some(Template::Grpc),
        2 => Some(Template::WebSocket),
        3 => Some(Template::GraphQL),
        4 => Some(Template::Microservices),
        _ => None,
    };

    // Step 4: Protocol selection (if custom template)
    let protocols = if template.is_none() {
        let protocol_names: Vec<&str> = Protocol::all().iter().map(|p| p.name()).collect();

        let selections = MultiSelect::with_theme(&theme)
            .with_prompt("Select protocols to enable")
            .items(&protocol_names)
            .defaults(&[true, false, false, false, false, false, false]) // HTTP selected by default
            .interact()?;

        selections.into_iter().map(|i| Protocol::all()[i]).collect()
    } else {
        // Auto-select protocols based on template
        match template.unwrap() {
            Template::RestApi => vec![Protocol::Http],
            Template::Grpc => vec![Protocol::Http, Protocol::Grpc],
            Template::WebSocket => vec![Protocol::Http, Protocol::WebSocket],
            Template::GraphQL => vec![Protocol::Http, Protocol::GraphQL],
            Template::Microservices => vec![
                Protocol::Http,
                Protocol::Grpc,
                Protocol::WebSocket,
                Protocol::Kafka,
            ],
        }
    };

    // Step 5: Admin UI
    let enable_admin = Confirm::with_theme(&theme)
        .with_prompt("Enable Admin UI?")
        .default(true)
        .interact()?;

    // Step 6: Examples
    let enable_examples = Confirm::with_theme(&theme)
        .with_prompt("Include example files?")
        .default(true)
        .interact()?;

    Ok(WizardConfig {
        project_name,
        project_dir,
        protocols,
        enable_admin,
        enable_examples,
        template,
    })
}

/// Generate project files based on wizard configuration
pub async fn generate_project(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create project directory
    if !config.project_dir.exists() {
        fs::create_dir_all(&config.project_dir)?;
        println!("\n{} Created directory: {}", "✅".green(), config.project_dir.display());
    }

    // Generate configuration file
    let config_path = config.project_dir.join("mockforge.yaml");
    let config_content = generate_config_yaml(config);
    fs::write(&config_path, config_content)?;
    println!("{} Created {}", "✅".green(), config_path.display());

    // Generate template-specific files
    if let Some(template) = config.template {
        generate_template_files(config, template).await?;
    } else if config.enable_examples {
        generate_example_files(config).await?;
    }

    // Generate README
    let readme_path = config.project_dir.join("README.md");
    let readme_content = generate_readme(config);
    fs::write(&readme_path, readme_content)?;
    println!("{} Created {}", "✅".green(), readme_path.display());

    // Summary
    println!("\n{}", "🎉 Project created successfully!".bright_green().bold());
    println!("\n{}", "Next steps:".bright_cyan().bold());
    println!("  1. cd {}", config.project_dir.display());
    println!("  2. Review mockforge.yaml configuration");
    if config.enable_examples {
        println!("  3. Check examples/ directory for sample files");
    }
    println!("  4. Run: {}", "mockforge serve".bright_yellow());
    if config.enable_admin {
        println!("  5. Open Admin UI: {}", "http://localhost:9080".bright_yellow());
    }

    Ok(())
}

/// Generate configuration YAML based on wizard config
fn generate_config_yaml(config: &WizardConfig) -> String {
    let mut yaml = String::from("# MockForge Configuration\n");
    yaml.push_str("# Generated by MockForge Wizard\n");
    yaml.push_str("# Full configuration reference: https://docs.mockforge.dev/config\n\n");

    // HTTP Server
    if config.protocols.contains(&Protocol::Http) {
        yaml.push_str("# HTTP Server\n");
        yaml.push_str("http:\n");
        yaml.push_str("  port: 3000\n");
        yaml.push_str("  host: \"0.0.0.0\"\n");
        yaml.push_str("  cors_enabled: true\n");
        yaml.push_str("  request_validation: \"enforce\"\n");
        yaml.push_str("  response_template_expand: true\n");
        yaml.push_str("\n");
    }

    // WebSocket Server
    if config.protocols.contains(&Protocol::WebSocket) {
        yaml.push_str("# WebSocket Server\n");
        yaml.push_str("websocket:\n");
        yaml.push_str("  port: 3001\n");
        yaml.push_str("  host: \"0.0.0.0\"\n");
        yaml.push_str("\n");
    }

    // gRPC Server
    if config.protocols.contains(&Protocol::Grpc) {
        yaml.push_str("# gRPC Server\n");
        yaml.push_str("grpc:\n");
        yaml.push_str("  port: 50051\n");
        yaml.push_str("  host: \"0.0.0.0\"\n");
        yaml.push_str("\n");
    }

    // GraphQL Server
    if config.protocols.contains(&Protocol::GraphQL) {
        yaml.push_str("# GraphQL Server\n");
        yaml.push_str("graphql:\n");
        yaml.push_str("  port: 4000\n");
        yaml.push_str("  host: \"0.0.0.0\"\n");
        yaml.push_str("\n");
    }

    // Kafka
    if config.protocols.contains(&Protocol::Kafka) {
        yaml.push_str("# Kafka Broker\n");
        yaml.push_str("kafka:\n");
        yaml.push_str("  enabled: true\n");
        yaml.push_str("  port: 9092\n");
        yaml.push_str("\n");
    }

    // MQTT
    if config.protocols.contains(&Protocol::Mqtt) {
        yaml.push_str("# MQTT Broker\n");
        yaml.push_str("mqtt:\n");
        yaml.push_str("  enabled: true\n");
        yaml.push_str("  port: 1883\n");
        yaml.push_str("\n");
    }

    // AMQP
    if config.protocols.contains(&Protocol::Amqp) {
        yaml.push_str("# AMQP Broker\n");
        yaml.push_str("amqp:\n");
        yaml.push_str("  enabled: true\n");
        yaml.push_str("  port: 5672\n");
        yaml.push_str("\n");
    }

    // Admin UI
    if config.enable_admin {
        yaml.push_str("# Admin UI\n");
        yaml.push_str("admin:\n");
        yaml.push_str("  enabled: true\n");
        yaml.push_str("  port: 9080\n");
        yaml.push_str("  host: \"127.0.0.1\"\n");
        yaml.push_str("\n");
    }

    // Observability
    yaml.push_str("# Observability\n");
    yaml.push_str("observability:\n");
    yaml.push_str("  prometheus:\n");
    yaml.push_str("    enabled: true\n");
    yaml.push_str("    port: 9090\n");
    yaml.push_str("\n");

    // Logging
    yaml.push_str("# Logging\n");
    yaml.push_str("logging:\n");
    yaml.push_str("  level: \"info\"\n");

    yaml
}

/// Generate template-specific files
async fn generate_template_files(
    config: &WizardConfig,
    template: Template,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match template {
        Template::RestApi => generate_rest_api_template(config).await?,
        Template::Grpc => generate_grpc_template(config).await?,
        Template::WebSocket => generate_websocket_template(config).await?,
        Template::GraphQL => generate_graphql_template(config).await?,
        Template::Microservices => generate_microservices_template(config).await?,
    }
    Ok(())
}

/// Generate REST API template
async fn generate_rest_api_template(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let examples_dir = config.project_dir.join("examples");
    fs::create_dir_all(&examples_dir)?;

    // OpenAPI spec
    let openapi_path = examples_dir.join("openapi.json");
    let openapi_content = r##"{
  "openapi": "3.0.0",
  "info": {
    "title": "User Management API",
    "version": "1.0.0",
    "description": "Example REST API for user management"
  },
  "servers": [
    {
      "url": "http://localhost:3000",
      "description": "Local development server"
    }
  ],
  "paths": {
    "/users": {
      "get": {
        "summary": "List all users",
        "operationId": "listUsers",
        "responses": {
          "200": {
            "description": "List of users",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/User"
                  }
                },
                "example": [
                  {
                    "id": 1,
                    "name": "Alice Johnson",
                    "email": "alice@example.com",
                    "createdAt": "2024-01-01T00:00:00Z"
                  }
                ]
              }
            }
          }
        }
      },
      "post": {
        "summary": "Create a new user",
        "operationId": "createUser",
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
    "/users/{id}": {
      "get": {
        "summary": "Get user by ID",
        "operationId": "getUser",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "integer"
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
          },
          "404": {
            "description": "User not found"
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
            "type": "integer",
            "format": "int64"
          },
          "name": {
            "type": "string"
          },
          "email": {
            "type": "string",
            "format": "email"
          },
          "createdAt": {
            "type": "string",
            "format": "date-time"
          }
        },
        "required": ["id", "name", "email"]
      },
      "CreateUserRequest": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string"
          },
          "email": {
            "type": "string",
            "format": "email"
          }
        },
        "required": ["name", "email"]
      }
    }
  }
}"##;
    fs::write(&openapi_path, openapi_content)?;
    println!("{} Created {}", "✅".green(), openapi_path.display());

    // Update config to reference OpenAPI spec
    let config_path = config.project_dir.join("mockforge.yaml");
    let mut config_content = fs::read_to_string(&config_path)?;
    config_content.push_str("\n# OpenAPI Specification\n");
    config_content.push_str("http:\n");
    config_content.push_str("  openapi_spec: \"./examples/openapi.json\"\n");
    fs::write(&config_path, config_content)?;

    Ok(())
}

/// Generate gRPC template
async fn generate_grpc_template(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proto_dir = config.project_dir.join("proto");
    fs::create_dir_all(&proto_dir)?;

    let proto_path = proto_dir.join("example.proto");
    let proto_content = r#"syntax = "proto3";

package example;

// Example gRPC service
service ExampleService {
  rpc GetUser (GetUserRequest) returns (GetUserResponse);
  rpc CreateUser (CreateUserRequest) returns (CreateUserResponse);
  rpc ListUsers (ListUsersRequest) returns (ListUsersResponse);
}

message GetUserRequest {
  int64 user_id = 1;
}

message GetUserResponse {
  int64 id = 1;
  string name = 2;
  string email = 3;
}

message CreateUserRequest {
  string name = 1;
  string email = 2;
}

message CreateUserResponse {
  int64 id = 1;
  string name = 2;
  string email = 3;
}

message ListUsersRequest {
  int32 page = 1;
  int32 page_size = 2;
}

message ListUsersResponse {
  repeated GetUserResponse users = 1;
  int32 total = 2;
}
"#;
    fs::write(&proto_path, proto_content)?;
    println!("{} Created {}", "✅".green(), proto_path.display());

    // Update config
    let config_path = config.project_dir.join("mockforge.yaml");
    let mut config_content = fs::read_to_string(&config_path)?;
    config_content.push_str("\n# gRPC Configuration\n");
    config_content.push_str("grpc:\n");
    config_content.push_str("  dynamic:\n");
    config_content.push_str("    enabled: true\n");
    config_content.push_str("    proto_dir: \"./proto\"\n");
    fs::write(&config_path, config_content)?;

    Ok(())
}

/// Generate WebSocket template
async fn generate_websocket_template(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let examples_dir = config.project_dir.join("examples");
    fs::create_dir_all(&examples_dir)?;

    let ws_replay_path = examples_dir.join("ws-replay.jsonl");
    let ws_content = r#"{"ts":0,"dir":"out","text":"Welcome! Type 'hello' to get started.","waitFor":"^CLIENT_READY$"}
{"ts":1000,"dir":"out","text":"{\"type\":\"message\",\"content\":\"Hello from server!\"}","waitFor":"^hello$"}
{"ts":2000,"dir":"out","text":"{\"type\":\"data\",\"value\":\"{{randInt 1 100}}\"}"}
"#;
    fs::write(&ws_replay_path, ws_content)?;
    println!("{} Created {}", "✅".green(), ws_replay_path.display());

    // Update config
    let config_path = config.project_dir.join("mockforge.yaml");
    let mut config_content = fs::read_to_string(&config_path)?;
    config_content.push_str("\n# WebSocket Configuration\n");
    config_content.push_str("websocket:\n");
    config_content.push_str("  replay_file: \"./examples/ws-replay.jsonl\"\n");
    fs::write(&config_path, config_content)?;

    Ok(())
}

/// Generate GraphQL template
async fn generate_graphql_template(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let examples_dir = config.project_dir.join("examples");
    fs::create_dir_all(&examples_dir)?;

    let schema_path = examples_dir.join("schema.graphql");
    let schema_content = r#"type Query {
  users: [User!]!
  user(id: ID!): User
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
}

type User {
  id: ID!
  name: String!
  email: String!
  createdAt: String!
}

input CreateUserInput {
  name: String!
  email: String!
}

input UpdateUserInput {
  name: String
  email: String
}
"#;
    fs::write(&schema_path, schema_content)?;
    println!("{} Created {}", "✅".green(), schema_path.display());

    // Update config
    let config_path = config.project_dir.join("mockforge.yaml");
    let mut config_content = fs::read_to_string(&config_path)?;
    config_content.push_str("\n# GraphQL Configuration\n");
    config_content.push_str("graphql:\n");
    config_content.push_str("  schema_file: \"./examples/schema.graphql\"\n");
    fs::write(&config_path, config_content)?;

    Ok(())
}

/// Generate microservices template
async fn generate_microservices_template(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create examples for multiple protocols
    generate_rest_api_template(config).await?;
    generate_grpc_template(config).await?;
    generate_websocket_template(config).await?;

    // Add Kafka example
    let examples_dir = config.project_dir.join("examples");
    let kafka_dir = examples_dir.join("kafka");
    fs::create_dir_all(&kafka_dir)?;

    let kafka_fixture = kafka_dir.join("orders.yaml");
    let kafka_content = r#"# Kafka fixture example
- identifier: "order-created"
  topic: "orders.created"
  key_pattern: "order-{{uuid}}"
  value_template:
    order_id: "{{uuid}}"
    customer_id: "customer-{{faker.int 1000 9999}}"
    total: "{{faker.float 10.0 1000.0 | round 2}}"
    status: "pending"
    created_at: "{{now}}"
"#;
    fs::write(&kafka_fixture, kafka_content)?;
    println!("{} Created {}", "✅".green(), kafka_fixture.display());

    Ok(())
}

/// Generate example files (fallback)
async fn generate_example_files(
    config: &WizardConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let examples_dir = config.project_dir.join("examples");
    fs::create_dir_all(&examples_dir)?;

    // Simple OpenAPI example
    let openapi_path = examples_dir.join("openapi.json");
    let openapi_content = r##"{
  "openapi": "3.0.0",
  "info": {
    "title": "Example API",
    "version": "1.0.0"
  },
  "paths": {
    "/health": {
      "get": {
        "summary": "Health check",
        "responses": {
          "200": {
            "description": "OK",
            "content": {
              "application/json": {
                "schema": {
                  "type": "object",
                  "properties": {
                    "status": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}"##;
    fs::write(&openapi_path, openapi_content)?;
    println!("{} Created {}", "✅".green(), openapi_path.display());

    Ok(())
}

/// Generate README
fn generate_readme(config: &WizardConfig) -> String {
    let mut readme = String::from("# ");
    readme.push_str(&config.project_name);
    readme.push_str("\n\n");
    readme.push_str("MockForge project generated by wizard.\n\n");
    readme.push_str("## Quick Start\n\n");
    readme.push_str("```bash\n");
    readme.push_str("# Start the mock server\n");
    readme.push_str("mockforge serve\n");
    readme.push_str("```\n\n");

    if config.enable_admin {
        readme.push_str("## Admin UI\n\n");
        readme.push_str("Access the Admin UI at: http://localhost:9080\n\n");
    }

    readme.push_str("## Protocols Enabled\n\n");
    for protocol in &config.protocols {
        readme.push_str("- ");
        readme.push_str(protocol.name());
        readme.push_str(" (port ");
        readme.push_str(&protocol.port().to_string());
        readme.push_str(")\n");
    }

    readme.push_str("\n## Documentation\n\n");
    readme.push_str("For more information, visit: https://docs.mockforge.dev\n");

    readme
}

/// Auto-detect environment and suggest optimal configuration
pub fn detect_environment() -> Vec<String> {
    let mut suggestions = Vec::new();

    // Check for common API files
    if Path::new("package.json").exists() {
        suggestions.push("Node.js project detected - consider REST API template".to_string());
    }

    if Path::new("Cargo.toml").exists() {
        suggestions.push("Rust project detected - consider gRPC template".to_string());
    }

    if Path::new("go.mod").exists() {
        suggestions.push("Go project detected - consider gRPC template".to_string());
    }

    // Check for existing API specs
    if Path::new("openapi.yaml").exists() || Path::new("openapi.json").exists() {
        suggestions.push("OpenAPI specification found - will be used automatically".to_string());
    }

    if Path::new("schema.graphql").exists() {
        suggestions.push("GraphQL schema found - GraphQL template recommended".to_string());
    }

    suggestions
}
