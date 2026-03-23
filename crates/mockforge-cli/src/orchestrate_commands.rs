//! Orchestrate and Suggest commands
//!
//! CLI commands for chaos orchestration and AI-powered spec suggestion.

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum OrchestrateCommands {
    /// Start a chaos orchestration from file
    ///
    /// Example:
    ///   mockforge orchestrate start --file orchestration.yaml --base-url http://localhost:3000
    #[command(verbatim_doc_comment)]
    Start {
        /// Orchestration file (JSON or YAML)
        #[arg(short, long)]
        file: PathBuf,

        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Get orchestration status
    Status {
        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Stop running orchestration
    Stop {
        /// Base URL for API requests
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },

    /// Validate an orchestration file
    Validate {
        /// Orchestration file (JSON or YAML)
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Export an orchestration template
    ///
    /// Example:
    ///   mockforge orchestrate template --output my_orchestration.yaml --format yaml
    #[command(verbatim_doc_comment)]
    Template {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Format (json or yaml)
        #[arg(long, default_value = "yaml")]
        format: String,
    },
}

pub(crate) async fn handle_orchestrate(
    command: OrchestrateCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        OrchestrateCommands::Start { file, base_url } => {
            println!("\u{1f680} Starting chaos orchestration from: {}", file.display());

            // Read orchestration file
            let content = std::fs::read_to_string(&file)?;
            let format = if file.extension().and_then(|s| s.to_str()) == Some("json") {
                "json"
            } else {
                "yaml"
            };

            // Send to API
            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/import", base_url);

            let response = client
                .post(&url)
                .json(&serde_json::json!({
                    "content": content,
                    "format": format
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!(
                    "\u{2705} {}",
                    result["message"].as_str().unwrap_or("Orchestration imported")
                );

                // Now start it
                let _start_url = format!("{}/api/chaos/orchestration/start", base_url);
                // Note: This is a simplified version - would need to parse and send proper request
                println!("   Use the API to start the orchestration");
            } else {
                eprintln!("\u{274c} Failed to import orchestration: {}", response.status());
            }
        }

        OrchestrateCommands::Status { base_url } => {
            println!("\u{1f4ca} Checking orchestration status...");

            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/status", base_url);

            let response = client.get(&url).send().await?;

            if response.status().is_success() {
                let status: serde_json::Value = response.json().await?;

                if status["is_running"].as_bool().unwrap_or(false) {
                    println!("\u{2705} Orchestration is running");
                    println!("   Name: {}", status["name"].as_str().unwrap_or("Unknown"));
                    println!(
                        "   Progress: {:.1}%",
                        status["progress"].as_f64().unwrap_or(0.0) * 100.0
                    );
                } else {
                    println!("\u{23f8}\u{fe0f}  No orchestration currently running");
                }
            } else {
                eprintln!("\u{274c} Failed to get status: {}", response.status());
            }
        }

        OrchestrateCommands::Stop { base_url } => {
            println!("\u{1f6d1} Stopping orchestration...");

            let client = reqwest::Client::new();
            let url = format!("{}/api/chaos/orchestration/stop", base_url);

            let response = client.post(&url).send().await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!(
                    "\u{2705} {}",
                    result["message"].as_str().unwrap_or("Orchestration stopped")
                );
            } else {
                eprintln!("\u{274c} Failed to stop orchestration: {}", response.status());
            }
        }

        OrchestrateCommands::Validate { file } => {
            println!("\u{1f50d} Validating orchestration file: {}", file.display());

            // Check if file exists
            if !file.exists() {
                eprintln!("\u{274c} File not found: {}", file.display());
                return Err("File not found".into());
            }

            // Read and parse file
            let content = std::fs::read_to_string(&file)?;
            let is_json = file.extension().and_then(|s| s.to_str()) == Some("json");

            let parse_result: Result<serde_json::Value, String> = if is_json {
                serde_json::from_str::<serde_json::Value>(&content)
                    .map_err(|e| crate::config_commands::format_json_error(&content, e))
            } else {
                // Parse as YAML, then convert to JSON Value for uniform handling
                serde_yaml::from_str::<serde_yaml::Value>(&content)
                    .map_err(|e| crate::config_commands::format_yaml_error(&content, e))
                    .and_then(|yaml_val| {
                        serde_json::to_value(yaml_val)
                            .map_err(|e| format!("Failed to convert YAML to JSON: {}", e))
                    })
            };

            match parse_result {
                Ok(value) => {
                    // Validate structure
                    let mut errors = Vec::new();
                    let mut warnings = Vec::new();

                    // Check for required fields
                    if value.get("name").is_none() {
                        errors.push("Missing required field 'name'".to_string());
                    } else if !value["name"].is_string() {
                        errors.push("Field 'name' must be a string".to_string());
                    }

                    // Validate steps array
                    match value.get("steps") {
                        None => {
                            errors.push("Missing required field 'steps'".to_string());
                        }
                        Some(steps) => {
                            if let Some(steps_arr) = steps.as_array() {
                                if steps_arr.is_empty() {
                                    warnings.push(
                                        "Steps array is empty - orchestration won't do anything"
                                            .to_string(),
                                    );
                                }

                                // Validate each step
                                for (idx, step) in steps_arr.iter().enumerate() {
                                    let step_num = idx + 1;

                                    if !step.is_object() {
                                        errors.push(format!("Step #{} is not an object", step_num));
                                        continue;
                                    }

                                    // Check step name
                                    if step.get("name").is_none() {
                                        errors.push(format!(
                                            "Step #{} is missing 'name' field",
                                            step_num
                                        ));
                                    }

                                    // Check scenario
                                    match step.get("scenario") {
                                        None => {
                                            errors.push(format!(
                                                "Step #{} is missing 'scenario' field",
                                                step_num
                                            ));
                                        }
                                        Some(scenario) => {
                                            if scenario.get("name").is_none() {
                                                errors.push(format!(
                                                    "Step #{} scenario is missing 'name' field",
                                                    step_num
                                                ));
                                            }
                                            if scenario.get("config").is_none() {
                                                errors.push(format!(
                                                    "Step #{} scenario is missing 'config' field",
                                                    step_num
                                                ));
                                            }
                                        }
                                    }

                                    // Check duration
                                    if step.get("duration_seconds").is_none() {
                                        warnings.push(format!("Step #{} is missing 'duration_seconds' - using default", step_num));
                                    } else if !step["duration_seconds"].is_number() {
                                        errors.push(format!(
                                            "Step #{} 'duration_seconds' must be a number",
                                            step_num
                                        ));
                                    }

                                    // Check delay
                                    if let Some(delay) = step.get("delay_before_seconds") {
                                        if !delay.is_number() {
                                            errors.push(format!(
                                                "Step #{} 'delay_before_seconds' must be a number",
                                                step_num
                                            ));
                                        }
                                    }
                                }
                            } else {
                                errors.push("Field 'steps' must be an array".to_string());
                            }
                        }
                    }

                    // Print results
                    if !errors.is_empty() {
                        println!("\u{274c} Orchestration file has errors:");
                        for error in &errors {
                            println!("   \u{2717} {}", error);
                        }
                        return Err("Validation failed".into());
                    }

                    println!("\u{2705} Orchestration file is valid");

                    // Show summary
                    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                        println!("\n\u{1f4ca} Summary:");
                        println!("   Name: {}", name);
                        if let Some(desc) = value.get("description").and_then(|v| v.as_str()) {
                            println!("   Description: {}", desc);
                        }
                        if let Some(steps) = value.get("steps").and_then(|v| v.as_array()) {
                            println!("   Steps: {}", steps.len());
                        }
                    }

                    if !warnings.is_empty() {
                        println!("\n\u{26a0}\u{fe0f}  Warnings:");
                        for warning in warnings {
                            println!("   - {}", warning);
                        }
                    }
                }
                Err(error_msg) => {
                    println!("\u{274c} Orchestration file validation failed:\n");
                    println!("{}", error_msg);
                    return Err("Invalid orchestration file".into());
                }
            }
        }

        OrchestrateCommands::Template { output, format } => {
            println!("\u{1f4dd} Generating orchestration template...");

            let template = if format == "json" {
                serde_json::to_string_pretty(&serde_json::json!({
                    "name": "example_orchestration",
                    "description": "Example chaos orchestration",
                    "steps": [
                        {
                            "name": "warmup",
                            "scenario": {
                                "name": "network_degradation",
                                "config": {
                                    "enabled": true,
                                    "latency": {
                                        "enabled": true,
                                        "fixed_delay_ms": 100
                                    }
                                }
                            },
                            "duration_seconds": 60,
                            "delay_before_seconds": 0,
                            "continue_on_failure": false
                        },
                        {
                            "name": "peak_load",
                            "scenario": {
                                "name": "peak_traffic",
                                "config": {
                                    "enabled": true,
                                    "rate_limit": {
                                        "enabled": true,
                                        "requests_per_second": 100
                                    }
                                }
                            },
                            "duration_seconds": 120,
                            "delay_before_seconds": 10,
                            "continue_on_failure": true
                        }
                    ],
                    "parallel": false,
                    "loop_orchestration": false,
                    "max_iterations": 1,
                    "tags": ["example", "test"]
                }))?
            } else {
                "name: example_orchestration
description: Example chaos orchestration
steps:
  - name: warmup
    scenario:
      name: network_degradation
      config:
        enabled: true
        latency:
          enabled: true
          fixed_delay_ms: 100
    duration_seconds: 60
    delay_before_seconds: 0
    continue_on_failure: false
  - name: peak_load
    scenario:
      name: peak_traffic
      config:
        enabled: true
        rate_limit:
          enabled: true
          requests_per_second: 100
    duration_seconds: 120
    delay_before_seconds: 10
    continue_on_failure: true
parallel: false
loop_orchestration: false
max_iterations: 1
tags:
  - example
  - test
"
                .to_string()
            };

            std::fs::write(&output, template)?;
            println!("\u{2705} Template saved to: {}", output.display());
        }
    }

    Ok(())
}

/// Handle AI-powered spec suggestion command
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_suggest(
    from: Option<PathBuf>,
    from_description: Option<String>,
    format: String,
    output: Option<PathBuf>,
    num_suggestions: usize,
    include_examples: bool,
    domain: Option<String>,
    llm_provider: String,
    llm_model: Option<String>,
    llm_endpoint: Option<String>,
    llm_api_key: Option<String>,
    temperature: f64,
    print_json: bool,
) -> anyhow::Result<()> {
    use mockforge_core::intelligent_behavior::{
        config::BehaviorModelConfig, OutputFormat, SpecSuggestionEngine, SuggestionConfig,
        SuggestionInput,
    };

    // Determine output format
    let output_format = format.parse::<OutputFormat>().map_err(|e| anyhow::anyhow!("{}", e))?;

    // Build LLM config
    let default_model = match llm_provider.to_lowercase().as_str() {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-3-5-sonnet-20241022",
        "ollama" => "llama3.1",
        _ => "gpt-4o-mini",
    };

    let llm_config = BehaviorModelConfig {
        llm_provider: llm_provider.clone(),
        model: llm_model.unwrap_or_else(|| default_model.to_string()),
        api_endpoint: llm_endpoint,
        api_key: llm_api_key,
        temperature,
        max_tokens: 4000,
        ..Default::default()
    };

    // Build suggestion config
    let suggestion_config = SuggestionConfig {
        llm_config,
        output_format,
        num_suggestions,
        include_examples,
        domain_hint: domain,
    };

    // Parse input
    let input = if let Some(description) = from_description {
        SuggestionInput::Description { text: description }
    } else if let Some(input_path) = from {
        let content = tokio::fs::read_to_string(&input_path).await?;
        let json_value: serde_json::Value = serde_json::from_str(&content)?;

        // Try to detect input type
        if let Some(method) = json_value.get("method").and_then(|v| v.as_str()) {
            // Single endpoint format
            SuggestionInput::Endpoint {
                method: method.to_string(),
                path: json_value
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'path' field in endpoint input"))?
                    .to_string(),
                request: json_value.get("request").cloned(),
                response: json_value.get("response").cloned(),
                description: json_value
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            }
        } else if json_value.get("openapi").is_some() || json_value.get("paths").is_some() {
            // Partial OpenAPI spec
            SuggestionInput::PartialSpec { spec: json_value }
        } else if let Some(paths_array) = json_value.get("paths").and_then(|v| v.as_array()) {
            // List of paths
            let paths = paths_array.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            SuggestionInput::Paths { paths }
        } else {
            return Err(anyhow::anyhow!(
                "Unable to detect input type. Expected 'method' field for endpoint, \
                 'openapi' for spec, or 'paths' array"
            ));
        }
    } else {
        return Err(anyhow::anyhow!(
            "Must provide either --from <file> or --from-description <text>"
        ));
    };

    println!("\u{1f916} Generating API specification suggestions...");
    println!("   Provider: {}", llm_provider);
    println!("   Model: {}", suggestion_config.llm_config.model);
    println!("   Suggestions: {}", num_suggestions);
    if let Some(ref d) = suggestion_config.domain_hint {
        println!("   Domain: {}", d);
    }
    println!();

    // Create engine and generate suggestions
    let engine = SpecSuggestionEngine::new(suggestion_config);
    let result = engine.suggest(&input).await?;

    // Print results
    if print_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\u{2705} Generated {} endpoint suggestions", result.metadata.endpoint_count);
        if let Some(domain) = &result.metadata.detected_domain {
            println!("   Detected domain: {}", domain);
        }
        println!();

        // Print endpoint suggestions
        println!("\u{1f4dd} Suggested Endpoints:");
        for (i, suggestion) in result.suggestions.iter().enumerate() {
            println!("\n{}. {} {}", i + 1, suggestion.method, suggestion.path);
            println!("   {}", suggestion.description);
            if !suggestion.parameters.is_empty() {
                println!("   Parameters:");
                for param in &suggestion.parameters {
                    let req = if param.required {
                        "required"
                    } else {
                        "optional"
                    };
                    println!(
                        "     - {} ({}): {} [{}]",
                        param.name, param.location, param.data_type, req
                    );
                }
            }
            if !suggestion.reasoning.is_empty() {
                println!("   \u{1f4a1} {}", suggestion.reasoning);
            }
        }
        println!();

        // Save specs to file(s)
        if let Some(base_path) = output {
            match output_format {
                OutputFormat::OpenAPI => {
                    if let Some(spec) = &result.openapi_spec {
                        let yaml = serde_yaml::to_string(spec)?;
                        tokio::fs::write(&base_path, yaml).await?;
                        println!("\u{2705} OpenAPI spec saved to: {}", base_path.display());
                    } else {
                        println!("\u{26a0}\u{fe0f}  No OpenAPI spec generated");
                    }
                }
                OutputFormat::MockForge => {
                    if let Some(config) = &result.mockforge_config {
                        let yaml = serde_yaml::to_string(config)?;
                        tokio::fs::write(&base_path, yaml).await?;
                        println!("\u{2705} MockForge config saved to: {}", base_path.display());
                    } else {
                        println!("\u{26a0}\u{fe0f}  No MockForge config generated");
                    }
                }
                OutputFormat::Both => {
                    // Save both with different extensions
                    let openapi_path = base_path.with_extension("openapi.yaml");
                    let mockforge_path = base_path.with_extension("mockforge.yaml");

                    if let Some(spec) = &result.openapi_spec {
                        let yaml = serde_yaml::to_string(spec)?;
                        tokio::fs::write(&openapi_path, yaml).await?;
                        println!("\u{2705} OpenAPI spec saved to: {}", openapi_path.display());
                    }

                    if let Some(config) = &result.mockforge_config {
                        let yaml = serde_yaml::to_string(config)?;
                        tokio::fs::write(&mockforge_path, yaml).await?;
                        println!(
                            "\u{2705} MockForge config saved to: {}",
                            mockforge_path.display()
                        );
                    }
                }
            }
        } else {
            println!("\u{1f4a1} Tip: Use --output <file> to save the generated specification");
        }
    }

    Ok(())
}
