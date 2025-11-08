//! MockAI (Behavioral Mock Intelligence) CLI commands
//!
//! This module provides CLI commands for managing MockAI features including
//! learning from examples, generating rules from OpenAPI, and enabling
//! intelligent behavior for endpoints.

use chrono::{DateTime, Utc};
use clap::Subcommand;
use mockforge_core::intelligent_behavior::{
    openapi_generator::{OpenApiGenerationConfig, OpenApiSpecGenerator},
    rule_generator::{ExamplePair, RuleGenerator},
    IntelligentBehaviorConfig, MockAI,
};
use mockforge_core::OpenApiSpec;
use mockforge_recorder::{
    database::RecorderDatabase,
    openapi_export::{QueryFilters, RecordingsToOpenApi},
};
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

    /// Generate OpenAPI specification from recorded traffic
    ///
    /// Analyzes recorded HTTP traffic and generates an OpenAPI 3.0 specification
    /// using pattern detection and optional LLM inference.
    ///
    /// Examples:
    ///   mockforge mockai generate-from-traffic --database ./recordings.db
    ///   mockforge mockai generate-from-traffic --database ./recordings.db --output openapi.yaml
    ///   mockforge mockai generate-from-traffic --database ./recordings.db --since 2025-01-01 --min-confidence 0.8
    GenerateFromTraffic {
        /// Path to recorder database (default: ./recordings.db)
        #[arg(long)]
        database: Option<PathBuf>,
        /// Output file for generated OpenAPI spec
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Start time for filtering (ISO 8601 format, e.g., 2025-01-01T00:00:00Z)
        #[arg(long)]
        since: Option<String>,
        /// End time for filtering (ISO 8601 format)
        #[arg(long)]
        until: Option<String>,
        /// Path pattern filter (supports wildcards, e.g., /api/*)
        #[arg(long)]
        path_pattern: Option<String>,
        /// Minimum confidence score for including paths (0.0 to 1.0)
        #[arg(long, default_value = "0.7")]
        min_confidence: f64,
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
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
        MockAICommands::GenerateFromTraffic {
            database,
            output,
            since,
            until,
            path_pattern,
            min_confidence,
            verbose,
        } => {
            handle_generate_from_traffic(
                database,
                output,
                since,
                until,
                path_pattern,
                min_confidence,
                verbose,
            )
            .await?;
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

/// Handle generate-from-traffic command
async fn handle_generate_from_traffic(
    database: Option<PathBuf>,
    output: Option<PathBuf>,
    since: Option<String>,
    until: Option<String>,
    path_pattern: Option<String>,
    min_confidence: f64,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Determine database path
    let db_path = database.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("recordings.db")
    });

    if verbose {
        println!("📂 Using recorder database: {:?}", db_path);
    }

    // Open database
    let db = RecorderDatabase::new(&db_path).await?;

    // Parse time filters
    let since_dt = if let Some(ref since_str) = since {
        Some(
            DateTime::parse_from_rfc3339(since_str)
                .map_err(|e| format!("Invalid --since format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e))?
                .with_timezone(&Utc),
        )
    } else {
        None
    };

    let until_dt = if let Some(ref until_str) = until {
        Some(
            DateTime::parse_from_rfc3339(until_str)
                .map_err(|e| format!("Invalid --until format: {}. Use ISO 8601 format (e.g., 2025-01-01T00:00:00Z)", e))?
                .with_timezone(&Utc),
        )
    } else {
        None
    };

    // Build query filters
    let query_filters = QueryFilters {
        since: since_dt,
        until: until_dt,
        path_pattern,
        min_status_code: None,
        max_requests: Some(1000),
    };

    // Query HTTP exchanges
    if verbose {
        println!("🔍 Querying recorded HTTP traffic...");
    }

    let exchanges = RecordingsToOpenApi::query_http_exchanges(&db, Some(query_filters)).await?;

    if exchanges.is_empty() {
        return Err("No HTTP exchanges found matching the specified filters".into());
    }

    if verbose {
        println!("📊 Found {} HTTP exchanges", exchanges.len());
    }

    // Create OpenAPI generator config
    let behavior_config = IntelligentBehaviorConfig::default();
    let gen_config = OpenApiGenerationConfig {
        min_confidence,
        behavior_model: Some(behavior_config.behavior_model),
    };

    // Generate OpenAPI spec
    if verbose {
        println!("🤖 Generating OpenAPI specification...");
    }

    let generator = OpenApiSpecGenerator::new(gen_config);
    let result = generator.generate_from_exchanges(exchanges).await?;

    if verbose {
        println!("✅ Generated OpenAPI specification");
        println!("   - Requests analyzed: {}", result.metadata.requests_analyzed);
        println!("   - Paths inferred: {}", result.metadata.paths_inferred);
        println!("   - Generation time: {}ms", result.metadata.duration_ms);
        println!("\n📈 Confidence Scores:");
        for (path, score) in &result.metadata.path_confidence {
            if score.value >= min_confidence {
                println!("   - {}: {:.2} - {}", path, score.value, score.reason);
            }
        }
    }

    // Output spec
    // Use raw_document if available, otherwise serialize the spec
    let spec_json = if let Some(ref raw) = result.spec.raw_document {
        raw.clone()
    } else {
        serde_json::to_value(&result.spec.spec)?
    };

    let output_content = if let Some(ref output_path) = output {
        let is_yaml = output_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "yaml" || s == "yml")
            .unwrap_or(false);

        if is_yaml {
            serde_yaml::to_string(&spec_json)?
        } else {
            serde_json::to_string_pretty(&spec_json)?
        }
    } else {
        // Default to JSON for stdout
        serde_json::to_string_pretty(&spec_json)?
    };

    if let Some(output_path) = output {
        tokio::fs::write(&output_path, output_content).await?;
        println!("💾 Saved OpenAPI specification to {:?}", output_path);
    } else {
        // Print to stdout
        println!("{}", output_content);
    }

    Ok(())
}
