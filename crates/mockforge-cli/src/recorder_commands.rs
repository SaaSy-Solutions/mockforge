//! Recorder commands for stub mapping conversion

use clap::Subcommand;
use mockforge_recorder::{RecorderDatabase, StubFormat, StubMappingConverter};
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
            filter.protocol = Some(proto.clone());
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

        for request in query_result.requests {
            let exchange = db
                .get_exchange(&request.id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Exchange {} not found", request.id))?;

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
                    eprintln!("⚠️  Failed to convert {}: {}", request.id, e);
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
