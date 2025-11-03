//! Rust/Axum backend generator
//!
//! Generates complete Rust backend servers using Axum framework from OpenAPI specifications.

use anyhow::{Context, Result};
use chrono::Utc;
use mockforge_core::codegen::backend_generator::{
    extract_routes, extract_schemas, generate_handler_name, sanitize_name, schema_to_rust_type,
    to_pascal_case, to_snake_case, RouteInfo,
};
use mockforge_core::openapi::spec::OpenApiSpec;
use mockforge_plugin_core::backend_generator::{
    BackendGenerationMetadata, BackendGenerationResult, BackendGeneratorConfig,
    BackendGeneratorPlugin, Complexity, TodoCategory, TodoItem,
};
use mockforge_plugin_core::types::{PluginError, PluginMetadata};
use mockforge_plugin_core::GeneratedFile;
use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};
use std::collections::HashMap;

/// Rust/Axum backend generator
pub struct RustAxumGenerator;

impl RustAxumGenerator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl BackendGeneratorPlugin for RustAxumGenerator {
    fn backend_type(&self) -> &str {
        "rust-axum"
    }

    fn backend_name(&self) -> &str {
        "Rust/Axum"
    }

    fn supported_spec_versions(&self) -> Vec<&str> {
        vec!["3.0.0", "3.0.1", "3.0.2", "3.0.3", "3.1.0"]
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["rs", "toml"]
    }

    fn default_port(&self) -> u16 {
        3000
    }

    fn supports_database(&self, db_type: &str) -> bool {
        matches!(db_type, "postgres" | "mysql" | "sqlite" | "mongodb")
    }

    async fn generate_backend(
        &self,
        _spec: &mockforge_plugin_core::client_generator::OpenApiSpec,
        _config: &BackendGeneratorConfig,
    ) -> Result<BackendGenerationResult, PluginError> {
        // This requires conversion from plugin spec to core spec
        // For now, this will be called via generate_rust_axum_backend directly
        Err(PluginError::ConfigurationError {
            message: "Use generate_rust_axum_backend with core OpenApiSpec".to_string(),
        })
    }

    async fn get_metadata(&self) -> PluginMetadata {
        PluginMetadata::new("Rust/Axum Backend Generator")
            .with_capability("backend_generator")
            .with_prefix("rust-axum")
    }

    async fn validate_config(&self, config: &BackendGeneratorConfig) -> Result<(), PluginError> {
        if config.output_dir.is_empty() {
            return Err(PluginError::ConfigurationError {
                message: "output_dir cannot be empty".to_string(),
            });
        }
        Ok(())
    }
}

/// Generate Rust/Axum backend from core OpenApiSpec
/// This is the main entry point that will be called from the CLI
pub async fn generate_rust_axum_backend(
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
) -> Result<BackendGenerationResult> {
    let routes = extract_routes(spec).context("Failed to extract routes from OpenAPI spec")?;
    let schemas = extract_schemas(spec);

    let mut files = Vec::new();
    let mut todos = Vec::new();
    let mut warnings = Vec::new();

    let port = config.port.unwrap_or(3000);

    // Generate Cargo.toml
    files.push(generate_cargo_toml(spec, config, port)?);

    // Generate main.rs
    let (main_rs, main_todos) = generate_main_rs(spec, config, port, &routes)?;
    files.push(main_rs);
    todos.extend(main_todos);

    // Generate models
    let (model_files, model_todos) = generate_models(&schemas, config)?;
    files.extend(model_files);
    todos.extend(model_todos);

    // Generate handlers
    let (handler_files, handler_todos) = generate_handlers(&routes, spec, config)?;
    files.extend(handler_files);
    todos.extend(handler_todos);

    // Generate routes.rs
    let (routes_file, routes_todos) = generate_routes_file(&routes, config)?;
    files.push(routes_file);
    todos.extend(routes_todos);

    // Generate errors.rs
    files.push(generate_errors_rs(config)?);

    // Generate .env.example
    files.push(generate_env_example(config, port)?);

    // Generate README.md
    files.push(generate_readme(spec, config, port)?);

    // Generate TODO.md if requested
    if config.generate_todo_md {
        files.push(generate_todo_md(spec, config, &todos, &routes)?);
    }

    let metadata = BackendGenerationMetadata {
        framework: "rust-axum".to_string(),
        backend_name: sanitize_name(spec.spec.info.title.as_str()),
        api_title: spec.spec.info.title.clone(),
        api_version: spec.spec.info.version.clone(),
        operation_count: routes.len(),
        schema_count: schemas.len(),
        default_port: port,
    };

    Ok(BackendGenerationResult {
        files,
        warnings,
        metadata,
        todos,
    })
}

/// Generate Cargo.toml file
fn generate_cargo_toml(
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
    port: u16,
) -> Result<GeneratedFile> {
    let project_name = sanitize_name(&spec.spec.info.title);
    let version = &spec.spec.info.version;

    let db_deps = if let Some(db_type) = &config.database {
        match db_type.as_str() {
            "postgres" => {
                r#"
# Database (PostgreSQL)
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "chrono", "uuid"] }
diesel = { version = "2.1", features = ["postgres", "chrono", "uuid", "r2d2"] }
"#
            }
            "mysql" => {
                r#"
# Database (MySQL)
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "mysql", "chrono", "uuid"] }
diesel = { version = "2.1", features = ["mysql", "chrono", "uuid", "r2d2"] }
"#
            }
            "sqlite" => {
                r#"
# Database (SQLite)
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "sqlite", "chrono", "uuid"] }
diesel = { version = "2.1", features = ["sqlite", "chrono", "uuid"] }
"#
            }
            _ => "",
        }
    } else {
        ""
    };

    let content = format!(
        r#"[package]
name = "{}"
version = "{}"
edition = "2021"
authors = ["Generated by MockForge"]

[dependencies]
# Web framework
axum = {{ version = "0.7", features = ["macros", "json"] }}
tokio = {{ version = "1.0", features = ["full"] }}
tower = "0.5"
tower-http = {{ version = "0.6", features = ["cors", "trace"] }}

# Serialization
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Utilities
chrono = {{ version = "0.4", features = ["serde"] }}
uuid = {{ version = "1.0", features = ["v4", "serde"] }}

# Logging
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter", "fmt"] }}
{}# Environment variables
dotenv = "0.15"

[dev-dependencies]
# Testing
tokio-test = "0.4"
"#,
        project_name, version, db_deps
    );

    Ok(GeneratedFile {
        path: "Cargo.toml".to_string(),
        content,
        file_type: "toml".to_string(),
    })
}

/// Generate main.rs file
fn generate_main_rs(
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
    port: u16,
    routes: &[RouteInfo],
) -> Result<(GeneratedFile, Vec<TodoItem>)> {
    let mut todos = Vec::new();

    let app_name = sanitize_name(&spec.spec.info.title);
    let handler_count = routes.len();
    let addr_str = format!("http://0.0.0.0:{}", port);

    let content = format!(
        r#"//! {} Backend Server
//!
//! Generated by MockForge from OpenAPI specification
//! API: {} v{}
//!
//! This server was auto-generated. Implement the TODO items in handlers/ to add business logic.

use axum::{{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router}};
use std::{{net::SocketAddr, sync::Arc}};
use tracing::info;
use tower_http::cors::CorsLayer;

pub mod errors;
pub mod handlers;
pub mod models;
pub mod routes;

/// Application state
#[derive(Clone)]
pub struct AppState {{
    // TODO: Add your application state here
    // Example: database connection pool, cache, configuration, etc.
    // pub db: Arc<Pool<Postgres>>,
    // pub cache: Arc<RedisClient>,
}}

impl AppState {{
    pub fn new() -> Self {{
        Self {{
            // TODO: Initialize your application state
        }}
    }}
}}

#[tokio::main]
async fn main() -> anyhow::Result<()> {{
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Create application state
    let state = AppState::new();

    // Build router with all routes
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], {}));
    info!("🚀 Server starting on {{}}");
    info!("📚 {} endpoints available", {});

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}}

/// Create the Axum router with all routes
fn create_router(state: AppState) -> Router {{
    routes::create_routes()
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {{
    (StatusCode::OK, "OK")
}}
"#,
        app_name, spec.spec.info.title, spec.spec.info.version, port, handler_count, addr_str
    );

    todos.push(TodoItem {
        description: "Initialize application state (database, cache, etc.)".to_string(),
        file_path: "src/main.rs".to_string(),
        line_number: 30,
        related_operation: None,
        category: TodoCategory::Config,
        definition_of_done: vec![
            "Add database connection pool to AppState".to_string(),
            "Initialize database connection".to_string(),
            "Handle connection errors gracefully".to_string(),
            "Add connection health checks".to_string(),
        ],
        complexity: Complexity::Medium,
        dependencies: Vec::new(),
    });

    Ok((
        GeneratedFile {
            path: "src/main.rs".to_string(),
            content,
            file_type: "rust".to_string(),
        },
        todos,
    ))
}

/// Generate model files from OpenAPI schemas
fn generate_models(
    schemas: &HashMap<String, Schema>,
    config: &BackendGeneratorConfig,
) -> Result<(Vec<GeneratedFile>, Vec<TodoItem>)> {
    let mut files = Vec::new();
    let mut todos = Vec::new();

    if schemas.is_empty() {
        // Generate a placeholder models file
        let content = r#"//! Data models
//!
//! This module contains data models generated from OpenAPI schemas.
//! Add your custom models here as needed.

use serde::{Deserialize, Serialize};

// TODO: Add models generated from OpenAPI schemas
"#
        .to_string();

        files.push(GeneratedFile {
            path: "src/models/mod.rs".to_string(),
            content,
            file_type: "rust".to_string(),
        });

        return Ok((files, todos));
    }

    let mut mod_content =
        String::from("//! Data models\n//!\n//! Generated from OpenAPI schemas\n\n");
    mod_content.push_str("use serde::{Deserialize, Serialize};\n\n");

    for (name, schema) in schemas {
        let struct_name = to_pascal_case(name);
        mod_content.push_str(&format!("pub mod {};\n", to_snake_case(name)));
        mod_content.push_str(&format!("pub use {}::{};\n\n", to_snake_case(name), struct_name));

        let (model_content, model_todos) = generate_schema_struct(name, schema, config)?;
        files.push(GeneratedFile {
            path: format!("src/models/{}.rs", to_snake_case(name)),
            content: model_content,
            file_type: "rust".to_string(),
        });
        todos.extend(model_todos);
    }

    files.push(GeneratedFile {
        path: "src/models/mod.rs".to_string(),
        content: mod_content,
        file_type: "rust".to_string(),
    });

    Ok((files, todos))
}

/// Generate a Rust struct from an OpenAPI schema
fn generate_schema_struct(
    name: &str,
    schema: &Schema,
    _config: &BackendGeneratorConfig,
) -> Result<(String, Vec<TodoItem>)> {
    let mut todos = Vec::new();
    let struct_name = to_pascal_case(name);

    let mut content =
        format!("//! {} model\n//!\n//! Generated from OpenAPI schema\n\n", struct_name);
    content.push_str("use serde::{Deserialize, Serialize};\n\n");

    if let SchemaKind::Type(Type::Object(obj)) = &schema.schema_kind {
        content.push_str(&format!("#[derive(Debug, Clone, Serialize, Deserialize)]\n"));
        content.push_str(&format!("pub struct {} {{\n", struct_name));

        for (prop_name, prop_schema_ref) in &obj.properties {
            let is_required = obj.required.contains(prop_name);
            let prop_type = match prop_schema_ref {
                ReferenceOr::Item(prop_schema) => schema_to_rust_type(
                    prop_schema,
                    Some(&format!("{}_{}", struct_name, prop_name)),
                ),
                ReferenceOr::Reference { reference } => {
                    // Extract schema name from reference
                    if let Some(ref_name) = reference.strip_prefix("#/components/schemas/") {
                        to_pascal_case(ref_name)
                    } else {
                        "serde_json::Value".to_string()
                    }
                }
            };

            let optional = if is_required { "" } else { "Option<" };
            let optional_close = if is_required { "" } else { ">" };

            content.push_str(&format!("    /// {}\n", prop_name));
            content.push_str(&format!(
                "    pub {}: {}{}{},\n",
                to_snake_case(prop_name),
                optional,
                prop_type,
                optional_close
            ));
        }

        content.push_str("}\n");
    } else {
        // For non-object schemas, create a type alias
        let rust_type = schema_to_rust_type(schema, Some(name));
        content.push_str(&format!("pub type {} = {};\n", struct_name, rust_type));
    }

    todos.push(TodoItem {
        description: format!("Add database annotations and relationships for {}", struct_name),
        file_path: format!("src/models/{}.rs", to_snake_case(name)),
        line_number: 5,
        related_operation: None,
        category: TodoCategory::Model,
        definition_of_done: vec![
            format!("Add ORM table mapping for {}", struct_name),
            "Define primary key constraint".to_string(),
            "Add indexes for frequently queried fields".to_string(),
            "Set up relationships if applicable".to_string(),
        ],
        complexity: Complexity::Medium,
        dependencies: Vec::new(),
    });

    Ok((content, todos))
}

/// Generate handler files from routes
fn generate_handlers(
    routes: &[RouteInfo],
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
) -> Result<(Vec<GeneratedFile>, Vec<TodoItem>)> {
    let mut files = Vec::new();
    let mut todos = Vec::new();

    // Group routes by tag/resource
    let mut routes_by_tag: HashMap<String, Vec<&RouteInfo>> = HashMap::new();
    for route in routes {
        if route.tags.is_empty() {
            routes_by_tag.entry("default".to_string()).or_insert_with(Vec::new).push(route);
        } else {
            for tag in &route.tags {
                routes_by_tag.entry(tag.clone()).or_insert_with(Vec::new).push(route);
            }
        }
    }

    let mut mod_content =
        String::from("//! Request handlers\n//!\n//! Generated from OpenAPI operations\n\n");

    for (tag, tag_routes) in &routes_by_tag {
        let file_name = to_snake_case(tag);
        mod_content.push_str(&format!("pub mod {};\n", file_name));
        mod_content.push_str(&format!("pub use {}::*;\n\n", file_name));

        let mut handler_content =
            format!("//! {} handlers\n//!\n//! Generated from OpenAPI operations\n\n", tag);
        handler_content.push_str("use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};\n");
        handler_content.push_str("use serde_json::Value;\n");
        handler_content.push_str("use std::sync::Arc;\n");
        handler_content.push_str("use crate::errors::ApiError;\n");
        handler_content.push_str("use crate::models::*;\n");
        handler_content.push_str("use crate::AppState;\n\n");

        for route in tag_routes {
            let (handler_code, handler_todos) = generate_handler_function(route, spec, config)?;
            handler_content.push_str(&handler_code);
            handler_content.push_str("\n\n");
            todos.extend(handler_todos);
        }

        files.push(GeneratedFile {
            path: format!("src/handlers/{}.rs", file_name),
            content: handler_content,
            file_type: "rust".to_string(),
        });
    }

    files.push(GeneratedFile {
        path: "src/handlers/mod.rs".to_string(),
        content: mod_content,
        file_type: "rust".to_string(),
    });

    Ok((files, todos))
}

/// Generate a single handler function
fn generate_handler_function(
    route: &RouteInfo,
    _spec: &OpenApiSpec,
    _config: &BackendGeneratorConfig,
) -> Result<(String, Vec<TodoItem>)> {
    let mut todos = Vec::new();
    let handler_name = generate_handler_name(route);
    let mut code = String::new();

    // Function documentation
    code.push_str(&format!("/// Handler for {} {}\n", route.method, route.path));
    if let Some(summary) = &route.summary {
        code.push_str(&format!("/// \n/// {}\n", summary));
    }
    if let Some(desc) = &route.description {
        code.push_str(&format!("/// \n/// {}\n", desc));
    }
    code.push_str(&format!("/// \n/// Tags: {}\n", route.tags.join(", ")));

    // Function signature
    code.push_str(&format!("pub async fn {}(\n", handler_name));

    // Add State parameter
    code.push_str("    State(state): State<Arc<AppState>>,\n");

    // Path parameters
    if !route.path_params.is_empty() {
        if route.path_params.len() == 1 {
            let param = &route.path_params[0];
            code.push_str(&format!("    Path({}): Path<String>,\n", to_snake_case(param)));
        } else {
            code.push_str("    Path(params): Path<std::collections::HashMap<String, String>>,\n");
        }
    }

    // Query parameters
    if !route.query_params.is_empty() {
        code.push_str("    Query(query): Query<std::collections::HashMap<String, String>>,\n");
    }

    // Request body
    if matches!(route.method.as_str(), "POST" | "PUT" | "PATCH")
        && route.request_body_schema.is_some()
    {
        code.push_str("    Json(body): Json<Value>,\n");
    }

    // Remove trailing comma
    if code.ends_with(",\n") {
        code.pop();
        code.pop();
        code.push('\n');
    }

    code.push_str(") -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {\n");

    // Function body with TODOs
    code.push_str("    // TODO: Implement business logic\n");

    match route.method.as_str() {
        "GET" => {
            if !route.path_params.is_empty() {
                code.push_str("    // TODO: Query database for resource\n");
                code.push_str("    // TODO: Handle case where resource not found (404)\n");
            } else {
                code.push_str("    // TODO: Query database for list of resources\n");
                code.push_str("    // TODO: Apply pagination if needed\n");
                code.push_str("    // TODO: Apply filtering/sorting if needed\n");
            }
        }
        "POST" => {
            code.push_str("    // TODO: Validate request body\n");
            code.push_str("    // TODO: Create new resource in database\n");
            code.push_str("    // TODO: Return created resource with 201 status\n");
        }
        "PUT" | "PATCH" => {
            code.push_str("    // TODO: Validate request body\n");
            code.push_str("    // TODO: Update resource in database\n");
            code.push_str("    // TODO: Handle case where resource not found (404)\n");
            code.push_str("    // TODO: Return updated resource\n");
        }
        "DELETE" => {
            code.push_str("    // TODO: Delete resource from database\n");
            code.push_str("    // TODO: Handle case where resource not found (404)\n");
            code.push_str("    // TODO: Return 204 No Content or deleted resource\n");
        }
        _ => {}
    }

    code.push_str("    // TODO: Add authorization check if needed\n");
    code.push_str("    // TODO: Add logging\n");
    code.push_str("    // TODO: Handle errors properly\n");
    code.push_str("\n");
    code.push_str("    // Placeholder response (remove when implementing)\n");

    // Generate placeholder response
    let status_code = route.responses.keys().next().copied().unwrap_or(200);
    code.push_str(&format!(
        "    Ok((StatusCode::from_u16({}).unwrap(), Json(serde_json::json!({{\n",
        status_code
    ));
    code.push_str("        \"message\": \"TODO: Implement this endpoint\",\n");
    if !route.path_params.is_empty() {
        if route.path_params.len() == 1 {
            code.push_str(&format!(
                "        \"path_param\": {},\n",
                to_snake_case(&route.path_params[0])
            ));
        } else {
            code.push_str("        \"path_params\": params,\n");
        }
    }
    code.push_str("    }))))\n");
    code.push_str("}\n");

    // Create TODOs for this handler
    todos.push(TodoItem {
        description: format!("Implement {} handler", handler_name),
        file_path: format!(
            "src/handlers/{}.rs",
            to_snake_case(route.tags.first().unwrap_or(&"default".to_string()))
        ),
        line_number: 15,
        related_operation: route.operation_id.clone(),
        category: TodoCategory::Handler,
        definition_of_done: vec![
            "Implement business logic".to_string(),
            format!("Query/update database for {}", route.path),
            "Handle errors properly (404, 400, etc.)".to_string(),
            "Add authorization if needed".to_string(),
            "Add logging".to_string(),
            "Write unit tests".to_string(),
        ],
        complexity: match route.method.as_str() {
            "GET" if route.path_params.is_empty() => Complexity::Medium,
            "GET" => Complexity::Low,
            "POST" => Complexity::Medium,
            "PUT" | "PATCH" => Complexity::Medium,
            "DELETE" => Complexity::Low,
            _ => Complexity::Low,
        },
        dependencies: Vec::new(),
    });

    Ok((code, todos))
}

/// Generate routes.rs file
fn generate_routes_file(
    routes: &[RouteInfo],
    config: &BackendGeneratorConfig,
) -> Result<(GeneratedFile, Vec<TodoItem>)> {
    let mut todos = Vec::new();
    let mut content =
        String::from("//! Route definitions\n//!\n//! Generated from OpenAPI paths\n\n");
    content.push_str("use axum::{routing::{get, post, put, patch, delete}, Router};\n");
    content.push_str("use crate::handlers::*;\n\n");
    content.push_str("/// Create router with all routes\n");
    content.push_str("/// Note: State will be added in main.rs via .with_state()\n");
    content.push_str("pub fn create_routes() -> Router {\n");
    content.push_str("    Router::new()\n");

    for route in routes {
        let handler_name = generate_handler_name(route);
        let method = route.method.to_lowercase();
        let axum_path = format_axum_path(&route.path, &route.path_params);

        // Handlers are exported via `pub use` in handlers/mod.rs, so we can reference them directly
        content.push_str(&format!(
            "        .route(\"{}\", {}({}))\n",
            axum_path, method, handler_name
        ));
    }

    content.push_str("}\n");

    Ok((
        GeneratedFile {
            path: "src/routes.rs".to_string(),
            content,
            file_type: "rust".to_string(),
        },
        todos,
    ))
}

/// Format path for Axum router (convert {id} to :id)
fn format_axum_path(path: &str, path_params: &[String]) -> String {
    let mut axum_path = path.to_string();
    for param in path_params {
        axum_path = axum_path.replace(&format!("{{{}}}", param), &format!(":{}", param));
    }
    axum_path
}

/// Generate errors.rs file
fn generate_errors_rs(_config: &BackendGeneratorConfig) -> Result<GeneratedFile> {
    let content = r#"//! API Error types
//!
//! Error handling for the API

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal Server Error: {0}")]
    InternalServerError(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
"#
    .to_string();

    Ok(GeneratedFile {
        path: "src/errors.rs".to_string(),
        content,
        file_type: "rust".to_string(),
    })
}

/// Generate .env.example file
fn generate_env_example(_config: &BackendGeneratorConfig, port: u16) -> Result<GeneratedFile> {
    let db_config = if let Some(db_type) = &_config.database {
        match db_type.as_str() {
            "postgres" => "DATABASE_URL=postgres://user:password@localhost:5432/dbname\n",
            "mysql" => "DATABASE_URL=mysql://user:password@localhost:3306/dbname\n",
            "sqlite" => "DATABASE_URL=sqlite:./database.db\n",
            _ => "",
        }
    } else {
        "# DATABASE_URL=postgres://user:password@localhost:5432/dbname\n"
    };

    let content = format!(
        r#"# Server Configuration
PORT={}

# Database Configuration
{}

# Logging
RUST_LOG=info

# Add your environment variables here
"#,
        port, db_config
    );

    Ok(GeneratedFile {
        path: ".env.example".to_string(),
        content,
        file_type: "env".to_string(),
    })
}

/// Generate README.md file
fn generate_readme(
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
    port: u16,
) -> Result<GeneratedFile> {
    let project_name = sanitize_name(&spec.spec.info.title);
    let description = spec.spec.info.description.as_deref().unwrap_or("Generated backend server");

    let content = format!(
        r#"# {}

{}

Generated by MockForge from OpenAPI specification.

## Quick Start

1. **Install dependencies:**
   ```bash
   cargo build
   ```

2. **Set up environment:**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. **Run the server:**
   ```bash
   cargo run
   ```

   The server will start on `http://localhost:{}`

## Project Structure

```
src/
├── main.rs          # Server entry point and setup
├── handlers/        # Request handlers (TODO: implement business logic)
├── models/          # Data models (generated from OpenAPI schemas)
├── routes.rs        # Route definitions
└── errors.rs        # Error types and handling
```

## Development

### Implementing Handlers

Each handler in `src/handlers/` contains TODO comments indicating what needs to be implemented:

- Database queries
- Business logic
- Authorization checks
- Error handling

### Adding Database Support

If you specified a database type during generation, uncomment the database dependencies in `Cargo.toml` and configure the connection in `src/main.rs`.

### Testing

```bash
cargo test
```

## API Endpoints

Generated from OpenAPI specification:

- **API Title:** {}
- **API Version:** {}
- **Total Endpoints:** {}

See the OpenAPI specification for detailed endpoint documentation.

## Next Steps

1. Review and implement TODOs in `TODO.md`
2. Set up your database connection
3. Implement business logic in handlers
4. Add authentication/authorization as needed
5. Write tests
6. Deploy!

## Generated By

MockForge - API Mocking and Testing Platform
"#,
        project_name,
        description,
        port,
        spec.spec.info.title,
        spec.spec.info.version,
        config.port.unwrap_or(0)
    );

    Ok(GeneratedFile {
        path: "README.md".to_string(),
        content,
        file_type: "markdown".to_string(),
    })
}

/// Generate TODO.md file with all TODOs and DoD criteria
fn generate_todo_md(
    spec: &OpenApiSpec,
    config: &BackendGeneratorConfig,
    todos: &[TodoItem],
    routes: &[RouteInfo],
) -> Result<GeneratedFile> {
    let project_name = sanitize_name(&spec.spec.info.title);
    let date_str = Utc::now().format("%Y-%m-%d").to_string();

    let mut content = format!(
        r#"# Backend Implementation TODO List

Generated from: `{}`
Backend Type: Rust/Axum
Generated: {}

## Overview

This file contains all implementation TODOs extracted from generated code.
Each TODO includes:
- Location in codebase
- Description
- Related endpoints/operations
- Definition of Done (DoD) criteria
- Estimated complexity

---

## Endpoints

"#,
        spec.spec.info.title, date_str
    );

    // Group TODOs by endpoint/operation
    let mut todos_by_operation: HashMap<Option<String>, Vec<&TodoItem>> = HashMap::new();
    for todo in todos {
        todos_by_operation
            .entry(todo.related_operation.clone())
            .or_insert_with(Vec::new)
            .push(todo);
    }

    // Add endpoint sections
    for route in routes {
        let operation_id = route.operation_id.as_ref();
        if let Some(op_todos) = todos_by_operation.get(&operation_id.cloned()) {
            content.push_str(&format!(
                "### {} {} - `{}`\n\n",
                route.method,
                route.path,
                generate_handler_name(route)
            ));
            if let Some(summary) = &route.summary {
                content.push_str(&format!("**Summary:** {}\n\n", summary));
            }
            content.push_str(&format!("**File:** `{}`\n\n", op_todos[0].file_path));
            content.push_str("**TODOs:**\n\n");

            for (idx, todo) in op_todos.iter().enumerate() {
                content.push_str(&format!("{}. **{}**\n", idx + 1, todo.description));
                content.push_str(&format!(
                    "   - Location: `{}:{}`\n",
                    todo.file_path, todo.line_number
                ));
                content.push_str(&format!("   - Category: {}\n", todo.category));
                content.push_str(&format!("   - Complexity: {}\n", todo.complexity));
                content.push_str("   - DoD:\n");
                for dod in &todo.definition_of_done {
                    content.push_str(&format!("     - [ ] {}\n", dod));
                }
                if !todo.dependencies.is_empty() {
                    content.push_str(&format!(
                        "   - Dependencies: {}\n",
                        todo.dependencies.join(", ")
                    ));
                }
                content.push_str("\n");
            }
            content.push_str("---\n\n");
        }
    }

    // Add other TODOs (non-endpoint specific)
    let other_todos: Vec<_> = todos.iter().filter(|t| t.related_operation.is_none()).collect();

    if !other_todos.is_empty() {
        content.push_str("## Other Tasks\n\n");
        for todo in &other_todos {
            content.push_str(&format!("### {}\n\n", todo.description));
            content.push_str(&format!("- **File:** `{}:{}`\n", todo.file_path, todo.line_number));
            content.push_str(&format!("- **Category:** {}\n", todo.category));
            content.push_str(&format!("- **Complexity:** {}\n", todo.complexity));
            content.push_str("- **DoD:**\n");
            for dod in &todo.definition_of_done {
                content.push_str(&format!("  - [ ] {}\n", dod));
            }
            content.push_str("\n");
        }
    }

    // Summary section
    let handler_todos =
        todos.iter().filter(|t| matches!(t.category, TodoCategory::Handler)).count();
    let model_todos = todos.iter().filter(|t| matches!(t.category, TodoCategory::Model)).count();
    let config_todos = todos.iter().filter(|t| matches!(t.category, TodoCategory::Config)).count();
    let test_todos = todos.iter().filter(|t| matches!(t.category, TodoCategory::Test)).count();

    content.push_str(&format!(
        r#"## Summary

- Total TODOs: {}
- Endpoints: {} TODOs
- Models: {} TODOs
- Configuration: {} TODOs
- Testing: {} TODOs

## Priority Recommendations

1. **High Priority**: Database connection setup, basic CRUD operations
2. **Medium Priority**: Authorization, pagination, validation
3. **Low Priority**: Advanced features, optimization

## AI Agent Instructions

This TODO list is optimized for AI-assisted development. When implementing:

1. Work through TODOs in priority order
2. Mark items complete by checking [x] in source code
3. Update this file when TODOs are completed
4. Generate tests for each implemented feature
5. Ensure DoD criteria are met before marking complete
"#,
        todos.len(),
        handler_todos,
        model_todos,
        config_todos,
        test_todos
    ));

    Ok(GeneratedFile {
        path: "TODO.md".to_string(),
        content,
        file_type: "markdown".to_string(),
    })
}
