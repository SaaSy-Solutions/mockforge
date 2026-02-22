use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::{Component, PathBuf};
use std::sync::Arc;

use crate::spec_registry::FtpSpecRegistry;
use crate::vfs::VirtualFileSystem;

/// FTP-related CLI commands
#[derive(Debug, Subcommand)]
pub enum FtpCommands {
    /// Virtual filesystem management
    Vfs(VfsCommands),
    /// Upload management
    Uploads(UploadsCommands),
    /// Fixture management
    Fixtures(FixturesCommands),
}

/// Virtual filesystem commands
#[derive(Debug, Args)]
pub struct VfsCommands {
    #[command(subcommand)]
    pub command: VfsSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum VfsSubcommands {
    /// List all virtual files
    List,
    /// Show directory tree
    Tree,
    /// Add a virtual file
    Add {
        /// File path
        path: PathBuf,
        /// File content
        #[arg(short, long)]
        content: Option<String>,
        /// Template file
        #[arg(short, long)]
        template: Option<String>,
        /// Generate file with pattern
        #[arg(short, long)]
        generate: Option<String>,
        /// File size for generated files
        #[arg(short, long)]
        size: Option<usize>,
    },
    /// Remove a virtual file
    Remove {
        /// File path
        path: PathBuf,
    },
    /// Clear all virtual files
    Clear,
}

/// Upload management commands
#[derive(Debug, Args)]
pub struct UploadsCommands {
    #[command(subcommand)]
    pub command: UploadsSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum UploadsSubcommands {
    /// List received uploads
    List,
    /// Show upload details
    Show {
        /// Upload ID
        id: String,
    },
    /// Export uploads to directory
    Export {
        /// Output directory
        #[arg(short, long)]
        dir: PathBuf,
    },
}

/// Fixture management commands
#[derive(Debug, Args)]
pub struct FixturesCommands {
    #[command(subcommand)]
    pub command: FixturesSubcommands,
}

#[derive(Debug, Subcommand)]
pub enum FixturesSubcommands {
    /// Load fixtures from directory
    Load {
        /// Directory containing fixture files
        dir: PathBuf,
    },
    /// List loaded fixtures
    List,
    /// Reload fixtures
    Reload,
}

/// Execute FTP commands
pub async fn execute_ftp_command(
    command: FtpCommands,
    vfs: Arc<VirtualFileSystem>,
    spec_registry: Arc<FtpSpecRegistry>,
) -> Result<()> {
    match command {
        FtpCommands::Vfs(vfs_cmd) => execute_vfs_command(vfs_cmd, vfs).await,
        FtpCommands::Uploads(uploads_cmd) => {
            execute_uploads_command(uploads_cmd, spec_registry).await
        }
        FtpCommands::Fixtures(fixtures_cmd) => {
            execute_fixtures_command(fixtures_cmd, spec_registry).await
        }
    }
}

async fn execute_vfs_command(command: VfsCommands, vfs: Arc<VirtualFileSystem>) -> Result<()> {
    match command.command {
        VfsSubcommands::List => {
            let files = vfs.list_files(&PathBuf::from("/"));
            if files.is_empty() {
                println!("No virtual files found.");
            } else {
                println!("Virtual files:");
                println!("{:<50} {:<10} {:<10} {:<20}", "Path", "Size", "Permissions", "Modified");
                println!("{}", "-".repeat(90));
                for file in files {
                    println!(
                        "{:<50} {:<10} {:<10} {}",
                        file.path.display(),
                        file.metadata.size,
                        file.metadata.permissions,
                        file.modified_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
            Ok(())
        }
        VfsSubcommands::Tree => {
            let files = vfs.list_files(&PathBuf::from("/"));
            if files.is_empty() {
                println!("No virtual files found.");
            } else {
                println!("/");
                print_tree(&files, &PathBuf::from("/"), "");
            }
            Ok(())
        }
        VfsSubcommands::Add {
            path,
            content,
            template,
            generate,
            size,
        } => {
            use crate::vfs::{FileContent, GenerationPattern, VirtualFile};

            let file_content = if let Some(content) = content {
                FileContent::Static(content.into_bytes())
            } else if let Some(template) = template {
                FileContent::Template(template)
            } else if let Some(pattern) = generate {
                let gen_pattern = match pattern.as_str() {
                    "random" => GenerationPattern::Random,
                    "zeros" => GenerationPattern::Zeros,
                    "ones" => GenerationPattern::Ones,
                    "incremental" => GenerationPattern::Incremental,
                    _ => {
                        println!(
                            "Invalid generation pattern. Use: random, zeros, ones, incremental"
                        );
                        return Ok(());
                    }
                };
                let file_size = size.unwrap_or(1024);
                FileContent::Generated {
                    size: file_size,
                    pattern: gen_pattern,
                }
            } else {
                println!("Must specify one of: --content, --template, or --generate");
                return Ok(());
            };

            let virtual_file = VirtualFile::new(path.clone(), file_content, Default::default());

            vfs.add_file(path.clone(), virtual_file)?;
            println!("Added virtual file: {}", path.display());
            Ok(())
        }
        VfsSubcommands::Remove { path } => {
            vfs.remove_file(&path)?;
            println!("Removed virtual file: {}", path.display());
            Ok(())
        }
        VfsSubcommands::Clear => {
            vfs.clear()?;
            println!("Cleared all virtual files.");
            Ok(())
        }
    }
}

fn print_tree(files: &[crate::vfs::VirtualFile], current_path: &std::path::Path, prefix: &str) {
    use std::collections::HashMap;

    let mut dirs: HashMap<String, Vec<crate::vfs::VirtualFile>> = HashMap::new();
    let mut current_files = Vec::new();

    for file in files {
        if let Ok(relative) = file.path.strip_prefix(current_path) {
            let components: Vec<_> = relative.components().collect();
            if components.len() == 1 {
                // File in current directory
                current_files.push(file.clone());
            } else if let Component::Normal(name) = components[0] {
                let dir_name = name.to_string_lossy().to_string();
                let sub_path = current_path.join(&dir_name);
                let remaining_path = components[1..].iter().collect::<PathBuf>();
                let full_sub_path = sub_path.join(remaining_path);

                dirs.entry(dir_name).or_default().push(crate::vfs::VirtualFile {
                    path: full_sub_path,
                    ..file.clone()
                });
            }
        }
    }

    // Print files in current directory
    for (i, file) in current_files.iter().enumerate() {
        let is_last = i == current_files.len() - 1 && dirs.is_empty();
        let connector = if is_last { "└── " } else { "├── " };
        println!(
            "{}{}{}",
            prefix,
            connector,
            file.path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    // Print subdirectories
    let dir_keys: Vec<_> = dirs.keys().cloned().collect();
    for (i, dir_name) in dir_keys.iter().enumerate() {
        let is_last = i == dir_keys.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        println!("{}{}{}/", prefix, connector, dir_name);

        if let Some(sub_files) = dirs.get(dir_name) {
            print_tree(sub_files, &current_path.join(dir_name), &new_prefix);
        }
    }
}

async fn execute_uploads_command(
    command: UploadsCommands,
    spec_registry: Arc<FtpSpecRegistry>,
) -> Result<()> {
    match command.command {
        UploadsSubcommands::List => {
            let uploads = spec_registry.get_uploads();
            if uploads.is_empty() {
                println!("No uploads found.");
            } else {
                println!("Uploaded files:");
                println!("{:<40} {:<50} {:<10} {:<20}", "ID", "Path", "Size", "Uploaded");
                println!("{}", "-".repeat(120));
                for upload in uploads {
                    println!(
                        "{:<40} {:<50} {:<10} {}",
                        upload.id,
                        upload.path.display(),
                        upload.size,
                        upload.uploaded_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
            Ok(())
        }
        UploadsSubcommands::Show { id } => {
            if let Some(upload) = spec_registry.get_upload(&id) {
                println!("Upload Details:");
                println!("ID: {}", upload.id);
                println!("Path: {}", upload.path.display());
                println!("Size: {} bytes", upload.size);
                println!("Uploaded: {}", upload.uploaded_at.format("%Y-%m-%d %H:%M:%S"));
                if let Some(rule) = &upload.rule_name {
                    println!("Rule: {}", rule);
                }
            } else {
                println!("Upload with ID '{}' not found.", id);
            }
            Ok(())
        }
        UploadsSubcommands::Export { dir } => {
            use tokio::fs;

            // Create directory if it doesn't exist
            fs::create_dir_all(&dir).await?;

            let uploads = spec_registry.get_uploads();
            if uploads.is_empty() {
                println!("No uploads to export.");
                return Ok(());
            }

            for upload in uploads {
                if let Some(file) = spec_registry.vfs.get_file(&upload.path) {
                    if let Ok(content) = file.render_content() {
                        let export_path =
                            dir.join(upload.path.strip_prefix("/").unwrap_or(&upload.path));
                        if let Some(parent) = export_path.parent() {
                            fs::create_dir_all(parent).await?;
                        }
                        fs::write(&export_path, content).await?;
                        println!(
                            "Exported: {} -> {}",
                            upload.path.display(),
                            export_path.display()
                        );
                    }
                }
            }
            println!("Export complete.");
            Ok(())
        }
    }
}

async fn execute_fixtures_command(
    command: FixturesCommands,
    spec_registry: Arc<FtpSpecRegistry>,
) -> Result<()> {
    match command.command {
        FixturesSubcommands::Load { dir } => {
            use serde_yaml;
            use std::fs;

            let mut loaded_fixtures = Vec::new();

            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                    || path.extension().and_then(|s| s.to_str()) == Some("yml")
                {
                    let content = fs::read_to_string(&path)?;
                    let fixture: crate::fixtures::FtpFixture = serde_yaml::from_str(&content)?;
                    loaded_fixtures.push(fixture);
                    println!("Loaded fixture: {}", path.display());
                }
            }

            if !loaded_fixtures.is_empty() {
                // Note: This is a simplified implementation. In a real scenario,
                // we'd need to update the spec_registry properly, but since it's Arc,
                // we'd need a different approach. For now, we'll just report what we found.
                println!("Found {} fixture files. (Note: Loading not fully implemented in this CLI context)", loaded_fixtures.len());
            } else {
                println!("No YAML fixture files found in {}", dir.display());
            }

            Ok(())
        }
        FixturesSubcommands::List => {
            if spec_registry.fixtures.is_empty() {
                println!("No fixtures loaded.");
            } else {
                println!("Loaded fixtures:");
                for fixture in &spec_registry.fixtures {
                    println!("- {}: {}", fixture.identifier, fixture.name);
                    if let Some(desc) = &fixture.description {
                        println!("  Description: {}", desc);
                    }
                    println!("  Virtual files: {}", fixture.virtual_files.len());
                    println!("  Upload rules: {}", fixture.upload_rules.len());
                    println!();
                }
            }
            Ok(())
        }
        FixturesSubcommands::Reload => {
            if spec_registry.fixtures.is_empty() {
                println!(
                    "No fixtures loaded. Use `mockforge ftp fixtures load --dir <path>` first."
                );
                return Ok(());
            }

            let mut vfs_fixtures = Vec::new();
            for fixture in &spec_registry.fixtures {
                for virtual_file in &fixture.virtual_files {
                    vfs_fixtures.push(virtual_file.clone().to_file_fixture());
                }
            }

            spec_registry
                .vfs
                .load_fixtures(vfs_fixtures)
                .map_err(|e| anyhow::anyhow!("Failed to reload fixtures into VFS: {}", e))?;

            println!(
                "Reloaded {} fixture(s) into virtual filesystem.",
                spec_registry.fixtures.len()
            );
            Ok(())
        }
    }
}
