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
                .get(&format!("{}/status", base_url))
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
                .post(&format!("{}/enable", base_url))
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
                .post(&format!("{}/disable", base_url))
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
                .post(&format!("{}/advance", base_url))
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
                .post(&format!("{}/enable", base_url))
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
                .post(&format!("{}/scale", base_url))
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
                .post(&format!("{}/reset", base_url))
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
                .post(&format!("{}/scenario/save", base_url))
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
                .post(&format!("{}/scenario/load", base_url))
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
                                .post(&format!("{}/enable", base_url))
                                .json(&set_body)
                                .send()
                                .await;
                        }
                        if let Some(scale) = scenario["scale_factor"].as_f64() {
                            let scale_body = json!({ "scale": scale });
                            let _ = client
                                .post(&format!("{}/scale", base_url))
                                .json(&scale_body)
                                .send()
                                .await;
                        }
                    } else {
                        let _ = client.post(&format!("{}/disable", base_url)).send().await;
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
    }

    Ok(())
}

/// Parse duration string (e.g., "1h", "30m", "1month", "2d")
fn parse_duration(s: &str) -> Result<chrono::Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }

    // Handle months and years (approximate)
    if s.ends_with("month") || s.ends_with("months") {
        let num_str = s.trim_end_matches("month").trim_end_matches("months").trim();
        let amount: i64 = num_str.parse().context("Invalid number for months")?;
        // Approximate: 1 month = 30 days
        return Ok(chrono::Duration::days(amount * 30));
    }
    if s.ends_with('y') || s.ends_with("year") || s.ends_with("years") {
        let num_str = s
            .trim_end_matches('y')
            .trim_end_matches("year")
            .trim_end_matches("years")
            .trim();
        let amount: i64 = num_str.parse().context("Invalid number for years")?;
        // Approximate: 1 year = 365 days
        return Ok(chrono::Duration::days(amount * 365));
    }

    // Extract number and unit for standard durations
    let (num_str, unit) = if let Some(pos) = s.chars().position(|c| !c.is_numeric() && c != '-') {
        (&s[..pos], &s[pos..])
    } else {
        anyhow::bail!("No unit specified (use s, m, h, d, month, or year)");
    };

    let amount: i64 = num_str.parse().context("Invalid number")?;

    match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(chrono::Duration::seconds(amount)),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(chrono::Duration::minutes(amount)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(chrono::Duration::hours(amount)),
        "d" | "day" | "days" => Ok(chrono::Duration::days(amount)),
        _ => anyhow::bail!("Unknown unit: {}. Use s, m, h, d, month, or year", unit),
    }
}
