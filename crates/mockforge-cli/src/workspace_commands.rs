//! Workspace management commands for multi-tenant MockForge

use clap::Subcommand;
use colored::*;
use serde_json::json;

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    /// List all workspaces
    ///
    /// Examples:
    ///   mockforge workspace list
    ///   mockforge workspace list --admin-url http://localhost:9080
    #[command(verbatim_doc_comment)]
    List {
        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Create a new workspace
    ///
    /// Examples:
    ///   mockforge workspace create my-workspace --name "My Workspace"
    ///   mockforge workspace create frontend-dev --name "Frontend Development" --description "Frontend team mocks"
    #[command(verbatim_doc_comment)]
    Create {
        /// Workspace ID (unique identifier)
        workspace_id: String,

        /// Workspace name
        #[arg(long)]
        name: String,

        /// Workspace description
        #[arg(long)]
        description: Option<String>,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,
    },

    /// Get workspace information
    ///
    /// Examples:
    ///   mockforge workspace info my-workspace
    ///   mockforge workspace info frontend-dev --format json
    #[command(verbatim_doc_comment)]
    Info {
        /// Workspace ID
        workspace_id: String,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Delete a workspace
    ///
    /// Examples:
    ///   mockforge workspace delete my-workspace
    ///   mockforge workspace delete frontend-dev --yes
    #[command(verbatim_doc_comment)]
    Delete {
        /// Workspace ID
        workspace_id: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,
    },

    /// Enable a workspace
    ///
    /// Examples:
    ///   mockforge workspace enable my-workspace
    #[command(verbatim_doc_comment)]
    Enable {
        /// Workspace ID
        workspace_id: String,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,
    },

    /// Disable a workspace
    ///
    /// Examples:
    ///   mockforge workspace disable my-workspace
    #[command(verbatim_doc_comment)]
    Disable {
        /// Workspace ID
        workspace_id: String,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,
    },

    /// Update workspace details
    ///
    /// Examples:
    ///   mockforge workspace update my-workspace --name "Updated Name"
    ///   mockforge workspace update frontend-dev --description "New description"
    #[command(verbatim_doc_comment)]
    Update {
        /// Workspace ID
        workspace_id: String,

        /// New workspace name
        #[arg(long)]
        name: Option<String>,

        /// New workspace description
        #[arg(long)]
        description: Option<String>,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,
    },

    /// Get workspace statistics
    ///
    /// Examples:
    ///   mockforge workspace stats my-workspace
    ///   mockforge workspace stats frontend-dev --format json
    #[command(verbatim_doc_comment)]
    Stats {
        /// Workspace ID
        workspace_id: String,

        /// Admin UI URL
        #[arg(long, default_value = "http://localhost:9080")]
        admin_url: String,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
}

pub async fn handle_workspace_command(
    command: WorkspaceCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match command {
        WorkspaceCommands::List { admin_url, format } => {
            list_workspaces(&admin_url, &format).await?;
        }
        WorkspaceCommands::Create {
            workspace_id,
            name,
            description,
            admin_url,
        } => {
            create_workspace(&admin_url, &workspace_id, &name, description.as_deref()).await?;
        }
        WorkspaceCommands::Info {
            workspace_id,
            admin_url,
            format,
        } => {
            get_workspace_info(&admin_url, &workspace_id, &format).await?;
        }
        WorkspaceCommands::Delete {
            workspace_id,
            yes,
            admin_url,
        } => {
            delete_workspace(&admin_url, &workspace_id, yes).await?;
        }
        WorkspaceCommands::Enable {
            workspace_id,
            admin_url,
        } => {
            update_workspace_enabled(&admin_url, &workspace_id, true).await?;
        }
        WorkspaceCommands::Disable {
            workspace_id,
            admin_url,
        } => {
            update_workspace_enabled(&admin_url, &workspace_id, false).await?;
        }
        WorkspaceCommands::Update {
            workspace_id,
            name,
            description,
            admin_url,
        } => {
            update_workspace(&admin_url, &workspace_id, name.as_deref(), description.as_deref())
                .await?;
        }
        WorkspaceCommands::Stats {
            workspace_id,
            admin_url,
            format,
        } => {
            get_workspace_stats(&admin_url, &workspace_id, &format).await?;
        }
    }

    Ok(())
}

async fn list_workspaces(
    admin_url: &str,
    format: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces", admin_url);
    let client = reqwest::Client::new();

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        eprintln!("{} Failed to list workspaces: {}", "✗".red(), response.status());
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let body: serde_json::Value = response.json().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }

    // Table format
    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        if data.is_empty() {
            println!("{}", "No workspaces found.".yellow());
            return Ok(());
        }

        println!("{}", "Workspaces:".bold());
        println!();
        println!(
            "{:<20} {:<30} {:<10} {:<10} {:<15}",
            "ID".bold(),
            "Name".bold(),
            "Enabled".bold(),
            "Requests".bold(),
            "Routes".bold()
        );
        println!("{}", "-".repeat(90));

        for workspace in data {
            let id = workspace["id"].as_str().unwrap_or("N/A");
            let name = workspace["name"].as_str().unwrap_or("N/A");
            let enabled = workspace["enabled"].as_bool().unwrap_or(false);
            let stats = &workspace["stats"];
            let total_requests = stats["total_requests"].as_u64().unwrap_or(0);
            let active_routes = stats["active_routes"].as_u64().unwrap_or(0);

            let enabled_str = if enabled { "Yes".green() } else { "No".red() };

            println!(
                "{:<20} {:<30} {:<10} {:<10} {:<15}",
                id.cyan(),
                name,
                enabled_str,
                total_requests,
                active_routes
            );
        }
    }

    Ok(())
}

async fn create_workspace(
    admin_url: &str,
    workspace_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces", admin_url);
    let client = reqwest::Client::new();

    let body = json!({
        "id": workspace_id,
        "name": name,
        "description": description,
    });

    let response = client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or(json!({}));
        let error_msg = error_body["error"].as_str().unwrap_or("Unknown error");
        eprintln!("{} Failed to create workspace: {}", "✗".red(), error_msg);
        return Err(format!("Failed to create workspace: {}", error_msg).into());
    }

    println!("{} Workspace '{}' created successfully", "✓".green(), workspace_id.cyan());

    Ok(())
}

async fn get_workspace_info(
    admin_url: &str,
    workspace_id: &str,
    format: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces/{}", admin_url, workspace_id);
    let client = reqwest::Client::new();

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        eprintln!("{} Workspace '{}' not found", "✗".red(), workspace_id);
        return Err(format!("Workspace not found: {}", workspace_id).into());
    }

    let body: serde_json::Value = response.json().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }

    // Table format
    if let Some(data) = body.get("data") {
        println!("{}", "Workspace Information:".bold());
        println!();
        println!("  {:<20} {}", "ID:".bold(), data["id"].as_str().unwrap_or("N/A").cyan());
        println!("  {:<20} {}", "Name:".bold(), data["name"].as_str().unwrap_or("N/A"));

        if let Some(desc) = data["description"].as_str() {
            println!("  {:<20} {}", "Description:".bold(), desc);
        }

        let enabled = data["enabled"].as_bool().unwrap_or(false);
        let enabled_str = if enabled { "Yes".green() } else { "No".red() };
        println!("  {:<20} {}", "Enabled:".bold(), enabled_str);

        println!();
        println!("{}", "Statistics:".bold());
        if let Some(stats) = data.get("stats") {
            println!(
                "  {:<20} {}",
                "Total Requests:".bold(),
                stats["total_requests"].as_u64().unwrap_or(0)
            );
            println!(
                "  {:<20} {}",
                "Active Routes:".bold(),
                stats["active_routes"].as_u64().unwrap_or(0)
            );
            println!(
                "  {:<20} {:.2} ms",
                "Avg Response Time:".bold(),
                stats["avg_response_time_ms"].as_f64().unwrap_or(0.0)
            );
        }
    }

    Ok(())
}

async fn delete_workspace(
    admin_url: &str,
    workspace_id: &str,
    skip_confirm: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !skip_confirm {
        print!("Are you sure you want to delete workspace '{}'? (y/N): ", workspace_id.cyan());
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let url = format!("{}/__mockforge/workspaces/{}", admin_url, workspace_id);
    let client = reqwest::Client::new();

    let response = client.delete(&url).send().await?;

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or(json!({}));
        let error_msg = error_body["error"].as_str().unwrap_or("Unknown error");
        eprintln!("{} Failed to delete workspace: {}", "✗".red(), error_msg);
        return Err(format!("Failed to delete workspace: {}", error_msg).into());
    }

    println!("{} Workspace '{}' deleted successfully", "✓".green(), workspace_id.cyan());

    Ok(())
}

async fn update_workspace_enabled(
    admin_url: &str,
    workspace_id: &str,
    enabled: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces/{}", admin_url, workspace_id);
    let client = reqwest::Client::new();

    let body = json!({
        "enabled": enabled,
    });

    let response = client.put(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or(json!({}));
        let error_msg = error_body["error"].as_str().unwrap_or("Unknown error");
        eprintln!("{} Failed to update workspace: {}", "✗".red(), error_msg);
        return Err(format!("Failed to update workspace: {}", error_msg).into());
    }

    let status = if enabled { "enabled" } else { "disabled" };
    println!("{} Workspace '{}' {} successfully", "✓".green(), workspace_id.cyan(), status);

    Ok(())
}

async fn update_workspace(
    admin_url: &str,
    workspace_id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces/{}", admin_url, workspace_id);
    let client = reqwest::Client::new();

    let mut body = json!({});

    if let Some(n) = name {
        body["name"] = json!(n);
    }

    if let Some(d) = description {
        body["description"] = json!(d);
    }

    let response = client.put(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or(json!({}));
        let error_msg = error_body["error"].as_str().unwrap_or("Unknown error");
        eprintln!("{} Failed to update workspace: {}", "✗".red(), error_msg);
        return Err(format!("Failed to update workspace: {}", error_msg).into());
    }

    println!("{} Workspace '{}' updated successfully", "✓".green(), workspace_id.cyan());

    Ok(())
}

async fn get_workspace_stats(
    admin_url: &str,
    workspace_id: &str,
    format: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}/__mockforge/workspaces/{}/stats", admin_url, workspace_id);
    let client = reqwest::Client::new();

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        eprintln!("{} Workspace '{}' not found", "✗".red(), workspace_id);
        return Err(format!("Workspace not found: {}", workspace_id).into());
    }

    let body: serde_json::Value = response.json().await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&body)?);
        return Ok(());
    }

    // Table format
    if let Some(data) = body.get("data") {
        println!("{}", format!("Statistics for workspace '{}':", workspace_id).bold());
        println!();
        println!(
            "  {:<25} {}",
            "Total Requests:".bold(),
            data["total_requests"].as_u64().unwrap_or(0)
        );
        println!(
            "  {:<25} {}",
            "Active Routes:".bold(),
            data["active_routes"].as_u64().unwrap_or(0)
        );
        println!(
            "  {:<25} {:.2} ms",
            "Avg Response Time:".bold(),
            data["avg_response_time_ms"].as_f64().unwrap_or(0.0)
        );

        if let Some(last_request) = data["last_request_at"].as_str() {
            println!("  {:<25} {}", "Last Request:".bold(), last_request);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_commands_list_variant() {
        let _cmd = WorkspaceCommands::List {
            admin_url: "http://localhost:9080".to_string(),
            format: "table".to_string(),
        };
    }

    #[test]
    fn test_workspace_commands_create_variant() {
        let _cmd = WorkspaceCommands::Create {
            workspace_id: "test-workspace".to_string(),
            name: "Test Workspace".to_string(),
            description: Some("A test workspace".to_string()),
            admin_url: "http://localhost:9080".to_string(),
        };
    }

    #[test]
    fn test_workspace_commands_info_variant() {
        let _cmd = WorkspaceCommands::Info {
            workspace_id: "my-workspace".to_string(),
            admin_url: "http://localhost:9080".to_string(),
            format: "table".to_string(),
        };
    }

    #[test]
    fn test_workspace_commands_delete_variant() {
        let _cmd = WorkspaceCommands::Delete {
            workspace_id: "old-workspace".to_string(),
            yes: false,
            admin_url: "http://localhost:9080".to_string(),
        };
    }

    #[test]
    fn test_workspace_list_json_format() {
        let _cmd = WorkspaceCommands::List {
            admin_url: "http://localhost:9080".to_string(),
            format: "json".to_string(),
        };
    }

    #[test]
    fn test_workspace_create_without_description() {
        let _cmd = WorkspaceCommands::Create {
            workspace_id: "simple-workspace".to_string(),
            name: "Simple Workspace".to_string(),
            description: None,
            admin_url: "http://localhost:9080".to_string(),
        };
    }

    #[test]
    fn test_workspace_delete_with_confirmation_skip() {
        let _cmd = WorkspaceCommands::Delete {
            workspace_id: "workspace-to-delete".to_string(),
            yes: true,
            admin_url: "http://localhost:9080".to_string(),
        };
    }
}
