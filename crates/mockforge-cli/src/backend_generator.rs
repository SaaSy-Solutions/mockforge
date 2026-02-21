//! Backend code generation from OpenAPI specifications
//!
//! This module provides functionality for generating complete backend server code
//! from OpenAPI specifications, supporting multiple backend frameworks through plugins.

pub mod rust_axum;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use mockforge_core::openapi::spec::OpenApiSpec;
use mockforge_plugin_core::backend_generator::{
    BackendGenerationResult, BackendGeneratorConfig, BackendGeneratorPlugin,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Backend generation subcommand
#[derive(Debug, Subcommand)]
pub enum BackendCommand {
    /// Generate backend server code for a specific framework
    Generate(GenerateArgs),
    /// List available backend frameworks
    List,
}

/// Arguments for the generate command
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// OpenAPI specification file path
    #[arg(short, long)]
    pub spec: PathBuf,

    /// Backend framework (rust, python, nextjs)
    #[arg(short, long, default_value = "rust")]
    pub backend: String,

    /// Output directory for generated files
    #[arg(short, long, default_value = "./generated-backend")]
    pub output: PathBuf,

    /// Server port
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Generate test files
    #[arg(long)]
    pub with_tests: bool,

    /// Generate API documentation stubs
    #[arg(long)]
    pub with_docs: bool,

    /// Database type for integration hints (postgres, mysql, sqlite, mongo)
    #[arg(long)]
    pub database: Option<String>,

    /// Generate TODO.md file
    #[arg(long, default_value = "true")]
    pub generate_todo_md: bool,
}

/// Get available backend generators
pub fn get_available_generators() -> Vec<Box<dyn BackendGeneratorPlugin>> {
    vec![
        Box::new(rust_axum::RustAxumGenerator::new()),
        // Add more generators here as they are implemented
    ]
}

/// Get a backend generator by type name
pub fn get_generator(backend_type: &str) -> Option<Box<dyn BackendGeneratorPlugin>> {
    match backend_type {
        "rust" | "rust-axum" | "axum" => Some(Box::new(rust_axum::RustAxumGenerator::new())),
        _ => None,
    }
}

/// List all available backend generators
pub fn list_generators() -> Vec<(String, String)> {
    get_available_generators()
        .iter()
        .map(|gen| (gen.backend_type().to_string(), gen.backend_name().to_string()))
        .collect()
}

/// Generate backend code using the specified generator and core OpenApiSpec
pub async fn generate_backend_with_spec(
    spec: &OpenApiSpec,
    backend_type: &str,
    config: &BackendGeneratorConfig,
) -> Result<BackendGenerationResult> {
    let normalized = backend_type.trim().to_lowercase();
    if get_generator(&normalized).is_none() {
        let supported = list_generators()
            .into_iter()
            .map(|(backend, _)| backend)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow::anyhow!(
            "Unknown backend type '{}'. Supported backends: {}",
            backend_type,
            supported
        ));
    }

    // Current implementation supports all registered aliases through Rust Axum generation.
    rust_axum::generate_rust_axum_backend(spec, config).await
}

/// Handle backend command
pub async fn handle_backend_command(command: BackendCommand) -> Result<()> {
    match command {
        BackendCommand::Generate(args) => handle_generate(args).await,
        BackendCommand::List => handle_list(),
    }
}

/// Handle generate command
async fn handle_generate(args: GenerateArgs) -> Result<()> {
    use colored::*;
    use std::fs;

    println!("{}", "🚀 Generating backend server code...".bright_green().bold());

    // Load OpenAPI specification
    println!("📄 Loading OpenAPI specification from: {}", args.spec.display());

    // Convert to absolute path if relative
    let spec_path = if args.spec.is_absolute() {
        args.spec.clone()
    } else {
        // If relative, resolve from current working directory
        std::env::current_dir()
            .context("Failed to get current working directory")?
            .join(&args.spec)
    };

    // Try to canonicalize the path, but fall back to the original if it fails
    let spec_path_str = match spec_path.canonicalize() {
        Ok(canonical) => canonical.to_string_lossy().to_string(),
        Err(_) => spec_path.to_string_lossy().to_string(), // If canonicalize fails, use the path as-is
    };

    let spec = OpenApiSpec::from_file(&spec_path_str)
        .await
        .context("Failed to load OpenAPI specification")?;

    println!("✅ Loaded specification: {} v{}", spec.spec.info.title, spec.spec.info.version);

    // Create output directory
    fs::create_dir_all(&args.output).context("Failed to create output directory")?;

    // Build configuration
    let mut config = BackendGeneratorConfig {
        output_dir: args.output.to_string_lossy().to_string(),
        port: args.port,
        base_url: None,
        with_tests: args.with_tests,
        with_docs: args.with_docs,
        database: args.database.clone(),
        generate_todo_md: args.generate_todo_md,
        options: HashMap::new(),
    };

    // Set default port from generator if not specified
    if config.port.is_none() {
        if let Some(gen) = get_generator(&args.backend) {
            config.port = Some(gen.default_port());
        }
    }

    // Generate backend
    println!("⚙️  Generating {} backend...", args.backend);
    let result = generate_backend_with_spec(&spec, &args.backend, &config)
        .await
        .context("Failed to generate backend code")?;

    // Write generated files
    println!("📝 Writing {} files...", result.files.len());
    for file in &result.files {
        let file_path = args.output.join(PathBuf::from(&file.path));
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, &file.content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
    }

    println!("{}", "✅ Backend generation complete!".bright_green().bold());
    println!("\n📊 Summary:");
    println!("   - Framework: {}", result.metadata.backend_name);
    println!("   - Operations: {}", result.metadata.operation_count);
    println!("   - Schemas: {}", result.metadata.schema_count);
    println!("   - Files generated: {}", result.files.len());
    println!("   - TODOs: {}", result.todos.len());
    println!("\n📁 Output directory: {}", args.output.display());
    println!("\n🚀 Next steps:");
    println!("   1. cd {}", args.output.display());
    println!("   2. Review TODO.md for implementation tasks");
    println!("   3. Implement business logic in handlers/");
    println!("   4. Run: cargo run");

    if !result.warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in &result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Handle list command
fn handle_list() -> Result<()> {
    use colored::*;

    println!("{}", "📋 Available Backend Generators:".bright_green().bold());
    println!();

    let generators = list_generators();
    for (backend_type, name) in generators {
        println!("   {} - {}", backend_type.bright_cyan(), name);
    }

    Ok(())
}
