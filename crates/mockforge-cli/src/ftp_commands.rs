use anyhow::{Context, Result};
use clap::Subcommand;
use mockforge_core::config::FtpConfig;
use mockforge_ftp::{
    FileContent, FileMetadata, FtpServer, GenerationPattern, VirtualFile, VirtualFileSystem,
};
use std::path::{Path, PathBuf};

/// FTP server management commands
#[derive(Subcommand)]
pub enum FtpCommands {
    /// Start FTP server
    ///
    /// Examples:
    ///   mockforge ftp serve --port 2121
    ///   mockforge ftp serve --config ftp-config.yaml
    #[command(verbatim_doc_comment)]
    Serve {
        /// FTP server port (defaults to 2121 without a configuration file)
        #[arg(short, long)]
        port: Option<u16>,

        /// FTP server host (defaults to 127.0.0.1 without a configuration file)
        #[arg(long)]
        host: Option<String>,

        /// FTP settings YAML, or server YAML with an ftp section
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Virtual root directory
        #[arg(long)]
        virtual_root: Option<String>,
    },

    /// Manage FTP fixtures
    ///
    /// Examples:
    ///   mockforge ftp fixtures list
    ///   mockforge ftp fixtures load ./fixtures/ftp/
    ///   mockforge ftp fixtures validate fixture.yaml
    #[command(verbatim_doc_comment)]
    Fixtures {
        #[command(subcommand)]
        fixtures_command: FtpFixturesCommands,
    },

    /// Manage virtual file system
    ///
    /// Examples:
    ///   mockforge ftp vfs list /
    ///   mockforge ftp vfs add /test.txt --content "Hello World"
    ///   mockforge ftp vfs remove /test.txt
    #[command(verbatim_doc_comment)]
    Vfs {
        #[command(subcommand)]
        vfs_command: FtpVfsCommands,
    },
}

#[derive(Subcommand)]
pub enum FtpFixturesCommands {
    /// List all FTP fixtures
    List,

    /// Load fixtures from directory
    ///
    /// Example:
    ///   mockforge ftp fixtures load ./fixtures/ftp/
    Load {
        /// Directory containing fixture files
        directory: PathBuf,
    },

    /// Validate fixture file
    ///
    /// Example:
    ///   mockforge ftp fixtures validate fixture.yaml
    Validate {
        /// Fixture file to validate
        file: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum FtpVfsCommands {
    /// List files in virtual directory
    ///
    /// Example:
    ///   mockforge ftp vfs list /
    List {
        /// Directory path to list
        path: String,
    },

    /// Add file to virtual file system
    ///
    /// Examples:
    ///   mockforge ftp vfs add /test.txt --content "Hello World"
    ///   mockforge ftp vfs add /data.bin --generate random --size 1024
    ///   mockforge ftp vfs add /template.txt --template "{{faker.name}}"
    #[command(verbatim_doc_comment)]
    Add {
        /// File path
        path: String,

        /// Static content
        #[arg(long, conflicts_with_all = ["template", "generate"])]
        content: Option<String>,

        /// Template content (Handlebars)
        #[arg(long, conflicts_with_all = ["content", "generate"])]
        template: Option<String>,

        /// Generate content
        #[arg(long, conflicts_with_all = ["content", "template"], value_enum)]
        generate: Option<GenerationType>,

        /// Size for generated content (in bytes)
        #[arg(long, requires = "generate")]
        size: Option<usize>,
    },

    /// Remove file from virtual file system
    ///
    /// Example:
    ///   mockforge ftp vfs remove /test.txt
    Remove {
        /// File path to remove
        path: String,
    },

    /// Get file information
    ///
    /// Example:
    ///   mockforge ftp vfs info /test.txt
    Info {
        /// File path
        path: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum GenerationType {
    /// Generate random bytes
    Random,
    /// Generate all zeros
    Zeros,
    /// Generate all ones
    Ones,
    /// Generate incremental bytes (0, 1, 2, ...)
    Incremental,
}

/// Handle FTP commands
pub async fn handle_ftp_command(command: FtpCommands) -> Result<()> {
    match command {
        FtpCommands::Serve {
            port,
            host,
            config,
            virtual_root,
        } => handle_ftp_serve(port, host, config, virtual_root).await,
        FtpCommands::Fixtures { fixtures_command } => handle_ftp_fixtures(fixtures_command).await,
        FtpCommands::Vfs { vfs_command } => handle_ftp_vfs(vfs_command).await,
    }
}

async fn resolve_ftp_config(
    port: Option<u16>,
    host: Option<String>,
    config_path: Option<&Path>,
    virtual_root: Option<String>,
) -> Result<FtpConfig> {
    let mut config = if let Some(path) = config_path {
        let contents = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Could not read FTP configuration {}", path.display()))?;
        let document: serde_yaml::Value = serde_yaml::from_str(&contents)
            .with_context(|| format!("Invalid FTP configuration {}", path.display()))?;
        // Accept standalone FTP settings or the FTP section of a server config.
        let settings = document.get("ftp").cloned().unwrap_or(document);
        let mut config: FtpConfig = serde_yaml::from_value(settings)
            .with_context(|| format!("Invalid FTP settings in {}", path.display()))?;
        if let Some(directory) = &mut config.fixtures_dir {
            if directory.is_relative() {
                *directory = path.parent().unwrap_or_else(|| Path::new(".")).join(&*directory);
            }
        }
        config
    } else {
        // Preserve the standalone CLI defaults when no file is supplied.
        FtpConfig {
            host: "127.0.0.1".to_string(),
            virtual_root: PathBuf::from("/"),
            ..Default::default()
        }
    };
    if let Some(port) = port {
        config.port = port;
    }
    if let Some(host) = host {
        config.host = host;
    }
    if let Some(root) = virtual_root {
        config.virtual_root = root.into();
    }
    if config.passive_ports.0 > config.passive_ports.1 {
        anyhow::bail!("FTP passive port range must start at or below its end");
    }
    Ok(config)
}

async fn load_server_fixtures(directory: Option<&Path>) -> Result<Vec<mockforge_ftp::FtpFixture>> {
    let Some(directory) = directory else {
        return Ok(Vec::new());
    };
    let mut entries = tokio::fs::read_dir(directory).await.with_context(|| {
        format!("Could not read FTP fixtures directory {}", directory.display())
    })?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if entry.file_type().await?.is_file()
            && matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
        {
            files.push(path);
        }
    }
    files.sort();
    let mut fixtures = Vec::new();
    for file in files {
        let contents = tokio::fs::read_to_string(&file).await?;
        fixtures.push(
            serde_yaml::from_str(&contents)
                .with_context(|| format!("Invalid FTP fixture {}", file.display()))?,
        );
    }
    Ok(fixtures)
}

async fn handle_ftp_serve(
    port: Option<u16>,
    host: Option<String>,
    config_path: Option<PathBuf>,
    virtual_root: Option<String>,
) -> Result<()> {
    let config = resolve_ftp_config(port, host, config_path.as_deref(), virtual_root).await?;
    let fixtures = load_server_fixtures(config.fixtures_dir.as_deref()).await?;
    println!("Starting FTP server on {}:{}", config.host, config.port);
    let server = FtpServer::new_with_fixtures(config, fixtures)?;
    server.start().await?;
    Ok(())
}

async fn handle_ftp_fixtures(command: FtpFixturesCommands) -> Result<()> {
    match command {
        FtpFixturesCommands::List => {
            let cwd = std::env::current_dir()?;
            let fixture_dirs = ["fixtures/ftp", "fixtures", "ftp-fixtures"];
            let mut found_any = false;

            println!("FTP fixtures:");
            for dir_name in &fixture_dirs {
                let dir = cwd.join(dir_name);
                if !dir.exists() {
                    continue;
                }
                for entry in std::fs::read_dir(&dir)?.flatten() {
                    let path = entry.path();
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if matches!(ext, "yaml" | "yml") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            match serde_yaml::from_str::<mockforge_ftp::FtpFixture>(&content) {
                                Ok(fixture) => {
                                    println!(
                                        "  {} - {} ({} files, {} upload rules)",
                                        fixture.identifier,
                                        fixture.name,
                                        fixture.virtual_files.len(),
                                        fixture.upload_rules.len()
                                    );
                                    found_any = true;
                                }
                                Err(_) => {
                                    // Not a valid FTP fixture, skip
                                }
                            }
                        }
                    }
                }
            }
            if !found_any {
                println!("  No fixtures found. Place YAML fixture files in fixtures/ftp/");
            }
        }
        FtpFixturesCommands::Load { directory } => {
            println!("Loading FTP fixtures from: {}", directory.display());
            if !directory.exists() {
                anyhow::bail!("Directory does not exist: {}", directory.display());
            }

            let mut loaded = 0;
            let mut errors = 0;
            for entry in std::fs::read_dir(&directory)?.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if !matches!(ext, "yaml" | "yml") {
                    continue;
                }
                let content = std::fs::read_to_string(&path)?;
                match serde_yaml::from_str::<mockforge_ftp::FtpFixture>(&content) {
                    Ok(fixture) => {
                        println!(
                            "  Loaded: {} ({} files, {} upload rules)",
                            fixture.name,
                            fixture.virtual_files.len(),
                            fixture.upload_rules.len()
                        );
                        loaded += 1;
                    }
                    Err(e) => {
                        println!("  Error in {}: {}", path.display(), e);
                        errors += 1;
                    }
                }
            }
            println!("Loaded {} fixture(s), {} error(s)", loaded, errors);
        }
        FtpFixturesCommands::Validate { file } => {
            println!("Validating FTP fixture: {}", file.display());
            if !file.exists() {
                anyhow::bail!("File does not exist: {}", file.display());
            }
            let content = std::fs::read_to_string(&file)?;
            match serde_yaml::from_str::<mockforge_ftp::FtpFixture>(&content) {
                Ok(fixture) => {
                    println!("  Valid fixture: {}", fixture.name);
                    println!("  Identifier: {}", fixture.identifier);
                    println!(
                        "  Description: {}",
                        fixture.description.as_deref().unwrap_or("(none)")
                    );
                    println!("  Virtual files: {}", fixture.virtual_files.len());
                    for vf in &fixture.virtual_files {
                        println!("    - {} ({})", vf.path.display(), vf.permissions);
                    }
                    println!("  Upload rules: {}", fixture.upload_rules.len());
                    for rule in &fixture.upload_rules {
                        println!(
                            "    - pattern: {} (auto_accept: {})",
                            rule.path_pattern, rule.auto_accept
                        );
                    }
                }
                Err(e) => {
                    println!("  Invalid fixture: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

async fn handle_ftp_vfs(command: FtpVfsCommands) -> Result<()> {
    let vfs = VirtualFileSystem::new(PathBuf::from("/"));

    match command {
        FtpVfsCommands::List { path } => {
            println!("Files in {}:", path);
            let files = vfs.list_files(PathBuf::from(path).as_path());
            for file in files {
                println!("  {} ({} bytes)", file.path.display(), file.metadata.size);
            }
        }
        FtpVfsCommands::Add {
            path,
            content,
            template,
            generate,
            size,
        } => {
            let file_content = if let Some(content) = content {
                FileContent::Static(content.into_bytes())
            } else if let Some(template) = template {
                FileContent::Template(template)
            } else if let Some(gen_type) = generate {
                let size = size.unwrap_or(1024);
                let pattern = match gen_type {
                    GenerationType::Random => GenerationPattern::Random,
                    GenerationType::Zeros => GenerationPattern::Zeros,
                    GenerationType::Ones => GenerationPattern::Ones,
                    GenerationType::Incremental => GenerationPattern::Incremental,
                };
                FileContent::Generated { size, pattern }
            } else {
                FileContent::Static(b"".to_vec())
            };

            let file = VirtualFile::new(
                PathBuf::from(path.clone()),
                file_content,
                FileMetadata::default(),
            );

            vfs.add_file(PathBuf::from(path), file)?;
            println!("File added to virtual file system");
        }
        FtpVfsCommands::Remove { path } => {
            if vfs.remove_file(PathBuf::from(path).as_path()).is_ok() {
                println!("File removed from virtual file system");
            } else {
                println!("File not found");
            }
        }
        FtpVfsCommands::Info { path } => {
            if let Some(file) = vfs.get_file(PathBuf::from(path).as_path()) {
                println!("File: {}", file.path.display());
                println!("Size: {} bytes", file.metadata.size);
                println!("Permissions: {}", file.metadata.permissions);
                println!("Owner: {}", file.metadata.owner);
                println!("Modified: {}", file.modified_at);
            } else {
                println!("File not found");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftp_commands_serve_variant() {
        let _cmd = FtpCommands::Serve {
            port: Some(2121),
            host: Some("127.0.0.1".to_string()),
            config: None,
            virtual_root: Some("/".to_string()),
        };
    }

    #[test]
    fn test_ftp_fixtures_list_variant() {
        let _cmd = FtpFixturesCommands::List;
    }

    #[test]
    fn test_ftp_fixtures_load_variant() {
        let _cmd = FtpFixturesCommands::Load {
            directory: PathBuf::from("./fixtures/ftp"),
        };
    }

    #[test]
    fn test_ftp_fixtures_validate_variant() {
        let _cmd = FtpFixturesCommands::Validate {
            file: PathBuf::from("fixture.yaml"),
        };
    }

    #[test]
    fn test_ftp_vfs_list_variant() {
        let _cmd = FtpVfsCommands::List {
            path: "/".to_string(),
        };
    }
}
