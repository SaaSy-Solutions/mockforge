//! Recorder commands for stub mapping conversion

use clap::Subcommand;
use mockforge_recorder::{models::Protocol, RecorderDatabase, StubFormat, StubMappingConverter};
use std::path::PathBuf;
use tracing::info;

#[derive(Subcommand)]
pub enum RecorderCommands {
    /// Convert recorded requests to stub mappings (fixtures)
    ///
    /// Examples:
    ///   mockforge recorder convert --recording-id abc123 --output fixtures/user-api.yaml
    ///   mockforge recorder convert --input recordings.db --output fixtures/ --format yaml
    Convert {
        /// Recording ID to convert (single conversion)
        #[arg(long)]
        recording_id: Option<String>,

        /// Input database file path (for batch conversion)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output file or directory path
        #[arg(short, long)]
        output: PathBuf,

        /// Output format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,

        /// Detect and replace dynamic values (UUIDs, timestamps) with template variables
        #[arg(long, default_value = "true")]
        detect_dynamic_values: bool,

        /// Deduplicate similar recordings (batch mode only)
        #[arg(long)]
        deduplicate: bool,

        /// Filter by protocol (http, grpc, websocket, graphql)
        #[arg(long)]
        protocol: Option<String>,

        /// Filter by HTTP method
        #[arg(long)]
        method: Option<String>,

        /// Filter by path pattern
        #[arg(long)]
        path: Option<String>,

        /// Limit number of recordings to convert (batch mode)
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
}

pub async fn handle_recorder_command(command: RecorderCommands) -> anyhow::Result<()> {
    match command {
        RecorderCommands::Convert {
            recording_id,
            input,
            output,
            format,
            detect_dynamic_values,
            deduplicate,
            protocol,
            method,
            path,
            limit,
        } => {
            handle_convert(
                recording_id,
                input,
                output,
                format,
                detect_dynamic_values,
                deduplicate,
                protocol,
                method,
                path,
                limit,
            )
            .await
        }
    }
}

async fn handle_convert(
    recording_id: Option<String>,
    input: Option<PathBuf>,
    output: PathBuf,
    format: String,
    detect_dynamic_values: bool,
    deduplicate: bool,
    protocol: Option<String>,
    method: Option<String>,
    path: Option<String>,
    limit: usize,
) -> anyhow::Result<()> {
    let stub_format = match format.to_lowercase().as_str() {
        "json" => StubFormat::Json,
        "yaml" | _ => StubFormat::Yaml,
    };

    let converter = StubMappingConverter::new(detect_dynamic_values);

    if let Some(id) = recording_id {
        // Single recording conversion
        println!("🔄 Converting recording {} to stub mapping...", id);

        // Default database path if not provided
        let db_path = input.unwrap_or_else(|| PathBuf::from("./mockforge-recordings.db"));
        let db = RecorderDatabase::new(&db_path).await?;

        let exchange = db
            .get_exchange(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Recording {} not found", id))?;

        let stub = converter.convert(&exchange)?;
        let content = converter.to_string(&stub, stub_format)?;

        // Write to file
        tokio::fs::write(&output, content).await?;
        println!("✅ Stub mapping written to: {}", output.display());
    } else {
        // Batch conversion
        let db_path = input
            .ok_or_else(|| anyhow::anyhow!("Either --recording-id or --input must be specified"))?;

        println!("🔄 Converting recordings from database: {}", db_path.display());
        println!("📁 Output directory: {}", output.display());

        let db = RecorderDatabase::new(&db_path).await?;

        // Query recordings with filters
        use mockforge_recorder::{query::execute_query, QueryFilter};
        let mut filter = QueryFilter {
            limit: Some(limit as i32),
            ..Default::default()
        };

        if let Some(ref proto) = protocol {
            // Parse protocol string to Protocol enum
            let protocol_enum = match proto.to_lowercase().as_str() {
                "http" => Protocol::Http,
                "grpc" => Protocol::Grpc,
                "websocket" => Protocol::WebSocket,
                "graphql" => Protocol::GraphQL,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Invalid protocol: {}. Must be one of: http, grpc, websocket, graphql",
                        proto
                    ));
                }
            };
            filter.protocol = Some(protocol_enum);
        }
        if let Some(ref m) = method {
            filter.method = Some(m.clone());
        }
        if let Some(ref p) = path {
            filter.path = Some(p.clone());
        }
        let query_result = execute_query(&db, filter).await?;

        println!("📊 Found {} recordings to convert", query_result.total);

        // Create output directory if it doesn't exist
        if output.is_dir() || !output.exists() {
            tokio::fs::create_dir_all(&output).await?;
        }

        let mut converted = 0;
        let mut errors = 0;
        let mut seen_identifiers = std::collections::HashSet::new();

        for exchange in query_result.exchanges {
            let exchange_id = exchange.request.id.clone();

            match converter.convert(&exchange) {
                Ok(stub) => {
                    // Check deduplication
                    if deduplicate && seen_identifiers.contains(&stub.identifier) {
                        continue;
                    }
                    seen_identifiers.insert(stub.identifier.clone());

                    let content = converter.to_string(&stub, stub_format)?;

                    // Generate filename from identifier
                    let extension = match stub_format {
                        StubFormat::Yaml => "yaml",
                        StubFormat::Json => "json",
                    };
                    let filename = format!("{}.{}", stub.identifier, extension);
                    let file_path = if output.is_dir() {
                        output.join(&filename)
                    } else {
                        output.clone()
                    };

                    tokio::fs::write(&file_path, content).await?;
                    converted += 1;

                    if converted % 10 == 0 {
                        info!("Converted {} recordings...", converted);
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to convert {}: {}", exchange_id, e);
                    errors += 1;
                }
            }
        }

        println!("\n✅ Conversion complete!");
        println!("   Converted: {}", converted);
        println!("   Errors: {}", errors);
        println!("   Output: {}", output.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_commands_convert_variant() {
        let cmd = RecorderCommands::Convert {
            recording_id: Some("abc123".to_string()),
            input: Some(PathBuf::from("input.db")),
            output: PathBuf::from("output/fixtures"),
            format: "yaml".to_string(),
            detect_dynamic_values: true,
            deduplicate: false,
            protocol: Some("http".to_string()),
            method: Some("GET".to_string()),
            path: Some("/api/users".to_string()),
            limit: 100,
        };

        // Verify the command can be constructed
        match cmd {
            RecorderCommands::Convert {
                recording_id,
                input,
                output,
                format,
                detect_dynamic_values,
                deduplicate,
                protocol,
                method,
                path,
                limit,
            } => {
                assert_eq!(recording_id, Some("abc123".to_string()));
                assert_eq!(input, Some(PathBuf::from("input.db")));
                assert_eq!(output, PathBuf::from("output/fixtures"));
                assert_eq!(format, "yaml");
                assert!(detect_dynamic_values);
                assert!(!deduplicate);
                assert_eq!(protocol, Some("http".to_string()));
                assert_eq!(method, Some("GET".to_string()));
                assert_eq!(path, Some("/api/users".to_string()));
                assert_eq!(limit, 100);
            }
        }
    }

    #[test]
    fn test_recorder_commands_convert_minimal() {
        let cmd = RecorderCommands::Convert {
            recording_id: None,
            input: None,
            output: PathBuf::from("output"),
            format: "json".to_string(),
            detect_dynamic_values: false,
            deduplicate: true,
            protocol: None,
            method: None,
            path: None,
            limit: 50,
        };

        match cmd {
            RecorderCommands::Convert {
                recording_id,
                input,
                format,
                detect_dynamic_values,
                deduplicate,
                protocol,
                method,
                path,
                limit,
                ..
            } => {
                assert!(recording_id.is_none());
                assert!(input.is_none());
                assert_eq!(format, "json");
                assert!(!detect_dynamic_values);
                assert!(deduplicate);
                assert!(protocol.is_none());
                assert!(method.is_none());
                assert!(path.is_none());
                assert_eq!(limit, 50);
            }
        }
    }

    #[test]
    fn test_format_parsing() {
        // Test format string conversion (mimics the logic in handle_convert)
        let formats = vec![
            ("json", "json"),
            ("JSON", "json"),
            ("Json", "json"),
            ("yaml", "yaml"),
            ("YAML", "yaml"),
            ("Yaml", "yaml"),
            ("other", "yaml"), // defaults to yaml
        ];

        for (input, expected) in formats {
            let result = match input.to_lowercase().as_str() {
                "json" => "json",
                "yaml" | _ => "yaml",
            };
            assert_eq!(result, expected, "Format '{}' should map to '{}'", input, expected);
        }
    }

    #[test]
    fn test_protocol_parsing() {
        // Test protocol string conversion (mimics the logic in handle_convert)
        let valid_protocols = vec![
            ("http", Protocol::Http),
            ("HTTP", Protocol::Http),
            ("grpc", Protocol::Grpc),
            ("GRPC", Protocol::Grpc),
            ("websocket", Protocol::WebSocket),
            ("WebSocket", Protocol::WebSocket),
            ("graphql", Protocol::GraphQL),
            ("GraphQL", Protocol::GraphQL),
        ];

        for (input, expected) in valid_protocols {
            let result = match input.to_lowercase().as_str() {
                "http" => Some(Protocol::Http),
                "grpc" => Some(Protocol::Grpc),
                "websocket" => Some(Protocol::WebSocket),
                "graphql" => Some(Protocol::GraphQL),
                _ => None,
            };
            assert_eq!(result, Some(expected), "Protocol '{}' should parse correctly", input);
        }
    }

    #[test]
    fn test_invalid_protocol_parsing() {
        let invalid_protocols = vec!["invalid", "tcp", "udp", "mqtt", "amqp"];

        for proto in invalid_protocols {
            let result = match proto.to_lowercase().as_str() {
                "http" => Some(Protocol::Http),
                "grpc" => Some(Protocol::Grpc),
                "websocket" => Some(Protocol::WebSocket),
                "graphql" => Some(Protocol::GraphQL),
                _ => None,
            };
            assert!(result.is_none(), "Protocol '{}' should be invalid", proto);
        }
    }

    #[test]
    fn test_output_path_handling() {
        // Test various output path scenarios
        let paths = vec![
            PathBuf::from("fixtures/"),
            PathBuf::from("./output"),
            PathBuf::from("/absolute/path"),
            PathBuf::from("relative/path/file.yaml"),
        ];

        for path in paths {
            // Just verify paths can be created
            let cmd = RecorderCommands::Convert {
                recording_id: None,
                input: None,
                output: path.clone(),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: None,
                path: None,
                limit: 100,
            };

            match cmd {
                RecorderCommands::Convert { output, .. } => {
                    assert_eq!(output, path);
                }
            }
        }
    }

    #[test]
    fn test_http_methods() {
        let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"];

        for method in methods {
            let cmd = RecorderCommands::Convert {
                recording_id: None,
                input: None,
                output: PathBuf::from("output"),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: Some(method.to_string()),
                path: None,
                limit: 100,
            };

            match cmd {
                RecorderCommands::Convert { method: m, .. } => {
                    assert_eq!(m, Some(method.to_string()));
                }
            }
        }
    }

    #[test]
    fn test_path_filter_patterns() {
        let path_patterns = vec![
            "/api/users",
            "/api/users/*",
            "/api/v1/items/**",
            "/health",
            "/api/search?q=*",
        ];

        for pattern in path_patterns {
            let cmd = RecorderCommands::Convert {
                recording_id: None,
                input: None,
                output: PathBuf::from("output"),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: None,
                path: Some(pattern.to_string()),
                limit: 100,
            };

            match cmd {
                RecorderCommands::Convert { path, .. } => {
                    assert_eq!(path, Some(pattern.to_string()));
                }
            }
        }
    }

    #[test]
    fn test_limit_values() {
        let limits = vec![1, 10, 100, 1000, 10000];

        for limit_val in limits {
            let cmd = RecorderCommands::Convert {
                recording_id: None,
                input: None,
                output: PathBuf::from("output"),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: None,
                path: None,
                limit: limit_val,
            };

            match cmd {
                RecorderCommands::Convert { limit, .. } => {
                    assert_eq!(limit, limit_val);
                }
            }
        }
    }

    #[test]
    fn test_recording_id_formats() {
        // Test various recording ID formats
        let ids = vec![
            "abc123".to_string(),
            "uuid-like-id-12345".to_string(),
            "123456".to_string(),
            "rec_001".to_string(),
            "a".repeat(100), // long ID
        ];

        for id in ids {
            let cmd = RecorderCommands::Convert {
                recording_id: Some(id.clone()),
                input: None,
                output: PathBuf::from("output.yaml"),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: None,
                path: None,
                limit: 100,
            };

            match cmd {
                RecorderCommands::Convert { recording_id, .. } => {
                    assert_eq!(recording_id, Some(id));
                }
            }
        }
    }

    #[test]
    fn test_input_database_paths() {
        let db_paths = vec![
            PathBuf::from("recordings.db"),
            PathBuf::from("./data/mockforge-recordings.db"),
            PathBuf::from("/var/lib/mockforge/recordings.db"),
        ];

        for db_path in db_paths {
            let cmd = RecorderCommands::Convert {
                recording_id: None,
                input: Some(db_path.clone()),
                output: PathBuf::from("output"),
                format: "yaml".to_string(),
                detect_dynamic_values: true,
                deduplicate: false,
                protocol: None,
                method: None,
                path: None,
                limit: 100,
            };

            match cmd {
                RecorderCommands::Convert { input, .. } => {
                    assert_eq!(input, Some(db_path));
                }
            }
        }
    }
}
