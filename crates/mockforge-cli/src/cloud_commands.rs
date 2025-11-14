//! Cloud sync commands for MockForge Cloud integration
//!
//! Provides commands for authenticating with MockForge Cloud, syncing workspaces,
//! managing cloud workspaces, and team collaboration features.

use anyhow::{Context, Result};
use colored::*;
use mockforge_core::workspace::sync::{SyncConfig, SyncDirection, SyncProvider};
use mockforge_core::SyncService;
use serde_json::json;
use std::path::PathBuf;
use tracing::{info, warn};

/// Cloud command subcommands
#[derive(clap::Subcommand)]
pub enum CloudCommands {
    /// Authenticate with MockForge Cloud
    ///
    /// Examples:
    ///   mockforge cloud login
    ///   mockforge cloud login --token <api-token>
    ///   mockforge cloud login --provider github
    #[command(verbatim_doc_comment)]
    Login {
        /// API token for authentication
        #[arg(long)]
        token: Option<String>,

        /// OAuth provider (github, google)
        #[arg(long)]
        provider: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Check authentication status
    ///
    /// Examples:
    ///   mockforge cloud whoami
    #[command(verbatim_doc_comment)]
    Whoami {
        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Logout from MockForge Cloud
    ///
    /// Examples:
    ///   mockforge cloud logout
    #[command(verbatim_doc_comment)]
    Logout {},

    /// Sync commands
    Sync {
        #[command(subcommand)]
        sync_command: SyncCommands,
    },

    /// Workspace management commands
    Workspace {
        #[command(subcommand)]
        workspace_command: CloudWorkspaceCommands,
    },

    /// Team collaboration commands
    Team {
        #[command(subcommand)]
        team_command: TeamCommands,
    },

    /// View activity feed
    ///
    /// Examples:
    ///   mockforge cloud activity --workspace my-workspace
    #[command(verbatim_doc_comment)]
    Activity {
        /// Workspace ID
        #[arg(long)]
        workspace: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,

        /// Number of activities to show
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}

/// Sync command subcommands
#[derive(clap::Subcommand)]
pub enum SyncCommands {
    /// Start syncing a workspace
    ///
    /// Examples:
    ///   mockforge cloud sync --workspace my-workspace
    ///   mockforge cloud sync --all
    ///   mockforge cloud sync --workspace my-workspace --watch
    #[command(verbatim_doc_comment)]
    Start {
        /// Workspace ID to sync
        #[arg(long)]
        workspace: Option<String>,

        /// Sync all workspaces
        #[arg(long)]
        all: bool,

        /// Project ID
        #[arg(long)]
        project: Option<String>,

        /// Watch for file changes and auto-sync
        #[arg(long)]
        watch: bool,

        /// Conflict resolution strategy (local, remote, merge, manual)
        #[arg(long, default_value = "merge")]
        strategy: String,

        /// Sync direction (up, down, both)
        #[arg(long, default_value = "both")]
        direction: String,

        /// Local workspace directory
        #[arg(long)]
        local_dir: Option<PathBuf>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Check sync status
    ///
    /// Examples:
    ///   mockforge cloud sync status
    #[command(verbatim_doc_comment)]
    Status {
        /// Workspace ID
        #[arg(long)]
        workspace: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// View sync history
    ///
    /// Examples:
    ///   mockforge cloud sync history --workspace my-workspace
    #[command(verbatim_doc_comment)]
    History {
        /// Workspace ID
        #[arg(long)]
        workspace: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,

        /// Number of history entries to show
        #[arg(long, default_value = "20")]
        limit: u32,
    },

    /// View pending changes
    ///
    /// Examples:
    ///   mockforge cloud sync pending
    #[command(verbatim_doc_comment)]
    Pending {
        /// Workspace ID
        #[arg(long)]
        workspace: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },
}

/// Cloud workspace management commands
#[derive(clap::Subcommand)]
pub enum CloudWorkspaceCommands {
    /// List cloud workspaces
    ///
    /// Examples:
    ///   mockforge cloud workspace list
    #[command(verbatim_doc_comment)]
    List {
        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Create a cloud workspace
    ///
    /// Examples:
    ///   mockforge cloud workspace create my-workspace --name "My Workspace"
    #[command(verbatim_doc_comment)]
    Create {
        /// Workspace ID
        workspace_id: String,

        /// Workspace name
        #[arg(long)]
        name: String,

        /// Workspace description
        #[arg(long)]
        description: Option<String>,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Link local workspace to cloud
    ///
    /// Examples:
    ///   mockforge cloud workspace link local-workspace cloud-workspace-id
    #[command(verbatim_doc_comment)]
    Link {
        /// Local workspace path
        local_workspace: PathBuf,

        /// Cloud workspace ID
        cloud_workspace_id: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Unlink workspace from cloud
    ///
    /// Examples:
    ///   mockforge cloud workspace unlink local-workspace
    #[command(verbatim_doc_comment)]
    Unlink {
        /// Local workspace path
        local_workspace: PathBuf,
    },

    /// View workspace information
    ///
    /// Examples:
    ///   mockforge cloud workspace info cloud-workspace-id
    #[command(verbatim_doc_comment)]
    Info {
        /// Cloud workspace ID
        workspace_id: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },
}

/// Team collaboration commands
#[derive(clap::Subcommand)]
pub enum TeamCommands {
    /// List team members
    ///
    /// Examples:
    ///   mockforge cloud team members --workspace my-workspace
    #[command(verbatim_doc_comment)]
    Members {
        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Invite team member
    ///
    /// Examples:
    ///   mockforge cloud team invite user@example.com --workspace my-workspace --role editor
    #[command(verbatim_doc_comment)]
    Invite {
        /// Email address
        email: String,

        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Role (admin, editor, viewer)
        #[arg(long, default_value = "editor")]
        role: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Remove team member
    ///
    /// Examples:
    ///   mockforge cloud team remove user@example.com --workspace my-workspace
    #[command(verbatim_doc_comment)]
    Remove {
        /// Email address
        email: String,

        /// Workspace ID
        #[arg(long)]
        workspace: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },
}

/// Handle cloud commands
pub async fn handle_cloud_command(cmd: CloudCommands) -> Result<()> {
    match cmd {
        CloudCommands::Login {
            token,
            provider,
            service_url,
        } => handle_login(token, provider, service_url).await,
        CloudCommands::Whoami { service_url } => handle_whoami(service_url).await,
        CloudCommands::Logout {} => handle_logout().await,
        CloudCommands::Sync { sync_command } => handle_sync_command(sync_command).await,
        CloudCommands::Workspace { workspace_command } => {
            handle_cloud_workspace_command(workspace_command).await
        }
        CloudCommands::Team { team_command } => handle_team_command(team_command).await,
        CloudCommands::Activity {
            workspace,
            service_url,
            limit,
        } => handle_activity(workspace, service_url, limit).await,
    }
}

/// Handle login command
async fn handle_login(
    token: Option<String>,
    provider: Option<String>,
    service_url: String,
) -> Result<()> {
    info!("Authenticating with MockForge Cloud at {}", service_url);

    // Get API token from various sources
    let api_token = token
        .or_else(|| std::env::var("MOCKFORGE_API_KEY").ok())
        .or_else(|| {
            // Try to read from config file
            let config_path = dirs::home_dir()
                .map(|p| p.join(".mockforge").join("cloud.json"))
                .unwrap_or_else(|| PathBuf::from(".mockforge/cloud.json"));

            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        return config.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
            }
            None
        });

    if let Some(provider_name) = provider {
        // OAuth flow
        println!("{}", "🔐 OAuth authentication not yet implemented".yellow());
        println!("   Provider: {}", provider_name);
        println!("   Service URL: {}", service_url);
        println!();
        println!("{}", "Please use --token or set MOCKFORGE_API_KEY environment variable".yellow());
        return Ok(());
    }

    if let Some(token) = api_token {
        // Validate token by making a test API call
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/auth/verify", service_url))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to verify token with cloud service")?;

        if response.status().is_success() {
            // Save token to config file
            let config_dir = dirs::home_dir()
                .map(|p| p.join(".mockforge"))
                .unwrap_or_else(|| PathBuf::from(".mockforge"));

            std::fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;

            let config_path = config_dir.join("cloud.json");
            let config = json!({
                "api_key": token,
                "service_url": service_url,
                "authenticated_at": chrono::Utc::now().to_rfc3339(),
            });

            std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
                .context("Failed to save authentication config")?;

            println!("{}", "✅ Successfully authenticated with MockForge Cloud".green());
            println!("   Config saved to: {}", config_path.display());
        } else {
            return Err(anyhow::anyhow!("Authentication failed: Invalid token"));
        }
    } else {
        // Interactive login
        println!("{}", "🔐 Interactive login not yet implemented".yellow());
        println!();
        println!("Please provide an API token:");
        println!("  mockforge cloud login --token <your-token>");
        println!();
        println!("Or set the MOCKFORGE_API_KEY environment variable");
    }

    Ok(())
}

/// Handle whoami command
async fn handle_whoami(service_url: String) -> Result<()> {
    // Read config file
    let config_path = dirs::home_dir()
        .map(|p| p.join(".mockforge").join("cloud.json"))
        .unwrap_or_else(|| PathBuf::from(".mockforge/cloud.json"));

    if !config_path.exists() {
        println!("{}", "❌ Not authenticated".red());
        println!("   Run 'mockforge cloud login' to authenticate");
        return Ok(());
    }

    let config_content = std::fs::read_to_string(&config_path)
        .context("Failed to read config file")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)
        .context("Failed to parse config file")?;

    let api_key = config
        .get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No API key found in config"))?;

    // Verify token and get user info
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/api/v1/auth/me", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch user info")?;

    if response.status().is_success() {
        let user_info: serde_json::Value = response.json().await?;
        println!("{}", "✅ Authenticated".green());
        println!("   Email: {}", user_info.get("email").and_then(|v| v.as_str()).unwrap_or("N/A"));
        println!("   Service URL: {}", service_url);
        if let Some(authenticated_at) = config.get("authenticated_at").and_then(|v| v.as_str()) {
            println!("   Authenticated at: {}", authenticated_at);
        }
    } else {
        println!("{}", "❌ Authentication expired or invalid".red());
        println!("   Run 'mockforge cloud login' to re-authenticate");
    }

    Ok(())
}

/// Handle logout command
async fn handle_logout() -> Result<()> {
    let config_path = dirs::home_dir()
        .map(|p| p.join(".mockforge").join("cloud.json"))
        .unwrap_or_else(|| PathBuf::from(".mockforge/cloud.json"));

    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .context("Failed to remove config file")?;
        println!("{}", "✅ Logged out successfully".green());
    } else {
        println!("{}", "ℹ️  Not logged in".yellow());
    }

    Ok(())
}

/// Handle sync commands
async fn handle_sync_command(cmd: SyncCommands) -> Result<()> {
    match cmd {
        SyncCommands::Start {
            workspace,
            all,
            project,
            watch,
            strategy,
            direction,
            local_dir,
            service_url,
        } => {
            handle_sync_start(workspace, all, project, watch, strategy, direction, local_dir, service_url).await
        }
        SyncCommands::Status { workspace, service_url } => {
            handle_sync_status(workspace, service_url).await
        }
        SyncCommands::History {
            workspace,
            service_url,
            limit,
        } => handle_sync_history(workspace, service_url, limit).await,
        SyncCommands::Pending {
            workspace,
            service_url,
        } => handle_sync_pending(workspace, service_url).await,
    }
}

/// Handle sync start command
async fn handle_sync_start(
    workspace: Option<String>,
    all: bool,
    project: Option<String>,
    watch: bool,
    strategy: String,
    direction: String,
    local_dir: Option<PathBuf>,
    service_url: String,
) -> Result<()> {
    // Get API key from config
    let api_key = get_api_key()?;

    // Determine sync direction
    let sync_direction = match direction.as_str() {
        "up" => SyncDirection::LocalToRemote,
        "down" => SyncDirection::RemoteToLocal,
        "both" => SyncDirection::Bidirectional,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid direction: {}. Must be 'up', 'down', or 'both'",
                direction
            ));
        }
    };

    // Create sync config
    let sync_config = SyncConfig {
        enabled: true,
        provider: SyncProvider::Cloud {
            service_url: service_url.clone(),
            api_key: api_key.clone(),
            project_id: project.unwrap_or_else(|| "default".to_string()),
        },
        interval_seconds: if watch { 5 } else { 60 },
        conflict_strategy: match strategy.as_str() {
            "local" => mockforge_core::workspace::sync::ConflictResolutionStrategy::LocalWins,
            "remote" => mockforge_core::workspace::sync::ConflictResolutionStrategy::RemoteWins,
            "merge" => mockforge_core::workspace::sync::ConflictResolutionStrategy::LastModified,
            "manual" => mockforge_core::workspace::sync::ConflictResolutionStrategy::Manual,
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid strategy: {}. Must be 'local', 'remote', 'merge', or 'manual'",
                    strategy
                ));
            }
        },
        auto_commit: true,
        auto_push: true,
        directory_structure: mockforge_core::workspace::sync::SyncDirectoryStructure::PerWorkspace,
        sync_direction,
    };

    let local_workspace_dir = local_dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    if all {
        println!("{}", "🔄 Syncing all workspaces...".cyan());
        // TODO: Implement sync all workspaces
        println!("{}", "⚠️  Syncing all workspaces not yet fully implemented".yellow());
    } else if let Some(workspace_id) = workspace {
        println!("{}", format!("🔄 Syncing workspace: {}", workspace_id).cyan());

        // Create sync service
        // Create sync service with workspace directory
        let sync_service = SyncService::new(&local_workspace_dir);

        // Start sync service
        sync_service.start().await
            .context("Failed to start sync service")?;

        // Start monitoring workspace if watch is enabled
        if watch {
            println!("{}", "👀 Watching for file changes...".cyan());
            sync_service.monitor_workspace(&workspace_id, &local_workspace_dir.to_string_lossy())
                .await
                .context("Failed to start monitoring workspace")?;
            println!("{}", "✅ File watching started".green());
        }

        // TODO: Implement actual sync operation
        // For now, just verify the service is running
        println!("{}", "✅ Sync service started successfully".green());
        println!("   Workspace: {}", workspace_id);
        println!("   Directory: {}", local_workspace_dir.display());

        // Note: Actual sync implementation would go here
        // This is a placeholder for the cloud sync functionality
    } else {
        return Err(anyhow::anyhow!(
            "Either --workspace or --all must be specified"
        ));
    }

    Ok(())
}

/// Handle sync status command
async fn handle_sync_status(workspace: Option<String>, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let url = if let Some(ws) = workspace {
        format!("{}/api/v1/sync/status?workspace={}", service_url, ws)
    } else {
        format!("{}/api/v1/sync/status", service_url)
    };

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch sync status")?;

    if response.status().is_success() {
        let status: serde_json::Value = response.json().await?;
        println!("{}", "📊 Sync Status".cyan());
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("{}", "❌ Failed to fetch sync status".red());
    }

    Ok(())
}

/// Handle sync history command
async fn handle_sync_history(
    workspace: Option<String>,
    service_url: String,
    limit: u32,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let mut url = format!("{}/api/v1/sync/history?limit={}", service_url, limit);
    if let Some(ws) = workspace {
        url.push_str(&format!("&workspace={}", ws));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch sync history")?;

    if response.status().is_success() {
        let history: serde_json::Value = response.json().await?;
        println!("{}", "📜 Sync History".cyan());
        println!("{}", serde_json::to_string_pretty(&history)?);
    } else {
        println!("{}", "❌ Failed to fetch sync history".red());
    }

    Ok(())
}

/// Handle sync pending command
async fn handle_sync_pending(workspace: Option<String>, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let mut url = format!("{}/api/v1/sync/pending", service_url);
    if let Some(ws) = workspace {
        url.push_str(&format!("?workspace={}", ws));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch pending changes")?;

    if response.status().is_success() {
        let pending: serde_json::Value = response.json().await?;
        println!("{}", "⏳ Pending Changes".cyan());
        println!("{}", serde_json::to_string_pretty(&pending)?);
    } else {
        println!("{}", "❌ Failed to fetch pending changes".red());
    }

    Ok(())
}

/// Handle cloud workspace commands
async fn handle_cloud_workspace_command(cmd: CloudWorkspaceCommands) -> Result<()> {
    match cmd {
        CloudWorkspaceCommands::List { service_url } => {
            handle_cloud_workspace_list(service_url).await
        }
        CloudWorkspaceCommands::Create {
            workspace_id,
            name,
            description,
            service_url,
        } => handle_cloud_workspace_create(workspace_id, name, description, service_url).await,
        CloudWorkspaceCommands::Link {
            local_workspace,
            cloud_workspace_id,
            service_url: _,
        } => handle_cloud_workspace_link(local_workspace, cloud_workspace_id).await,
        CloudWorkspaceCommands::Unlink { local_workspace } => {
            handle_cloud_workspace_unlink(local_workspace).await
        }
        CloudWorkspaceCommands::Info {
            workspace_id,
            service_url,
        } => handle_cloud_workspace_info(workspace_id, service_url).await,
    }
}

/// Handle cloud workspace list command
async fn handle_cloud_workspace_list(service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/v1/workspaces", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch workspaces")?;

    if response.status().is_success() {
        let workspaces: serde_json::Value = response.json().await?;
        println!("{}", "📁 Cloud Workspaces".cyan());
        println!("{}", serde_json::to_string_pretty(&workspaces)?);
    } else {
        println!("{}", "❌ Failed to fetch workspaces".red());
    }

    Ok(())
}

/// Handle cloud workspace create command
async fn handle_cloud_workspace_create(
    workspace_id: String,
    name: String,
    description: Option<String>,
    service_url: String,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let payload = json!({
        "id": workspace_id,
        "name": name,
        "description": description,
    });

    let response = client
        .post(&format!("{}/api/v1/workspaces", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to create workspace")?;

    if response.status().is_success() {
        let workspace: serde_json::Value = response.json().await?;
        println!("{}", "✅ Workspace created successfully".green());
        println!("{}", serde_json::to_string_pretty(&workspace)?);
    } else {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Failed to create workspace: {}", error_text));
    }

    Ok(())
}

/// Handle cloud workspace link command
async fn handle_cloud_workspace_link(
    local_workspace: PathBuf,
    cloud_workspace_id: String,
) -> Result<()> {
    // Create or update .mockforge/sync.yaml
    let sync_config_path = local_workspace.join(".mockforge").join("sync.yaml");
    std::fs::create_dir_all(sync_config_path.parent().unwrap())
        .context("Failed to create .mockforge directory")?;

    // TODO: Read existing config and update it
    println!("{}", format!("🔗 Linking local workspace to cloud workspace: {}", cloud_workspace_id).cyan());
    println!("   Local: {}", local_workspace.display());
    println!("   Cloud: {}", cloud_workspace_id);
    println!("{}", "⚠️  Workspace linking not yet fully implemented".yellow());

    Ok(())
}

/// Handle cloud workspace unlink command
async fn handle_cloud_workspace_unlink(local_workspace: PathBuf) -> Result<()> {
    let sync_config_path = local_workspace.join(".mockforge").join("sync.yaml");

    if sync_config_path.exists() {
        // TODO: Update config to remove cloud link
        println!("{}", "🔓 Unlinking workspace from cloud".cyan());
        println!("{}", "⚠️  Workspace unlinking not yet fully implemented".yellow());
    } else {
        println!("{}", "ℹ️  No sync configuration found".yellow());
    }

    Ok(())
}

/// Handle cloud workspace info command
async fn handle_cloud_workspace_info(workspace_id: String, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/v1/workspaces/{}", service_url, workspace_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch workspace info")?;

    if response.status().is_success() {
        let workspace: serde_json::Value = response.json().await?;
        println!("{}", "📁 Workspace Information".cyan());
        println!("{}", serde_json::to_string_pretty(&workspace)?);
    } else {
        println!("{}", "❌ Failed to fetch workspace info".red());
    }

    Ok(())
}

/// Handle team commands
async fn handle_team_command(cmd: TeamCommands) -> Result<()> {
    match cmd {
        TeamCommands::Members { workspace, service_url } => {
            handle_team_members(workspace, service_url).await
        }
        TeamCommands::Invite {
            email,
            workspace,
            role,
            service_url,
        } => handle_team_invite(email, workspace, role, service_url).await,
        TeamCommands::Remove {
            email,
            workspace,
            service_url,
        } => handle_team_remove(email, workspace, service_url).await,
    }
}

/// Handle team members command
async fn handle_team_members(workspace: String, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/api/v1/workspaces/{}/members", service_url, workspace))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch team members")?;

    if response.status().is_success() {
        let members: serde_json::Value = response.json().await?;
        println!("{}", "👥 Team Members".cyan());
        println!("{}", serde_json::to_string_pretty(&members)?);
    } else {
        println!("{}", "❌ Failed to fetch team members".red());
    }

    Ok(())
}

/// Handle team invite command
async fn handle_team_invite(
    email: String,
    workspace: String,
    role: String,
    service_url: String,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let payload = json!({
        "email": email,
        "role": role,
    });

    let response = client
        .post(&format!("{}/api/v1/workspaces/{}/members", service_url, workspace))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to invite team member")?;

    if response.status().is_success() {
        println!("{}", format!("✅ Invited {} to workspace {}", email, workspace).green());
    } else {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Failed to invite team member: {}", error_text));
    }

    Ok(())
}

/// Handle team remove command
async fn handle_team_remove(
    email: String,
    workspace: String,
    service_url: String,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .delete(&format!("{}/api/v1/workspaces/{}/members/{}", service_url, workspace, email))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to remove team member")?;

    if response.status().is_success() {
        println!("{}", format!("✅ Removed {} from workspace {}", email, workspace).green());
    } else {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("Failed to remove team member: {}", error_text));
    }

    Ok(())
}

/// Handle activity command
async fn handle_activity(
    workspace: Option<String>,
    service_url: String,
    limit: u32,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let mut url = format!("{}/api/v1/activity?limit={}", service_url, limit);
    if let Some(ws) = workspace {
        url.push_str(&format!("&workspace={}", ws));
    }

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch activity")?;

    if response.status().is_success() {
        let activity: serde_json::Value = response.json().await?;
        println!("{}", "📊 Activity Feed".cyan());
        println!("{}", serde_json::to_string_pretty(&activity)?);
    } else {
        println!("{}", "❌ Failed to fetch activity".red());
    }

    Ok(())
}

/// Get API key from config or environment
fn get_api_key() -> Result<String> {
    // Try environment variable first
    if let Ok(key) = std::env::var("MOCKFORGE_API_KEY") {
        return Ok(key);
    }

    // Try config file
    let config_path = dirs::home_dir()
        .map(|p| p.join(".mockforge").join("cloud.json"))
        .unwrap_or_else(|| PathBuf::from(".mockforge/cloud.json"));

    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
        let config: serde_json::Value = serde_json::from_str(&config_content)
            .context("Failed to parse config file")?;

        if let Some(api_key) = config.get("api_key").and_then(|v| v.as_str()) {
            return Ok(api_key.to_string());
        }
    }

    Err(anyhow::anyhow!(
        "No API key found. Run 'mockforge cloud login' or set MOCKFORGE_API_KEY environment variable"
    ))
}
