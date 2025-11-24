//! Voice + LLM Interface CLI commands
//!
//! This module provides CLI commands for voice-based mock creation using
//! natural language commands and LLM interpretation.

#[path = "speech_to_text.rs"]
mod speech_to_text;

use clap::Subcommand;
use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
use mockforge_core::multi_tenant::{MultiTenantConfig, MultiTenantWorkspaceRegistry};
use mockforge_core::openapi::OpenApiSpec;
use mockforge_core::voice::WorkspaceBuilder;
use mockforge_core::{ConversationManager, HookTranspiler, VoiceCommandParser, VoiceSpecGenerator};
use speech_to_text::{InteractiveVoiceInput, SpeechToTextManager};
use std::io::{self, Write};
use std::path::PathBuf;

/// Voice CLI commands
#[derive(Subcommand, Debug)]
pub enum VoiceCommands {
    /// Create a mock API from voice command (single-shot mode)
    ///
    /// Examples:
    ///   mockforge voice create --output api.yaml
    ///   mockforge voice create --serve --port 3000
    Create {
        /// Output file for generated OpenAPI spec
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Auto-start mock server with generated spec
        #[arg(long)]
        serve: bool,

        /// HTTP server port (used with --serve)
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Text command (if not provided, will prompt or use stdin)
        #[arg(short, long)]
        command: Option<String>,
    },

    /// Interactive conversational mode
    ///
    /// Examples:
    ///   mockforge voice interactive
    ///   mockforge voice interactive --output api.yaml
    Interactive {
        /// Output file for generated OpenAPI spec
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Auto-start mock server when done
        #[arg(long)]
        serve: bool,

        /// HTTP server port (used with --serve)
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Transpile a natural language hook description to hook configuration
    ///
    /// Examples:
    ///   mockforge voice transpile-hook --description "For VIP users, webhooks fire instantly"
    ///   mockforge voice transpile-hook --output hook.yaml
    TranspileHook {
        /// Natural language description of the hook logic
        #[arg(short, long)]
        description: Option<String>,

        /// Output file for generated hook configuration (YAML format)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: yaml or json (default: yaml)
        #[arg(long, default_value = "yaml")]
        format: String,
    },

    /// Create a complete workspace from natural language description
    ///
    /// Examples:
    ///   mockforge voice create-workspace --command "Create an e-commerce workspace with customers, orders, and payments"
    ///   mockforge voice create-workspace
    CreateWorkspace {
        /// Text command (if not provided, will prompt or use voice input)
        #[arg(short, long)]
        command: Option<String>,

        /// Skip confirmation prompt (auto-confirm)
        #[arg(long)]
        yes: bool,
    },
}

/// Handle voice CLI commands
pub async fn handle_voice_command(
    command: VoiceCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        VoiceCommands::Create {
            output,
            serve,
            port,
            command,
        } => {
            handle_create(output, serve, port, command).await?;
        }
        VoiceCommands::Interactive {
            output,
            serve,
            port,
        } => {
            handle_interactive(output, serve, port).await?;
        }
        VoiceCommands::TranspileHook {
            description,
            output,
            format,
        } => {
            handle_transpile_hook(description, output, format).await?;
        }
        VoiceCommands::CreateWorkspace { command, yes } => {
            handle_create_workspace(command, yes).await?;
        }
    }

    Ok(())
}

/// Handle create command (single-shot mode)
async fn handle_create(
    output: Option<PathBuf>,
    serve: bool,
    port: u16,
    command: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🎤 Voice + LLM Interface - Single Shot Mode");
    println!();

    // Get command text
    let command_text = if let Some(cmd) = command {
        cmd
    } else {
        // Use speech-to-text manager to get input
        let stt_manager = SpeechToTextManager::new();
        let available_backends = stt_manager.list_backends();

        if available_backends.len() > 1 {
            println!("🎤 Available input methods: {}", available_backends.join(", "));
        }

        stt_manager.transcribe().map_err(|e| format!("Failed to get input: {}", e))?
    };

    if command_text.is_empty() {
        return Err("No command provided".into());
    }

    println!("📝 Command: {}", command_text);
    println!("🤖 Parsing command with LLM...");

    // Create parser with default config
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);

    // Parse command
    let parsed = parser.parse_command(&command_text).await?;

    println!("✅ Parsed command successfully");
    println!("   - API Type: {}", parsed.api_type);
    println!("   - Endpoints: {}", parsed.endpoints.len());
    println!("   - Models: {}", parsed.models.len());

    // Generate OpenAPI spec
    println!("📋 Generating OpenAPI specification...");
    let spec_generator = VoiceSpecGenerator::new();
    let spec = spec_generator.generate_spec(&parsed).await?;

    println!("✅ Generated OpenAPI spec: {} v{}", spec.title(), spec.api_version());

    // Save to file if output specified
    if let Some(output_path) = output {
        let spec_json = serde_json::to_value(&spec.spec)?;
        let content = if output_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "yaml" || s == "yml")
            .unwrap_or(false)
        {
            serde_yaml::to_string(&spec_json)?
        } else {
            serde_json::to_string_pretty(&spec_json)?
        };

        tokio::fs::write(&output_path, content).await?;
        println!("💾 Saved OpenAPI spec to: {}", output_path.display());
    }

    // Start server if requested
    if serve {
        println!("🚀 Starting mock server on port {}...", port);
        println!("📡 Server will be available at: http://localhost:{}", port);
        println!("🛑 Press Ctrl+C to stop the server");
        println!();

        // Save spec to temp file
        let temp_spec =
            std::env::temp_dir().join(format!("voice-spec-{}.json", uuid::Uuid::new_v4()));
        let spec_json = serde_json::to_value(&spec.spec)?;
        let content = serde_json::to_string_pretty(&spec_json)?;
        tokio::fs::write(&temp_spec, content).await?;

        // Start server using the existing serve infrastructure
        use crate::handle_serve;
        handle_serve(
            None,                      // config_path
            None,                      // profile
            Some(port),                // http_port
            None,                      // ws_port
            None,                      // grpc_port
            None,                      // smtp_port
            None,                      // tcp_port
            true,                      // admin (enable admin UI)
            None,                      // admin_port
            false,                     // metrics
            None,                      // metrics_port
            false,                     // tracing
            "mockforge".to_string(),   // tracing_service_name
            "development".to_string(), // tracing_environment
            String::new(),             // jaeger_endpoint
            1.0,                       // tracing_sampling_rate
            false,                     // recorder
            String::new(),             // recorder_db
            false,                     // recorder_no_api
            None,                      // recorder_api_port
            0,                         // recorder_max_requests
            0,                         // recorder_retention_days
            false,                     // chaos
            None,                      // chaos_scenario
            None,                      // chaos_latency_ms
            None,                      // chaos_latency_range
            0.0,                       // chaos_latency_probability
            None,                      // chaos_http_errors
            0.0,                       // chaos_http_error_probability
            None,                      // chaos_rate_limit
            None,                      // chaos_bandwidth_limit
            None,                      // chaos_packet_loss
            Some(temp_spec),           // spec
            None,                      // ws_replay_file
            None,                      // graphql
            None,                      // graphql_port
            None,                      // graphql_upstream
            false,                     // traffic_shaping
            0,                         // bandwidth_limit
            0,                         // burst_size
            None,                      // network_profile
            false,                     // chaos_random
            0.0,                       // chaos_random_error_rate
            0.0,                       // chaos_random_delay_rate
            0,                         // chaos_random_min_delay
            0,                         // chaos_random_max_delay
            None,                      // chaos_profile
            false,                     // ai_enabled
            None,                      // reality_level
            None,                      // rag_provider
            None,                      // rag_model
            None,                      // rag_api_key
            false,                     // dry_run
            false,                     // progress
            false,                     // verbose
        )
        .await?;
    }

    Ok(())
}

/// Handle interactive command (conversational mode)
async fn handle_interactive(
    output: Option<PathBuf>,
    serve: bool,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🎤 Voice + LLM Interface - Interactive Mode");
    println!("💬 Start a conversation to build your API incrementally");
    println!("   Type 'done' or 'exit' when finished");
    println!("   Type 'help' for available commands");
    println!();

    // Create conversation manager
    let mut conversation_manager = ConversationManager::new();
    let conversation_id = conversation_manager.start_conversation();

    // Create parser
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);
    let spec_generator = VoiceSpecGenerator::new();

    let mut current_spec: Option<OpenApiSpec> = None;

    // Initialize voice input handler
    let voice_input = InteractiveVoiceInput::new();
    let stt_manager = SpeechToTextManager::new();
    let available_backends = stt_manager.list_backends();

    if available_backends.len() > 1 {
        println!("🎤 Available input methods: {}", available_backends.join(", "));
    }
    println!();

    loop {
        print!("🎤 > ");
        std::io::Write::flush(&mut std::io::stdout())?;

        // Use speech-to-text manager for input
        let command = match stt_manager.transcribe() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("⚠️  Error getting input: {}", e);
                continue;
            }
        };

        if command.is_empty() {
            continue;
        }

        // Handle special commands
        match command.to_lowercase().as_str() {
            "done" | "exit" | "quit" => {
                println!("✅ Conversation complete!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  - Describe your API: 'Create an e-commerce API'");
                println!("  - Add endpoints: 'Add a products endpoint'");
                println!("  - Modify: 'Add checkout flow'");
                println!("  - View: 'show spec' or 'show endpoints'");
                println!("  - Exit: 'done', 'exit', or 'quit'");
                continue;
            }
            "show spec" | "show endpoints" => {
                if let Some(ref spec) = current_spec {
                    println!("📋 Current API: {} v{}", spec.title(), spec.api_version());
                    let paths = spec.all_paths_and_operations();
                    for (path, ops) in paths {
                        println!(
                            "   {} ({})",
                            path,
                            ops.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                        );
                    }
                } else {
                    println!("ℹ️  No API created yet. Start by describing what you want to build.");
                }
                continue;
            }
            _ => {}
        }

        println!("🤖 Processing: {}", command);

        // Get conversation context
        let conversation_state = conversation_manager
            .get_conversation(&conversation_id)
            .ok_or("Conversation not found")?;

        // Parse command
        let parsed = if current_spec.is_some() {
            // Conversational mode - use context
            parser
                .parse_conversational_command(&command, &conversation_state.context)
                .await?
        } else {
            // First command - single shot
            parser.parse_command(&command).await?
        };

        // Generate or merge spec
        let new_spec = if let Some(ref existing) = current_spec {
            spec_generator.merge_spec(existing, &parsed).await?
        } else {
            spec_generator.generate_spec(&parsed).await?
        };

        // Update conversation
        conversation_manager.update_conversation(
            &conversation_id,
            &command,
            Some(new_spec.clone()),
        )?;

        current_spec = Some(new_spec.clone());

        println!("✅ Updated API: {} v{}", new_spec.title(), new_spec.api_version());
        println!("   Endpoints: {}", new_spec.all_paths_and_operations().len());
    }

    // Finalize
    if let Some(ref spec) = current_spec {
        // Save to file if output specified
        if let Some(output_path) = output {
            let spec_json = serde_json::to_value(&spec.spec)?;
            let content = if output_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "yaml" || s == "yml")
                .unwrap_or(false)
            {
                serde_yaml::to_string(&spec_json)?
            } else {
                serde_json::to_string_pretty(&spec_json)?
            };

            tokio::fs::write(&output_path, content).await?;
            println!("💾 Saved OpenAPI spec to: {}", output_path.display());
        }

        // Start server if requested
        if serve {
            println!("🚀 Starting mock server on port {}...", port);
            println!("📡 Server will be available at: http://localhost:{}", port);
            println!("🛑 Press Ctrl+C to stop the server");
            println!();

            // Save spec to temp file
            let temp_spec =
                std::env::temp_dir().join(format!("voice-spec-{}.json", uuid::Uuid::new_v4()));
            let spec_json = serde_json::to_value(&spec.spec)?;
            let content = serde_json::to_string_pretty(&spec_json)?;
            tokio::fs::write(&temp_spec, content).await?;

            // Start server using the existing serve infrastructure
            use crate::handle_serve;
            handle_serve(
                None,                      // config_path
                None,                      // profile
                Some(port),                // http_port
                None,                      // ws_port
                None,                      // grpc_port
                None,                      // smtp_port
                None,                      // tcp_port
                true,                      // admin (enable admin UI)
                None,                      // admin_port
                false,                     // metrics
                None,                      // metrics_port
                false,                     // tracing
                "mockforge".to_string(),   // tracing_service_name
                "development".to_string(), // tracing_environment
                String::new(),             // jaeger_endpoint
                1.0,                       // tracing_sampling_rate
                false,                     // recorder
                String::new(),             // recorder_db
                false,                     // recorder_no_api
                None,                      // recorder_api_port
                0,                         // recorder_max_requests
                0,                         // recorder_retention_days
                false,                     // chaos
                None,                      // chaos_scenario
                None,                      // chaos_latency_ms
                None,                      // chaos_latency_range
                0.0,                       // chaos_latency_probability
                None,                      // chaos_http_errors
                0.0,                       // chaos_http_error_probability
                None,                      // chaos_rate_limit
                None,                      // chaos_bandwidth_limit
                None,                      // chaos_packet_loss
                Some(temp_spec),           // spec
                None,                      // ws_replay_file
                None,                      // graphql
                None,                      // graphql_port
                None,                      // graphql_upstream
                false,                     // traffic_shaping
                0,                         // bandwidth_limit
                0,                         // burst_size
                None,                      // network_profile
                false,                     // chaos_random
                0.0,                       // chaos_random_error_rate
                0.0,                       // chaos_random_delay_rate
                0,                         // chaos_random_min_delay
                0,                         // chaos_random_max_delay
                None,                      // chaos_profile
                false,                     // ai_enabled
                None,                      // reality_level
                None,                      // rag_provider
                None,                      // rag_model
                None,                      // rag_api_key
                false,                     // dry_run
                false,                     // progress
                false,                     // verbose
            )
            .await?;
        }
    } else {
        println!("ℹ️  No API was created. Exiting.");
    }

    Ok(())
}

/// Handle transpile-hook command
async fn handle_transpile_hook(
    description: Option<String>,
    output: Option<PathBuf>,
    format: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔧 Hook Transpiler - Natural Language to Hook Configuration");
    println!();

    // Get description text
    let description_text = if let Some(desc) = description {
        desc
    } else {
        // Prompt for description
        print!("Enter hook description: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    if description_text.is_empty() {
        return Err("No description provided".into());
    }

    println!("📝 Description: {}", description_text);
    println!("🤖 Transpiling hook description with LLM...");

    // Create transpiler with default config
    let config = IntelligentBehaviorConfig::default();
    let transpiler = HookTranspiler::new(config);

    // Transpile the description
    let hook = match transpiler.transpile(&description_text).await {
        Ok(hook) => hook,
        Err(e) => {
            return Err(format!("Failed to transpile hook: {}", e).into());
        }
    };

    println!("✅ Hook transpiled successfully");
    // Note: Hook is now serde_json::Value, so we extract fields from JSON
    let hook_name = hook.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
    let hook_type = hook.get("hook_type").and_then(|v| v.as_str()).unwrap_or("unknown");
    let actions_count =
        hook.get("actions").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let has_condition = hook.get("condition").is_some();
    println!("   - Name: {}", hook_name);
    println!("   - Type: {:?}", hook_type);
    println!("   - Actions: {}", actions_count);
    if has_condition {
        println!("   - Has condition: Yes");
    }

    // Serialize hook
    let content = match format.to_lowercase().as_str() {
        "yaml" | "yml" => serde_yaml::to_string(&hook)
            .map_err(|e| format!("Failed to serialize hook to YAML: {}", e))?,
        "json" => serde_json::to_string_pretty(&hook)
            .map_err(|e| format!("Failed to serialize hook to JSON: {}", e))?,
        _ => {
            return Err(format!("Unsupported format: {}. Use 'yaml' or 'json'", format).into());
        }
    };

    // Output hook configuration
    if let Some(output_path) = output {
        // Write to file
        tokio::fs::write(&output_path, content).await?;
        println!("💾 Saved hook configuration to: {}", output_path.display());
    } else {
        // Print to stdout
        println!();
        println!("📄 Generated Hook Configuration:");
        println!("{}", "─".repeat(60));
        println!("{}", content);
        println!("{}", "─".repeat(60));
    }

    Ok(())
}

/// Handle create-workspace command
async fn handle_create_workspace(
    command: Option<String>,
    auto_confirm: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🏗️  Workspace Creator - Natural Language to Complete Workspace");
    println!();
    println!("This will create a complete workspace with:");
    println!("  • Endpoints and API structure");
    println!("  • Personas with relationships");
    println!("  • Behavioral scenarios (happy path, failure, slow path)");
    println!("  • Reality continuum configuration");
    println!("  • Drift budget configuration");
    println!();

    // Get command text
    let command_text = if let Some(cmd) = command {
        cmd
    } else {
        // Use speech-to-text manager to get input
        let stt_manager = SpeechToTextManager::new();
        let available_backends = stt_manager.list_backends();

        if available_backends.len() > 1 {
            println!("🎤 Available input methods: {}", available_backends.join(", "));
        }

        println!("🎤 Describe your workspace (or type your command):");
        stt_manager.transcribe().map_err(|e| format!("Failed to get input: {}", e))?
    };

    if command_text.is_empty() {
        return Err("No command provided".into());
    }

    println!("📝 Command: {}", command_text);
    println!("🤖 Parsing workspace creation command with LLM...");

    // Create parser with default config
    let config = IntelligentBehaviorConfig::default();
    let parser = VoiceCommandParser::new(config);

    // Parse command
    let parsed = match parser.parse_workspace_creation_command(&command_text).await {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(format!("Failed to parse command: {}", e).into());
        }
    };

    println!("✅ Parsed command successfully");
    println!();

    // Display preview
    println!("📋 Workspace Preview:");
    println!("{}", "═".repeat(60));
    println!("Name: {}", parsed.workspace_name);
    println!("Description: {}", parsed.workspace_description);
    println!();
    println!("Entities: {}", parsed.entities.len());
    for entity in &parsed.entities {
        println!("  • {} ({} endpoints)", entity.name, entity.endpoints.len());
    }
    println!();
    println!("Personas: {}", parsed.personas.len());
    for persona in &parsed.personas {
        println!("  • {} ({} relationships)", persona.name, persona.relationships.len());
    }
    println!();
    println!("Scenarios: {}", parsed.scenarios.len());
    for scenario in &parsed.scenarios {
        println!("  • {} ({})", scenario.name, scenario.r#type);
    }
    if parsed.reality_continuum.is_some() {
        println!();
        println!("Reality Continuum: Configured");
    }
    if parsed.drift_budget.is_some() {
        println!("Drift Budget: Configured");
    }
    println!("{}", "═".repeat(60));
    println!();

    // Confirmation
    if !auto_confirm {
        print!("Create this workspace? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            println!("❌ Workspace creation cancelled.");
            return Ok(());
        }
    }

    println!("🏗️  Creating workspace...");
    println!();

    // Create workspace registry (in-memory for CLI)
    let mt_config = MultiTenantConfig {
        enabled: true,
        default_workspace: "default".to_string(),
        ..Default::default()
    };
    let mut registry = MultiTenantWorkspaceRegistry::new(mt_config);

    // Create workspace builder
    let mut builder = WorkspaceBuilder::new();

    // Build workspace
    let built = match builder.build_workspace(&mut registry, &parsed).await {
        Ok(built) => built,
        Err(e) => {
            return Err(format!("Failed to create workspace: {}", e).into());
        }
    };

    // Display creation log
    println!("✅ Workspace created successfully!");
    println!();
    println!("📊 Creation Summary:");
    println!("{}", "─".repeat(60));
    for log_entry in &built.creation_log {
        println!("  {}", log_entry);
    }
    println!("{}", "─".repeat(60));
    println!();

    println!("📦 Workspace Details:");
    println!("  ID: {}", built.workspace_id);
    println!("  Name: {}", built.name);
    if let Some(ref spec) = built.openapi_spec {
        println!("  OpenAPI Spec: {} endpoints", spec.all_paths_and_operations().len());
    }
    println!("  Personas: {}", built.personas.len());
    println!("  Scenarios: {}", built.scenarios.len());
    if built.reality_continuum.is_some() {
        println!("  Reality Continuum: Enabled");
    }
    if built.drift_budget.is_some() {
        println!("  Drift Budget: Configured");
    }
    println!();

    println!("🎉 Workspace '{}' is ready to use!", built.workspace_id);
    println!();
    println!("💡 Next steps:");
    println!("  • Start the MockForge server to use this workspace");
    println!("  • Access the workspace via: /workspace/{}", built.workspace_id);
    println!("  • View personas and scenarios in the Admin UI");

    Ok(())
}
