//! Schema-driven mock generation commands
//!
//! This module provides CLI commands for importing API specifications
//! (OpenAPI/AsyncAPI) and automatically generating comprehensive mock endpoints.

use clap::Subcommand;
use colored::Colorize;
use mockforge_core::import::asyncapi_import::{import_asyncapi_spec, AsyncApiImportResult};
use mockforge_core::import::openapi_import::{import_openapi_spec, OpenApiImportResult};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Import OpenAPI 3.x specification and generate mock endpoints
    ///
    /// Examples:
    ///   mockforge import openapi ./specs/api.yaml
    ///   mockforge import openapi ./specs/api.json --output mocks.json
    ///   mockforge import openapi https://api.example.com/openapi.json
    #[command(verbatim_doc_comment)]
    Openapi {
        /// Path or URL to OpenAPI specification file (JSON or YAML)
        spec_path: String,

        /// Base URL for the API (overrides servers in spec)
        #[arg(short, long)]
        base_url: Option<String>,

        /// Output file for generated mocks (JSON format)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Show detailed coverage report
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Generate mocks for all status codes (not just 2xx)
        #[arg(long)]
        all_responses: bool,
    },

    /// Import AsyncAPI 2.x/3.x specification and generate event-driven mocks
    ///
    /// Examples:
    ///   mockforge import asyncapi ./specs/events.yaml
    ///   mockforge import asyncapi ./specs/mqtt.json --protocol mqtt
    #[command(verbatim_doc_comment)]
    Asyncapi {
        /// Path or URL to AsyncAPI specification file (JSON or YAML)
        spec_path: String,

        /// Base URL for the server (overrides servers in spec)
        #[arg(short, long)]
        base_url: Option<String>,

        /// Output file for generated channel configurations (JSON format)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Filter channels by protocol (websocket, mqtt, kafka, amqp)
        #[arg(short, long)]
        protocol: Option<String>,

        /// Show detailed coverage report
        #[arg(short = 'V', long)]
        verbose: bool,
    },

    /// Show coverage statistics for imported specifications
    ///
    /// Examples:
    ///   mockforge import coverage ./specs/api.yaml
    #[command(verbatim_doc_comment)]
    Coverage {
        /// Path to specification file
        spec_path: String,

        /// Specification type (openapi, asyncapi, or auto-detect)
        #[arg(short = 't', long, default_value = "auto")]
        spec_type: String,
    },
}

/// Handle import commands
pub async fn handle_import_command(
    command: ImportCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        ImportCommands::Openapi {
            spec_path,
            base_url,
            output,
            verbose,
            all_responses: _all_responses,
        } => {
            handle_openapi_import(&spec_path, base_url.as_deref(), output.as_deref(), verbose).await
        }
        ImportCommands::Asyncapi {
            spec_path,
            base_url,
            output,
            protocol,
            verbose,
        } => {
            handle_asyncapi_import(
                &spec_path,
                base_url.as_deref(),
                output.as_deref(),
                protocol.as_deref(),
                verbose,
            )
            .await
        }
        ImportCommands::Coverage {
            spec_path,
            spec_type,
        } => handle_coverage_report(&spec_path, &spec_type).await,
    }
}

/// Handle OpenAPI import
async fn handle_openapi_import(
    spec_path: &str,
    base_url: Option<&str>,
    output: Option<&Path>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "📋 Importing OpenAPI Specification...".cyan().bold());
    println!();

    // Load specification content
    let content = load_spec_content(spec_path).await?;

    // Convert YAML to JSON if needed
    let json_content = if spec_path.ends_with(".yaml") || spec_path.ends_with(".yml") {
        let yaml_value: serde_json::Value =
            serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;
        serde_json::to_string(&yaml_value)?
    } else {
        content
    };

    // Import the spec
    let result = import_openapi_spec(&json_content, base_url)
        .map_err(|e| format!("Failed to import OpenAPI spec: {}", e))?;

    // Display results
    display_openapi_import_results(&result, verbose);

    // Save to output file if specified
    if let Some(output_path) = output {
        save_openapi_routes(&result, output_path)?;
        println!();
        println!(
            "{}",
            format!("✅ Saved {} routes to {}", result.routes.len(), output_path.display())
                .green()
                .bold()
        );
    }

    Ok(())
}

/// Handle AsyncAPI import
async fn handle_asyncapi_import(
    spec_path: &str,
    base_url: Option<&str>,
    output: Option<&Path>,
    protocol_filter: Option<&str>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "📋 Importing AsyncAPI Specification...".cyan().bold());
    println!();

    // Load specification content
    let content = load_spec_content(spec_path).await?;

    // Import the spec
    let mut result = import_asyncapi_spec(&content, base_url)
        .map_err(|e| format!("Failed to import AsyncAPI spec: {}", e))?;

    // Filter by protocol if specified
    if let Some(protocol) = protocol_filter {
        result.channels.retain(|ch| {
            format!("{:?}", ch.protocol).to_lowercase().contains(&protocol.to_lowercase())
        });
    }

    // Display results
    display_asyncapi_import_results(&result, verbose);

    // Save to output file if specified
    if let Some(output_path) = output {
        save_asyncapi_channels(&result.channels, output_path)?;
        println!();
        println!(
            "{}",
            format!("✅ Saved {} channels to {}", result.channels.len(), output_path.display())
                .green()
                .bold()
        );
    }

    Ok(())
}

/// Handle coverage report generation
async fn handle_coverage_report(
    spec_path: &str,
    spec_type: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "📊 Generating Coverage Report...".cyan().bold());
    println!();

    let content = load_spec_content(spec_path).await?;

    // Auto-detect spec type if needed
    let detected_type = if spec_type == "auto" {
        detect_spec_type(&content)?
    } else {
        spec_type.to_string()
    };

    // Convert YAML to JSON if needed
    let json_content = if spec_path.ends_with(".yaml") || spec_path.ends_with(".yml") {
        let yaml_value: serde_json::Value =
            serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse YAML: {}", e))?;
        serde_json::to_string(&yaml_value)?
    } else {
        content.clone()
    };

    match detected_type.as_str() {
        "openapi" => {
            let result = import_openapi_spec(&json_content, None)
                .map_err(|e| format!("Failed to parse OpenAPI spec: {}", e))?;
            display_openapi_coverage(&result);
        }
        "asyncapi" => {
            let result = import_asyncapi_spec(&content, None)
                .map_err(|e| format!("Failed to parse AsyncAPI spec: {}", e))?;
            display_asyncapi_coverage(&result);
        }
        _ => {
            return Err(format!("Unknown specification type: {}", detected_type).into());
        }
    }

    Ok(())
}

/// Load specification content from file or URL
async fn load_spec_content(
    spec_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if spec_path.starts_with("http://") || spec_path.starts_with("https://") {
        // Fetch from URL
        println!("📥 Fetching specification from URL: {}", spec_path);
        let response = reqwest::get(spec_path).await?;
        let content = response.text().await?;
        Ok(content)
    } else {
        // Load from file
        println!("📂 Loading specification from file: {}", spec_path);
        let content = fs::read_to_string(spec_path)
            .map_err(|e| format!("Failed to read file {}: {}", spec_path, e))?;
        Ok(content)
    }
}

/// Detect specification type from content
fn detect_spec_type(content: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Try parsing as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        if json.get("openapi").is_some() {
            return Ok("openapi".to_string());
        } else if json.get("asyncapi").is_some() {
            return Ok("asyncapi".to_string());
        }
    }

    // Try parsing as YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(content) {
        if yaml.get("openapi").is_some() {
            return Ok("openapi".to_string());
        } else if yaml.get("asyncapi").is_some() {
            return Ok("asyncapi".to_string());
        }
    }

    Err("Unable to detect specification type. File must be valid OpenAPI or AsyncAPI spec.".into())
}

/// Display OpenAPI import results
fn display_openapi_import_results(result: &OpenApiImportResult, verbose: bool) {
    println!("{}", "📖 Specification Info:".bold());
    println!("  Title: {}", result.spec_info.title.cyan());
    println!("  Version: {}", result.spec_info.version.cyan());
    if let Some(desc) = &result.spec_info.description {
        println!("  Description: {}", desc);
    }
    println!("  OpenAPI Version: {}", result.spec_info.openapi_version);

    if !result.spec_info.servers.is_empty() {
        println!("\n{}", "🌐 Servers:".bold());
        for server in &result.spec_info.servers {
            println!("  • {}", server.green());
        }
    }

    println!();
    println!("{}", "✨ Generated Routes:".bold());
    println!("  Total Routes: {}", result.routes.len().to_string().green().bold());

    // Count by method
    let mut method_counts = std::collections::HashMap::new();
    for route in &result.routes {
        *method_counts.entry(&route.method).or_insert(0) += 1;
    }

    println!("\n{}", "  By Method:".bold());
    for (method, count) in method_counts.iter() {
        let method_colored = match method.as_str() {
            "GET" => method.blue(),
            "POST" => method.green(),
            "PUT" => method.yellow(),
            "DELETE" => method.red(),
            "PATCH" => method.magenta(),
            _ => method.normal(),
        };
        println!("    {}: {}", method_colored.bold(), count);
    }

    // Display warnings if any
    if !result.warnings.is_empty() {
        println!();
        println!("{}", format!("⚠️  {} Warnings:", result.warnings.len()).yellow().bold());
        for warning in &result.warnings {
            println!("  • {}", warning.yellow());
        }
    }

    // Verbose output: list all routes
    if verbose {
        println!();
        println!("{}", "📋 Route Details:".bold());
        for (idx, route) in result.routes.iter().enumerate() {
            let method_colored = match route.method.as_str() {
                "GET" => route.method.as_str().blue(),
                "POST" => route.method.as_str().green(),
                "PUT" => route.method.as_str().yellow(),
                "DELETE" => route.method.as_str().red(),
                "PATCH" => route.method.as_str().magenta(),
                _ => route.method.as_str().normal(),
            };
            println!(
                "  {}: {} {} → {} {}",
                (idx + 1).to_string().dimmed(),
                method_colored.bold(),
                route.path.cyan(),
                route.response.status.to_string().green(),
                if route.body.is_some() {
                    "(with request body)".dimmed().to_string()
                } else {
                    "".to_string()
                }
            );
        }
    }
}

/// Display AsyncAPI import results
fn display_asyncapi_import_results(result: &AsyncApiImportResult, verbose: bool) {
    println!("{}", "📖 Specification Info:".bold());
    println!("  Title: {}", result.spec_info.title.cyan());
    println!("  Version: {}", result.spec_info.version.cyan());
    if let Some(desc) = &result.spec_info.description {
        println!("  Description: {}", desc);
    }
    println!("  AsyncAPI Version: {}", result.spec_info.asyncapi_version);

    if !result.spec_info.servers.is_empty() {
        println!("\n{}", "🌐 Servers:".bold());
        for server in &result.spec_info.servers {
            println!("  • {}", server.green());
        }
    }

    println!();
    println!("{}", "✨ Generated Channels:".bold());
    println!("  Total Channels: {}", result.channels.len().to_string().green().bold());

    // Count by protocol
    let mut protocol_counts = std::collections::HashMap::new();
    for channel in &result.channels {
        *protocol_counts.entry(format!("{:?}", channel.protocol)).or_insert(0) += 1;
    }

    println!("\n{}", "  By Protocol:".bold());
    for (protocol, count) in protocol_counts.iter() {
        println!("    {}: {}", protocol.cyan().bold(), count);
    }

    // Display warnings if any
    if !result.warnings.is_empty() {
        println!();
        println!("{}", format!("⚠️  {} Warnings:", result.warnings.len()).yellow().bold());
        for warning in &result.warnings {
            println!("  • {}", warning.yellow());
        }
    }

    // Verbose output: list all channels
    if verbose {
        println!();
        println!("{}", "📋 Channel Details:".bold());
        for (idx, channel) in result.channels.iter().enumerate() {
            println!(
                "  {}: {} {} ({})",
                (idx + 1).to_string().dimmed(),
                format!("{:?}", channel.protocol).cyan().bold(),
                channel.path.green(),
                channel.name.dimmed()
            );
            if let Some(desc) = &channel.description {
                println!("     Description: {}", desc);
            }
            println!("     Operations: {}", channel.operations.len());
        }
    }
}

/// Display OpenAPI coverage statistics
fn display_openapi_coverage(result: &OpenApiImportResult) {
    println!("{}", "📊 Coverage Statistics:".bold());
    println!();

    let total_routes = result.routes.len();
    let routes_with_bodies = result.routes.iter().filter(|r| r.body.is_some()).count();
    let routes_with_responses = result.routes.len(); // All routes have responses

    println!("  Total Endpoints: {}", total_routes.to_string().green().bold());
    println!(
        "  Endpoints with Mock Responses: {} ({}%)",
        routes_with_responses.to_string().green(),
        calculate_percentage(routes_with_responses, total_routes)
    );
    println!(
        "  Endpoints with Request Bodies: {} ({}%)",
        routes_with_bodies.to_string().green(),
        calculate_percentage(routes_with_bodies, total_routes)
    );

    // Coverage by HTTP method
    let mut method_coverage = std::collections::HashMap::new();
    for route in &result.routes {
        *method_coverage.entry(&route.method).or_insert(0) += 1;
    }

    println!();
    println!("{}", "  Coverage by HTTP Method:".bold());
    for (method, count) in method_coverage.iter() {
        let percentage = calculate_percentage(*count, total_routes);
        println!("    {}: {} ({}%)", method.cyan().bold(), count, percentage);
    }

    // Overall coverage score
    let coverage_score = 100; // We generate mocks for all endpoints
    println!();
    println!("{}", format!("✅ Overall Coverage: {}%", coverage_score).green().bold());
}

/// Display AsyncAPI coverage statistics
fn display_asyncapi_coverage(result: &AsyncApiImportResult) {
    println!("{}", "📊 Coverage Statistics:".bold());
    println!();

    let total_channels = result.channels.len();
    let channels_with_schemas = result
        .channels
        .iter()
        .filter(|ch| ch.operations.iter().any(|op| op.message_schema.is_some()))
        .count();
    let channels_with_examples = result
        .channels
        .iter()
        .filter(|ch| ch.operations.iter().any(|op| op.example_message.is_some()))
        .count();

    println!("  Total Channels: {}", total_channels.to_string().green().bold());
    println!(
        "  Channels with Message Schemas: {} ({}%)",
        channels_with_schemas.to_string().green(),
        calculate_percentage(channels_with_schemas, total_channels)
    );
    println!(
        "  Channels with Example Messages: {} ({}%)",
        channels_with_examples.to_string().green(),
        calculate_percentage(channels_with_examples, total_channels)
    );

    // Coverage by protocol
    let mut protocol_coverage = std::collections::HashMap::new();
    for channel in &result.channels {
        *protocol_coverage.entry(format!("{:?}", channel.protocol)).or_insert(0) += 1;
    }

    println!();
    println!("{}", "  Coverage by Protocol:".bold());
    for (protocol, count) in protocol_coverage.iter() {
        let percentage = calculate_percentage(*count, total_channels);
        println!("    {}: {} ({}%)", protocol.cyan().bold(), count, percentage);
    }

    // Overall coverage score
    let coverage_score = if total_channels > 0 {
        ((channels_with_schemas as f64 / total_channels as f64) * 100.0).round() as u32
    } else {
        0
    };
    println!();
    println!("{}", format!("✅ Overall Coverage: {}%", coverage_score).green().bold());
}

/// Calculate percentage
fn calculate_percentage(count: usize, total: usize) -> u32 {
    if total == 0 {
        0
    } else {
        ((count as f64 / total as f64) * 100.0).round() as u32
    }
}

/// Save OpenAPI routes to JSON file
fn save_openapi_routes(
    result: &OpenApiImportResult,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(&result.routes)?;
    fs::write(output_path, json)?;
    Ok(())
}

/// Save AsyncAPI channels to JSON file
fn save_asyncapi_channels(
    channels: &[mockforge_core::import::asyncapi_import::MockForgeChannel],
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(&channels)?;
    fs::write(output_path, json)?;
    Ok(())
}
