use clap::Subcommand;
use mockforge_core::ServerConfig;
use serde_json::json;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum ConfigCommands {
    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Show warnings in addition to errors
        #[arg(long)]
        warnings: bool,
    },

    /// Generate a configuration template file
    ///
    /// Creates a well-documented YAML configuration template with all
    /// available options and their default values.
    ///
    /// Examples:
    ///   mockforge config generate-template
    ///   mockforge config generate-template --output mockforge.yaml
    ///   mockforge config generate-template --format json --full
    #[command(verbatim_doc_comment)]
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
    #[command(verbatim_doc_comment)]
    ListEnvVars {
        /// Filter by category (server, protocols, database, security, ai, traffic, files, logging)
        #[arg(short, long)]
        category: Option<String>,

        /// Output format (table, markdown, json)
        #[arg(short, long, default_value = "table")]
        format: String,
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
    #[command(verbatim_doc_comment)]
    Show {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },
}

/// Environment variable definition for listing
pub(crate) struct EnvVarDef {
    name: &'static str,
    category: &'static str,
    default: &'static str,
    description: &'static str,
    required: bool,
}

/// Handle config commands
pub(crate) async fn handle_config(
    config_command: ConfigCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match config_command {
        ConfigCommands::Validate { config, warnings } => {
            handle_config_validate(config, warnings).await?;
        }
        ConfigCommands::GenerateTemplate {
            output,
            format,
            full,
        } => {
            handle_config_generate_template(output, &format, full).await?;
        }
        ConfigCommands::ListEnvVars { category, format } => {
            handle_config_list_env_vars(category.as_deref(), &format).await?;
        }
        ConfigCommands::Show { config, format } => {
            handle_config_show(config, &format).await?;
        }
    }
    Ok(())
}

/// Handle config validation
pub(crate) async fn handle_config_validate(
    config_path: Option<PathBuf>,
    _show_warnings: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Validating MockForge configuration...");

    // Auto-discover config file if not provided
    let config_file = if let Some(path) = config_path {
        path
    } else {
        discover_config_file()?
    };

    println!("📄 Checking configuration file: {}", config_file.display());

    // Check if file exists
    if !config_file.exists() {
        return Err(format!("Configuration file not found: {}", config_file.display()).into());
    }

    // Read and parse YAML/JSON
    let config_content = tokio::fs::read_to_string(&config_file).await?;
    let is_yaml = config_file
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext == "yaml" || ext == "yml")
        .unwrap_or(true);

    // First, try to parse with ServerConfig for full schema validation
    let config_result = if is_yaml {
        serde_yaml::from_str::<ServerConfig>(&config_content)
            .map_err(|e| format_yaml_error(&config_content, e))
    } else {
        serde_json::from_str::<ServerConfig>(&config_content)
            .map_err(|e| format_json_error(&config_content, e))
    };

    match config_result {
        Ok(config) => {
            // Successfully parsed - now validate content
            let mut warnings = Vec::new();
            let mut errors = Vec::new();

            // Validate HTTP section
            if config.http.host.is_empty() {
                errors.push("HTTP host is empty".to_string());
            }
            if config.http.port == 0 {
                errors.push("HTTP port cannot be 0".to_string());
            }

            // Check OpenAPI spec if provided
            if let Some(ref spec_path) = config.http.openapi_spec {
                if !std::path::Path::new(spec_path).exists() {
                    errors.push(format!("OpenAPI spec file not found: {}", spec_path));
                } else {
                    println!("   ✓ OpenAPI spec: {}", spec_path);
                }
            } else {
                warnings.push(
                    "No OpenAPI spec configured. HTTP endpoints will need to be defined manually."
                        .to_string(),
                );
            }

            // Validate request validation mode
            let valid_modes = ["off", "warn", "enforce"];
            if let Some(validation) = &config.http.validation {
                if !valid_modes.contains(&validation.mode.as_str()) {
                    errors.push(format!(
                        "Invalid request validation mode '{}'. Must be one of: off, warn, enforce",
                        validation.mode
                    ));
                }
            }

            // Validate HTTP auth if configured
            if let Some(ref auth) = config.http.auth {
                if let Some(ref jwt) = auth.jwt {
                    if let Some(ref secret) = jwt.secret {
                        if secret.is_empty() {
                            errors.push(
                                "HTTP JWT auth is configured but secret is empty".to_string(),
                            );
                        }
                    } else if jwt.rsa_public_key.is_none() && jwt.ecdsa_public_key.is_none() {
                        errors.push("HTTP JWT auth requires at least one key (secret, rsa_public_key, or ecdsa_public_key)".to_string());
                    }
                }
                if let Some(ref basic) = auth.basic_auth {
                    if basic.credentials.is_empty() {
                        warnings.push(
                            "HTTP Basic auth is configured but no credentials are defined"
                                .to_string(),
                        );
                    }
                }
            }

            // Validate WebSocket section
            if config.websocket.port == 0 {
                errors.push("WebSocket port cannot be 0".to_string());
            }
            if config.websocket.port == config.http.port {
                errors.push("WebSocket port conflicts with HTTP port".to_string());
            }

            // Validate gRPC section
            if config.grpc.port == 0 {
                errors.push("gRPC port cannot be 0".to_string());
            }
            if config.grpc.port == config.http.port || config.grpc.port == config.websocket.port {
                errors.push("gRPC port conflicts with HTTP or WebSocket port".to_string());
            }

            // Validate chaining configuration
            if config.chaining.enabled {
                if config.chaining.max_chain_length == 0 {
                    errors.push("Chaining is enabled but max_chain_length is 0".to_string());
                }
                if config.chaining.global_timeout_secs == 0 {
                    warnings.push("Chaining global timeout is 0 (no timeout)".to_string());
                }
                println!(
                    "   ✓ Request chaining: enabled (max length: {})",
                    config.chaining.max_chain_length
                );
            }

            // Validate admin UI configuration
            if config.admin.enabled {
                if config.admin.port == 0 {
                    errors.push("Admin UI is enabled but port is 0".to_string());
                }
                if config.admin.port == config.http.port
                    || config.admin.port == config.websocket.port
                    || config.admin.port == config.grpc.port
                {
                    errors.push("Admin UI port conflicts with another service port".to_string());
                }
                if config.admin.auth_required
                    && (config.admin.username.is_none() || config.admin.password.is_none())
                {
                    errors.push(
                        "Admin UI auth is required but username/password not configured"
                            .to_string(),
                    );
                }
            } else {
                warnings
                    .push("Admin UI is disabled. Enable with 'admin.enabled: true'.".to_string());
            }

            // Validate observability
            if config.observability.prometheus.enabled && config.observability.prometheus.port == 0
            {
                errors.push("Prometheus metrics enabled but port is 0".to_string());
            }

            if let Some(ref otel) = config.observability.opentelemetry {
                if otel.enabled {
                    if otel.service_name.is_empty() {
                        warnings.push("OpenTelemetry service name is empty".to_string());
                    }
                    println!("   ✓ OpenTelemetry: enabled (service: {})", otel.service_name);
                }
            }

            if let Some(ref recorder) = config.observability.recorder {
                if recorder.enabled {
                    if recorder.database_path.is_empty() {
                        errors.push("Recorder is enabled but database path is empty".to_string());
                    }
                    println!("   ✓ Recorder: enabled (db: {})", recorder.database_path);
                }
            }

            // Print results
            if !errors.is_empty() {
                println!("\n❌ Configuration has errors:");
                for error in &errors {
                    println!("   ✗ {}", error);
                }
                return Err("Configuration validation failed".into());
            }

            println!("\n✅ Configuration is valid");
            println!("\n📊 Summary:");
            println!("   HTTP server: {}:{}", config.http.host, config.http.port);
            println!("   WebSocket server: {}:{}", config.websocket.host, config.websocket.port);
            println!("   gRPC server: {}:{}", config.grpc.host, config.grpc.port);

            if config.admin.enabled {
                println!("   Admin UI: http://{}:{}", config.admin.host, config.admin.port);
            }

            if config.observability.prometheus.enabled {
                println!(
                    "   Prometheus metrics: http://{}:{}/metrics",
                    config.http.host, config.observability.prometheus.port
                );
            }

            if !warnings.is_empty() {
                println!("\n⚠️  Warnings:");
                for warning in warnings {
                    println!("   - {}", warning);
                }
            }

            Ok(())
        }
        Err(error_msg) => {
            println!("❌ Configuration validation failed:\n");
            println!("{}", error_msg);
            Err("Invalid configuration".into())
        }
    }
}

/// Format YAML parsing errors with line numbers and better field path extraction
pub(crate) fn format_yaml_error(content: &str, error: serde_yaml::Error) -> String {
    let mut message = String::from("❌ Configuration parsing error:\n\n");

    // Extract field path from error message if possible
    let error_str = error.to_string();
    let field_path = extract_field_path(&error_str);

    if let Some(location) = error.location() {
        let line = location.line();
        let column = location.column();

        message.push_str(&format!("📍 Location: line {}, column {}\n\n", line, column));

        // Show the problematic line with context
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(2);
        let end = (line + 1).min(lines.len());

        for (idx, line_content) in lines[start..end].iter().enumerate() {
            let line_num = start + idx + 1;
            if line_num == line {
                message.push_str(&format!("  > {} | {}\n", line_num, line_content));
                if column > 0 {
                    message.push_str(&format!(
                        "    {}^\n",
                        " ".repeat(column + 5 + line_num.to_string().len())
                    ));
                }
            } else {
                message.push_str(&format!("    {} | {}\n", line_num, line_content));
            }
        }

        message.push('\n');
    }

    // Show the error with field path if extracted
    if let Some(path) = &field_path {
        message.push_str(&format!("🔍 Field path: {}\n", path));
        message.push_str(&format!("❌ Error: {}\n\n", error));
    } else {
        message.push_str(&format!("❌ Error: {}\n\n", error));
    }

    // Add helpful suggestions based on error type and field path
    if error_str.contains("duplicate key") {
        message.push_str("💡 Tip: You have a duplicate key in your YAML. Each key must be unique within its section.\n");
    } else if error_str.contains("invalid type") {
        message.push_str("💡 Tip: Check that your values match the expected types (strings, numbers, booleans, arrays, objects).\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Check the type for field: {}\n", path));
        }
    } else if error_str.contains("missing field") {
        // Most fields in MockForge are optional with defaults
        message.push_str("💡 Tip: This field is usually optional and has a default value.\n");
        message.push_str(
            "   Most configuration fields can be omitted - MockForge will use sensible defaults.\n",
        );
        if let Some(path) = &field_path {
            message.push_str(&format!("   \n   To fix: Either add the field at path '{}' or remove it entirely (defaults will be used).\n", path));
            message.push_str(
                "   See config.template.yaml for all available options and their defaults.\n",
            );
        } else {
            message.push_str(
                "   See config.template.yaml for all available options and their defaults.\n",
            );
        }
    } else if error_str.contains("unknown field") {
        message.push_str("💡 Tip: You may have a typo in a field name.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Unknown field at path: {}\n", path));
            message.push_str(
                "   Check the spelling against the documentation or config.template.yaml.\n",
            );
        } else {
            message.push_str(
                "   Check the spelling against the documentation or config.template.yaml.\n",
            );
        }
    } else if error_str.contains("expected") {
        message.push_str("💡 Tip: There's a type mismatch or syntax error.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Check the value type for field: {}\n", path));
        }
    }

    message.push_str("\n📚 For a complete example configuration, see: config.template.yaml\n");
    message.push_str("   Or run: mockforge init .\n");

    message
}

/// Format JSON parsing errors with line numbers and better field path extraction
pub(crate) fn format_json_error(content: &str, error: serde_json::Error) -> String {
    let mut message = String::from("❌ Configuration parsing error:\n\n");

    // Extract field path from error message if possible
    let error_str = error.to_string();
    let field_path = extract_field_path(&error_str);

    let line = error.line();
    let column = error.column();

    message.push_str(&format!("📍 Location: line {}, column {}\n\n", line, column));

    // Show the problematic line with context
    let lines: Vec<&str> = content.lines().collect();
    let start = line.saturating_sub(2);
    let end = (line + 1).min(lines.len());

    for (idx, line_content) in lines[start..end].iter().enumerate() {
        let line_num = start + idx + 1;
        if line_num == line {
            message.push_str(&format!("  > {} | {}\n", line_num, line_content));
            if column > 0 {
                message.push_str(&format!(
                    "    {}^\n",
                    " ".repeat(column + 5 + line_num.to_string().len())
                ));
            }
        } else {
            message.push_str(&format!("    {} | {}\n", line_num, line_content));
        }
    }

    message.push('\n');

    // Show the error with field path if extracted
    if let Some(path) = &field_path {
        message.push_str(&format!("🔍 Field path: {}\n", path));
        message.push_str(&format!("❌ Error: {}\n\n", error));
    } else {
        message.push_str(&format!("❌ Error: {}\n\n", error));
    }

    // Add helpful suggestions based on error type
    if error_str.contains("trailing comma") {
        message.push_str(
            "💡 Tip: JSON doesn't allow trailing commas. Remove the comma after the last item.\n",
        );
    } else if error_str.contains("missing field") {
        message.push_str("💡 Tip: This field is usually optional and has a default value.\n");
        message.push_str(
            "   Most configuration fields can be omitted - MockForge will use sensible defaults.\n",
        );
        if let Some(path) = &field_path {
            message.push_str(&format!("   \n   To fix: Either add the field at path '{}' or remove it entirely (defaults will be used).\n", path));
        }
        message.push_str(
            "   See config.template.yaml for all available options and their defaults.\n",
        );
    } else if error_str.contains("duplicate field") {
        message.push_str(
            "💡 Tip: You have a duplicate key. Each key must be unique within its object.\n",
        );
    } else if error_str.contains("expected") {
        message
            .push_str("💡 Tip: Check for missing or extra brackets, braces, quotes, or commas.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Or check the value type for field: {}\n", path));
        }
    } else if error_str.contains("unknown field") {
        message.push_str("💡 Tip: You may have a typo in a field name.\n");
        if let Some(path) = &field_path {
            message.push_str(&format!("   Unknown field at path: {}\n", path));
        }
        message
            .push_str("   Check the spelling against the documentation or config.template.yaml.\n");
    }

    message.push_str("\n📚 For a complete example configuration, see: config.template.yaml\n");
    message.push_str("   Or run: mockforge init .\n");

    message
}

/// Extract field path from serde error messages
/// Examples:
///   "missing field `host` at line 2 column 1" -> Some("host")
///   "unknown field `foo`, expected one of `bar`, `baz`" -> Some("foo")
///   "invalid type: string \"x\", expected u16 at line 5" -> None (type error, not field path)
pub(crate) fn extract_field_path(error_msg: &str) -> Option<String> {
    // Try to extract field name from "missing field `X`" or "unknown field `X`"
    if let Some(start) = error_msg.find("field `") {
        let after_field = &error_msg[start + 7..];
        if let Some(end) = after_field.find('`') {
            let field_name = &after_field[..end];

            // Try to find parent path context if available
            // Serde errors sometimes include path context like "http.host"
            if let Some(path_context_start) = error_msg.rfind(" at ") {
                let path_context = &error_msg[..path_context_start];
                // Look for common patterns like "http.host" or "admin.port"
                for section in ["http", "admin", "websocket", "grpc", "core", "logging"] {
                    if path_context.contains(section) {
                        return Some(format!("{}.{}", section, field_name));
                    }
                }
            }

            return Some(field_name.to_string());
        }
    }

    // Try to extract from "invalid type" with context
    if let Some(start) = error_msg.find("invalid type") {
        // Look backwards for field context
        if let Some(field_start) = error_msg[..start].rfind("field `") {
            let after_field = &error_msg[field_start + 7..];
            if let Some(end) = after_field.find('`') {
                return Some(after_field[..end].to_string());
            }
        }
    }

    None
}

/// Discover configuration file in current directory and parents
pub(crate) fn discover_config_file() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let current_dir = std::env::current_dir()?;
    let config_names = vec![
        "mockforge.yaml",
        "mockforge.yml",
        ".mockforge.yaml",
        ".mockforge.yml",
    ];

    // Check current directory
    for name in &config_names {
        let path = current_dir.join(name);
        if path.exists() {
            return Ok(path);
        }
    }

    // Check parent directories (up to 5 levels)
    let mut dir = current_dir.clone();
    for _ in 0..5 {
        if let Some(parent) = dir.parent() {
            for name in &config_names {
                let path = parent.join(name);
                if path.exists() {
                    return Ok(path);
                }
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err("No configuration file found. Expected one of: mockforge.yaml, mockforge.yml, .mockforge.yaml, .mockforge.yml".into())
}

/// Handle config generate-template command
pub(crate) async fn handle_config_generate_template(
    output: Option<PathBuf>,
    format: &str,
    full: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let template = if full {
        generate_full_config_template()
    } else {
        generate_minimal_config_template()
    };

    let content = match format {
        "json" => serde_json::to_string_pretty(&template)?,
        _ => serde_yaml::to_string(&template)?,
    };

    if let Some(path) = output {
        tokio::fs::write(&path, &content).await?;
        println!("✅ Configuration template written to: {}", path.display());
    } else {
        println!("{}", content);
    }

    Ok(())
}

pub(crate) fn generate_minimal_config_template() -> serde_json::Value {
    json!({
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

pub(crate) fn generate_full_config_template() -> serde_json::Value {
    json!({
        "# MockForge Full Configuration Template": "https://docs.mockforge.dev/configuration",
        "server": {
            "http": {
                "port": 3000,
                "host": "0.0.0.0",
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
            "smtp": { "enabled": false, "port": 1025 },
            "mqtt": { "enabled": false, "port": 1883, "tls_enabled": false, "tls_port": 8883 },
            "kafka": { "enabled": false, "port": 9092 },
            "amqp": { "enabled": false, "port": 5672, "tls_enabled": false, "tls_port": 5671 },
            "tcp": { "enabled": false, "port": 8000 }
        },
        "openapi": {
            "spec_path": "./openapi.yaml",
            "validate_requests": true,
            "validate_responses": false
        },
        "fixtures": { "directory": "./fixtures", "hot_reload": true },
        "latency": { "enabled": false, "min_ms": 0, "max_ms": 100, "distribution": "normal" },
        "failures": { "enabled": false, "rate": 0.0, "status_codes": [500, 502, 503] },
        "traffic_shaping": { "enabled": false, "bandwidth_limit_bytes": 1000000, "burst_size_bytes": 100000 },
        "logging": { "level": "info", "format": "json" },
        "ai": { "enabled": false, "provider": "openai" },
        "proxy": { "enabled": false, "upstream_url": "", "record": false, "replay": false },
        "metrics": { "enabled": false, "prometheus_endpoint": "/metrics" },
        "security": { "rate_limit_rpm": 0, "tls": { "enabled": false, "cert_path": "", "key_path": "" } },
        "performance": {
            "compression": { "enabled": true, "algorithm": "gzip", "min_size": 1024, "level": 6 },
            "connection_pool": { "enabled": true, "max_connections": 100, "max_idle_per_host": 10, "idle_timeout_secs": 90 },
            "request_limits": { "max_body_size": 10485760, "max_header_size": 16384, "max_headers": 100 },
            "workers": { "threads": 0, "blocking_threads": 512 },
            "circuit_breaker": { "enabled": false, "failure_threshold": 5, "success_threshold": 2, "half_open_timeout_secs": 30 }
        },
        "plugins": {
            "enabled": true,
            "max_memory_per_plugin": 10485760,
            "max_cpu_per_plugin": 0.5,
            "max_execution_time_ms": 5000,
            "allow_network_access": false,
            "max_concurrent_executions": 10,
            "max_module_size": 5242880,
            "max_table_elements": 1000,
            "max_stack_size": 2097152
        },
        "hot_reload": {
            "enabled": false,
            "check_interval_secs": 5,
            "debounce_delay_ms": 1000,
            "reload_on_spec_change": true,
            "reload_on_fixture_change": true,
            "reload_on_plugin_change": true,
            "graceful_reload": true,
            "graceful_timeout_secs": 30,
            "validate_before_reload": true,
            "rollback_on_failure": true
        },
        "secrets": {
            "provider": "none",
            "cache_ttl_secs": 300,
            "retry_attempts": 3,
            "retry_delay_ms": 1000,
            "vault": {
                "address": "http://127.0.0.1:8200",
                "auth_method": "token",
                "mount_path": "secret",
                "path_prefix": "mockforge",
                "skip_verify": false,
                "timeout_secs": 30
            }
        }
    })
}

pub(crate) fn get_env_var_definitions() -> Vec<EnvVarDef> {
    vec![
        // Server
        EnvVarDef {
            name: "MOCKFORGE_HTTP_PORT",
            category: "server",
            default: "3000",
            description: "HTTP server port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HTTP_HOST",
            category: "server",
            default: "0.0.0.0",
            description: "HTTP server bind host",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_WS_PORT",
            category: "server",
            default: "3001",
            description: "WebSocket server port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_GRPC_PORT",
            category: "server",
            default: "50051",
            description: "gRPC server port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_ADMIN_PORT",
            category: "server",
            default: "9080",
            description: "Admin interface port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_ADMIN_ENABLED",
            category: "server",
            default: "false",
            description: "Enable admin interface",
            required: false,
        },
        // Protocols
        EnvVarDef {
            name: "MOCKFORGE_SMTP_PORT",
            category: "protocols",
            default: "1025",
            description: "SMTP server port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_SMTP_ENABLED",
            category: "protocols",
            default: "false",
            description: "Enable SMTP server",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_MQTT_PORT",
            category: "protocols",
            default: "1883",
            description: "MQTT broker port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_KAFKA_PORT",
            category: "protocols",
            default: "9092",
            description: "Kafka broker port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_AMQP_PORT",
            category: "protocols",
            default: "5672",
            description: "AMQP broker port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_TCP_PORT",
            category: "protocols",
            default: "8000",
            description: "TCP proxy port",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_TCP_ENABLED",
            category: "protocols",
            default: "false",
            description: "Enable TCP proxy",
            required: false,
        },
        // Database
        EnvVarDef {
            name: "DATABASE_URL",
            category: "database",
            default: "",
            description: "Database connection URL (required for registry)",
            required: true,
        },
        EnvVarDef {
            name: "REDIS_URL",
            category: "database",
            default: "",
            description: "Redis connection URL for caching",
            required: false,
        },
        EnvVarDef {
            name: "RECORDER_DATABASE_PATH",
            category: "database",
            default: "recordings.db",
            description: "Recorder SQLite database path",
            required: false,
        },
        // Security
        EnvVarDef {
            name: "JWT_SECRET",
            category: "security",
            default: "",
            description: "JWT signing secret (required for auth)",
            required: true,
        },
        EnvVarDef {
            name: "MOCKFORGE_API_KEY",
            category: "security",
            default: "",
            description: "API key for MockForge cloud",
            required: false,
        },
        // AI
        EnvVarDef {
            name: "MOCKFORGE_RAG_PROVIDER",
            category: "ai",
            default: "openai",
            description: "RAG provider (openai/anthropic/ollama)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_RAG_API_KEY",
            category: "ai",
            default: "",
            description: "RAG service API key",
            required: false,
        },
        EnvVarDef {
            name: "OPENAI_API_KEY",
            category: "ai",
            default: "",
            description: "OpenAI API key (fallback)",
            required: false,
        },
        // Traffic
        EnvVarDef {
            name: "MOCKFORGE_LATENCY_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable latency injection",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_FAILURES_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable failure injection",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_OVERRIDES_ENABLED",
            category: "traffic",
            default: "false",
            description: "Enable response overrides",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_RATE_LIMIT_RPM",
            category: "traffic",
            default: "",
            description: "Requests per minute rate limit",
            required: false,
        },
        // Files
        EnvVarDef {
            name: "MOCKFORGE_FIXTURES_DIR",
            category: "files",
            default: "fixtures",
            description: "Directory for test fixtures",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_MOCK_FILES_DIR",
            category: "files",
            default: "mock-files",
            description: "Directory for mock files",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_SNAPSHOT_DIR",
            category: "files",
            default: "",
            description: "Snapshot storage directory",
            required: false,
        },
        // Logging
        EnvVarDef {
            name: "MOCKFORGE_LOG_LEVEL",
            category: "logging",
            default: "info",
            description: "Log level (debug/info/warn/error)",
            required: false,
        },
        EnvVarDef {
            name: "RUST_LOG",
            category: "logging",
            default: "",
            description: "Rust logging level (standard)",
            required: false,
        },
        // Performance
        EnvVarDef {
            name: "MOCKFORGE_COMPRESSION_ENABLED",
            category: "performance",
            default: "true",
            description: "Enable response compression",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_COMPRESSION_ALGORITHM",
            category: "performance",
            default: "gzip",
            description: "Compression algorithm (gzip/deflate/br/zstd)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_COMPRESSION_LEVEL",
            category: "performance",
            default: "6",
            description: "Compression level (1-9 for gzip)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_MAX_BODY_SIZE",
            category: "performance",
            default: "10485760",
            description: "Max request body size in bytes (10MB)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_WORKER_THREADS",
            category: "performance",
            default: "0",
            description: "Worker threads (0=auto-detect)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_POOL_MAX_CONNECTIONS",
            category: "performance",
            default: "100",
            description: "Max connection pool size",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_POOL_IDLE_TIMEOUT",
            category: "performance",
            default: "90",
            description: "Connection pool idle timeout (secs)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_CIRCUIT_BREAKER_ENABLED",
            category: "performance",
            default: "false",
            description: "Enable circuit breaker",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_CIRCUIT_BREAKER_THRESHOLD",
            category: "performance",
            default: "5",
            description: "Circuit breaker failure threshold",
            required: false,
        },
        // Plugins
        EnvVarDef {
            name: "MOCKFORGE_PLUGINS_ENABLED",
            category: "plugins",
            default: "true",
            description: "Enable plugin system",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_MAX_MEMORY",
            category: "plugins",
            default: "10485760",
            description: "Max memory per plugin (10MB)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_MAX_CPU",
            category: "plugins",
            default: "0.5",
            description: "Max CPU per plugin (0.5 = 50%)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_TIMEOUT_MS",
            category: "plugins",
            default: "5000",
            description: "Plugin execution timeout (ms)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_NETWORK_ACCESS",
            category: "plugins",
            default: "false",
            description: "Allow plugins network access",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_MAX_CONCURRENT",
            category: "plugins",
            default: "10",
            description: "Max concurrent plugin executions",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_CACHE_DIR",
            category: "plugins",
            default: "",
            description: "Plugin cache directory",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_PLUGIN_MAX_MODULE_SIZE",
            category: "plugins",
            default: "5242880",
            description: "Max WASM module size (5MB)",
            required: false,
        },
        // Hot Reload
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_ENABLED",
            category: "hot_reload",
            default: "false",
            description: "Enable config hot-reload",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_INTERVAL",
            category: "hot_reload",
            default: "5",
            description: "Config check interval (secs)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_DEBOUNCE",
            category: "hot_reload",
            default: "1000",
            description: "Debounce delay (ms)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_SPEC",
            category: "hot_reload",
            default: "true",
            description: "Reload on OpenAPI spec change",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_FIXTURES",
            category: "hot_reload",
            default: "true",
            description: "Reload on fixture change",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_GRACEFUL",
            category: "hot_reload",
            default: "true",
            description: "Wait for in-flight requests",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_TIMEOUT",
            category: "hot_reload",
            default: "30",
            description: "Graceful reload timeout (secs)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_HOT_RELOAD_VALIDATE",
            category: "hot_reload",
            default: "true",
            description: "Validate config before reload",
            required: false,
        },
        // Secrets
        EnvVarDef {
            name: "MOCKFORGE_SECRET_PROVIDER",
            category: "secrets",
            default: "none",
            description: "Secret provider (vault/aws/azure/gcp/k8s)",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_SECRET_CACHE_TTL",
            category: "secrets",
            default: "300",
            description: "Secret cache TTL (secs)",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_ADDR",
            category: "secrets",
            default: "",
            description: "Vault server address",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_TOKEN",
            category: "secrets",
            default: "",
            description: "Vault token",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_ROLE_ID",
            category: "secrets",
            default: "",
            description: "Vault AppRole role ID",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_SECRET_ID",
            category: "secrets",
            default: "",
            description: "Vault AppRole secret ID",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_NAMESPACE",
            category: "secrets",
            default: "",
            description: "Vault namespace (Enterprise)",
            required: false,
        },
        EnvVarDef {
            name: "VAULT_SKIP_VERIFY",
            category: "secrets",
            default: "false",
            description: "Skip Vault TLS verification",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_AWS_SECRETS_REGION",
            category: "secrets",
            default: "us-east-1",
            description: "AWS Secrets Manager region",
            required: false,
        },
        EnvVarDef {
            name: "AZURE_KEY_VAULT_URL",
            category: "secrets",
            default: "",
            description: "Azure Key Vault URL",
            required: false,
        },
        EnvVarDef {
            name: "GCP_SECRET_PROJECT",
            category: "secrets",
            default: "",
            description: "GCP Secret Manager project",
            required: false,
        },
        EnvVarDef {
            name: "MOCKFORGE_MASTER_KEY",
            category: "secrets",
            default: "",
            description: "Master key for encrypted secrets",
            required: false,
        },
    ]
}

/// Handle config list-env-vars command
pub(crate) async fn handle_config_list_env_vars(
    category: Option<&str>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let all_vars = get_env_var_definitions();

    let vars: Vec<_> = if let Some(cat) = category {
        all_vars.into_iter().filter(|v| v.category == cat).collect()
    } else {
        all_vars
    };

    match format {
        "markdown" => {
            println!("# MockForge Environment Variables\n");
            let mut current_category = "";
            for var in &vars {
                if var.category != current_category {
                    current_category = var.category;
                    println!("\n## {}\n", capitalize_first(current_category));
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
        "json" => {
            let json_vars: Vec<serde_json::Value> = vars
                .iter()
                .map(|v| {
                    json!({
                        "name": v.name,
                        "category": v.category,
                        "default": v.default,
                        "description": v.description,
                        "required": v.required
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_vars)?);
        }
        _ => {
            // Table format
            println!("{:<40} {:<12} {:<15} Description", "Variable", "Category", "Default");
            println!("{}", "-".repeat(100));
            for var in &vars {
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
            println!("Categories: server, protocols, database, security, ai, traffic, files, logging, performance, plugins, hot_reload, secrets");
        }
    }

    Ok(())
}

pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Handle config show command
pub(crate) async fn handle_config_show(
    config_path: Option<PathBuf>,
    format: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_file = if let Some(path) = config_path {
        path
    } else {
        discover_config_file()?
    };

    let content = tokio::fs::read_to_string(&config_file).await?;
    let parsed: serde_json::Value = if config_file.extension().is_some_and(|e| e == "json") {
        serde_json::from_str(&content)?
    } else {
        serde_yaml::from_str(&content)?
    };

    let output = match format {
        "json" => serde_json::to_string_pretty(&parsed)?,
        _ => serde_yaml::to_string(&parsed)?,
    };

    println!("{}", output);
    Ok(())
}
