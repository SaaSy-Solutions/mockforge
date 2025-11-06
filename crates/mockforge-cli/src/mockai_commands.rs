//! MockAI (Behavioral Mock Intelligence) CLI commands
//!
//! This module provides CLI commands for managing MockAI features including
//! learning from examples, generating rules from OpenAPI, and enabling
//! intelligent behavior for endpoints.

use clap::Subcommand;
use mockforge_core::intelligent_behavior::{
    rule_generator::{ExamplePair, RuleGenerator},
    IntelligentBehaviorConfig, MockAI,
};
use mockforge_core::OpenApiSpec;
use serde_json::Value;
use std::path::PathBuf;

/// MockAI CLI commands
#[derive(Subcommand, Debug)]
pub enum MockAICommands {
    /// Learn behavioral rules from example payloads
    ///
    /// Analyzes example request/response pairs to automatically generate
    /// behavioral rules, validation rules, and state machines.
    ///
    /// Examples:
    ///   mockforge mockai learn --from-examples examples.json
    ///   mockforge mockai learn --from-examples examples.json --output rules.yaml
    Learn {
        /// Path to examples file (JSON or YAML)
        #[arg(long)]
        from_examples: Option<PathBuf>,
        /// Path to OpenAPI specification
        #[arg(long)]
        from_openapi: Option<PathBuf>,
        /// Output file for generated rules
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Generate rules from OpenAPI specification
    ///
    /// Automatically generates behavioral rules from an OpenAPI spec by
    /// analyzing endpoints, schemas, and examples.
    ///
    /// Examples:
    ///   mockforge mockai generate --from-openapi api.yaml
    ///   mockforge mockai generate --from-openapi api.json --output rules.yaml
    Generate {
        /// Path to OpenAPI specification
        #[arg(long, required = true)]
        from_openapi: PathBuf,
        /// Output file for generated rules
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Enable MockAI for specific endpoints
    ///
    /// Enables intelligent behavior for one or more endpoints. If no
    /// endpoints are specified, enables MockAI for all endpoints.
    ///
    /// Examples:
    ///   mockforge mockai enable --endpoint "/api/users"
    ///   mockforge mockai enable --endpoint "/api/users" --endpoint "/api/products"
    ///   mockforge mockai enable  # Enable for all endpoints
    Enable {
        /// Endpoint paths to enable MockAI for
        #[arg(long)]
        endpoint: Vec<String>,
        /// Configuration file to update
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Disable MockAI for specific endpoints
    ///
    /// Disables intelligent behavior for specified endpoints.
    ///
    /// Examples:
    ///   mockforge mockai disable --endpoint "/api/users"
    ///   mockforge mockai disable  # Disable for all endpoints
    Disable {
        /// Endpoint paths to disable MockAI for
        #[arg(long)]
        endpoint: Vec<String>,
        /// Configuration file to update
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Show MockAI status and configuration
    ///
    /// Displays current MockAI configuration and enabled endpoints.
    ///
    /// Examples:
    ///   mockforge mockai status
    ///   mockforge mockai status --config config.yaml
    Status {
        /// Configuration file to read
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
}

/// Handle MockAI CLI commands
pub async fn handle_mockai_command(
    command: MockAICommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        MockAICommands::Learn {
            from_examples,
            from_openapi,
            output,
            verbose,
        } => {
            handle_learn(from_examples, from_openapi, output, verbose).await?;
        }
        MockAICommands::Generate {
            from_openapi,
            output,
            verbose,
        } => {
            handle_generate(from_openapi, output, verbose).await?;
        }
        MockAICommands::Enable { endpoint, config } => {
            handle_enable(endpoint, config).await?;
        }
        MockAICommands::Disable { endpoint, config } => {
            handle_disable(endpoint, config).await?;
        }
        MockAICommands::Status { config } => {
            handle_status(config).await?;
        }
    }

    Ok(())
}

/// Handle learn command
async fn handle_learn(
    from_examples: Option<PathBuf>,
    from_openapi: Option<PathBuf>,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = IntelligentBehaviorConfig::default();

    let examples = if let Some(examples_path) = from_examples {
        // Load examples from file
        let content = tokio::fs::read_to_string(&examples_path).await?;
        let examples: Vec<ExamplePair> = if examples_path.extension().and_then(|s| s.to_str())
            == Some("yaml")
            || examples_path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };

        if verbose {
            println!("📚 Loaded {} examples from {:?}", examples.len(), examples_path);
        }

        examples
    } else if let Some(openapi_path) = from_openapi {
        // Load OpenAPI spec and extract examples
        let content = tokio::fs::read_to_string(&openapi_path).await?;
        let spec_json: Value = if openapi_path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || openapi_path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };

        let spec = OpenApiSpec::from_json(spec_json)?;
        let examples = MockAI::extract_examples_from_openapi(&spec)?;

        if verbose {
            println!("📚 Extracted {} examples from OpenAPI spec", examples.len());
        }

        examples
    } else {
        return Err("Either --from-examples or --from-openapi must be specified".into());
    };

    // Generate rules
    let rule_generator = RuleGenerator::new(config.behavior_model.clone());
    let rules = rule_generator.generate_rules_from_examples(examples).await?;

    if verbose {
        println!("✅ Generated {} consistency rules", rules.consistency_rules.len());
        println!("✅ Generated {} schemas", rules.schemas.len());
        println!("✅ Generated {} state machines", rules.state_transitions.len());
    }

    // Output rules
    if let Some(output_path) = output {
        let output_content = if output_path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || output_path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::to_string(&rules)?
        } else {
            serde_json::to_string_pretty(&rules)?
        };

        tokio::fs::write(&output_path, output_content).await?;
        println!("💾 Saved rules to {:?}", output_path);
    } else {
        // Print to stdout
        let output_content = serde_json::to_string_pretty(&rules)?;
        println!("{}", output_content);
    }

    Ok(())
}

/// Handle generate command
async fn handle_generate(
    from_openapi: PathBuf,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load OpenAPI spec
    let content = tokio::fs::read_to_string(&from_openapi).await?;
    let spec_json: Value = if from_openapi.extension().and_then(|s| s.to_str()) == Some("yaml")
        || from_openapi.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let spec = OpenApiSpec::from_json(spec_json)?;

    if verbose {
        println!("📋 Loaded OpenAPI specification: {}", spec.title());
    }

    // Generate MockAI from OpenAPI
    let config = IntelligentBehaviorConfig::default();
    let mockai = MockAI::from_openapi(&spec, config).await?;

    if verbose {
        println!("✅ Generated behavioral rules");
        println!("   - {} consistency rules", mockai.rules().consistency_rules.len());
        println!("   - {} schemas", mockai.rules().schemas.len());
        println!("   - {} state machines", mockai.rules().state_transitions.len());
    }

    // Output rules
    if let Some(output_path) = output {
        let output_content = if output_path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || output_path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::to_string(mockai.rules())?
        } else {
            serde_json::to_string_pretty(mockai.rules())?
        };

        tokio::fs::write(&output_path, output_content).await?;
        println!("💾 Saved rules to {:?}", output_path);
    } else {
        // Print to stdout
        let output_content = serde_json::to_string_pretty(mockai.rules())?;
        println!("{}", output_content);
    }

    Ok(())
}

/// Handle enable command
async fn handle_enable(
    endpoints: Vec<String>,
    config_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = if let Some(path) = config_path {
        path
    } else {
        // Try to discover config file (synchronous fallback)
        match std::env::current_dir() {
            Ok(current_dir) => {
                let possible_paths = vec![
                    current_dir.join("mockforge.yaml"),
                    current_dir.join("mockforge.yml"),
                    current_dir.join(".mockforge.yaml"),
                ];
                possible_paths.into_iter().find(|p| p.exists()).ok_or_else(|| {
                    "No configuration file found. Specify --config or create mockforge.yaml"
                })?
            }
            Err(_) => {
                return Err(
                    "No configuration file found. Specify --config or create mockforge.yaml".into(),
                );
            }
        }
    };

    // Load config
    let mut config = mockforge_core::config::load_config_auto(&config_path).await?;

    // Enable MockAI
    config.mockai.enabled = true;

    // Add endpoints to enabled list if specified
    let endpoint_count = endpoints.len();
    if !endpoints.is_empty() {
        config.mockai.enabled_endpoints.extend(endpoints);
    }

    // Save config
    mockforge_core::config::save_config(&config_path, &config).await?;

    if endpoint_count == 0 {
        println!("✅ Enabled MockAI for all endpoints");
    } else {
        println!("✅ Enabled MockAI for {} endpoint(s)", endpoint_count);
    }

    Ok(())
}

/// Handle disable command
async fn handle_disable(
    endpoints: Vec<String>,
    config_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = if let Some(path) = config_path {
        path
    } else {
        // Try to discover config file (synchronous fallback)
        match std::env::current_dir() {
            Ok(current_dir) => {
                let possible_paths = vec![
                    current_dir.join("mockforge.yaml"),
                    current_dir.join("mockforge.yml"),
                    current_dir.join(".mockforge.yaml"),
                ];
                possible_paths.into_iter().find(|p| p.exists()).ok_or_else(|| {
                    "No configuration file found. Specify --config or create mockforge.yaml"
                })?
            }
            Err(_) => {
                return Err(
                    "No configuration file found. Specify --config or create mockforge.yaml".into(),
                );
            }
        }
    };

    // Load config
    let mut config = mockforge_core::config::load_config_auto(&config_path).await?;

    let endpoint_count = endpoints.len();
    if endpoints.is_empty() {
        // Disable for all endpoints
        config.mockai.enabled = false;
        config.mockai.enabled_endpoints.clear();
        println!("✅ Disabled MockAI for all endpoints");
    } else {
        // Remove specified endpoints
        for endpoint in &endpoints {
            config.mockai.enabled_endpoints.retain(|e| e != endpoint);
        }
        println!("✅ Disabled MockAI for {} endpoint(s)", endpoint_count);
    }

    // Save config
    mockforge_core::config::save_config(&config_path, &config).await?;

    Ok(())
}

/// Handle status command
async fn handle_status(
    config_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = if let Some(path) = config_path {
        path
    } else {
        // Try to discover config file (synchronous fallback)
        match std::env::current_dir() {
            Ok(current_dir) => {
                let possible_paths = vec![
                    current_dir.join("mockforge.yaml"),
                    current_dir.join("mockforge.yml"),
                    current_dir.join(".mockforge.yaml"),
                ];
                possible_paths.into_iter().find(|p| p.exists()).ok_or_else(|| {
                    "No configuration file found. Specify --config or create mockforge.yaml"
                })?
            }
            Err(_) => {
                return Err(
                    "No configuration file found. Specify --config or create mockforge.yaml".into(),
                );
            }
        }
    };

    // Load config
    let config = mockforge_core::config::load_config_auto(&config_path).await?;

    println!("📊 MockAI Status");
    println!("   Enabled: {}", config.mockai.enabled);
    println!("   Auto-learn: {}", config.mockai.auto_learn);
    println!("   Mutation detection: {}", config.mockai.mutation_detection);
    println!("   AI validation errors: {}", config.mockai.ai_validation_errors);
    println!("   Intelligent pagination: {}", config.mockai.intelligent_pagination);

    if config.mockai.enabled_endpoints.is_empty() {
        println!("   Endpoints: All endpoints");
    } else {
        println!("   Endpoints: {} specific endpoint(s)", config.mockai.enabled_endpoints.len());
        for endpoint in &config.mockai.enabled_endpoints {
            println!("     - {}", endpoint);
        }
    }

    Ok(())
}
