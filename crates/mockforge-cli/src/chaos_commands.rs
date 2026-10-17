//! Chaos engineering CLI commands
//!
//! CLI commands for chaos profile management.

use crate::management_auth::{management_client, print_unauthorized_hint};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum ChaosCommands {
    /// Profile management operations
    Profile {
        /// Management token for the server's control plane
        /// (sent as X-MockForge-Management-Token)
        #[arg(
            long,
            global = true,
            env = "MOCKFORGE_MANAGEMENT_TOKEN",
            hide_env_values = true
        )]
        management_token: Option<String>,
        #[command(subcommand)]
        profile_command: ProfileCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProfileCommands {
    /// Apply a network profile by name
    Apply {
        /// Profile name (e.g., slow_3g, flaky_wifi)
        name: String,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// Export a profile to JSON or YAML
    Export {
        /// Profile name to export
        name: String,
        /// Output format (json or yaml)
        #[arg(long, default_value = "json")]
        format: String,
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// Import a profile from JSON or YAML file
    Import {
        /// Input file path
        #[arg(short, long)]
        file: PathBuf,
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
    /// List all available profiles
    List {
        /// Base URL of the MockForge server (default: http://localhost:3000)
        #[arg(long, default_value = "http://localhost:3000")]
        base_url: String,
    },
}

/// Handle chaos engineering commands
pub(crate) async fn handle_chaos_command(
    chaos_command: ChaosCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match chaos_command {
        ChaosCommands::Profile {
            management_token,
            profile_command,
        } => {
            let client = management_client(management_token.as_deref())?;
            handle_profile_command(&client, profile_command).await?;
        }
    }
    Ok(())
}

/// Report a failed control-plane response and exit.
async fn fail(action: &str, response: reqwest::Response) -> ! {
    let status = response.status();
    let error_text = response.text().await.unwrap_or_default();
    eprintln!("\u{274c} Failed to {}: {} {}", action, status, error_text);
    print_unauthorized_hint(status);
    std::process::exit(1);
}

async fn handle_profile_command(
    client: &reqwest::Client,
    profile_command: ProfileCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match profile_command {
        ProfileCommands::Apply { name, base_url } => {
            println!("\u{1f527} Applying chaos profile: {}", name);
            let url = format!("{}/api/chaos/profiles/{}/apply", base_url, name);
            let response = client.post(&url).send().await?;
            if response.status().is_success() {
                println!("\u{2705} Profile '{}' applied successfully", name);
            } else {
                fail("apply profile", response).await;
            }
        }
        ProfileCommands::Export {
            name,
            format,
            output,
            base_url,
        } => {
            println!("\u{1f4e4} Exporting profile: {}", name);
            let url = format!("{}/api/chaos/profiles/{}/export?format={}", base_url, name, format);
            let response = client.get(&url).send().await?;
            if response.status().is_success() {
                let content = response.text().await?;
                if let Some(output_path) = output {
                    tokio::fs::write(&output_path, content).await?;
                    println!("\u{2705} Profile exported to: {}", output_path.display());
                } else {
                    println!("{}", content);
                }
            } else {
                fail("export profile", response).await;
            }
        }
        ProfileCommands::Import { file, base_url } => {
            println!("\u{1f4e5} Importing profile from: {}", file.display());
            let content = tokio::fs::read_to_string(&file).await?;
            let format = if file.extension().and_then(|s| s.to_str()) == Some("yaml")
                || file.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                "yaml"
            } else {
                "json"
            };
            let url = format!("{}/api/chaos/profiles/import", base_url);
            let response = client
                .post(&url)
                .json(&serde_json::json!({
                    "content": content,
                    "format": format
                }))
                .send()
                .await?;
            if response.status().is_success() {
                println!("\u{2705} Profile imported successfully");
            } else {
                fail("import profile", response).await;
            }
        }
        ProfileCommands::List { base_url } => {
            println!("\u{1f4cb} Listing available chaos profiles...");
            let url = format!("{}/api/chaos/profiles", base_url);
            let response = client.get(&url).send().await?;
            if response.status().is_success() {
                let profiles: Vec<serde_json::Value> = response.json().await?;
                println!("\nAvailable Profiles:");
                println!("{:-<80}", "");
                for profile in profiles {
                    let name = profile["name"].as_str().unwrap_or("unknown");
                    let description = profile["description"].as_str().unwrap_or("");
                    let builtin = profile["builtin"].as_bool().unwrap_or(false);
                    let tags = profile["tags"]
                        .as_array()
                        .map(|arr| {
                            arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                        })
                        .unwrap_or_default();
                    println!(
                        "  \u{2022} {} {}",
                        name,
                        if builtin { "(built-in)" } else { "(custom)" }
                    );
                    if !description.is_empty() {
                        println!("    {}", description);
                    }
                    if !tags.is_empty() {
                        println!("    Tags: {}", tags);
                    }
                    println!();
                }
            } else {
                fail("list profiles", response).await;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        chaos: ChaosCommands,
    }

    #[test]
    fn management_token_flag_parses_after_subcommand() {
        TestCli::command().debug_assert();
        let cli = TestCli::try_parse_from(["t", "profile", "list", "--management-token", "tok"])
            .expect("flag is global to profile subcommands");
        let ChaosCommands::Profile {
            management_token, ..
        } = cli.chaos;
        assert_eq!(management_token.as_deref(), Some("tok"));
    }
}
