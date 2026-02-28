//! Git Watch Mode Commands
//!
//! Commands for monitoring Git repositories for OpenAPI spec changes and auto-syncing mocks.

use mockforge_core::{git_watch::GitWatchConfig, git_watch::GitWatchService, Error, Result};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Handle the git-watch command
pub async fn handle_git_watch(
    repository_url: String,
    branch: Option<String>,
    spec_paths: Vec<String>,
    poll_interval: Option<u64>,
    auth_token: Option<String>,
    cache_dir: Option<PathBuf>,
    reload_command: Option<String>,
) -> Result<()> {
    info!("Starting Git watch mode");

    // Build configuration
    let config = GitWatchConfig {
        repository_url,
        branch: branch.unwrap_or_else(|| "main".to_string()),
        spec_paths: if spec_paths.is_empty() {
            vec![
                "**/*.yaml".to_string(),
                "**/*.json".to_string(),
                "**/openapi*.yaml".to_string(),
                "**/openapi*.json".to_string(),
            ]
        } else {
            spec_paths
        },
        poll_interval_seconds: poll_interval.unwrap_or(60),
        auth_token,
        cache_dir: cache_dir.unwrap_or_else(|| PathBuf::from("./.mockforge-git-cache")),
        enabled: true,
    };

    // Create watch service
    let mut watch_service = GitWatchService::new(config)?;

    // Initialize repository
    watch_service.initialize().await?;

    // Get initial spec files
    let initial_specs = watch_service.get_spec_files()?;
    info!("Found {} OpenAPI spec file(s) initially", initial_specs.len());
    for spec in &initial_specs {
        info!("  - {}", spec.display());
    }

    // Handle initial load if requested
    if let Some(ref cmd) = reload_command {
        info!("Executing initial reload command: {}", cmd);
        if let Err(e) = execute_reload_command(cmd, &initial_specs).await {
            warn!("Initial reload command failed: {}", e);
        }
    }

    // Start watching
    info!("Watching for changes... (Press Ctrl+C to stop)");

    watch_service
        .watch(|spec_files| {
            info!("OpenAPI spec files changed:");
            for spec in &spec_files {
                info!("  - {}", spec.display());
            }

            // Emit pipeline event for schema changes
            #[cfg(feature = "pipelines")]
            {
                use mockforge_pipelines::{publish_event, PipelineEvent};
                use uuid::Uuid;

                // Determine schema type from file extension
                let schema_type = spec_files
                    .first()
                    .and_then(|path| {
                        path.extension().and_then(|ext| ext.to_str()).map(|ext| {
                            if ext == "proto" || ext == "protobuf" {
                                "protobuf"
                            } else {
                                "openapi"
                            }
                        })
                    })
                    .unwrap_or("openapi");

                // Create schema changed event
                // CLI context has no persistent workspace, so each event gets a unique ID
                let event = PipelineEvent::schema_changed(
                    Uuid::new_v4(),
                    schema_type.to_string(),
                    std::collections::HashMap::new(),
                );

                if let Err(e) = publish_event(event) {
                    warn!("Failed to publish schema changed event: {}", e);
                }
            }

            // Execute reload command if provided
            if let Some(ref cmd) = reload_command {
                info!("Executing reload command: {}", cmd);
                let cmd_clone = cmd.clone();
                let spec_files_clone = spec_files.clone();
                tokio::spawn(async move {
                    if let Err(e) = execute_reload_command(&cmd_clone, &spec_files_clone).await {
                        error!("Reload command failed: {}", e);
                    }
                });
            } else {
                info!("No reload command specified. Spec files updated but no action taken.");
                info!("Use --reload-command to specify a command to run when specs change.");
            }

            Ok(())
        })
        .await?;

    Ok(())
}

/// Execute a reload command with spec file paths as arguments
async fn execute_reload_command(command: &str, spec_files: &[PathBuf]) -> Result<()> {
    use std::process::Command;

    // Parse command (simple split on spaces for now)
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(Error::generic("Empty reload command".to_string()));
    }

    let program = parts[0];
    let args: Vec<&str> = parts[1..].to_vec();

    // Add spec file paths as additional arguments
    let mut all_args = args;
    for spec_file in spec_files {
        let path_str = spec_file
            .to_str()
            .ok_or_else(|| Error::generic(format!("Non-UTF8 path: {}", spec_file.display())))?;
        all_args.push(path_str);
    }

    let output = Command::new(program)
        .args(&all_args)
        .output()
        .map_err(|e| Error::generic(format!("Failed to execute reload command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::generic(format!("Reload command failed: {}", stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        info!("Reload command output: {}", stdout);
    }

    Ok(())
}
