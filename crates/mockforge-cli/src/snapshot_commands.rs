//! Snapshot management commands
//!
//! Provides CLI commands for saving, loading, listing, and managing snapshots.

use clap::Subcommand;
use mockforge_core::snapshots::{SnapshotComponents, SnapshotManager};
use mockforge_core::Result;
use std::path::PathBuf;
use tracing::{error, info};

/// Snapshot management subcommands
#[derive(Subcommand, Debug)]
pub enum SnapshotCommands {
    /// Save current system state
    Save {
        /// Snapshot name
        name: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
        /// Components to include (comma-separated: unified_state,vbr_state,recorder_state,workspace_config)
        /// Default: all
        #[arg(long, value_delimiter = ',')]
        components: Option<Vec<String>>,
    },
    /// Restore system state from snapshot
    Load {
        /// Snapshot name
        name: String,
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
        /// Components to restore (comma-separated, default: all)
        #[arg(long, value_delimiter = ',')]
        components: Option<Vec<String>>,
        /// Dry run (validate without restoring)
        #[arg(long)]
        dry_run: bool,
    },
    /// List all snapshots
    List {
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
    },
    /// Delete a snapshot
    Delete {
        /// Snapshot name
        name: String,
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
    },
    /// Show snapshot information
    Info {
        /// Snapshot name
        name: String,
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
    },
    /// Validate snapshot integrity
    Validate {
        /// Snapshot name
        name: String,
        /// Workspace ID (defaults to "default")
        #[arg(long, default_value = "default")]
        workspace: String,
    },
}

/// Parse component list from string vector
fn parse_components(components: Option<Vec<String>>) -> SnapshotComponents {
    if let Some(comp_list) = components {
        let comp_set: std::collections::HashSet<String> = comp_list
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        SnapshotComponents {
            unified_state: comp_set.is_empty() || comp_set.contains("unified_state") || comp_set.contains("unified-state"),
            vbr_state: comp_set.is_empty() || comp_set.contains("vbr_state") || comp_set.contains("vbr-state"),
            recorder_state: comp_set.is_empty() || comp_set.contains("recorder_state") || comp_set.contains("recorder-state"),
            workspace_config: comp_set.is_empty() || comp_set.contains("workspace_config") || comp_set.contains("workspace-config"),
            protocols: Vec::new(), // Empty = all protocols
        }
    } else {
        SnapshotComponents::all()
    }
}

/// Handle snapshot commands
pub async fn handle_snapshot_command(command: SnapshotCommands) -> Result<()> {
    // Initialize snapshot manager
    let snapshot_dir = std::env::var("MOCKFORGE_SNAPSHOT_DIR")
        .ok()
        .map(PathBuf::from);
    let manager = SnapshotManager::new(snapshot_dir);

    match command {
        SnapshotCommands::Save {
            name,
            description,
            workspace,
            components,
        } => {
            info!("Saving snapshot '{}' for workspace '{}'", name, workspace);
            let components = parse_components(components);

            // TODO: Get consistency engine from server state when integrated
            // For now, we'll create a placeholder that can be extended
            let manifest = manager
                .save_snapshot(name.clone(), description, workspace.clone(), components, None)
                .await?;

            println!("✓ Snapshot '{}' saved successfully", name);
            println!("  Workspace: {}", workspace);
            println!("  Size: {} bytes", manifest.size_bytes);
            println!("  Checksum: {}", manifest.checksum);
            if let Some(desc) = &manifest.description {
                println!("  Description: {}", desc);
            }
        }
        SnapshotCommands::Load {
            name,
            workspace,
            components,
            dry_run,
        } => {
            if dry_run {
                info!("Validating snapshot '{}' for workspace '{}' (dry run)", name, workspace);
                let is_valid = manager.validate_snapshot(name.clone(), workspace.clone()).await?;
                if is_valid {
                    println!("✓ Snapshot '{}' is valid", name);
                } else {
                    println!("✗ Snapshot '{}' failed validation", name);
                    return Err(mockforge_core::Error::from("Snapshot validation failed"));
                }
            } else {
                info!("Loading snapshot '{}' for workspace '{}'", name, workspace);
                let components = components.map(parse_components);

                // TODO: Get consistency engine from server state when integrated
                let manifest = manager
                    .load_snapshot(name.clone(), workspace.clone(), components, None)
                    .await?;

                println!("✓ Snapshot '{}' loaded successfully", name);
                println!("  Workspace: {}", workspace);
                println!("  Created: {}", manifest.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                if let Some(desc) = &manifest.description {
                    println!("  Description: {}", desc);
                }
            }
        }
        SnapshotCommands::List { workspace } => {
            info!("Listing snapshots for workspace '{}'", workspace);
            let snapshots = manager.list_snapshots(&workspace).await?;

            if snapshots.is_empty() {
                println!("No snapshots found for workspace '{}'", workspace);
            } else {
                println!("Snapshots for workspace '{}':", workspace);
                println!();
                for snapshot in snapshots {
                    println!("  {}", snapshot.name);
                    if let Some(desc) = &snapshot.description {
                        println!("    Description: {}", desc);
                    }
                    println!("    Created: {}", snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("    Size: {} bytes", snapshot.size_bytes);
                    println!();
                }
            }
        }
        SnapshotCommands::Delete { name, workspace } => {
            info!("Deleting snapshot '{}' for workspace '{}'", name, workspace);
            manager.delete_snapshot(name.clone(), workspace.clone()).await?;
            println!("✓ Snapshot '{}' deleted successfully", name);
        }
        SnapshotCommands::Info { name, workspace } => {
            info!("Getting info for snapshot '{}' in workspace '{}'", name, workspace);
            let manifest = manager.get_snapshot_info(name.clone(), workspace.clone()).await?;

            println!("Snapshot: {}", manifest.name);
            println!("  Workspace: {}", manifest.workspace_id);
            println!("  Created: {}", manifest.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("  Size: {} bytes", manifest.size_bytes);
            println!("  Checksum: {}", manifest.checksum);
            if let Some(desc) = &manifest.description {
                println!("  Description: {}", desc);
            }
            println!("  Components:");
            println!("    Unified State: {}", manifest.components.unified_state);
            println!("    VBR State: {}", manifest.components.vbr_state);
            println!("    Recorder State: {}", manifest.components.recorder_state);
            println!("    Workspace Config: {}", manifest.components.workspace_config);
            if manifest.components.protocols.is_empty() {
                println!("    Protocols: all");
            } else {
                println!("    Protocols: {}", manifest.components.protocols.join(", "));
            }
        }
        SnapshotCommands::Validate { name, workspace } => {
            info!("Validating snapshot '{}' for workspace '{}'", name, workspace);
            let is_valid = manager.validate_snapshot(name.clone(), workspace.clone()).await?;
            if is_valid {
                println!("✓ Snapshot '{}' is valid", name);
            } else {
                println!("✗ Snapshot '{}' failed validation (checksum mismatch)", name);
                return Err(mockforge_core::Error::from("Snapshot validation failed"));
            }
        }
    }

    Ok(())
}

