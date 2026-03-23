//! AI test and test generation commands
//!
//! CLI commands for AI-powered testing features.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum AiTestCommands {
    /// Test intelligent mock generation
    ///
    /// Example:
    ///   mockforge test-ai intelligent-mock --prompt "Generate a REST API for a blog" --output mock.json
    #[command(verbatim_doc_comment)]
    IntelligentMock {
        /// Natural language prompt for generation
        #[arg(short, long)]
        prompt: String,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Test data drift simulation
    Drift {
        /// Initial data file (JSON)
        #[arg(short, long)]
        initial_data: PathBuf,

        /// Number of drift iterations to simulate
        #[arg(short = 'n', long, default_value = "5")]
        iterations: usize,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Test AI event stream generation
    ///
    /// Example:
    ///   mockforge test-ai event-stream --narrative "User login flow" --event-count 10 --output events.json
    #[command(verbatim_doc_comment)]
    EventStream {
        /// Narrative description for event generation
        #[arg(short, long)]
        narrative: String,

        /// Number of events to generate
        #[arg(short = 'c', long, default_value = "10")]
        event_count: usize,

        /// AI/RAG provider (openai, anthropic, ollama, openai_compatible)
        #[arg(long)]
        rag_provider: Option<String>,

        /// AI/RAG model name
        #[arg(long)]
        rag_model: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Handle AI testing commands
pub(crate) async fn handle_test_ai(
    ai_command: AiTestCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match ai_command {
        AiTestCommands::IntelligentMock {
            prompt,
            rag_provider,
            rag_model,
            output,
        } => {
            println!("\u{1f9e0} Testing Intelligent Mock Generation");
            println!("\u{1f4dd} Prompt: {}", prompt);

            // Load RAG configuration
            let rag_config =
                crate::data_commands::load_rag_config(rag_provider, rag_model, None, None, None);

            // Create intelligent mock generator
            use mockforge_data::{IntelligentMockConfig, IntelligentMockGenerator, ResponseMode};

            let config = IntelligentMockConfig::new(ResponseMode::Intelligent)
                .with_prompt(prompt)
                .with_rag_config(rag_config);

            let mut generator = IntelligentMockGenerator::new(config)?;

            // Generate mock data
            println!("\u{1f3af} Generating mock data...");
            let result = generator.generate().await?;

            // Output result
            let output_str = serde_json::to_string_pretty(&result)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("\u{1f4be} Output written to: {}", path.display());
            } else {
                println!("\n\u{1f4c4} Generated Mock Data:");
                println!("{}", output_str);
            }

            println!("\u{2705} Intelligent mock generation completed successfully!");
        }

        AiTestCommands::Drift {
            initial_data,
            iterations,
            output,
        } => {
            println!("\u{1f4ca} Testing Data Drift Simulation");
            println!("\u{1f4c1} Initial data: {}", initial_data.display());
            println!("\u{1f504} Iterations: {}", iterations);

            // Read initial data
            let data_content = tokio::fs::read_to_string(&initial_data).await?;
            let mut current_data: serde_json::Value = serde_json::from_str(&data_content)?;

            // Create a simple drift configuration
            use mockforge_data::drift::{DriftRule, DriftStrategy};
            use mockforge_data::DataDriftConfig;

            let rule = DriftRule::new("value".to_string(), DriftStrategy::Linear).with_rate(1.0);
            let drift_config = DataDriftConfig::new().with_rule(rule);

            let engine = mockforge_data::DataDriftEngine::new(drift_config)?;

            // Simulate drift iterations
            println!("\n\u{1f3af} Simulating drift:");
            let mut results = vec![current_data.clone()];

            for i in 1..=iterations {
                current_data = engine.apply_drift(current_data).await?;
                results.push(current_data.clone());
                println!("   Iteration {}: {:?}", i, current_data);
            }

            // Output results
            let output_str = serde_json::to_string_pretty(&results)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("\n\u{1f4be} Output written to: {}", path.display());
            } else {
                println!("\n\u{1f4c4} Final Drifted Data:");
                println!("{}", serde_json::to_string_pretty(&current_data)?);
            }

            println!("\u{2705} Data drift simulation completed successfully!");
        }

        AiTestCommands::EventStream {
            narrative,
            event_count,
            rag_provider,
            rag_model,
            output,
        } => {
            println!("\u{1f30a} Testing AI Event Stream Generation");
            println!("\u{1f4d6} Narrative: {}", narrative);
            println!("\u{1f522} Event count: {}", event_count);

            // Load RAG configuration
            let rag_config =
                crate::data_commands::load_rag_config(rag_provider, rag_model, None, None, None);

            // Create replay augmentation config
            use mockforge_data::{EventStrategy, ReplayAugmentationConfig, ReplayMode};

            let config = ReplayAugmentationConfig {
                mode: ReplayMode::Generated,
                strategy: EventStrategy::CountBased,
                narrative: Some(narrative),
                event_count: Some(event_count),
                rag_config: Some(rag_config),
                ..Default::default()
            };

            let mut engine = mockforge_data::ReplayAugmentationEngine::new(config)?;

            // Generate event stream
            println!("\u{1f3af} Generating event stream...");
            let events = engine.generate_stream().await?;

            // Output results
            let output_str = serde_json::to_string_pretty(&events)?;
            if let Some(path) = output {
                tokio::fs::write(&path, &output_str).await?;
                println!("\u{1f4be} Output written to: {}", path.display());
            } else {
                println!("\n\u{1f4c4} Generated Events:");
                for (i, event) in events.iter().enumerate() {
                    println!("\nEvent {}:", i + 1);
                    println!("  Type: {}", event.event_type);
                    println!("  Timestamp: {}", event.timestamp);
                    println!("  Data: {}", serde_json::to_string_pretty(&event.data)?);
                }
            }

            println!("\n\u{2705} Event stream generation completed successfully!");
            println!("   Generated {} events", events.len());
        }
    }

    Ok(())
}

/// Handle test generation from recorded API interactions
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_generate_tests(
    database: PathBuf,
    format: String,
    output: Option<PathBuf>,
    protocol: Option<String>,
    method: Option<String>,
    path: Option<String>,
    status_code: Option<u16>,
    limit: usize,
    suite_name: String,
    base_url: String,
    ai_descriptions: bool,
    llm_provider: String,
    llm_model: String,
    llm_endpoint: Option<String>,
    llm_api_key: Option<String>,
    validate_body: bool,
    validate_status: bool,
    validate_headers: bool,
    validate_timing: bool,
    max_duration_ms: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use mockforge_recorder::{
        LlmConfig, Protocol, QueryFilter, RecorderDatabase, TestFormat, TestGenerationConfig,
        TestGenerator,
    };

    println!("\u{1f9ea} Generating tests from recorded API interactions");
    println!("\u{1f4c1} Database: {}", database.display());
    println!("\u{1f4dd} Format: {}", format);
    println!("\u{1f3af} Suite name: {}", suite_name);

    // Open database with proper error handling for path conversion
    use crate::progress::{CliError, ExitCode};
    let db_path = database.to_str().ok_or_else(|| {
        CliError::new(
            format!("Invalid database path: {}", database.display()),
            ExitCode::FileNotFound,
        )
        .with_suggestion(
            "Ensure the database path contains only valid UTF-8 characters".to_string(),
        )
    })?;
    let db = RecorderDatabase::new(db_path).await?;
    println!("\u{2705} Database opened successfully");

    // Parse test format
    let test_format = match format.as_str() {
        "rust_reqwest" => TestFormat::RustReqwest,
        "http_file" => TestFormat::HttpFile,
        "curl" => TestFormat::Curl,
        "postman" => TestFormat::Postman,
        "k6" => TestFormat::K6,
        "python_pytest" => TestFormat::PythonPytest,
        "javascript_jest" => TestFormat::JavaScriptJest,
        "go_test" => TestFormat::GoTest,
        _ => {
            eprintln!("\u{274c} Invalid format: {}. Supported formats: rust_reqwest, http_file, curl, postman, k6, python_pytest, javascript_jest, go_test", format);
            return Err("Invalid format".into());
        }
    };

    // Parse protocol filter
    let protocol_filter = protocol.as_ref().and_then(|p| match p.to_lowercase().as_str() {
        "http" => Some(Protocol::Http),
        "grpc" => Some(Protocol::Grpc),
        "websocket" => Some(Protocol::WebSocket),
        "graphql" => Some(Protocol::GraphQL),
        _ => None,
    });

    // Create LLM config if AI descriptions enabled
    let llm_config = if ai_descriptions {
        let endpoint = llm_endpoint.unwrap_or_else(|| {
            if llm_provider == "ollama" {
                "http://localhost:11434/api/generate".to_string()
            } else {
                "https://api.openai.com/v1/chat/completions".to_string()
            }
        });

        Some(LlmConfig {
            provider: llm_provider.clone(),
            api_endpoint: endpoint,
            api_key: llm_api_key,
            model: llm_model.clone(),
            temperature: 0.3,
        })
    } else {
        None
    };

    // Create test generation config
    let config = TestGenerationConfig {
        format: test_format,
        include_assertions: true,
        validate_body,
        validate_status,
        validate_headers,
        validate_timing,
        max_duration_ms,
        suite_name: suite_name.clone(),
        base_url: Some(base_url.clone()),
        ai_descriptions,
        llm_config,
        group_by_endpoint: true,
        include_setup_teardown: true,
        generate_fixtures: ai_descriptions,
        suggest_edge_cases: ai_descriptions,
        analyze_test_gaps: ai_descriptions,
        deduplicate_tests: true,
        optimize_test_order: false,
    };

    // Create query filter
    let filter = QueryFilter {
        protocol: protocol_filter,
        method: method.clone(),
        path: path.clone(),
        status_code: status_code.map(|c| c as i32),
        trace_id: None,
        min_duration_ms: None,
        max_duration_ms: None,
        tags: None,
        limit: Some(limit as i32),
        offset: Some(0),
    };

    println!("\u{1f50d} Searching for recordings...");
    if let Some(p) = &protocol {
        println!("   Protocol: {}", p);
    }
    if let Some(m) = &method {
        println!("   Method: {}", m);
    }
    if let Some(p) = &path {
        println!("   Path: {}", p);
    }
    if let Some(s) = status_code {
        println!("   Status code: {}", s);
    }
    println!("   Limit: {}", limit);

    // Generate tests
    let generator = TestGenerator::new(db, config);
    println!("\n\u{1f3a8} Generating tests...");

    if ai_descriptions {
        println!("\u{1f916} Using {} ({}) for AI descriptions", llm_provider, llm_model);
    }

    let result = generator.generate_from_filter(filter).await?;

    println!("\n\u{2705} Test generation completed successfully!");
    println!("   Generated {} tests", result.metadata.test_count);
    println!("   Covering {} endpoints", result.metadata.endpoint_count);
    println!("   Protocols: {:?}", result.metadata.protocols);

    // Output test file
    if let Some(output_path) = output {
        tokio::fs::write(&output_path, &result.test_file).await?;
        println!("\n\u{1f4be} Tests written to: {}", output_path.display());
    } else {
        println!("\n\u{1f4c4} Generated Test File:");
        println!("{}", "=".repeat(60));
        println!("{}", result.test_file);
        println!("{}", "=".repeat(60));
    }

    // Print summary of generated tests
    println!("\n\u{1f4ca} Test Summary:");
    for (i, test) in result.tests.iter().enumerate() {
        println!("   {}. {} - {} {}", i + 1, test.name, test.method, test.endpoint);
        if ai_descriptions
            && !test.description.is_empty()
            && test.description != format!("Test {} {}", test.method, test.endpoint)
        {
            println!("      Description: {}", test.description);
        }
    }

    println!("\n\u{1f389} Done! You can now run the generated tests.");

    Ok(())
}
