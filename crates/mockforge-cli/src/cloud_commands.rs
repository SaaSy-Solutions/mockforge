//! Cloud sync commands for MockForge Cloud integration
//!
//! Provides commands for authenticating with MockForge Cloud, syncing workspaces,
//! managing cloud workspaces, and team collaboration features.

use anyhow::{Context, Result};
use colored::*;
use dialoguer::{Input, Password};
use mockforge_core::workspace::sync::{SyncConfig, SyncDirection, SyncProvider};
use mockforge_core::SyncService;
use serde_json::json;
use std::path::PathBuf;
use tracing::info;

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

    /// Deploy a mock service to MockForge Cloud
    ///
    /// Examples:
    ///   mockforge cloud deploy --spec api.json --name "My API"
    ///   mockforge cloud deploy --spec api.json --name "My API" --slug my-api --region iad --wait
    #[command(verbatim_doc_comment)]
    Deploy {
        /// Path to OpenAPI spec file (JSON or YAML)
        #[arg(long)]
        spec: PathBuf,

        /// Name for the deployment
        #[arg(long)]
        name: String,

        /// URL-friendly slug (auto-generated from name if not provided)
        #[arg(long)]
        slug: Option<String>,

        /// Deployment region (Fly.io region code, e.g., iad, lhr, sjc)
        #[arg(long, default_value = "iad")]
        region: String,

        /// Wait for deployment to become active
        #[arg(long)]
        wait: bool,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// List all cloud deployments
    ///
    /// Examples:
    ///   mockforge cloud deployments
    #[command(verbatim_doc_comment)]
    Deployments {
        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
    },

    /// Check status of a specific deployment
    ///
    /// Examples:
    ///   mockforge cloud deployment-status <id>
    #[command(verbatim_doc_comment, name = "deployment-status")]
    DeploymentStatus {
        /// Deployment ID
        id: String,

        /// Cloud service URL
        #[arg(long, default_value = "https://api.mockforge.dev")]
        service_url: String,
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
        CloudCommands::Deploy {
            spec,
            name,
            slug,
            region,
            wait,
            service_url,
        } => handle_deploy(spec, name, slug, region, wait, service_url).await,
        CloudCommands::Deployments { service_url } => handle_deployments(service_url).await,
        CloudCommands::DeploymentStatus { id, service_url } => {
            handle_deployment_status(id, service_url).await
        }
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
    let api_token = token.or_else(|| std::env::var("MOCKFORGE_API_KEY").ok()).or_else(|| {
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

    let api_token = if let Some(provider_name) = provider {
        println!(
            "{}",
            format!(
                "Using '{}' provider token flow. Paste an OAuth access token for this provider.",
                provider_name
            )
            .bright_blue()
        );
        api_token
            .or_else(|| std::env::var("MOCKFORGE_OAUTH_ACCESS_TOKEN").ok())
            .or_else(|| {
                Password::new()
                    .with_prompt(format!("{} access token", provider_name))
                    .allow_empty_password(false)
                    .interact()
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            })
    } else {
        api_token
    };

    if let Some(token) = api_token {
        verify_and_save_token(&service_url, &token).await?;
    } else {
        // Interactive login with username/password
        println!("{}", "Log in to MockForge Cloud".bright_blue());
        println!();

        let username: String = Input::new()
            .with_prompt("Username or email")
            .interact_text()
            .context("Failed to read username")?;

        let password = Password::new()
            .with_prompt("Password")
            .allow_empty_password(false)
            .interact()
            .context("Failed to read password")?;

        // Exchange credentials for a token
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/v1/auth/login", service_url))
            .json(&json!({
                "email": username,
                "password": password,
            }))
            .send()
            .await
            .context("Failed to connect to MockForge Cloud")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Login failed ({}): {}", status, body));
        }

        let body: serde_json::Value =
            response.json().await.context("Failed to parse login response")?;

        let token = body
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No token in login response"))?;

        verify_and_save_token(&service_url, token).await?;
    }

    Ok(())
}

async fn verify_and_save_token(service_url: &str, token: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/auth/verify", service_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to verify token with cloud service")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Authentication failed: Invalid token"));
    }

    let config_dir = dirs::home_dir()
        .map(|p| p.join(".mockforge"))
        .unwrap_or_else(|| PathBuf::from(".mockforge"));
    std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

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

    let config_content =
        std::fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: serde_json::Value =
        serde_json::from_str(&config_content).context("Failed to parse config file")?;

    let api_key = config
        .get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No API key found in config"))?;

    // Verify token and get user info
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/auth/me", service_url))
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
        std::fs::remove_file(&config_path).context("Failed to remove config file")?;
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
            handle_sync_start(
                workspace,
                all,
                project,
                watch,
                strategy,
                direction,
                local_dir,
                service_url,
            )
            .await
        }
        SyncCommands::Status {
            workspace,
            service_url,
        } => handle_sync_status(workspace, service_url).await,
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
#[allow(clippy::too_many_arguments)]
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

    let local_workspace_dir =
        local_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Create sync manager with the config
    let mut sync_manager = mockforge_core::workspace::sync::WorkspaceSyncManager::new(sync_config);

    // Create workspace persistence to load workspaces
    let persistence =
        mockforge_core::workspace_persistence::WorkspacePersistence::new(&local_workspace_dir);

    if all {
        println!("{}", "🔄 Syncing all workspaces...".cyan());

        // Get all workspace IDs
        let workspace_ids =
            persistence.list_workspace_ids().await.context("Failed to list workspace IDs")?;

        if workspace_ids.is_empty() {
            println!("{}", "ℹ️  No workspaces found to sync".yellow());
            return Ok(());
        }

        println!("   Found {} workspace(s) to sync", workspace_ids.len());

        let mut successful = 0;
        let mut failed = 0;
        let mut total_conflicts = 0;

        // Sync each workspace
        for workspace_id in workspace_ids {
            print!("   Syncing {}... ", workspace_id);
            match persistence.load_workspace(&workspace_id).await {
                Ok(mut workspace) => {
                    match sync_manager.sync_workspace(&mut workspace).await {
                        Ok(result) => {
                            if result.success {
                                successful += 1;
                                total_conflicts += result.conflicts.len();
                                println!("{}", "✓".green());

                                // Save workspace if it was modified
                                if result.changes_count > 0 {
                                    if let Err(e) = persistence.save_workspace(&workspace).await {
                                        eprintln!(
                                            "   Warning: Failed to save workspace after sync: {}",
                                            e
                                        );
                                    }
                                }

                                // Report conflicts if any
                                if !result.conflicts.is_empty() {
                                    println!(
                                        "     {} conflict(s) detected",
                                        result.conflicts.len()
                                    );
                                }
                            } else {
                                failed += 1;
                                println!("{}", "✗".red());
                                if let Some(error) = result.error {
                                    println!("     Error: {}", error);
                                }
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            println!("{}", "✗".red());
                            println!("     Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    failed += 1;
                    println!("{}", "✗".red());
                    println!("     Failed to load workspace: {}", e);
                }
            }
        }

        println!();
        println!("{}", "📊 Sync Summary".cyan());
        println!("   Successful: {}", successful);
        println!("   Failed: {}", failed);
        println!("   Total conflicts: {}", total_conflicts);

        if successful > 0 {
            println!("{}", "✅ Sync completed".green());
        } else if failed > 0 {
            println!("{}", "❌ All syncs failed".red());
        }
    } else if let Some(workspace_id) = workspace {
        println!("{}", format!("🔄 Syncing workspace: {}", workspace_id).cyan());

        // Load workspace
        let mut workspace = persistence
            .load_workspace(&workspace_id)
            .await
            .context(format!("Failed to load workspace: {}", workspace_id))?;

        // Perform sync
        match sync_manager.sync_workspace(&mut workspace).await {
            Ok(result) => {
                if result.success {
                    println!("{}", "✅ Sync completed successfully".green());
                    println!("   Changes: {}", result.changes_count);
                    println!("   Conflicts: {}", result.conflicts.len());

                    // Save workspace if it was modified
                    if result.changes_count > 0 {
                        persistence
                            .save_workspace(&workspace)
                            .await
                            .context("Failed to save workspace after sync")?;
                        println!("   Workspace saved");
                    }

                    // Report conflicts if any
                    if !result.conflicts.is_empty() {
                        println!("{}", "⚠️  Conflicts detected:".yellow());
                        for conflict in &result.conflicts {
                            println!("     - {} ({})", conflict.entity_id, conflict.entity_type);
                        }
                    }
                } else {
                    println!("{}", "❌ Sync failed".red());
                    if let Some(error) = result.error {
                        println!("   Error: {}", error);
                    }
                }
            }
            Err(e) => {
                println!("{}", "❌ Sync failed".red());
                return Err(anyhow::anyhow!("Sync error: {}", e));
            }
        }

        // Start sync service for monitoring if watch is enabled
        if watch {
            let sync_service = SyncService::new(&local_workspace_dir);
            sync_service.start().await.context("Failed to start sync service")?;

            println!("{}", "👀 Watching for file changes...".cyan());
            sync_service
                .monitor_workspace(&workspace_id, &local_workspace_dir.to_string_lossy())
                .await
                .context("Failed to start monitoring workspace")?;
            println!("{}", "✅ File watching started".green());
        }
    } else {
        return Err(anyhow::anyhow!("Either --workspace or --all must be specified"));
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
        .get(format!("{}/api/v1/workspaces", service_url))
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
        .post(format!("{}/api/v1/workspaces", service_url))
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
    if let Some(parent) = sync_config_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create .mockforge directory")?;
    }

    // Read cloud config to get service URL and API key
    let cloud_config_path = dirs::home_dir()
        .map(|p| p.join(".mockforge").join("cloud.json"))
        .unwrap_or_else(|| PathBuf::from(".mockforge/cloud.json"));

    if !cloud_config_path.exists() {
        return Err(anyhow::anyhow!(
            "Not authenticated with MockForge Cloud. Please run 'mockforge cloud login' first"
        ));
    }

    let cloud_config_content =
        std::fs::read_to_string(&cloud_config_path).context("Failed to read cloud config")?;
    let cloud_config: serde_json::Value =
        serde_json::from_str(&cloud_config_content).context("Failed to parse cloud config")?;

    let service_url = cloud_config
        .get("service_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.mockforge.dev")
        .to_string();

    let api_key = cloud_config
        .get("api_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("API key not found in cloud config"))?;

    // Read existing sync config if it exists, or create a new one
    let mut sync_config = if sync_config_path.exists() {
        let config_content = tokio::fs::read_to_string(&sync_config_path)
            .await
            .context("Failed to read existing sync config")?;

        serde_yaml::from_str::<SyncConfig>(&config_content)
            .context("Failed to parse existing sync config")?
    } else {
        // Create default sync config
        use mockforge_core::workspace::sync::{
            ConflictResolutionStrategy, SyncDirection, SyncDirectoryStructure,
        };
        SyncConfig {
            enabled: true,
            provider: SyncProvider::Cloud {
                service_url: service_url.clone(),
                api_key: api_key.to_string(),
                project_id: cloud_workspace_id.clone(),
            },
            interval_seconds: 60,
            conflict_strategy: ConflictResolutionStrategy::LastModified,
            auto_commit: true,
            auto_push: false,
            directory_structure: SyncDirectoryStructure::PerWorkspace,
            sync_direction: SyncDirection::Bidirectional,
        }
    };

    // Update the sync config with cloud provider settings
    sync_config.enabled = true;
    sync_config.provider = SyncProvider::Cloud {
        service_url: service_url.clone(),
        api_key: api_key.to_string(),
        project_id: cloud_workspace_id.clone(),
    };

    // Save updated config
    let updated_config =
        serde_yaml::to_string(&sync_config).context("Failed to serialize sync config")?;
    tokio::fs::write(&sync_config_path, updated_config)
        .await
        .context("Failed to write sync config")?;

    println!(
        "{}",
        format!("🔗 Linking local workspace to cloud workspace: {}", cloud_workspace_id).cyan()
    );
    println!("   Local: {}", local_workspace.display());
    println!("   Cloud: {}", cloud_workspace_id);
    println!("   Service: {}", service_url);
    println!("{}", "✅ Workspace linked successfully".green());
    println!("   Sync config saved to: {}", sync_config_path.display());

    Ok(())
}

/// Handle cloud workspace unlink command
async fn handle_cloud_workspace_unlink(local_workspace: PathBuf) -> Result<()> {
    let sync_config_path = local_workspace.join(".mockforge").join("sync.yaml");

    if sync_config_path.exists() {
        // Load existing sync config
        let config_content = tokio::fs::read_to_string(&sync_config_path)
            .await
            .context("Failed to read sync config")?;

        let mut sync_config: SyncConfig =
            serde_yaml::from_str(&config_content).context("Failed to parse sync config")?;

        // Disable sync or remove cloud provider
        match &mut sync_config.provider {
            SyncProvider::Cloud { .. } => {
                // Disable sync
                sync_config.enabled = false;

                // Save updated config
                let updated_config = serde_yaml::to_string(&sync_config)
                    .context("Failed to serialize sync config")?;
                tokio::fs::write(&sync_config_path, updated_config)
                    .await
                    .context("Failed to write sync config")?;

                println!("{}", "🔓 Unlinking workspace from cloud".cyan());
                println!("{}", "✅ Sync configuration disabled".green());
                println!("   Note: sync.yaml file still exists but sync is disabled");
                println!("   To fully remove, delete: {}", sync_config_path.display());
            }
            _ => {
                // Not a cloud provider, just disable
                sync_config.enabled = false;
                let updated_config = serde_yaml::to_string(&sync_config)
                    .context("Failed to serialize sync config")?;
                tokio::fs::write(&sync_config_path, updated_config)
                    .await
                    .context("Failed to write sync config")?;

                println!("{}", "🔓 Sync disabled".cyan());
            }
        }
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
        .get(format!("{}/api/v1/workspaces/{}", service_url, workspace_id))
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
        TeamCommands::Members {
            workspace,
            service_url,
        } => handle_team_members(workspace, service_url).await,
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
        .get(format!("{}/api/v1/workspaces/{}/members", service_url, workspace))
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
        .post(format!("{}/api/v1/workspaces/{}/members", service_url, workspace))
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
async fn handle_team_remove(email: String, workspace: String, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .delete(format!("{}/api/v1/workspaces/{}/members/{}", service_url, workspace, email))
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
async fn handle_activity(workspace: Option<String>, service_url: String, limit: u32) -> Result<()> {
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

/// Handle deploy command
async fn handle_deploy(
    spec: PathBuf,
    name: String,
    slug: Option<String>,
    region: String,
    wait: bool,
    service_url: String,
) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    // Read spec file
    let spec_content = std::fs::read_to_string(&spec).context("Failed to read spec file")?;

    // Parse as JSON or YAML to validate
    let spec_value: serde_json::Value =
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&spec_content) {
            v
        } else if let Ok(v) = serde_yaml::from_str::<serde_json::Value>(&spec_content) {
            v
        } else {
            anyhow::bail!("Spec file must be valid JSON or YAML");
        };

    if spec_value.get("openapi").is_none() && spec_value.get("swagger").is_none() {
        anyhow::bail!("Spec file must contain an 'openapi' or 'swagger' field");
    }

    println!("{}", "Uploading spec...".cyan());

    // Upload spec via multipart
    let file_name = spec.file_name().and_then(|n| n.to_str()).unwrap_or("spec.json").to_string();

    let spec_json = serde_json::to_vec_pretty(&spec_value)?;
    let form = reqwest::multipart::Form::new().part(
        "file",
        reqwest::multipart::Part::bytes(spec_json)
            .file_name(file_name)
            .mime_str("application/json")?,
    );

    let upload_response = client
        .post(format!("{}/api/v1/hosted-mocks/specs/upload", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .context("Failed to upload spec")?;

    if !upload_response.status().is_success() {
        let error_body = upload_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to upload spec: {}", error_body);
    }

    let upload_result: serde_json::Value = upload_response.json().await?;
    let spec_url = upload_result
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Upload response missing 'url' field"))?;

    println!("{}", "Creating deployment...".cyan());

    // Create deployment
    let deploy_body = json!({
        "name": name,
        "slug": slug,
        "config_json": spec_value,
        "openapi_spec_url": spec_url,
        "region": region,
    });

    let deploy_response = client
        .post(format!("{}/api/v1/hosted-mocks", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&deploy_body)
        .send()
        .await
        .context("Failed to create deployment")?;

    if !deploy_response.status().is_success() {
        let error_body = deploy_response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to create deployment: {}", error_body);
    }

    let deployment: serde_json::Value = deploy_response.json().await?;
    let deployment_id = deployment.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

    println!("{}", format!("Deployment created: {}", deployment_id).green());

    if wait {
        println!("{}", "Waiting for deployment to become active...".cyan());

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(300); // 5 minutes

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Deployment timed out after 5 minutes");
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let status_response = client
                .get(format!("{}/api/v1/hosted-mocks/{}", service_url, deployment_id))
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
                .context("Failed to check deployment status")?;

            if status_response.status().is_success() {
                let status: serde_json::Value = status_response.json().await?;
                let state = status.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

                match state {
                    "active" => {
                        let url = status
                            .get("deployment_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        println!("{}", format!("Deployment active! URL: {}", url).green().bold());
                        return Ok(());
                    }
                    "failed" => {
                        let error = status
                            .get("error_message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        anyhow::bail!("Deployment failed: {}", error);
                    }
                    _ => {
                        print!(".");
                    }
                }
            }
        }
    } else {
        println!(
            "{}",
            format!("Check status with: mockforge cloud deployment-status {}", deployment_id)
                .dimmed()
        );
    }

    Ok(())
}

/// Handle list deployments command
async fn handle_deployments(service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/v1/hosted-mocks", service_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch deployments")?;

    if !response.status().is_success() {
        let error_body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to list deployments: {}", error_body);
    }

    let deployments: Vec<serde_json::Value> = response.json().await?;

    if deployments.is_empty() {
        println!("{}", "No deployments found.".dimmed());
        println!(
            "{}",
            "Create one with: mockforge cloud deploy --spec api.json --name \"My API\"".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Cloud Deployments".cyan().bold());
    println!("{:<36}  {:<20}  {:<10}  {:<10}  URL", "ID", "NAME", "STATUS", "HEALTH");
    println!("{}", "-".repeat(110));

    for d in deployments {
        let id = d.get("id").and_then(|v| v.as_str()).unwrap_or("-");
        let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        let status = d.get("status").and_then(|v| v.as_str()).unwrap_or("-");
        let health = d.get("health_status").and_then(|v| v.as_str()).unwrap_or("-");
        let url = d.get("deployment_url").and_then(|v| v.as_str()).unwrap_or("-");

        let status_colored = match status {
            "active" => status.green().to_string(),
            "failed" => status.red().to_string(),
            "deploying" | "pending" => status.yellow().to_string(),
            _ => status.dimmed().to_string(),
        };

        println!("{:<36}  {:<20}  {:<10}  {:<10}  {}", id, name, status_colored, health, url);
    }

    Ok(())
}

/// Handle deployment status command
async fn handle_deployment_status(id: String, service_url: String) -> Result<()> {
    let api_key = get_api_key()?;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/api/v1/hosted-mocks/{}", service_url, id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to fetch deployment status")?;

    if !response.status().is_success() {
        let error_body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get deployment status: {}", error_body);
    }

    let deployment: serde_json::Value = response.json().await?;

    println!("{}", "Deployment Details".cyan().bold());
    println!("  ID:      {}", deployment.get("id").and_then(|v| v.as_str()).unwrap_or("-"));
    println!("  Name:    {}", deployment.get("name").and_then(|v| v.as_str()).unwrap_or("-"));
    println!(
        "  Status:  {}",
        deployment.get("status").and_then(|v| v.as_str()).unwrap_or("-")
    );
    println!(
        "  Health:  {}",
        deployment.get("health_status").and_then(|v| v.as_str()).unwrap_or("-")
    );
    println!(
        "  URL:     {}",
        deployment
            .get("deployment_url")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available")
    );
    println!(
        "  Created: {}",
        deployment.get("created_at").and_then(|v| v.as_str()).unwrap_or("-")
    );

    if let Some(error) = deployment.get("error_message").and_then(|v| v.as_str()) {
        if !error.is_empty() {
            println!("  Error:   {}", error.red());
        }
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
        let config_content =
            std::fs::read_to_string(&config_path).context("Failed to read config file")?;
        let config: serde_json::Value =
            serde_json::from_str(&config_content).context("Failed to parse config file")?;

        if let Some(api_key) = config.get("api_key").and_then(|v| v.as_str()) {
            return Ok(api_key.to_string());
        }
    }

    Err(anyhow::anyhow!(
        "No API key found. Run 'mockforge cloud login' or set MOCKFORGE_API_KEY environment variable"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // CloudCommands enum tests
    #[test]
    fn test_cloud_commands_login_variant() {
        let _cmd = CloudCommands::Login {
            token: Some("test-token".to_string()),
            provider: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_commands_login_with_provider() {
        let _cmd = CloudCommands::Login {
            token: None,
            provider: Some("github".to_string()),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_commands_whoami_variant() {
        let _cmd = CloudCommands::Whoami {
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_commands_logout_variant() {
        let _cmd = CloudCommands::Logout {};
    }

    // SyncCommands enum tests
    #[test]
    fn test_sync_commands_start_variant() {
        let _cmd = SyncCommands::Start {
            workspace: Some("my-workspace".to_string()),
            all: false,
            project: Some("my-project".to_string()),
            watch: true,
            strategy: "merge".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_start_all() {
        let _cmd = SyncCommands::Start {
            workspace: None,
            all: true,
            project: None,
            watch: false,
            strategy: "local".to_string(),
            direction: "up".to_string(),
            local_dir: Some(PathBuf::from(".")),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_status_variant() {
        let _cmd = SyncCommands::Status {
            workspace: Some("workspace-1".to_string()),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_history_variant() {
        let _cmd = SyncCommands::History {
            workspace: None,
            service_url: "https://api.mockforge.dev".to_string(),
            limit: 50,
        };
    }

    #[test]
    fn test_sync_commands_pending_variant() {
        let _cmd = SyncCommands::Pending {
            workspace: Some("workspace-1".to_string()),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    // CloudWorkspaceCommands enum tests
    #[test]
    fn test_cloud_workspace_commands_list_variant() {
        let _cmd = CloudWorkspaceCommands::List {
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_workspace_commands_create_variant() {
        let _cmd = CloudWorkspaceCommands::Create {
            workspace_id: "new-workspace".to_string(),
            name: "New Workspace".to_string(),
            description: Some("Test workspace".to_string()),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_workspace_commands_link_variant() {
        let _cmd = CloudWorkspaceCommands::Link {
            local_workspace: PathBuf::from("./workspace"),
            cloud_workspace_id: "cloud-123".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_cloud_workspace_commands_unlink_variant() {
        let _cmd = CloudWorkspaceCommands::Unlink {
            local_workspace: PathBuf::from("./workspace"),
        };
    }

    #[test]
    fn test_cloud_workspace_commands_info_variant() {
        let _cmd = CloudWorkspaceCommands::Info {
            workspace_id: "workspace-123".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    // TeamCommands enum tests
    #[test]
    fn test_team_commands_members_variant() {
        let _cmd = TeamCommands::Members {
            workspace: "team-workspace".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_team_commands_invite_variant() {
        let _cmd = TeamCommands::Invite {
            email: "user@example.com".to_string(),
            workspace: "team-workspace".to_string(),
            role: "editor".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_team_commands_invite_admin() {
        let _cmd = TeamCommands::Invite {
            email: "admin@example.com".to_string(),
            workspace: "team-workspace".to_string(),
            role: "admin".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_team_commands_remove_variant() {
        let _cmd = TeamCommands::Remove {
            email: "user@example.com".to_string(),
            workspace: "team-workspace".to_string(),
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    // get_api_key tests with environment variable
    #[test]
    fn test_get_api_key_from_env() {
        // Lock mutex to prevent parallel test interference
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Set environment variable
        std::env::set_var("MOCKFORGE_API_KEY", "test-api-key");

        let result = get_api_key();

        // Clean up
        std::env::remove_var("MOCKFORGE_API_KEY");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-api-key");
    }

    #[test]
    fn test_get_api_key_not_found() {
        // Lock mutex to prevent parallel test interference
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        // Make sure env var is not set
        std::env::remove_var("MOCKFORGE_API_KEY");

        // Use a temp dir to ensure no config file exists
        let _temp_dir = TempDir::new().unwrap();

        let result = get_api_key();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No API key found"));
    }

    // Activity command variant tests
    #[test]
    fn test_cloud_commands_activity_variant() {
        let _cmd = CloudCommands::Activity {
            workspace: Some("workspace-1".to_string()),
            service_url: "https://api.mockforge.dev".to_string(),
            limit: 20,
        };
    }

    #[test]
    fn test_cloud_commands_activity_no_workspace() {
        let _cmd = CloudCommands::Activity {
            workspace: None,
            service_url: "https://api.mockforge.dev".to_string(),
            limit: 50,
        };
    }

    // Sync direction tests
    #[test]
    fn test_sync_commands_direction_up() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "merge".to_string(),
            direction: "up".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_direction_down() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "merge".to_string(),
            direction: "down".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_direction_both() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "merge".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    // Conflict strategy tests
    #[test]
    fn test_sync_commands_strategy_local() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "local".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_strategy_remote() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "remote".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_strategy_merge() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "merge".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }

    #[test]
    fn test_sync_commands_strategy_manual() {
        let _cmd = SyncCommands::Start {
            workspace: Some("test".to_string()),
            all: false,
            project: None,
            watch: false,
            strategy: "manual".to_string(),
            direction: "both".to_string(),
            local_dir: None,
            service_url: "https://api.mockforge.dev".to_string(),
        };
    }
}
