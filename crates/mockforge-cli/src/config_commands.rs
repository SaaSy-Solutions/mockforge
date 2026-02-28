//! Configuration management commands for MockForge CLI
//!
//! Provides utilities for:
//! - Generating configuration templates
//! - Listing environment variables
//! - Validating configuration files
//! - Exporting current configuration

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

/// Configuration management subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Generate a configuration template file
    ///
    /// Creates a well-documented YAML configuration template with all
    /// available options and their default values.
    ///
    /// Examples:
    ///   mockforge config generate-template
    ///   mockforge config generate-template --output mockforge.yaml
    ///   mockforge config generate-template --format json
    GenerateTemplate {
        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Include all optional fields with their defaults
        #[arg(long)]
        full: bool,
    },

    /// List all supported environment variables
    ///
    /// Displays a comprehensive list of all environment variables that
    /// MockForge recognizes, along with their descriptions and defaults.
    ///
    /// Examples:
    ///   mockforge config list-env-vars
    ///   mockforge config list-env-vars --category server
    ///   mockforge config list-env-vars --format markdown
    ListEnvVars {
        /// Filter by category (server, database, ai, security, etc.)
        #[arg(short, long)]
        category: Option<String>,

        /// Output format (table, markdown, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Validate a configuration file
    ///
    /// Checks the configuration file for errors, missing required fields,
    /// and potential issues without starting the server.
    ///
    /// Examples:
    ///   mockforge config validate mockforge.yaml
    ///   mockforge config validate --config ./config/prod.yaml
    Validate {
        /// Configuration file to validate
        #[arg(short, long)]
        config: PathBuf,

        /// Show warnings in addition to errors
        #[arg(long)]
        warnings: bool,
    },

    /// Show current effective configuration
    ///
    /// Displays the merged configuration from all sources (defaults,
    /// config file, environment variables) as it would be used at runtime.
    ///
    /// Examples:
    ///   mockforge config show
    ///   mockforge config show --config mockforge.yaml
    ///   mockforge config show --format json
    Show {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Show only non-default values
        #[arg(long)]
        changed_only: bool,
    },
}

/// Environment variable definition
struct EnvVar {
    name: &'static str,
    category: &'static str,
    default: &'static str,
    description: &'static str,
    required: bool,
}

/// Get all environment variables
fn get_env_vars() -> Vec<EnvVar> {
    vec![
        // Server Configuration
        EnvVar {
            name: "MOCKFORGE_HTTP_PORT",
            category: "server",
            default: "3000",
            description: "HTTP server port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_HTTP_HOST",
            category: "server",
            default: "0.0.0.0",
            description: "HTTP server bind host",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_WS_PORT",
            category: "server",
            default: "3001",
            description: "WebSocket server port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_GRPC_PORT",
            category: "server",
            default: "50051",
            description: "gRPC server port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_ADMIN_PORT",
            category: "server",
            default: "9080",
            description: "Admin interface port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_ADMIN_ENABLED",
            category: "server",
            default: "false",
            description: "Enable admin interface",
            required: false,
        },
        // Protocol Ports
        EnvVar {
            name: "MOCKFORGE_SMTP_PORT",
            category: "protocols",
            default: "1025",
            description: "SMTP server port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_SMTP_ENABLED",
            category: "protocols",
            default: "false",
            description: "Enable SMTP server",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_MQTT_PORT",
            category: "protocols",
            default: "1883",
            description: "MQTT broker port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_KAFKA_PORT",
            category: "protocols",
            default: "9092",
            description: "Kafka broker port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_AMQP_PORT",
            category: "protocols",
            default: "5672",
            description: "AMQP broker port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_TCP_PORT",
            category: "protocols",
            default: "8000",
            description: "TCP proxy port",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_TCP_ENABLED",
            category: "protocols",
            default: "false",
            description: "Enable TCP proxy",
            required: false,
        },
        // Database
        EnvVar {
            name: "DATABASE_URL",
            category: "database",
            default: "",
            description: "Database connection URL (required for registry)",
            required: true,
        },
        EnvVar {
            name: "REDIS_URL",
            category: "database",
            default: "",
            description: "Redis connection URL for caching",
            required: false,
        },
        EnvVar {
            name: "RECORDER_DATABASE_PATH",
            category: "database",
            default: "recordings.db",
            description: "Recorder SQLite database path",
            required: false,
        },
        // Security
        EnvVar {
            name: "JWT_SECRET",
            category: "security",
            default: "",
            description: "JWT signing secret (required for auth)",
            required: true,
        },
        EnvVar {
            name: "MOCKFORGE_API_KEY",
            category: "security",
            default: "",
            description: "API key for MockForge cloud",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_REGISTRY_TOKEN",
            category: "security",
            default: "",
            description: "Registry authentication token",
            required: false,
        },
        // AI/RAG
        EnvVar {
            name: "MOCKFORGE_RAG_PROVIDER",
            category: "ai",
            default: "openai",
            description: "RAG provider (openai/anthropic/ollama)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_RAG_API_KEY",
            category: "ai",
            default: "",
            description: "RAG service API key",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_RAG_MODEL",
            category: "ai",
            default: "gpt-4",
            description: "RAG model name",
            required: false,
        },
        EnvVar {
            name: "OPENAI_API_KEY",
            category: "ai",
            default: "",
            description: "OpenAI API key (fallback)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_AI_PROVIDER",
            category: "ai",
            default: "openai",
            description: "AI provider for generation",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_SEMANTIC_SEARCH",
            category: "ai",
            default: "false",
            description: "Enable semantic search",
            required: false,
        },
        // Traffic Control
        EnvVar {
            name: "MOCKFORGE_LATENCY_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable latency injection",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_FAILURES_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable failure injection",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_OVERRIDES_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable response overrides",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_TRAFFIC_SHAPING_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable traffic shaping",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_BANDWIDTH_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable bandwidth limiting",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_RATE_LIMIT_RPM",
            category: "traffic",
            default: "",
            description: "Requests per minute rate limit",
            required: false,
        },
        // File Paths
        EnvVar {
            name: "MOCKFORGE_FIXTURES_DIR",
            category: "files",
            default: "fixtures",
            description: "Directory for test fixtures",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_MOCK_FILES_DIR",
            category: "files",
            default: "mock-files",
            description: "Directory for mock files",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_SNAPSHOT_DIR",
            category: "files",
            default: "",
            description: "Snapshot storage directory",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_HTTP_OVERRIDES_GLOB",
            category: "files",
            default: "",
            description: "Glob pattern for override files",
            required: false,
        },
        // Logging
        EnvVar {
            name: "MOCKFORGE_LOG_LEVEL",
            category: "logging",
            default: "info",
            description: "Log level (debug/info/warn/error)",
            required: false,
        },
        EnvVar {
            name: "RUST_LOG",
            category: "logging",
            default: "",
            description: "Rust logging level (standard)",
            required: false,
        },
        // Notifications
        EnvVar {
            name: "SLACK_WEBHOOK_URL",
            category: "notifications",
            default: "",
            description: "Slack webhook URL",
            required: false,
        },
        EnvVar {
            name: "SLACK_BOT_TOKEN",
            category: "notifications",
            default: "",
            description: "Slack bot token",
            required: false,
        },
        // Storage
        EnvVar {
            name: "S3_BUCKET",
            category: "storage",
            default: "mockforge-plugins",
            description: "S3 bucket for plugin storage",
            required: false,
        },
        EnvVar {
            name: "S3_REGION",
            category: "storage",
            default: "us-east-1",
            description: "AWS S3 region",
            required: false,
        },
        EnvVar {
            name: "S3_ENDPOINT",
            category: "storage",
            default: "",
            description: "Custom S3 endpoint (MinIO)",
            required: false,
        },
        EnvVar {
            name: "AWS_ACCESS_KEY_ID",
            category: "storage",
            default: "",
            description: "AWS S3 access key",
            required: false,
        },
        EnvVar {
            name: "AWS_SECRET_ACCESS_KEY",
            category: "storage",
            default: "",
            description: "AWS S3 secret access key",
            required: false,
        },
        // Performance
        EnvVar {
            name: "MOCKFORGE_COMPRESSION_ENABLED",
            category: "performance",
            default: "true",
            description: "Enable response compression",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_COMPRESSION_ALGORITHM",
            category: "performance",
            default: "gzip",
            description: "Compression algorithm (gzip/deflate/br/zstd)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_COMPRESSION_LEVEL",
            category: "performance",
            default: "6",
            description: "Compression level (1-9 for gzip)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_MAX_BODY_SIZE",
            category: "performance",
            default: "10485760",
            description: "Max request body size in bytes (10MB)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_WORKER_THREADS",
            category: "performance",
            default: "0",
            description: "Worker threads (0=auto-detect)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_POOL_MAX_CONNECTIONS",
            category: "performance",
            default: "100",
            description: "Max connection pool size",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_POOL_IDLE_TIMEOUT",
            category: "performance",
            default: "90",
            description: "Connection pool idle timeout (secs)",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_CIRCUIT_BREAKER_ENABLED",
            category: "performance",
            default: "false",
            description: "Enable circuit breaker",
            required: false,
        },
        EnvVar {
            name: "MOCKFORGE_CIRCUIT_BREAKER_THRESHOLD",
            category: "performance",
            default: "5",
            description: "Circuit breaker failure threshold",
            required: false,
        },
    ]
}

/// Execute a config command
pub async fn execute_config_command(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::GenerateTemplate {
            output,
            format,
            full,
        } => generate_template(output, &format, full).await,
        ConfigCommands::ListEnvVars { category, format } => {
            list_env_vars(category.as_deref(), &format).await
        }
        ConfigCommands::Validate { config, warnings } => validate_config(&config, warnings).await,
        ConfigCommands::Show {
            config,
            format,
            changed_only,
        } => show_config(config.as_ref(), &format, changed_only).await,
    }
}

async fn generate_template(output: Option<PathBuf>, format: &str, full: bool) -> Result<()> {
    let template = if full {
        generate_full_template()
    } else {
        generate_minimal_template()
    };

    let content = match format {
        "json" => serde_json::to_string_pretty(&template)?,
        _ => serde_yaml::to_string(&template)?,
    };

    if let Some(path) = output {
        std::fs::write(&path, &content)?;
        println!("Configuration template written to: {}", path.display());
    } else {
        println!("{}", content);
    }

    Ok(())
}

fn generate_minimal_template() -> serde_json::Value {
    serde_json::json!({
        "# MockForge Configuration": "https://docs.mockforge.dev/configuration",
        "server": {
            "http": {
                "port": 3000,
                "host": "0.0.0.0"
            }
        },
        "openapi": {
            "# Path to OpenAPI specification": "",
            "spec_path": "./openapi.yaml"
        },
        "logging": {
            "level": "info"
        }
    })
}

fn generate_full_template() -> serde_json::Value {
    serde_json::json!({
        "# MockForge Full Configuration Template": "https://docs.mockforge.dev/configuration",
        "server": {
            "http": {
                "port": 3000,
                "host": "0.0.0.0",
                "# Enable CORS": "",
                "cors_enabled": true
            },
            "websocket": {
                "enabled": false,
                "port": 3001
            },
            "grpc": {
                "enabled": false,
                "port": 50051
            },
            "admin": {
                "enabled": false,
                "port": 9080
            }
        },
        "protocols": {
            "smtp": {
                "enabled": false,
                "port": 1025
            },
            "mqtt": {
                "enabled": false,
                "port": 1883,
                "tls_enabled": false,
                "tls_port": 8883
            },
            "kafka": {
                "enabled": false,
                "port": 9092
            },
            "amqp": {
                "enabled": false,
                "port": 5672,
                "tls_enabled": false,
                "tls_port": 5671
            },
            "tcp": {
                "enabled": false,
                "port": 8000
            }
        },
        "openapi": {
            "spec_path": "./openapi.yaml",
            "validate_requests": true,
            "validate_responses": false
        },
        "fixtures": {
            "directory": "./fixtures",
            "hot_reload": true
        },
        "latency": {
            "enabled": false,
            "min_ms": 0,
            "max_ms": 100,
            "distribution": "normal"
        },
        "failures": {
            "enabled": false,
            "rate": 0.0,
            "status_codes": [500, 502, 503]
        },
        "traffic_shaping": {
            "enabled": false,
            "bandwidth_limit_bytes": 1000000,
            "burst_size_bytes": 100000
        },
        "logging": {
            "level": "info",
            "format": "json",
            "# Available: json, pretty, compact": ""
        },
        "ai": {
            "enabled": false,
            "provider": "openai",
            "# api_key loaded from OPENAI_API_KEY or MOCKFORGE_RAG_API_KEY": ""
        },
        "proxy": {
            "enabled": false,
            "upstream_url": "",
            "record": false,
            "replay": false
        },
        "metrics": {
            "enabled": false,
            "prometheus_endpoint": "/metrics"
        },
        "security": {
            "# Rate limiting": "",
            "rate_limit_rpm": 0,
            "# TLS configuration": "",
            "tls": {
                "enabled": false,
                "cert_path": "",
                "key_path": ""
            }
        },
        "performance": {
            "compression": {
                "enabled": true,
                "algorithm": "gzip",
                "min_size": 1024,
                "level": 6
            },
            "connection_pool": {
                "enabled": true,
                "max_connections": 100,
                "max_idle_per_host": 10,
                "idle_timeout_secs": 90
            },
            "request_limits": {
                "max_body_size": 10485760,
                "max_header_size": 16384,
                "max_headers": 100
            },
            "workers": {
                "threads": 0,
                "blocking_threads": 512
            },
            "circuit_breaker": {
                "enabled": false,
                "failure_threshold": 5,
                "success_threshold": 2,
                "half_open_timeout_secs": 30
            }
        }
    })
}

async fn list_env_vars(category: Option<&str>, format: &str) -> Result<()> {
    let all_vars = get_env_vars();

    let vars: Vec<_> = if let Some(cat) = category {
        all_vars.into_iter().filter(|v| v.category == cat).collect()
    } else {
        all_vars
    };

    match format {
        "markdown" => print_env_vars_markdown(&vars),
        "json" => print_env_vars_json(&vars)?,
        _ => print_env_vars_table(&vars),
    }

    Ok(())
}

fn print_env_vars_table(vars: &[EnvVar]) {
    println!("{:<40} {:<12} {:<15} {}", "Variable", "Category", "Default", "Description");
    println!("{}", "-".repeat(100));

    for var in vars {
        let required = if var.required { "*" } else { "" };
        let default = if var.default.is_empty() {
            "-"
        } else {
            var.default
        };
        println!(
            "{:<40} {:<12} {:<15} {}{}",
            var.name, var.category, default, var.description, required
        );
    }

    println!();
    println!("* = Required variable");
    println!();
    println!("Categories: server, protocols, database, security, ai, traffic, files, logging, notifications, storage, performance");
}

fn print_env_vars_markdown(vars: &[EnvVar]) {
    println!("# MockForge Environment Variables\n");

    let mut current_category = "";
    for var in vars {
        if var.category != current_category {
            current_category = var.category;
            println!("\n## {}\n", capitalize(current_category));
            println!("| Variable | Default | Description |");
            println!("|----------|---------|-------------|");
        }

        let default = if var.default.is_empty() {
            "-"
        } else {
            var.default
        };
        let required = if var.required { " **(required)**" } else { "" };
        println!("| `{}` | `{}` | {}{} |", var.name, default, var.description, required);
    }
}

fn print_env_vars_json(vars: &[EnvVar]) -> Result<()> {
    let json_vars: Vec<serde_json::Value> = vars
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "category": v.category,
                "default": v.default,
                "description": v.description,
                "required": v.required
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_vars)?);
    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

async fn validate_config(config_path: &PathBuf, show_warnings: bool) -> Result<()> {
    use mockforge_core::config::ServerConfig;

    println!("Validating configuration: {}", config_path.display());

    // Check file exists
    if !config_path.exists() {
        anyhow::bail!("Configuration file not found: {}", config_path.display());
    }

    // Try to parse the config
    let content = std::fs::read_to_string(config_path)?;
    let config: Result<ServerConfig, _> = serde_yaml::from_str(&content);

    match config {
        Ok(cfg) => {
            println!("  Configuration parsed successfully");

            // Run validation checks
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            // Check HTTP TLS configuration
            if let Some(ref tls) = cfg.http.tls {
                if tls.enabled {
                    if tls.cert_file.is_empty() {
                        errors.push("HTTP TLS enabled but cert_file is empty".to_string());
                    }
                    if tls.key_file.is_empty() {
                        errors.push("HTTP TLS enabled but key_file is empty".to_string());
                    }
                }
            }

            // Check gRPC configuration
            if cfg.grpc.enabled {
                // gRPC is enabled, no additional TLS checks needed for now
            }

            // Check SMTP fixtures directory
            if let Some(fixtures_dir) = &cfg.smtp.fixtures_dir {
                if !fixtures_dir.exists() {
                    warnings.push(format!(
                        "SMTP fixtures directory not found: {}",
                        fixtures_dir.display()
                    ));
                }
            }

            // Check MQTT fixtures directory
            if let Some(fixtures_dir) = &cfg.mqtt.fixtures_dir {
                if !fixtures_dir.exists() {
                    warnings.push(format!(
                        "MQTT fixtures directory not found: {}",
                        fixtures_dir.display()
                    ));
                }
            }

            // Print results
            if errors.is_empty() && (warnings.is_empty() || !show_warnings) {
                println!("  All checks passed!");
            } else {
                if !errors.is_empty() {
                    println!("\n  Errors:");
                    for err in &errors {
                        println!("    - {}", err);
                    }
                }
                if show_warnings && !warnings.is_empty() {
                    println!("\n  Warnings:");
                    for warn in &warnings {
                        println!("    - {}", warn);
                    }
                }

                if !errors.is_empty() {
                    anyhow::bail!("Configuration validation failed with {} error(s)", errors.len());
                }
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to parse configuration: {}", e);
        }
    }

    Ok(())
}

async fn show_config(
    config_path: Option<&PathBuf>,
    format: &str,
    changed_only: bool,
) -> Result<()> {
    use mockforge_core::config::ServerConfig;

    let config: ServerConfig = if let Some(path) = config_path {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content)?
    } else {
        ServerConfig::default()
    };

    let output_value = if changed_only {
        let current = serde_json::to_value(&config)?;
        let defaults = serde_json::to_value(ServerConfig::default())?;
        let diff = json_diff(&current, &defaults);
        if diff.is_null() || diff.as_object().is_some_and(|o| o.is_empty()) {
            println!("No changes from default configuration.");
            return Ok(());
        }
        diff
    } else {
        serde_json::to_value(&config)?
    };

    let output = match format {
        "json" => serde_json::to_string_pretty(&output_value)?,
        _ => {
            // Convert through JSON value to YAML for consistent output
            serde_yaml::to_string(&output_value)?
        }
    };

    println!("{}", output);

    Ok(())
}

/// Recursively compute only the keys in `current` that differ from `defaults`.
/// Returns a JSON object containing only the changed fields, preserving nesting.
fn json_diff(current: &serde_json::Value, defaults: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    match (current, defaults) {
        (Value::Object(cur_map), Value::Object(def_map)) => {
            let mut diff = serde_json::Map::new();
            for (key, cur_val) in cur_map {
                match def_map.get(key) {
                    Some(def_val) => {
                        let child_diff = json_diff(cur_val, def_val);
                        if !child_diff.is_null() {
                            diff.insert(key.clone(), child_diff);
                        }
                    }
                    // Key exists in current but not in defaults — it's new
                    None => {
                        diff.insert(key.clone(), cur_val.clone());
                    }
                }
            }
            if diff.is_empty() {
                Value::Null
            } else {
                Value::Object(diff)
            }
        }
        // For non-object values, return current if it differs from default
        _ => {
            if current == defaults {
                serde_json::Value::Null
            } else {
                current.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_vars_list() {
        let vars = get_env_vars();
        assert!(!vars.is_empty());

        // Check required vars exist
        let required: Vec<_> = vars.iter().filter(|v| v.required).collect();
        assert!(!required.is_empty());
    }

    #[test]
    fn test_generate_templates() {
        let minimal = generate_minimal_template();
        assert!(minimal.get("server").is_some());

        let full = generate_full_template();
        assert!(full.get("protocols").is_some());
    }
}
