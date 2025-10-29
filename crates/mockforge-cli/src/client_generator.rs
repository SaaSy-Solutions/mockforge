//! Client generation command for MockForge CLI
//!
//! This module provides CLI commands for generating framework-specific
//! mock clients from OpenAPI specifications.

use clap::{Args, Subcommand};
use mockforge_plugin_core::plugins::{AngularClientGenerator, SvelteClientGenerator};
use mockforge_plugin_core::types::{PluginError, Result};
use mockforge_plugin_core::{
    ClientGeneratorConfig, ClientGeneratorPlugin, OpenApiSpec, ReactClientGenerator,
    VueClientGenerator,
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Client generation subcommand
#[derive(Debug, Subcommand)]
pub enum ClientCommand {
    /// Generate client code for a specific framework
    Generate(GenerateArgs),
    /// List available frameworks
    List,
}

/// Arguments for the generate command
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// OpenAPI specification file path
    #[arg(short, long)]
    pub spec: String,

    /// Target framework (react, vue, angular, svelte)
    #[arg(short, long)]
    pub framework: String,

    /// Output directory for generated files
    #[arg(short, long, default_value = "./generated")]
    pub output: String,

    /// Base URL for the API
    #[arg(long)]
    pub base_url: Option<String>,

    /// Include TypeScript types
    #[arg(long, default_value = "true")]
    pub include_types: bool,

    /// Include mock data generation
    #[arg(long, default_value = "false")]
    pub include_mocks: bool,

    /// Custom template directory
    #[arg(long)]
    pub template_dir: Option<String>,

    /// Additional options as JSON
    #[arg(long)]
    pub options: Option<String>,
}

/// Client generator manager
pub struct ClientGeneratorManager {
    /// Available generators
    generators: HashMap<String, Box<dyn ClientGeneratorPlugin + Send + Sync>>,
}

impl ClientGeneratorManager {
    /// Create a new client generator manager with built-in generators
    pub fn new() -> Result<Self> {
        let mut generators: HashMap<String, Box<dyn ClientGeneratorPlugin + Send + Sync>> =
            HashMap::new();

        // Register built-in generators
        generators.insert("react".to_string(), Box::new(ReactClientGenerator::new()?));
        generators.insert("vue".to_string(), Box::new(VueClientGenerator::new()?));
        generators.insert("angular".to_string(), Box::new(AngularClientGenerator::new()?));
        generators.insert("svelte".to_string(), Box::new(SvelteClientGenerator::new()?));

        Ok(Self { generators })
    }

    /// List available frameworks
    pub fn list_frameworks(&self) -> Vec<&str> {
        self.generators.keys().map(|k| k.as_str()).collect()
    }

    /// Generate client code for a specific framework
    pub async fn generate_client(&self, args: &GenerateArgs) -> Result<()> {
        // Load OpenAPI specification
        let spec = self.load_openapi_spec(&args.spec)?;

        // Get the generator for the requested framework
        let generator = self.generators.get(&args.framework).ok_or_else(|| {
            PluginError::execution(format!(
                "Unsupported framework: {}. Available frameworks: {}",
                args.framework,
                self.list_frameworks().join(", ")
            ))
        })?;

        // Parse additional options
        let mut options = HashMap::new();
        if let Some(options_str) = &args.options {
            let parsed_options: Value = serde_json::from_str(options_str).map_err(|e| {
                PluginError::execution(format!("Failed to parse options JSON: {}", e))
            })?;

            if let Value::Object(map) = parsed_options {
                for (key, value) in map {
                    options.insert(key, value);
                }
            }
        }

        // Create configuration
        let config = ClientGeneratorConfig {
            output_dir: args.output.clone(),
            base_url: args.base_url.clone(),
            include_types: args.include_types,
            include_mocks: args.include_mocks,
            template_dir: args.template_dir.clone(),
            options,
        };

        // Validate configuration
        generator.validate_config(&config).await?;

        // Generate client code
        let result = generator.generate_client(&spec, &config).await?;

        // Create output directory
        fs::create_dir_all(&args.output).map_err(|e| {
            PluginError::execution(format!("Failed to create output directory: {}", e))
        })?;

        // Write generated files
        for file in &result.files {
            let file_path = Path::new(&args.output).join(&file.path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    PluginError::execution(format!(
                        "Failed to create directory for {}: {}",
                        file.path, e
                    ))
                })?;
            }

            fs::write(&file_path, &file.content).map_err(|e| {
                PluginError::execution(format!("Failed to write file {}: {}", file.path, e))
            })?;

            println!("Generated: {}", file_path.display());
        }

        // Print warnings if any
        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &result.warnings {
                println!("  - {}", warning);
            }
        }

        // Print generation summary
        println!("\nGeneration Summary:");
        println!("  Framework: {}", result.metadata.framework);
        println!("  Client Name: {}", result.metadata.client_name);
        println!("  API: {} v{}", result.metadata.api_title, result.metadata.api_version);
        println!("  Operations: {}", result.metadata.operation_count);
        println!("  Schemas: {}", result.metadata.schema_count);
        println!("  Files Generated: {}", result.files.len());

        Ok(())
    }

    /// Load OpenAPI specification from file
    fn load_openapi_spec(&self, spec_path: &str) -> Result<OpenApiSpec> {
        let content = fs::read_to_string(spec_path).map_err(|e| {
            PluginError::execution(format!("Failed to read specification file: {}", e))
        })?;

        // Try to parse as JSON first, then YAML
        let spec: OpenApiSpec = if spec_path.ends_with(".json") {
            serde_json::from_str(&content).map_err(|e| {
                PluginError::execution(format!("Failed to parse JSON specification: {}", e))
            })?
        } else {
            serde_yaml::from_str(&content).map_err(|e| {
                PluginError::execution(format!("Failed to parse YAML specification: {}", e))
            })?
        };

        Ok(spec)
    }
}

impl Default for ClientGeneratorManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ClientGeneratorManager")
    }
}

/// Execute client generation command
pub async fn execute_client_command(cmd: ClientCommand) -> Result<()> {
    let manager = ClientGeneratorManager::new()?;

    match cmd {
        ClientCommand::Generate(args) => {
            manager.generate_client(&args).await?;
        }
        ClientCommand::List => {
            println!("Available frameworks:");
            for framework in manager.list_frameworks() {
                println!("  - {}", framework);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_client_generator_manager_creation() {
        let manager = ClientGeneratorManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_list_frameworks() {
        let manager = ClientGeneratorManager::new().unwrap();
        let frameworks = manager.list_frameworks();
        assert!(frameworks.contains(&"react"));
        assert!(frameworks.contains(&"vue"));
    }

    #[tokio::test]
    async fn test_generate_client() {
        let manager = ClientGeneratorManager::new().unwrap();

        // Create a temporary OpenAPI spec file
        let temp_dir = tempdir().unwrap();
        let spec_path = temp_dir.path().join("spec.json");
        let output_dir = temp_dir.path().join("output");

        let spec = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "summary": "Get users",
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
            }
        }"#;

        fs::write(&spec_path, spec).unwrap();

        let args = GenerateArgs {
            spec: spec_path.to_string_lossy().to_string(),
            framework: "react".to_string(),
            output: output_dir.to_string_lossy().to_string(),
            base_url: Some("http://localhost:3000".to_string()),
            include_types: true,
            include_mocks: false,
            template_dir: None,
            options: None,
        };

        let result = manager.generate_client(&args).await;
        assert!(result.is_ok());

        // Check that files were generated
        assert!(output_dir.exists());
        assert!(output_dir.join("types.ts").exists());
        assert!(output_dir.join("hooks.ts").exists());
        assert!(output_dir.join("package.json").exists());
        assert!(output_dir.join("README.md").exists());
    }
}
