use clap::Subcommand;
use mockforge_data::rag::{EmbeddingProvider, LlmProvider, RagConfig};
use serde_json::json;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum DataCommands {
    /// Generate data from built-in templates
    ///
    /// Examples:
    ///   mockforge data template user --rows 100 --format json
    ///   mockforge data template product --rows 50 --output products.csv --format csv
    ///   mockforge data template order --rows 20 --rag --rag-provider openai --output orders.json
    #[command(verbatim_doc_comment)]
    Template {
        /// Template name (user, product, order)
        template: String,

        /// Number of rows to generate
        #[arg(short, long, default_value = "10")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable RAG mode for enhanced generation
        #[arg(long)]
        rag: bool,

        /// RAG LLM provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// RAG API endpoint
        #[arg(long)]
        rag_endpoint: Option<String>,

        /// RAG request timeout in seconds
        #[arg(long)]
        rag_timeout: Option<u64>,

        /// Maximum number of RAG API retries
        #[arg(long)]
        rag_max_retries: Option<usize>,
    },

    /// Generate data from JSON schema
    ///
    /// Example:
    ///   mockforge data schema my_schema.json --rows 100 --format jsonl --output data.jsonl
    #[command(verbatim_doc_comment)]
    Schema {
        /// JSON schema file path
        schema: PathBuf,

        /// Number of rows to generate
        #[arg(short, long, default_value = "10")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate mock data from OpenAPI specification
    ///
    /// Examples:
    ///   mockforge data mock-openapi api-spec.json --rows 50 --format json
    ///   mockforge data mock-openapi api-spec.yaml --realistic --output mock-data.json
    ///   mockforge data mock-openapi api-spec.json --validate --include-optional
    #[command(verbatim_doc_comment)]
    MockOpenapi {
        /// OpenAPI specification file path (JSON or YAML)
        spec: PathBuf,

        /// Number of rows to generate per schema
        #[arg(short, long, default_value = "5")]
        rows: usize,

        /// Output format (json, csv, jsonl)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable realistic data generation
        #[arg(long)]
        realistic: bool,

        /// Include optional fields in generated data
        #[arg(long)]
        include_optional: bool,

        /// Validate generated data against schemas
        #[arg(long)]
        validate: bool,

        /// Default array size for generated arrays
        #[arg(long, default_value = "3")]
        array_size: usize,

        /// Maximum array size for generated arrays
        #[arg(long, default_value = "10")]
        max_array_size: usize,
    },

    /// Start a mock server based on OpenAPI specification
    ///
    /// Examples:
    ///   mockforge data mock-server api-spec.json --port 8080
    ///   mockforge data mock-server api-spec.yaml --host 0.0.0.0 --port 3000 --cors
    ///   mockforge data mock-server api-spec.json --delay /api/users 100 --log-requests
    #[command(verbatim_doc_comment)]
    MockServer {
        /// OpenAPI specification file path (JSON or YAML)
        spec: PathBuf,

        /// Port to run the mock server on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS headers
        #[arg(long)]
        cors: bool,

        /// Log all incoming requests
        #[arg(long)]
        log_requests: bool,

        /// Response delay for specific endpoints (format: endpoint:delay_ms)
        #[arg(long)]
        delay: Vec<String>,

        /// Enable realistic data generation
        #[arg(long)]
        realistic: bool,

        /// Include optional fields in generated data
        #[arg(long)]
        include_optional: bool,

        /// Validate generated data against schemas
        #[arg(long)]
        validate: bool,
    },
}

pub(crate) async fn handle_data(
    data_command: DataCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match data_command {
        DataCommands::Template {
            template,
            rows,
            format,
            output,
            rag,
            rag_provider,
            rag_model,
            rag_endpoint,
            rag_timeout,
            rag_max_retries,
        } => {
            println!("🎯 Generating {} rows using '{}' template", rows, template);
            println!("📄 Output format: {}", format);
            if rag {
                println!("🧠 RAG mode enabled");
                if let Some(provider) = &rag_provider {
                    println!("🤖 RAG Provider: {}", provider);
                }
                if let Some(model) = &rag_model {
                    println!("🧠 RAG Model: {}", model);
                }
            }
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate data using the specified template
            let result = generate_from_template(
                &template,
                rows,
                rag,
                rag_provider,
                rag_model,
                rag_endpoint,
                rag_timeout,
                rag_max_retries,
            )
            .await?;

            // Format and output the result
            output_result(result, format, output).await?;
        }
        DataCommands::Schema {
            schema,
            rows,
            format,
            output,
        } => {
            println!("📋 Generating {} rows from schema: {}", rows, schema.display());
            println!("📄 Output format: {}", format);
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate data from JSON schema
            let result = generate_from_json_schema_file(&schema, rows).await?;

            // Format and output the result
            output_result(result, format, output).await?;
        }
        DataCommands::MockOpenapi {
            spec,
            rows,
            format,
            output,
            realistic,
            include_optional,
            validate,
            array_size,
            max_array_size,
        } => {
            println!("🚀 Generating mock data from OpenAPI spec: {}", spec.display());
            println!("📊 Rows per schema: {}", rows);
            println!("📄 Output format: {}", format);
            if realistic {
                println!("🎭 Realistic data generation enabled");
            }
            if include_optional {
                println!("📝 Including optional fields");
            }
            if validate {
                println!("✅ Schema validation enabled");
            }
            println!("📏 Array size: {} (max: {})", array_size, max_array_size);
            if let Some(output_path) = &output {
                println!("💾 Output file: {}", output_path.display());
            }

            // Generate mock data from OpenAPI spec
            let result = generate_mock_data_from_openapi(
                &spec,
                rows,
                realistic,
                include_optional,
                validate,
                array_size,
                max_array_size,
            )
            .await?;

            // Format and output the result
            output_mock_data_result(result, format, output).await?;
        }
        DataCommands::MockServer {
            spec,
            port,
            host,
            cors,
            log_requests,
            delay,
            realistic,
            include_optional,
            validate,
        } => {
            println!("🌐 Starting mock server based on OpenAPI spec: {}", spec.display());
            println!("🔗 Server will run on {}:{}", host, port);
            if cors {
                println!("🌍 CORS enabled");
            }
            if log_requests {
                println!("📝 Request logging enabled");
            }
            if !delay.is_empty() {
                println!("⏱️ Response delays configured: {:?}", delay);
            }
            if realistic {
                println!("🎭 Realistic data generation enabled");
            }
            if include_optional {
                println!("📝 Including optional fields");
            }
            if validate {
                println!("✅ Schema validation enabled");
            }

            // Start the mock server
            start_mock_server_from_spec(
                &spec,
                port,
                &host,
                cors,
                log_requests,
                delay,
                realistic,
                include_optional,
                validate,
            )
            .await?;
        }
    }

    Ok(())
}

/// Load RAG configuration from environment variables and CLI options
pub(crate) fn load_rag_config(
    provider_override: Option<String>,
    model_override: Option<String>,
    endpoint_override: Option<String>,
    timeout_override: Option<u64>,
    max_retries_override: Option<usize>,
) -> RagConfig {
    let provider = provider_override
        .or_else(|| std::env::var("MOCKFORGE_RAG_PROVIDER").ok())
        .unwrap_or_else(|| "openai".to_string());

    let llm_provider = match provider.to_lowercase().as_str() {
        "anthropic" => LlmProvider::Anthropic,
        "ollama" => LlmProvider::Ollama,
        "openai_compatible" => LlmProvider::OpenAICompatible,
        _ => LlmProvider::OpenAI,
    };

    let embedding_provider = match std::env::var("MOCKFORGE_EMBEDDING_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase()
        .as_str()
    {
        "openai_compatible" => EmbeddingProvider::OpenAICompatible,
        _ => EmbeddingProvider::OpenAI,
    };

    RagConfig {
        provider: llm_provider.clone(),
        api_endpoint: endpoint_override
            .or_else(|| std::env::var("MOCKFORGE_RAG_API_ENDPOINT").ok())
            .unwrap_or_else(|| match llm_provider {
                LlmProvider::OpenAI => "https://api.openai.com/v1/chat/completions".to_string(),
                LlmProvider::Anthropic => "https://api.anthropic.com/v1/messages".to_string(),
                LlmProvider::Ollama => "http://localhost:11434/api/generate".to_string(),
                LlmProvider::OpenAICompatible => {
                    "http://localhost:8000/v1/chat/completions".to_string()
                }
            }),
        api_key: std::env::var("MOCKFORGE_RAG_API_KEY").ok(),
        model: model_override
            .or_else(|| std::env::var("MOCKFORGE_RAG_MODEL").ok())
            .unwrap_or_else(|| match llm_provider {
                LlmProvider::OpenAI => "gpt-3.5-turbo".to_string(),
                LlmProvider::Anthropic => "claude-3-sonnet-20240229".to_string(),
                LlmProvider::Ollama => "llama2".to_string(),
                LlmProvider::OpenAICompatible => "gpt-3.5-turbo".to_string(),
            }),
        max_tokens: std::env::var("MOCKFORGE_RAG_MAX_TOKENS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000),
        temperature: std::env::var("MOCKFORGE_RAG_TEMPERATURE")
            .unwrap_or_else(|_| "0.7".to_string())
            .parse()
            .unwrap_or(0.7),
        context_window: std::env::var("MOCKFORGE_RAG_CONTEXT_WINDOW")
            .unwrap_or_else(|_| "4000".to_string())
            .parse()
            .unwrap_or(4000),
        semantic_search_enabled: std::env::var("MOCKFORGE_SEMANTIC_SEARCH")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
        embedding_provider,
        embedding_model: std::env::var("MOCKFORGE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string()),
        embedding_endpoint: std::env::var("MOCKFORGE_EMBEDDING_ENDPOINT").ok(),
        similarity_threshold: std::env::var("MOCKFORGE_SIMILARITY_THRESHOLD")
            .unwrap_or_else(|_| "0.7".to_string())
            .parse()
            .unwrap_or(0.7),
        max_chunks: std::env::var("MOCKFORGE_MAX_CHUNKS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5),
        request_timeout_seconds: timeout_override
            .or_else(|| {
                std::env::var("MOCKFORGE_RAG_TIMEOUT_SECONDS").ok().and_then(|s| s.parse().ok())
            })
            .unwrap_or(30),
        max_retries: max_retries_override
            .or_else(|| {
                std::env::var("MOCKFORGE_RAG_MAX_RETRIES").ok().and_then(|s| s.parse().ok())
            })
            .unwrap_or(3),
    }
}

/// Generate data from a predefined template
#[allow(clippy::too_many_arguments)]
async fn generate_from_template(
    template: &str,
    _rows: usize,
    rag_enabled: bool,
    rag_provider: Option<String>,
    rag_model: Option<String>,
    rag_endpoint: Option<String>,
    rag_timeout: Option<u64>,
    rag_max_retries: Option<usize>,
) -> Result<mockforge_data::GenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_data::schema::templates;

    let config = mockforge_data::DataConfig {
        rows: _rows,
        rag_enabled,
        ..Default::default()
    };

    let schema = match template.to_lowercase().as_str() {
        "user" | "users" => templates::user_schema(),
        "product" | "products" => templates::product_schema(),
        "order" | "orders" => templates::order_schema(),
        _ => {
            return Err(format!(
                "Unknown template: {}. Available templates: user, product, order",
                template
            )
            .into());
        }
    };

    let mut generator = mockforge_data::DataGenerator::new(schema, config)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Configure RAG if enabled
    if rag_enabled {
        let rag_config = load_rag_config(
            rag_provider.clone(),
            rag_model.clone(),
            rag_endpoint.clone(),
            rag_timeout,
            rag_max_retries,
        );
        generator
            .configure_rag(rag_config)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    }

    generator
        .generate()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Generate data from a JSON schema file
async fn generate_from_json_schema_file(
    schema_path: &PathBuf,
    rows: usize,
) -> Result<mockforge_data::GenerationResult, Box<dyn std::error::Error + Send + Sync>> {
    // Read the JSON schema file
    let schema_content = tokio::fs::read_to_string(schema_path).await?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)?;

    // Generate data from the schema
    mockforge_data::generate_from_json_schema(&schema_json, rows)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Output the generation result in the specified format
async fn output_result(
    result: mockforge_data::GenerationResult,
    format: String,
    output_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output_content = match format.to_lowercase().as_str() {
        "json" => result.to_json_string()?,
        "jsonl" | "jsonlines" => result.to_jsonl_string()?,
        "csv" => {
            // For CSV, we'll need to convert JSON to CSV format
            // This is a simplified implementation - in a real system you'd use a proper CSV library
            let mut csv_output = String::new();

            if let Some(first_row) = result.data.first() {
                if let Some(obj) = first_row.as_object() {
                    // Add header row
                    let headers: Vec<String> = obj.keys().map(|k| k.to_string()).collect();
                    csv_output.push_str(&headers.join(","));
                    csv_output.push('\n');

                    // Add data rows
                    for row in &result.data {
                        if let Some(obj) = row.as_object() {
                            let values: Vec<String> = headers
                                .iter()
                                .map(|header| {
                                    obj.get(header)
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                })
                                .collect();
                            csv_output.push_str(&values.join(","));
                            csv_output.push('\n');
                        }
                    }
                }
            }
            csv_output
        }
        _ => result.to_json_string()?, // Default to JSON
    };

    // Output to file or stdout
    if let Some(path) = output_path {
        tokio::fs::write(&path, &output_content).await?;
        println!("💾 Data written to: {}", path.display());
    } else {
        println!("{}", output_content);
    }

    println!("✅ Generated {} rows in {}ms", result.count, result.generation_time_ms);

    if !result.warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Generate mock data from OpenAPI specification
async fn generate_mock_data_from_openapi(
    spec_path: &PathBuf,
    _rows: usize,
    realistic: bool,
    include_optional: bool,
    validate: bool,
    array_size: usize,
    max_array_size: usize,
) -> Result<mockforge_data::MockDataResult, Box<dyn std::error::Error + Send + Sync>> {
    // Read the OpenAPI specification file
    let spec_content = tokio::fs::read_to_string(spec_path).await?;

    // Parse JSON or YAML
    let spec_json: serde_json::Value = if spec_path.extension().and_then(|s| s.to_str())
        == Some("yaml")
        || spec_path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&spec_content)?
    } else {
        serde_json::from_str(&spec_content)?
    };

    // Create generator configuration
    let config = mockforge_data::MockGeneratorConfig::new()
        .realistic_mode(realistic)
        .include_optional_fields(include_optional)
        .validate_generated_data(validate)
        .default_array_size(array_size)
        .max_array_size(max_array_size);

    // Generate mock data
    let mut generator = mockforge_data::MockDataGenerator::with_config(config);
    generator
        .generate_from_openapi_spec(&spec_json)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

/// Output mock data result in the specified format
async fn output_mock_data_result(
    result: mockforge_data::MockDataResult,
    format: String,
    output_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let output_content = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&result)?,
        "jsonl" | "jsonlines" => {
            // Convert to JSONL format
            let mut jsonl_output = String::new();

            // Add schemas
            for (schema_name, schema_data) in &result.schemas {
                let schema_line = json!({
                    "type": "schema",
                    "name": schema_name,
                    "data": schema_data
                });
                jsonl_output.push_str(&serde_json::to_string(&schema_line)?);
                jsonl_output.push('\n');
            }

            // Add responses
            for (endpoint, response) in &result.responses {
                let response_line = json!({
                    "type": "response",
                    "endpoint": endpoint,
                    "status": response.status,
                    "headers": response.headers,
                    "body": response.body
                });
                jsonl_output.push_str(&serde_json::to_string(&response_line)?);
                jsonl_output.push('\n');
            }

            jsonl_output
        }
        "csv" => {
            // For CSV, we'll create a simplified format
            let mut csv_output = String::new();
            csv_output.push_str("type,name,endpoint,status,data\n");

            // Add schemas
            for (schema_name, schema_data) in &result.schemas {
                csv_output.push_str(&format!(
                    "schema,{},\"\",\"\",{}\n",
                    schema_name,
                    serde_json::to_string(schema_data)?.replace("\"", "\"\"")
                ));
            }

            // Add responses
            for (endpoint, response) in &result.responses {
                csv_output.push_str(&format!(
                    "response,\"\",{},{},{}\n",
                    endpoint.replace("\"", "\"\""),
                    response.status,
                    serde_json::to_string(&response.body)?.replace("\"", "\"\"")
                ));
            }

            csv_output
        }
        _ => serde_json::to_string_pretty(&result)?, // Default to JSON
    };

    // Output to file or stdout
    if let Some(path) = output_path {
        tokio::fs::write(&path, &output_content).await?;
        println!("💾 Mock data written to: {}", path.display());
    } else {
        println!("{}", output_content);
    }

    println!(
        "✅ Generated mock data for {} schemas and {} endpoints",
        result.schemas.len(),
        result.responses.len()
    );

    if !result.warnings.is_empty() {
        println!("⚠️  Warnings:");
        for warning in result.warnings {
            println!("   - {}", warning);
        }
    }

    Ok(())
}

/// Start mock server from OpenAPI specification
#[allow(clippy::too_many_arguments)]
async fn start_mock_server_from_spec(
    spec_path: &PathBuf,
    port: u16,
    host: &str,
    cors: bool,
    log_requests: bool,
    delays: Vec<String>,
    realistic: bool,
    include_optional: bool,
    validate: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read the OpenAPI specification file
    let spec_content = tokio::fs::read_to_string(spec_path).await?;

    // Parse JSON or YAML
    let spec_json: serde_json::Value = if spec_path.extension().and_then(|s| s.to_str())
        == Some("yaml")
        || spec_path.extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&spec_content)?
    } else {
        serde_json::from_str(&spec_content)?
    };

    // Create server configuration
    let mut config = mockforge_data::MockServerConfig::new(spec_json)
        .port(port)
        .host(host.to_string())
        .enable_cors(cors)
        .log_requests(log_requests)
        .generator_config(
            mockforge_data::MockGeneratorConfig::new()
                .realistic_mode(realistic)
                .include_optional_fields(include_optional)
                .validate_generated_data(validate),
        );

    // Add response delays
    for delay_spec in delays {
        if let Some((endpoint, delay_ms)) = delay_spec.split_once(':') {
            if let Ok(delay) = delay_ms.parse::<u64>() {
                config = config.response_delay(endpoint.to_string(), delay);
            }
        }
    }

    // Start the mock server
    println!("🚀 Starting mock server...");
    println!("📡 Server will be available at: http://{}:{}", host, port);
    println!("📋 OpenAPI spec: {}", spec_path.display());
    println!("🛑 Press Ctrl+C to stop the server");

    mockforge_data::start_mock_server_with_config(config)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}
