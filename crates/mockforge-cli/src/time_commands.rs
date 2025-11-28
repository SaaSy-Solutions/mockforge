//! Time travel / temporal simulation CLI commands
//!
//! Provides command-line interface for controlling virtual clock and time travel features.

use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::json;

/// Time travel subcommands
#[derive(Subcommand, Debug)]
pub enum TimeCommands {
    /// Show current time travel status
    Status,
    /// Enable time travel
    Enable {
        /// Initial time (ISO 8601 format, e.g., "2025-01-01T00:00:00Z")
        #[arg(long)]
        time: Option<String>,
        /// Time scale factor (1.0 = real time, 2.0 = 2x speed)
        #[arg(long)]
        scale: Option<f64>,
    },
    /// Disable time travel (return to real time)
    Disable,
    /// Advance time by a duration
    ///
    /// Examples:
    ///   mockforge time advance 1h
    ///   mockforge time advance 30m
    ///   mockforge time advance 1month
    ///   mockforge time advance 2d
    Advance {
        /// Duration to advance (e.g., "1h", "30m", "1month", "2d")
        duration: String,
    },
    /// Set time to a specific point
    ///
    /// Examples:
    ///   mockforge time set "2025-01-01T00:00:00Z"
    Set {
        /// Time to set (ISO 8601 format)
        time: String,
    },
    /// Set time scale factor
    ///
    /// Examples:
    ///   mockforge time scale 2.0  # 2x speed
    ///   mockforge time scale 0.5  # Half speed
    Scale {
        /// Scale factor (1.0 = real time, 2.0 = 2x speed, 0.5 = half speed)
        factor: f64,
    },
    /// Reset time travel to real time
    Reset,
    /// Save current time travel state as a scenario
    Save {
        /// Scenario name
        name: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
        /// Output file path (default: ./scenarios/{name}.json)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Load a saved scenario
    Load {
        /// Scenario name or file path
        name: String,
    },
    /// List saved scenarios
    List {
        /// Scenarios directory (default: ./scenarios)
        #[arg(long)]
        dir: Option<String>,
    },
    /// Cron job management
    Cron {
        #[command(subcommand)]
        command: CronCommands,
    },
    /// Mutation rule management
    Mutation {
        #[command(subcommand)]
        command: MutationCommands,
    },
}

/// Cron job subcommands
#[derive(Subcommand, Debug)]
pub enum CronCommands {
    /// List all cron jobs
    List,
    /// Create a new cron job
    Create {
        /// Job ID
        id: String,
        /// Job name
        #[arg(long)]
        name: String,
        /// Cron schedule (e.g., "0 3 * * *")
        #[arg(long)]
        schedule: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
        /// Action type: "callback", "response", or "mutation"
        #[arg(long)]
        action_type: String,
        /// Action metadata JSON file path
        #[arg(long)]
        action_metadata: Option<String>,
    },
    /// Get a specific cron job
    Get {
        /// Job ID
        id: String,
    },
    /// Delete a cron job
    Delete {
        /// Job ID
        id: String,
    },
    /// Enable a cron job
    Enable {
        /// Job ID
        id: String,
    },
    /// Disable a cron job
    Disable {
        /// Job ID
        id: String,
    },
}

/// Mutation rule subcommands
#[derive(Subcommand, Debug)]
pub enum MutationCommands {
    /// List all mutation rules
    List,
    /// Create a new mutation rule
    Create {
        /// Rule ID
        id: String,
        /// Entity name
        #[arg(long)]
        entity: String,
        /// Trigger type: "interval", "attime", or "threshold"
        #[arg(long)]
        trigger_type: String,
        /// Trigger configuration JSON file
        #[arg(long)]
        trigger_config: String,
        /// Operation type: "set", "increment", "decrement", "transform", or "status"
        #[arg(long)]
        operation_type: String,
        /// Operation configuration JSON file
        #[arg(long)]
        operation_config: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },
    /// Get a specific mutation rule
    Get {
        /// Rule ID
        id: String,
    },
    /// Delete a mutation rule
    Delete {
        /// Rule ID
        id: String,
    },
    /// Enable a mutation rule
    Enable {
        /// Rule ID
        id: String,
    },
    /// Disable a mutation rule
    Disable {
        /// Rule ID
        id: String,
    },
}

/// Execute time travel command
pub async fn execute_time_command(command: TimeCommands, admin_url: Option<String>) -> Result<()> {
    // Default admin URL
    let admin_url = admin_url.unwrap_or_else(|| "http://localhost:9080".to_string());
    let base_url = format!("{}/__mockforge/time-travel", admin_url);

    match command {
        TimeCommands::Status => {
            let client = reqwest::Client::new();
            let response = client
                .get(format!("{}/status", base_url))
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                let status: serde_json::Value = response.json().await?;
                println!("Time Travel Status:");
                println!("  Enabled: {}", status["enabled"]);
                if let Some(virtual_time) = status["current_time"].as_str() {
                    println!("  Virtual time: {}", virtual_time);
                } else {
                    println!("  Virtual time: (using real time)");
                }
                println!("  Scale factor: {}x", status["scale_factor"]);
                println!("  Real time: {}", status["real_time"]);
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to get status: {}", error_text);
            }
        }
        TimeCommands::Enable { time, scale } => {
            let client = reqwest::Client::new();
            let mut body = json!({});
            if let Some(time_str) = time {
                body["time"] = json!(time_str);
            }
            if let Some(scale_factor) = scale {
                body["scale"] = json!(scale_factor);
            }

            let response = client
                .post(format!("{}/enable", base_url))
                .json(&body)
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Time travel enabled");
                if let Some(virtual_time) = result["status"]["current_time"].as_str() {
                    println!("   Virtual time: {}", virtual_time);
                }
                if let Some(scale_factor) = result["status"]["scale_factor"].as_f64() {
                    println!("   Scale factor: {}x", scale_factor);
                }
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to enable time travel: {}", error_text);
            }
        }
        TimeCommands::Disable => {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/disable", base_url))
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                println!("✅ Time travel disabled (using real time)");
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to disable time travel: {}", error_text);
            }
        }
        TimeCommands::Advance { duration } => {
            let client = reqwest::Client::new();
            let body = json!({ "duration": duration });

            let response = client
                .post(format!("{}/advance", base_url))
                .json(&body)
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Time advanced by {}", duration);
                if let Some(virtual_time) = result["status"]["current_time"].as_str() {
                    println!("   Current virtual time: {}", virtual_time);
                }
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to advance time: {}", error_text);
            }
        }
        TimeCommands::Set { time } => {
            let client = reqwest::Client::new();
            let body = json!({ "time": time });

            let response = client
                .post(format!("{}/enable", base_url))
                .json(&body)
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Time set");
                if let Some(virtual_time) = result["status"]["current_time"].as_str() {
                    println!("   Current virtual time: {}", virtual_time);
                }
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to set time: {}", error_text);
            }
        }
        TimeCommands::Scale { factor } => {
            if factor <= 0.0 {
                anyhow::bail!("Scale factor must be positive");
            }
            let client = reqwest::Client::new();
            let body = json!({ "scale": factor });

            let response = client
                .post(format!("{}/scale", base_url))
                .json(&body)
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                println!("✅ Time scale set to {}x", factor);
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to set time scale: {}", error_text);
            }
        }
        TimeCommands::Reset => {
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/reset", base_url))
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                println!("✅ Time travel reset to real time");
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to reset time travel: {}", error_text);
            }
        }
        TimeCommands::Save {
            name,
            description,
            output,
        } => {
            let client = reqwest::Client::new();
            let body = json!({
                "name": name,
                "description": description
            });

            let response = client
                .post(format!("{}/scenario/save", base_url))
                .json(&body)
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                let scenario: serde_json::Value = response.json().await?;

                // Determine output path
                let output_path = if let Some(path) = output {
                    std::path::PathBuf::from(path)
                } else {
                    let scenarios_dir = std::path::PathBuf::from("./scenarios");
                    std::fs::create_dir_all(&scenarios_dir)
                        .context("Failed to create scenarios directory")?;
                    scenarios_dir.join(format!("{}.json", name))
                };

                // Write scenario to file
                let scenario_json = serde_json::to_string_pretty(&scenario)
                    .context("Failed to serialize scenario")?;
                std::fs::write(&output_path, scenario_json)
                    .context(format!("Failed to write scenario to {:?}", output_path))?;

                println!("✅ Scenario '{}' saved to {:?}", name, output_path);
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to save scenario: {}", error_text);
            }
        }
        TimeCommands::Load { name } => {
            // Check if it's a file path or scenario name
            let scenario_path = if std::path::Path::new(&name).exists() {
                std::path::PathBuf::from(name)
            } else {
                // Try scenarios directory
                let scenarios_dir = std::path::PathBuf::from("./scenarios");
                scenarios_dir.join(format!("{}.json", name))
            };

            if !scenario_path.exists() {
                anyhow::bail!("Scenario file not found: {:?}", scenario_path);
            }

            let scenario_json = std::fs::read_to_string(&scenario_path)
                .context(format!("Failed to read scenario file: {:?}", scenario_path))?;

            let scenario: serde_json::Value =
                serde_json::from_str(&scenario_json).context("Failed to parse scenario JSON")?;

            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/scenario/load", base_url))
                .json(&json!({ "name": scenario["name"] }))
                .send()
                .await
                .context("Failed to connect to MockForge server. Is it running?")?;

            if response.status().is_success() {
                // Apply scenario by setting time and scale
                if let Some(enabled) = scenario["enabled"].as_bool() {
                    if enabled {
                        if let Some(time_str) = scenario["current_time"].as_str() {
                            let set_body = json!({ "time": time_str });
                            let _ = client
                                .post(format!("{}/enable", base_url))
                                .json(&set_body)
                                .send()
                                .await;
                        }
                        if let Some(scale) = scenario["scale_factor"].as_f64() {
                            let scale_body = json!({ "scale": scale });
                            let _ = client
                                .post(format!("{}/scale", base_url))
                                .json(&scale_body)
                                .send()
                                .await;
                        }
                    } else {
                        let _ = client.post(format!("{}/disable", base_url)).send().await;
                    }
                }

                println!("✅ Scenario '{}' loaded", scenario["name"]);
            } else {
                let error_text = response.text().await?;
                anyhow::bail!("Failed to load scenario: {}", error_text);
            }
        }
        TimeCommands::List { dir } => {
            let scenarios_dir = dir
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("./scenarios"));

            if !scenarios_dir.exists() {
                println!("No scenarios directory found at {:?}", scenarios_dir);
                return Ok(());
            }

            let entries = std::fs::read_dir(&scenarios_dir)
                .context(format!("Failed to read scenarios directory: {:?}", scenarios_dir))?;

            let mut scenarios = Vec::new();
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(scenario) = serde_json::from_str::<serde_json::Value>(&content) {
                            scenarios.push((path, scenario));
                        }
                    }
                }
            }

            if scenarios.is_empty() {
                println!("No scenarios found in {:?}", scenarios_dir);
            } else {
                println!("Saved scenarios:");
                for (path, scenario) in scenarios {
                    let name = scenario["name"].as_str().unwrap_or("unknown");
                    let created = scenario["created_at"].as_str().unwrap_or("unknown");
                    let enabled = scenario["enabled"].as_bool().unwrap_or(false);
                    let time = scenario["current_time"].as_str();
                    let scale = scenario["scale_factor"].as_f64().unwrap_or(1.0);

                    println!("  📁 {}", path.file_name().unwrap().to_string_lossy());
                    println!("     Name: {}", name);
                    println!("     Created: {}", created);
                    println!("     Enabled: {}", enabled);
                    if let Some(time_str) = time {
                        println!("     Virtual time: {}", time_str);
                    }
                    println!("     Scale: {}x", scale);
                    if let Some(desc) = scenario["description"].as_str() {
                        println!("     Description: {}", desc);
                    }
                    println!();
                }
            }
        }
        TimeCommands::Cron { command } => {
            let cron_url = format!("{}/cron", base_url);
            match command {
                CronCommands::List => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(&cron_url)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        let result: serde_json::Value = response.json().await?;
                        let empty: Vec<serde_json::Value> = vec![];
                        let jobs = result["jobs"].as_array().unwrap_or(&empty);

                        if jobs.is_empty() {
                            println!("No cron jobs configured");
                        } else {
                            println!("Cron Jobs:");
                            for job in jobs {
                                let id = job["id"].as_str().unwrap_or("unknown");
                                let name = job["name"].as_str().unwrap_or("unknown");
                                let schedule = job["schedule"].as_str().unwrap_or("unknown");
                                let enabled = job["enabled"].as_bool().unwrap_or(false);
                                let count = job["execution_count"].as_u64().unwrap_or(0);
                                let next = job["next_execution"].as_str();

                                println!("  ⏰ {}", id);
                                println!("     Name: {}", name);
                                println!("     Schedule: {}", schedule);
                                println!("     Enabled: {}", enabled);
                                println!("     Executions: {}", count);
                                if let Some(next_str) = next {
                                    println!("     Next execution: {}", next_str);
                                }
                                if let Some(desc) = job["description"].as_str() {
                                    println!("     Description: {}", desc);
                                }
                                println!();
                            }
                        }
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to list cron jobs: {}", error_text);
                    }
                }
                CronCommands::Create {
                    id,
                    name,
                    schedule,
                    description,
                    action_type,
                    action_metadata,
                } => {
                    let client = reqwest::Client::new();

                    // Load action metadata if provided
                    let metadata = if let Some(path) = action_metadata {
                        let content = std::fs::read_to_string(&path)
                            .context(format!("Failed to read action metadata file: {}", path))?;
                        serde_json::from_str(&content)
                            .context("Failed to parse action metadata JSON")?
                    } else {
                        json!({})
                    };

                    let body = json!({
                        "id": id,
                        "name": name,
                        "schedule": schedule,
                        "description": description,
                        "action_type": action_type,
                        "action_metadata": metadata
                    });

                    let response = client
                        .post(&cron_url)
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Cron job '{}' created", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to create cron job: {}", error_text);
                    }
                }
                CronCommands::Get { id } => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(format!("{}/{}", cron_url, id))
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        let result: serde_json::Value = response.json().await?;
                        let job = &result["job"];

                        println!("Cron Job: {}", id);
                        println!("  Name: {}", job["name"].as_str().unwrap_or("unknown"));
                        println!("  Schedule: {}", job["schedule"].as_str().unwrap_or("unknown"));
                        println!("  Enabled: {}", job["enabled"].as_bool().unwrap_or(false));
                        println!("  Executions: {}", job["execution_count"].as_u64().unwrap_or(0));
                        if let Some(desc) = job["description"].as_str() {
                            println!("  Description: {}", desc);
                        }
                        if let Some(last) = job["last_execution"].as_str() {
                            println!("  Last execution: {}", last);
                        }
                        if let Some(next) = job["next_execution"].as_str() {
                            println!("  Next execution: {}", next);
                        }
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to get cron job: {}", error_text);
                    }
                }
                CronCommands::Delete { id } => {
                    let client = reqwest::Client::new();
                    let response = client
                        .delete(format!("{}/{}", cron_url, id))
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Cron job '{}' deleted", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to delete cron job: {}", error_text);
                    }
                }
                CronCommands::Enable { id } => {
                    let client = reqwest::Client::new();
                    let body = json!({ "enabled": true });

                    let response = client
                        .post(format!("{}/{}/enable", cron_url, id))
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Cron job '{}' enabled", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to enable cron job: {}", error_text);
                    }
                }
                CronCommands::Disable { id } => {
                    let client = reqwest::Client::new();
                    let body = json!({ "enabled": false });

                    let response = client
                        .post(format!("{}/{}/enable", cron_url, id))
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Cron job '{}' disabled", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to disable cron job: {}", error_text);
                    }
                }
            }
        }
        TimeCommands::Mutation { command } => {
            let mutation_url = format!("{}/mutations", base_url);
            match command {
                MutationCommands::List => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(&mutation_url)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        let result: serde_json::Value = response.json().await?;
                        let empty: Vec<serde_json::Value> = vec![];
                        let rules = result["rules"].as_array().unwrap_or(&empty);

                        if rules.is_empty() {
                            println!("No mutation rules configured");
                        } else {
                            println!("Mutation Rules:");
                            for rule in rules {
                                let id = rule["id"].as_str().unwrap_or("unknown");
                                let entity = rule["entity_name"].as_str().unwrap_or("unknown");
                                let enabled = rule["enabled"].as_bool().unwrap_or(false);
                                let count = rule["execution_count"].as_u64().unwrap_or(0);
                                let next = rule["next_execution"].as_str();

                                println!("  🔄 {}", id);
                                println!("     Entity: {}", entity);
                                println!("     Enabled: {}", enabled);
                                println!("     Executions: {}", count);
                                if let Some(next_str) = next {
                                    println!("     Next execution: {}", next_str);
                                }
                                if let Some(desc) = rule["description"].as_str() {
                                    println!("     Description: {}", desc);
                                }
                                println!();
                            }
                        }
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to list mutation rules: {}", error_text);
                    }
                }
                MutationCommands::Create {
                    id,
                    entity,
                    trigger_type,
                    trigger_config,
                    operation_type,
                    operation_config,
                    description,
                } => {
                    let client = reqwest::Client::new();

                    // Load trigger config
                    let trigger_content = std::fs::read_to_string(&trigger_config).context(
                        format!("Failed to read trigger config file: {}", trigger_config),
                    )?;
                    let trigger_json: serde_json::Value = serde_json::from_str(&trigger_content)
                        .context("Failed to parse trigger config JSON")?;

                    // Build trigger based on type
                    let trigger = match trigger_type.as_str() {
                        "interval" => {
                            let duration =
                                trigger_json["duration_seconds"].as_u64().ok_or_else(|| {
                                    anyhow::anyhow!("Missing duration_seconds in trigger config")
                                })?;
                            serde_json::json!({
                                "type": "interval",
                                "duration_seconds": duration
                            })
                        }
                        "attime" => {
                            let hour = trigger_json["hour"]
                                .as_u64()
                                .ok_or_else(|| anyhow::anyhow!("Missing hour in trigger config"))?;
                            let minute = trigger_json["minute"].as_u64().ok_or_else(|| {
                                anyhow::anyhow!("Missing minute in trigger config")
                            })?;
                            serde_json::json!({
                                "type": "attime",
                                "hour": hour,
                                "minute": minute
                            })
                        }
                        _ => anyhow::bail!("Invalid trigger type: {}", trigger_type),
                    };

                    // Load operation config
                    let operation_content = std::fs::read_to_string(&operation_config).context(
                        format!("Failed to read operation config file: {}", operation_config),
                    )?;
                    let operation_json: serde_json::Value =
                        serde_json::from_str(&operation_content)
                            .context("Failed to parse operation config JSON")?;

                    // Build operation based on type
                    let operation = match operation_type.as_str() {
                        "set" => {
                            let field = operation_json["field"].as_str().ok_or_else(|| {
                                anyhow::anyhow!("Missing field in operation config")
                            })?;
                            let value = operation_json.get("value").ok_or_else(|| {
                                anyhow::anyhow!("Missing value in operation config")
                            })?;
                            serde_json::json!({
                                "type": "set",
                                "field": field,
                                "value": value
                            })
                        }
                        "increment" => {
                            let field = operation_json["field"].as_str().ok_or_else(|| {
                                anyhow::anyhow!("Missing field in operation config")
                            })?;
                            let amount = operation_json["amount"].as_f64().ok_or_else(|| {
                                anyhow::anyhow!("Missing amount in operation config")
                            })?;
                            serde_json::json!({
                                "type": "increment",
                                "field": field,
                                "amount": amount
                            })
                        }
                        "decrement" => {
                            let field = operation_json["field"].as_str().ok_or_else(|| {
                                anyhow::anyhow!("Missing field in operation config")
                            })?;
                            let amount = operation_json["amount"].as_f64().ok_or_else(|| {
                                anyhow::anyhow!("Missing amount in operation config")
                            })?;
                            serde_json::json!({
                                "type": "decrement",
                                "field": field,
                                "amount": amount
                            })
                        }
                        "status" => {
                            let status = operation_json["status"].as_str().ok_or_else(|| {
                                anyhow::anyhow!("Missing status in operation config")
                            })?;
                            serde_json::json!({
                                "type": "updatestatus",
                                "status": status
                            })
                        }
                        _ => anyhow::bail!("Invalid operation type: {}", operation_type),
                    };

                    let body = json!({
                        "id": id,
                        "entity_name": entity,
                        "trigger": trigger,
                        "operation": operation,
                        "description": description
                    });

                    let response = client
                        .post(&mutation_url)
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Mutation rule '{}' created", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to create mutation rule: {}", error_text);
                    }
                }
                MutationCommands::Get { id } => {
                    let client = reqwest::Client::new();
                    let response = client
                        .get(format!("{}/{}", mutation_url, id))
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        let result: serde_json::Value = response.json().await?;
                        let rule = &result["rule"];

                        println!("Mutation Rule: {}", id);
                        println!("  Entity: {}", rule["entity_name"].as_str().unwrap_or("unknown"));
                        println!("  Enabled: {}", rule["enabled"].as_bool().unwrap_or(false));
                        println!("  Executions: {}", rule["execution_count"].as_u64().unwrap_or(0));
                        if let Some(desc) = rule["description"].as_str() {
                            println!("  Description: {}", desc);
                        }
                        if let Some(last) = rule["last_execution"].as_str() {
                            println!("  Last execution: {}", last);
                        }
                        if let Some(next) = rule["next_execution"].as_str() {
                            println!("  Next execution: {}", next);
                        }
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to get mutation rule: {}", error_text);
                    }
                }
                MutationCommands::Delete { id } => {
                    let client = reqwest::Client::new();
                    let response =
                        client
                            .delete(format!("{}/{}", mutation_url, id))
                            .send()
                            .await
                            .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Mutation rule '{}' deleted", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to delete mutation rule: {}", error_text);
                    }
                }
                MutationCommands::Enable { id } => {
                    let client = reqwest::Client::new();
                    let body = json!({ "enabled": true });

                    let response = client
                        .post(format!("{}/{}/enable", mutation_url, id))
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Mutation rule '{}' enabled", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to enable mutation rule: {}", error_text);
                    }
                }
                MutationCommands::Disable { id } => {
                    let client = reqwest::Client::new();
                    let body = json!({ "enabled": false });

                    let response = client
                        .post(format!("{}/{}/enable", mutation_url, id))
                        .json(&body)
                        .send()
                        .await
                        .context("Failed to connect to MockForge server. Is it running?")?;

                    if response.status().is_success() {
                        println!("✅ Mutation rule '{}' disabled", id);
                    } else {
                        let error_text = response.text().await?;
                        anyhow::bail!("Failed to disable mutation rule: {}", error_text);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse duration string (e.g., "1h", "30m", "+1h", "+1 week", "1month", "2d")
///
/// Supports:
/// - Standard formats: "1h", "30m", "2d", etc.
/// - With + prefix: "+1h", "+1 week", "+2d"
/// - Week units: "1week", "1 week", "2weeks", "+1week"
/// - Month/year: "1month", "1year"
fn parse_duration(s: &str) -> Result<chrono::Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }

    // Strip leading + or - (for relative time notation)
    let s = s.strip_prefix('+').unwrap_or(s);
    let s = s.strip_prefix('-').unwrap_or(s);

    // Handle weeks (with or without space)
    if s.ends_with("week") || s.ends_with("weeks") || s.ends_with(" week") || s.ends_with(" weeks")
    {
        let num_str = s
            .trim_end_matches("week")
            .trim_end_matches("weeks")
            .trim_end_matches(" week")
            .trim_end_matches(" weeks")
            .trim();
        let amount: i64 = num_str.parse().context("Invalid number for weeks")?;
        // 1 week = 7 days
        return Ok(chrono::Duration::days(amount * 7));
    }

    // Handle months and years (approximate)
    if s.ends_with("month")
        || s.ends_with("months")
        || s.ends_with(" month")
        || s.ends_with(" months")
    {
        let num_str = s
            .trim_end_matches("month")
            .trim_end_matches("months")
            .trim_end_matches(" month")
            .trim_end_matches(" months")
            .trim();
        let amount: i64 = num_str.parse().context("Invalid number for months")?;
        // Approximate: 1 month = 30 days
        return Ok(chrono::Duration::days(amount * 30));
    }
    if s.ends_with('y')
        || s.ends_with("year")
        || s.ends_with("years")
        || s.ends_with(" year")
        || s.ends_with(" years")
    {
        let num_str = s
            .trim_end_matches('y')
            .trim_end_matches("year")
            .trim_end_matches("years")
            .trim_end_matches(" year")
            .trim_end_matches(" years")
            .trim();
        let amount: i64 = num_str.parse().context("Invalid number for years")?;
        // Approximate: 1 year = 365 days
        return Ok(chrono::Duration::days(amount * 365));
    }

    // Extract number and unit for standard durations
    let (num_str, unit) = if let Some(pos) = s.chars().position(|c| !c.is_numeric() && c != '-') {
        (&s[..pos], &s[pos..].trim())
    } else {
        anyhow::bail!("No unit specified (use s, m, h, d, week, month, or year)");
    };

    let amount: i64 = num_str.parse().context("Invalid number")?;

    match *unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(chrono::Duration::seconds(amount)),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(chrono::Duration::minutes(amount)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(chrono::Duration::hours(amount)),
        "d" | "day" | "days" => Ok(chrono::Duration::days(amount)),
        "w" | "week" | "weeks" => Ok(chrono::Duration::days(amount * 7)),
        _ => anyhow::bail!("Unknown unit: {}. Use s, m, h, d, week, month, or year", unit),
    }
}
