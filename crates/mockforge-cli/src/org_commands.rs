//! Organization context management CLI commands

use clap::Subcommand;
use anyhow::{Context, Result};
use colored::Colorize;
use mockforge_plugin_registry::config::load_config;

#[derive(Subcommand, Debug, Clone)]
pub enum OrgCommands {
    /// List organizations you belong to
    List,

    /// Set the active organization context
    Use {
        /// Organization ID or slug
        org: String,
    },

    /// Show current organization context
    Current,

    /// Clear organization context (use default personal org)
    Clear,
}

/// Handle organization commands
pub async fn handle_org_command(command: OrgCommands) -> Result<()> {
    match command {
        OrgCommands::List => list_organizations().await,
        OrgCommands::Use { org } => set_active_org(&org).await,
        OrgCommands::Current => show_current_org().await,
        OrgCommands::Clear => clear_active_org().await,
    }
}

async fn list_organizations() -> Result<()> {
    let config = load_config().await.context("Failed to load registry config")?;

    if config.token.is_none() {
        anyhow::bail!("Not logged in. Run 'mockforge plugin registry login' first.");
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/organizations", config.url))
        .header("Authorization", format!("Bearer {}", config.token.as_ref().unwrap()))
        .send()
        .await
        .context("Failed to fetch organizations")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to list organizations: {}", response.status());
    }

    let orgs: Vec<serde_json::Value> = response
        .json()
        .await
        .context("Failed to parse organizations response")?;

    println!("\n{} Organizations:", "📋".blue());
    for org in orgs {
        let id = org.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = org.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let slug = org.get("slug").and_then(|v| v.as_str()).unwrap_or("unknown");
        let plan = org.get("plan").and_then(|v| v.as_str()).unwrap_or("unknown");

        println!("  • {} ({})", name.bold(), slug);
        println!("    ID: {}", id);
        println!("    Plan: {}", plan);
    }

    Ok(())
}

async fn set_active_org(org: &str) -> Result<()> {
    let mut config = load_config().await.context("Failed to load registry config")?;

    if config.token.is_none() {
        anyhow::bail!("Not logged in. Run 'mockforge plugin registry login' first.");
    }

    // Try to find org by ID or slug
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/organizations", config.url))
        .header("Authorization", format!("Bearer {}", config.token.as_ref().unwrap()))
        .send()
        .await
        .context("Failed to fetch organizations")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to list organizations: {}", response.status());
    }

    let orgs: Vec<serde_json::Value> = response
        .json()
        .await
        .context("Failed to parse organizations response")?;

    // Find matching org
    let matching_org = orgs.iter().find(|o| {
        o.get("id").and_then(|v| v.as_str()) == Some(org)
            || o.get("slug").and_then(|v| v.as_str()) == Some(org)
    });

    match matching_org {
        Some(org_data) => {
            let org_id = org_data.get("id").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Organization missing ID"))?;
            let org_name = org_data.get("name").and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Save to a separate config file for org context
            let config_dir = dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to find config directory"))?
                .join("mockforge");
            std::fs::create_dir_all(&config_dir)?;

            let org_config_path = config_dir.join("org_context.json");
            let org_config = serde_json::json!({
                "active_org_id": org_id,
                "active_org_name": org_name,
            });
            std::fs::write(&org_config_path, serde_json::to_string_pretty(&org_config)?)?;

            println!("{} Active organization set to: {} ({})",
                "✓".green(), org_name.bold(), org_id);

            Ok(())
        }
        None => {
            anyhow::bail!("Organization '{}' not found. Use 'mockforge org list' to see available organizations.", org);
        }
    }
}

async fn show_current_org() -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to find config directory"))?
        .join("mockforge");

    let org_config_path = config_dir.join("org_context.json");

    if !org_config_path.exists() {
        println!("{} No active organization set. Using default personal organization.", "ℹ️".yellow());
        return Ok(());
    }

    let org_config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&org_config_path)?
    )?;

    if let (Some(org_id), Some(org_name)) = (
        org_config.get("active_org_id").and_then(|v| v.as_str()),
        org_config.get("active_org_name").and_then(|v| v.as_str()),
    ) {
        println!("\n{} Active Organization:", "📋".blue());
        println!("  Name: {}", org_name.bold());
        println!("  ID: {}", org_id);
    } else {
        println!("{} No active organization set.", "ℹ️".yellow());
    }

    Ok(())
}

async fn clear_active_org() -> Result<()> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to find config directory"))?
        .join("mockforge");

    let org_config_path = config_dir.join("org_context.json");

    if org_config_path.exists() {
        std::fs::remove_file(&org_config_path)?;
        println!("{} Organization context cleared. Using default personal organization.", "✓".green());
    } else {
        println!("{} No active organization to clear.", "ℹ️".yellow());
    }

    Ok(())
}
