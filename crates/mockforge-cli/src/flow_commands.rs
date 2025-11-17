//! Flow commands for behavioral cloning v1
//!
//! Commands for managing recorded flows, viewing timelines, and compiling scenarios.

use clap::Subcommand;
use mockforge_recorder::behavioral_cloning::{
    flow_recorder::{FlowRecorder, FlowRecordingConfig},
    FlowCompiler, ScenarioStorage,
};
use mockforge_recorder::RecorderDatabase;
use std::path::PathBuf;
use tracing::info;

#[derive(Subcommand)]
pub enum FlowCommands {
    /// List recorded flows
    ///
    /// Examples:
    ///   mockforge flow list
    ///   mockforge flow list --limit 50
    List {
        /// Maximum number of flows to list
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },

    /// View a flow timeline
    ///
    /// Examples:
    ///   mockforge flow view <flow-id>
    ///   mockforge flow view abc-123-def --verbose
    View {
        /// Flow ID to view
        flow_id: String,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Tag a flow as a named scenario
    ///
    /// Examples:
    ///   mockforge flow tag <flow-id> --name "checkout_success" --tags ecommerce,checkout
    Tag {
        /// Flow ID to tag
        flow_id: String,

        /// Name for the flow
        #[arg(short, long)]
        name: String,

        /// Tags to apply (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// Compile a flow into a behavioral scenario
    ///
    /// Examples:
    ///   mockforge flow compile <flow-id> --scenario-name "checkout_success"
    ///   mockforge flow compile <flow-id> --scenario-name "checkout" --flex-mode
    Compile {
        /// Flow ID to compile
        flow_id: String,

        /// Name for the compiled scenario
        #[arg(long)]
        scenario_name: String,

        /// Use flex mode (allow minor variations) instead of strict mode
        #[arg(long)]
        flex_mode: bool,
    },

    /// List compiled scenarios
    ///
    /// Examples:
    ///   mockforge flow scenarios
    ///   mockforge flow scenarios --limit 20
    Scenarios {
        /// Maximum number of scenarios to list
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },

    /// Export a scenario to YAML or JSON
    ///
    /// Examples:
    ///   mockforge flow export <scenario-id> --output scenario.yaml
    ///   mockforge flow export <scenario-id> --output scenario.json --format json
    Export {
        /// Scenario ID to export
        scenario_id: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },

    /// Import a scenario from YAML or JSON
    ///
    /// Examples:
    ///   mockforge flow import --input scenario.yaml
    ///   mockforge flow import --input scenario.json --format json
    Import {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Import format (yaml or json, auto-detected from extension if not specified)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Replay a scenario (for testing)
    ///
    /// Examples:
    ///   mockforge flow replay <scenario-id>
    ///   mockforge flow replay <scenario-id> --flex-mode
    Replay {
        /// Scenario ID to replay
        scenario_id: String,

        /// Use flex mode instead of strict mode
        #[arg(long)]
        flex_mode: bool,
    },
}

pub async fn handle_flow_command(command: FlowCommands) -> anyhow::Result<()> {
    // Get database path from environment or use default
    let db_path = std::env::var("MOCKFORGE_RECORDER_DB")
        .unwrap_or_else(|_| "recordings.db".to_string());
    let db = RecorderDatabase::new(&db_path).await?;

    match command {
        FlowCommands::List { limit } => handle_list(db, limit).await,
        FlowCommands::View { flow_id, verbose } => handle_view(db, flow_id, verbose).await,
        FlowCommands::Tag {
            flow_id,
            name,
            tags,
            description,
        } => handle_tag(db, flow_id, name, tags, description).await,
        FlowCommands::Compile {
            flow_id,
            scenario_name,
            flex_mode,
        } => handle_compile(db, flow_id, scenario_name, flex_mode).await,
        FlowCommands::Scenarios { limit } => handle_scenarios(db, limit).await,
        FlowCommands::Export {
            scenario_id,
            output,
            format,
        } => handle_export(db, scenario_id, output, format).await,
        FlowCommands::Import { input, format } => handle_import(db, input, format).await,
        FlowCommands::Replay {
            scenario_id,
            flex_mode,
        } => handle_replay(db, scenario_id, flex_mode).await,
    }
}

async fn handle_list(db: RecorderDatabase, limit: usize) -> anyhow::Result<()> {
    let config = FlowRecordingConfig::default();
    let recorder = FlowRecorder::new(db, config);
    let flows = recorder.list_flows(Some(limit)).await?;

    println!("Found {} flows:\n", flows.len());
    for flow in flows {
        let name = flow.name.as_deref().unwrap_or("(unnamed)");
        let step_count = flow.steps.len();
        println!("  {} - {} ({} steps)", flow.id, name, step_count);
        if let Some(desc) = &flow.description {
            println!("    {}", desc);
        }
    }

    Ok(())
}

async fn handle_view(db: RecorderDatabase, flow_id: String, verbose: bool) -> anyhow::Result<()> {
    let config = FlowRecordingConfig::default();
    let recorder = FlowRecorder::new(db, config);
    let flow = recorder
        .get_flow(&flow_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Flow not found: {}", flow_id))?;

    let name = flow.name.as_deref().unwrap_or("(unnamed)");
    println!("Flow: {} - {}\n", flow.id, name);
    if let Some(desc) = &flow.description {
        println!("Description: {}\n", desc);
    }

    println!("Timeline ({} steps):\n", flow.steps.len());
    println!("  Step | Label      | Timing  | Request ID");
    println!("  -----|------------|---------|------------");

    for (idx, step) in flow.steps.iter().enumerate() {
        let label = step.step_label.as_deref().unwrap_or("-");
        let timing = step
            .timing_ms
            .map(|t| format!("{}ms", t))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {:4} | {:10} | {:7} | {}",
            idx + 1,
            label,
            timing,
            step.request_id
        );

        if verbose {
            // Fetch and display request details
            if let Ok(Some(request)) = db.get_request(&step.request_id).await {
                println!("        Request Details:");
                println!("          Method: {}", request.method);
                println!("          Path: {}", request.path);
                if let Some(query) = &request.query_params {
                    println!("          Query: {}", query);
                }
                if let Some(body) = &request.body {
                    let body_preview = if body.len() > 200 {
                        format!("{}...", &body[..200])
                    } else {
                        body.clone()
                    };
                    println!("          Body: {}", body_preview);
                }
            }
            if let Ok(Some(response)) = db.get_response(&step.request_id).await {
                println!("        Response Details:");
                println!("          Status: {}", response.status_code);
                if let Some(body) = &response.body {
                    let body_preview = if body.len() > 200 {
                        format!("{}...", &body[..200])
                    } else {
                        body.clone()
                    };
                    println!("          Body: {}", body_preview);
                }
            }
        }
    }

    Ok(())
}

async fn handle_tag(
    db: RecorderDatabase,
    flow_id: String,
    name: String,
    tags: Option<String>,
    description: Option<String>,
) -> anyhow::Result<()> {
    let config = FlowRecordingConfig::default();
    let recorder = FlowRecorder::new(db, config);

    let tags_vec = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    recorder
        .update_flow_metadata(&flow_id, Some(&name), description.as_deref(), Some(tags_vec))
        .await?;

    info!("Tagged flow {} as '{}'", flow_id, name);
    println!("Flow {} tagged as '{}'", flow_id, name);

    Ok(())
}

async fn handle_compile(
    db: RecorderDatabase,
    flow_id: String,
    scenario_name: String,
    flex_mode: bool,
) -> anyhow::Result<()> {
    let config = FlowRecordingConfig::default();
    let recorder = FlowRecorder::new(db.clone(), config);
    let flow = recorder
        .get_flow(&flow_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Flow not found: {}", flow_id))?;

    let compiler = FlowCompiler::new(db.clone());
    let strict_mode = !flex_mode;
    let scenario = compiler
        .compile_flow(&flow, scenario_name.clone(), strict_mode)
        .await?;

    // Store the scenario
    let storage = ScenarioStorage::new(db);
    let version = storage.store_scenario_auto_version(&scenario).await?;

    info!("Compiled flow {} into scenario {} v{}", flow_id, scenario.id, version);
    println!(
        "Compiled flow {} into scenario '{}' (ID: {}, Version: {})",
        flow_id, scenario_name, scenario.id, version
    );

    Ok(())
}

async fn handle_scenarios(db: RecorderDatabase, limit: usize) -> anyhow::Result<()> {
    let storage = ScenarioStorage::new(db);
    let scenarios = storage.list_scenarios(Some(limit)).await?;

    println!("Found {} scenarios:\n", scenarios.len());
    for scenario in scenarios {
        println!(
            "  {} - {} v{}",
            scenario.id, scenario.name, scenario.version
        );
        if let Some(desc) = &scenario.description {
            println!("    {}", desc);
        }
        if !scenario.tags.is_empty() {
            println!("    Tags: {}", scenario.tags.join(", "));
        }
    }

    Ok(())
}

async fn handle_export(
    db: RecorderDatabase,
    scenario_id: String,
    output: PathBuf,
    format: String,
) -> anyhow::Result<()> {
    let storage = ScenarioStorage::new(db);
    storage.export_scenario_to_file(&scenario_id, &output).await?;

    println!("Exported scenario {} to {}", scenario_id, output.display());
    Ok(())
}

async fn handle_import(
    db: RecorderDatabase,
    input: PathBuf,
    format: Option<String>,
) -> anyhow::Result<()> {
    let storage = ScenarioStorage::new(db.clone());
    let scenario = storage.import_scenario_from_file(&input).await?;

    // Store the imported scenario
    let version = storage.store_scenario_auto_version(&scenario).await?;

    println!(
        "Imported scenario '{}' (ID: {}, Version: {})",
        scenario.name, scenario.id, version
    );
    Ok(())
}

async fn handle_replay(
    db: RecorderDatabase,
    scenario_id: String,
    flex_mode: bool,
) -> anyhow::Result<()> {
    let storage = ScenarioStorage::new(db.clone());
    let scenario = storage
        .get_scenario(&scenario_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scenario not found: {}", scenario_id))?;

    println!("Replaying scenario: {} ({} steps)", scenario.name, scenario.steps.len());
    println!("Mode: {}\n", if flex_mode { "flex" } else { "strict" });

    // For CLI replay, we just validate the scenario
    // Actual replay happens when the server is running with the scenario activated
    println!("Scenario is ready for replay.");
    println!("To use this scenario in a running server, activate it via the Admin UI or API.");
    println!("\nScenario details:");
    println!("  ID: {}", scenario.id);
    println!("  Name: {}", scenario.name);
    if let Some(desc) = &scenario.description {
        println!("  Description: {}", desc);
    }
    println!("  Steps: {}", scenario.steps.len());
    println!("  State variables: {}", scenario.state_variables.len());

    Ok(())
}

