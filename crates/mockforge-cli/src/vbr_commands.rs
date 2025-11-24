//! VBR (Virtual Backend Reality) CLI commands
//!
//! This module provides CLI commands for managing VBR entities, schemas,
//! and running VBR servers.

use clap::Subcommand;
use colored::Colorize;
use mockforge_data::{FieldDefinition, SchemaDefinition};
use mockforge_vbr::{
    config::{StorageBackend, VbrConfig},
    entities::Entity,
    schema::VbrSchemaDefinition,
    VbrEngine,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum VbrCommands {
    /// Create a new entity definition
    ///
    /// Examples:
    ///   mockforge vbr create entity User --fields id:string,name:string,email:string
    ///   mockforge vbr create entity Order --fields id:string,user_id:string,total:number
    #[command(verbatim_doc_comment)]
    Create {
        #[command(subcommand)]
        create_command: CreateCommands,
    },

    /// Serve VBR API
    ///
    /// Starts a server with VBR endpoints enabled.
    ///
    /// Examples:
    ///   mockforge vbr serve --port 3000
    ///   mockforge vbr serve --storage sqlite --db-path ./data/vbr.db
    #[command(verbatim_doc_comment)]
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,

        /// Database file path (for sqlite/json)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// API base path prefix
        #[arg(long, default_value = "/vbr-api")]
        api_prefix: String,

        /// Enable session-scoped data
        #[arg(long)]
        session_scoped: bool,
    },

    /// Manage VBR entities and data
    ///
    /// Examples:
    ///   mockforge vbr manage entities list
    ///   mockforge vbr manage entities show User
    ///   mockforge vbr manage data query "SELECT * FROM users"
    #[command(verbatim_doc_comment)]
    Manage {
        #[command(subcommand)]
        manage_command: ManageCommands,
    },

    /// Import entities from OpenAPI specification
    ///
    /// Examples:
    ///   mockforge vbr import openapi ./api-spec.yaml
    ///   mockforge vbr import openapi ./api-spec.json --output ./entities
    #[command(verbatim_doc_comment)]
    Import {
        #[command(subcommand)]
        import_command: ImportCommands,
    },

    /// Seed data into VBR database
    ///
    /// Examples:
    ///   mockforge vbr seed ./seed_data.json
    ///   mockforge vbr seed ./seed_data.yaml --entity User
    #[command(verbatim_doc_comment)]
    Seed {
        /// Seed file path (JSON or YAML)
        file: PathBuf,

        /// Specific entity to seed (optional, seeds all if not specified)
        #[arg(short, long)]
        entity: Option<String>,

        /// Database path (for SQLite/JSON backends)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,
    },

    /// Manage database snapshots
    ///
    /// Examples:
    ///   mockforge vbr snapshot create initial_state --description "Initial data"
    ///   mockforge vbr snapshot list
    ///   mockforge vbr snapshot restore initial_state
    ///   mockforge vbr snapshot delete old_snapshot
    #[command(verbatim_doc_comment)]
    Snapshot {
        #[command(subcommand)]
        snapshot_command: SnapshotCommands,
    },

    /// Reset database to empty state
    ///
    /// Examples:
    ///   mockforge vbr reset
    ///   mockforge vbr reset --entity User
    #[command(verbatim_doc_comment)]
    Reset {
        /// Specific entity to reset (optional, resets all if not specified)
        #[arg(short, long)]
        entity: Option<String>,

        /// Database path (for SQLite/JSON backends)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,
    },
}

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create a new entity
    Entity {
        /// Entity name (e.g., "User", "Order")
        name: String,

        /// Field definitions (comma-separated: name:type)
        /// Example: id:string,name:string,email:string,age:number
        #[arg(short, long)]
        fields: Option<String>,

        /// Schema file path (JSON or YAML)
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// Output file for entity definition
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ManageCommands {
    /// List all registered entities
    Entities {
        #[command(subcommand)]
        entities_command: EntitiesCommands,
    },

    /// Query database data
    Data {
        /// SQL query to execute
        query: String,
    },
}

#[derive(Subcommand)]
pub enum EntitiesCommands {
    /// List all entities
    List,

    /// Show entity details
    Show {
        /// Entity name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Import from OpenAPI specification
    Openapi {
        /// OpenAPI spec file path (JSON or YAML)
        file: PathBuf,

        /// Output directory for entity definitions
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Database path (for SQLite/JSON backends)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,
    },
}

#[derive(Subcommand)]
pub enum SnapshotCommands {
    /// Create a new snapshot
    Create {
        /// Snapshot name
        name: String,

        /// Snapshot description
        #[arg(short, long)]
        description: Option<String>,

        /// Snapshots directory
        #[arg(long, default_value = "./snapshots")]
        snapshots_dir: PathBuf,

        /// Database path (for SQLite/JSON backends)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,
    },

    /// List all snapshots
    List {
        /// Snapshots directory
        #[arg(long, default_value = "./snapshots")]
        snapshots_dir: PathBuf,
    },

    /// Restore a snapshot
    Restore {
        /// Snapshot name
        name: String,

        /// Snapshots directory
        #[arg(long, default_value = "./snapshots")]
        snapshots_dir: PathBuf,

        /// Database path (for SQLite/JSON backends)
        #[arg(long)]
        db_path: Option<PathBuf>,

        /// Storage backend (sqlite, json, memory)
        #[arg(short, long, default_value = "memory")]
        storage: String,
    },

    /// Delete a snapshot
    Delete {
        /// Snapshot name
        name: String,

        /// Snapshots directory
        #[arg(long, default_value = "./snapshots")]
        snapshots_dir: PathBuf,
    },
}

/// Execute VBR command
pub async fn execute_vbr_command(command: VbrCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        VbrCommands::Create { create_command } => execute_create_command(create_command).await,
        VbrCommands::Serve {
            port,
            storage,
            db_path,
            api_prefix,
            session_scoped,
        } => execute_serve_command(port, storage, db_path, api_prefix, session_scoped).await,
        VbrCommands::Manage { manage_command } => execute_manage_command(manage_command).await,
        VbrCommands::Import { import_command } => execute_import_command(import_command).await,
        VbrCommands::Seed {
            file,
            entity,
            db_path,
            storage,
        } => execute_seed_command(file, entity, db_path, storage).await,
        VbrCommands::Snapshot { snapshot_command } => {
            execute_snapshot_command(snapshot_command).await
        }
        VbrCommands::Reset {
            entity,
            db_path,
            storage,
        } => execute_reset_command(entity, db_path, storage).await,
    }
}

async fn execute_create_command(command: CreateCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        CreateCommands::Entity {
            name,
            fields,
            schema,
            output,
        } => {
            println!("{}", "Creating VBR entity...".bright_cyan());

            let entity_schema = if let Some(schema_path) = schema {
                // Load from file
                let content = std::fs::read_to_string(&schema_path)?;
                if schema_path.extension().and_then(|s| s.to_str()) == Some("yaml")
                    || schema_path.extension().and_then(|s| s.to_str()) == Some("yml")
                {
                    serde_yaml::from_str(&content)?
                } else {
                    serde_json::from_str(&content)?
                }
            } else if let Some(fields_str) = fields {
                // Create from fields string
                create_schema_from_fields(&name, &fields_str)?
            } else {
                return Err("Either --fields or --schema must be provided".into());
            };

            let entity = Entity::new(name.clone(), entity_schema);

            // Output entity definition (serialize schema)
            let schema_json = serde_json::to_string_pretty(&entity.schema)?;
            if let Some(output_path) = output {
                std::fs::write(&output_path, schema_json)?;
                println!(
                    "{} Entity '{}' created and saved to {}",
                    "✓".green(),
                    name,
                    output_path.display()
                );
            } else {
                println!("{} Entity '{}' definition:", "✓".green(), name);
                println!("{}", schema_json);
            }

            Ok(())
        }
    }
}

fn create_schema_from_fields(
    entity_name: &str,
    fields_str: &str,
) -> Result<VbrSchemaDefinition, Box<dyn std::error::Error>> {
    let mut field_defs = Vec::new();
    let mut primary_key = vec!["id".to_string()];

    for field_spec in fields_str.split(',') {
        let parts: Vec<&str> = field_spec.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid field specification: {}", field_spec).into());
        }

        let field_name = parts[0].trim();
        let field_type = parts[1].trim();

        // Check if this is the primary key
        if field_name == "id" {
            primary_key = vec![field_name.to_string()];
        }

        field_defs.push(FieldDefinition {
            name: field_name.to_string(),
            field_type: field_type.to_string(),
            required: field_name == "id", // id is always required
            description: None,
            default: None,
            constraints: HashMap::new(),
            faker_template: None,
        });
    }

    let base_schema = SchemaDefinition {
        name: entity_name.to_string(),
        fields: field_defs,
        description: Some(format!("{} entity", entity_name)),
        metadata: HashMap::new(),
        relationships: HashMap::new(),
    };

    Ok(VbrSchemaDefinition {
        base: base_schema,
        primary_key,
        foreign_keys: Vec::new(),
        unique_constraints: Vec::new(),
        indexes: Vec::new(),
        auto_generation: HashMap::new(),
        many_to_many: Vec::new(),
    })
}

async fn execute_serve_command(
    port: u16,
    storage: String,
    db_path: Option<PathBuf>,
    api_prefix: String,
    session_scoped: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Starting VBR server on port {}...", "🚀".bright_cyan(), port);

    // Create storage backend
    let storage_backend = match storage.as_str() {
        "sqlite" => {
            let path = db_path.unwrap_or_else(|| PathBuf::from("./data/vbr.db"));
            StorageBackend::Sqlite { path }
        }
        "json" => {
            let path = db_path.unwrap_or_else(|| PathBuf::from("./data/vbr.json"));
            StorageBackend::Json { path }
        }
        "memory" => StorageBackend::Memory,
        _ => {
            return Err(format!("Invalid storage backend: {}", storage).into());
        }
    };

    // Create VBR config
    let mut config = VbrConfig::default();
    config.storage = storage_backend;
    config.sessions.scoped_data = session_scoped;

    // Create VBR engine
    let engine = VbrEngine::new(config).await?;
    let database = engine.database_arc();
    let registry = engine.registry().clone();

    // Create handler context
    let context = mockforge_vbr::handlers::HandlerContext {
        database,
        registry,
        session_manager: None, // TODO: Initialize session manager if needed
        snapshots_dir: None,
    };

    // Create router
    let router = mockforge_vbr::integration::create_vbr_router_with_context(&api_prefix, context)?;

    // Start server
    use std::net::SocketAddr;
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("{} VBR server running at http://localhost:{}{}", "✓".green(), port, api_prefix);
    println!("{} Press Ctrl+C to stop", "ℹ".bright_blue());

    axum::serve(listener, router).await?;

    Ok(())
}

async fn execute_manage_command(command: ManageCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        ManageCommands::Entities { entities_command } => {
            match entities_command {
                EntitiesCommands::List => {
                    // For now, just show a message
                    // In a full implementation, this would load entities from a file or database
                    println!("{} Entity management:", "ℹ".bright_blue());
                    println!("  Use 'mockforge vbr create entity' to create entities");
                    println!("  Use 'mockforge vbr serve' to start a server with entities");
                }
                EntitiesCommands::Show { name } => {
                    println!("{} Showing entity: {}", "ℹ".bright_blue(), name);
                    println!("  Entity details would be shown here");
                }
            }
            Ok(())
        }
        ManageCommands::Data { query } => {
            println!("{} Executing query: {}", "ℹ".bright_blue(), query);
            println!("  Query execution would happen here");
            // In a full implementation, this would:
            // 1. Load the VBR engine
            // 2. Execute the query
            // 3. Display results
            Ok(())
        }
    }
}

async fn execute_import_command(command: ImportCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        ImportCommands::Openapi {
            file,
            output,
            db_path,
            storage,
        } => {
            println!("{} Importing entities from OpenAPI specification...", "📥".bright_cyan());

            // Read OpenAPI spec
            let content = std::fs::read_to_string(&file)?;
            let is_yaml = file
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "yaml" || s == "yml")
                .unwrap_or(false);

            // Create VBR engine from OpenAPI
            let config = create_config_from_storage(&storage, db_path)?;
            let (engine, result) = if is_yaml {
                VbrEngine::from_openapi_file(config, &file).await?
            } else {
                VbrEngine::from_openapi(config, &content).await?
            };

            println!("{} Created {} entities:", "✓".green(), result.entities.len());
            for (name, _) in &result.entities {
                println!("  - {}", name.bright_cyan());
            }

            if !result.warnings.is_empty() {
                println!("{} Warnings:", "⚠".yellow());
                for warning in &result.warnings {
                    println!("  - {}", warning);
                }
            }

            // Save entity definitions if output directory specified
            if let Some(output_dir) = output {
                std::fs::create_dir_all(&output_dir)?;
                for (name, _schema) in &result.entities {
                    if let Some(entity) = engine.registry().get(name) {
                        let schema_json = serde_json::to_string_pretty(&entity.schema)?;
                        let output_file = output_dir.join(format!("{}.json", name));
                        std::fs::write(&output_file, schema_json)?;
                        println!("  {} Saved to {}", "✓".green(), output_file.display());
                    }
                }
            }

            Ok(())
        }
    }
}

async fn execute_seed_command(
    file: PathBuf,
    entity: Option<String>,
    db_path: Option<PathBuf>,
    storage: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Seeding data from {}...", "🌱".bright_cyan(), file.display());

    // Create VBR engine
    let config = create_config_from_storage(&storage, db_path)?;
    let engine = VbrEngine::new(config).await?;

    // Seed data
    let results = if let Some(entity_name) = entity {
        // Load seed file and extract specific entity
        let seed_data = mockforge_vbr::seeding::load_seed_file(&file).await?;
        if let Some(records) = seed_data.get(&entity_name) {
            let count = engine.seed_entity(&entity_name, records).await?;
            let mut results = HashMap::new();
            results.insert(entity_name, count);
            results
        } else {
            return Err(format!("Entity '{}' not found in seed file", entity_name).into());
        }
    } else {
        // Seed all entities
        let seed_data = mockforge_vbr::seeding::load_seed_file(&file).await?;
        engine.seed_all(&seed_data).await?
    };

    println!("{} Seeded data:", "✓".green());
    for (entity_name, count) in results {
        println!("  - {}: {} records", entity_name.bright_cyan(), count);
    }

    Ok(())
}

async fn execute_snapshot_command(
    command: SnapshotCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        SnapshotCommands::Create {
            name,
            description,
            snapshots_dir,
            db_path,
            storage,
        } => {
            println!("{} Creating snapshot '{}'...", "📸".bright_cyan(), name);

            // Create VBR engine
            let config = create_config_from_storage(&storage, db_path)?;
            let engine = VbrEngine::new(config).await?;

            // Create snapshot
            let metadata = engine.create_snapshot(&name, description, &snapshots_dir).await?;

            println!("{} Snapshot created:", "✓".green());
            println!("  Name: {}", metadata.name.bright_cyan());
            println!("  Created: {}", metadata.created_at);
            println!("  Entity counts:");
            for (entity, count) in &metadata.entity_counts {
                println!("    - {}: {}", entity, count);
            }

            Ok(())
        }
        SnapshotCommands::List { snapshots_dir } => {
            println!("{} Listing snapshots...", "📋".bright_cyan());

            let snapshots = VbrEngine::list_snapshots(&snapshots_dir).await?;

            if snapshots.is_empty() {
                println!("  No snapshots found");
            } else {
                println!("{} Found {} snapshots:", "✓".green(), snapshots.len());
                for snapshot in snapshots {
                    println!("  - {} ({})", snapshot.name.bright_cyan(), snapshot.created_at);
                    if let Some(desc) = &snapshot.description {
                        println!("    Description: {}", desc);
                    }
                    println!("    Entity counts:");
                    for (entity, count) in &snapshot.entity_counts {
                        println!("      - {}: {}", entity, count);
                    }
                }
            }

            Ok(())
        }
        SnapshotCommands::Restore {
            name,
            snapshots_dir,
            db_path,
            storage,
        } => {
            println!("{} Restoring snapshot '{}'...", "🔄".bright_cyan(), name);

            // Create VBR engine
            let config = create_config_from_storage(&storage, db_path)?;
            let engine = VbrEngine::new(config).await?;

            // Restore snapshot
            engine.restore_snapshot(&name, &snapshots_dir).await?;

            println!("{} Snapshot '{}' restored successfully", "✓".green(), name);

            Ok(())
        }
        SnapshotCommands::Delete {
            name,
            snapshots_dir,
        } => {
            println!("{} Deleting snapshot '{}'...", "🗑️".bright_cyan(), name);

            VbrEngine::delete_snapshot(&name, &snapshots_dir).await?;

            println!("{} Snapshot '{}' deleted successfully", "✓".green(), name);

            Ok(())
        }
    }
}

async fn execute_reset_command(
    entity: Option<String>,
    db_path: Option<PathBuf>,
    storage: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{} Resetting database...", "🔄".bright_cyan());

    // Create VBR engine
    let config = create_config_from_storage(&storage, db_path)?;
    let engine = VbrEngine::new(config).await?;

    if let Some(entity_name) = entity {
        println!("  Resetting entity: {}", entity_name.bright_cyan());
        engine.clear_entity(&entity_name).await?;
        println!("{} Entity '{}' reset successfully", "✓".green(), entity_name);
    } else {
        println!("  Resetting all entities...");
        engine.reset().await?;
        println!("{} Database reset successfully", "✓".green());
    }

    Ok(())
}

/// Helper function to create VbrConfig from storage type
fn create_config_from_storage(
    storage: &str,
    db_path: Option<PathBuf>,
) -> Result<VbrConfig, Box<dyn std::error::Error>> {
    let storage_backend = match storage {
        "sqlite" => {
            let path = db_path.unwrap_or_else(|| PathBuf::from("./data/vbr.db"));
            StorageBackend::Sqlite { path }
        }
        "json" => {
            let path = db_path.unwrap_or_else(|| PathBuf::from("./data/vbr.json"));
            StorageBackend::Json { path }
        }
        "memory" => StorageBackend::Memory,
        _ => {
            return Err(format!("Invalid storage backend: {}", storage).into());
        }
    };

    let mut config = VbrConfig::default();
    config.storage = storage_backend;
    Ok(config)
}
