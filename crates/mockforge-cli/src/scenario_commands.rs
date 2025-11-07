//! Scenario management CLI commands

use base64::{engine::general_purpose, Engine as _};
use clap::Subcommand;
use mockforge_scenarios::{InstallOptions, ScenarioInstaller, ScenarioRegistry};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Subcommand)]
pub enum ScenarioCommands {
    /// Install a scenario from various sources
    Install {
        /// Scenario source (URL, Git repo, local path, or registry name)
        ///
        /// Examples:
        /// - ./scenarios/my-scenario
        /// - https://github.com/user/repo#main:scenarios/my-scenario
        /// - https://example.com/scenario.zip
        /// - ecommerce-store@1.0.0
        source: String,

        /// Force reinstall even if scenario exists
        #[arg(long)]
        force: bool,

        /// Skip validation checks
        #[arg(long)]
        skip_validation: bool,

        /// Expected SHA-256 checksum (for URL sources)
        #[arg(long)]
        checksum: Option<String>,
    },

    /// Uninstall a scenario
    Uninstall {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },

    /// List installed scenarios
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show scenario information
    Info {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },

    /// Apply scenario to current workspace
    Use {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },

    /// Search for scenarios in the registry
    Search {
        /// Search query
        query: String,

        /// Category filter
        #[arg(long)]
        category: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Publish a scenario to the registry
    Publish {
        /// Path to scenario directory
        path: String,

        /// Registry URL (optional)
        #[arg(long)]
        registry: Option<String>,
    },

    /// Update a scenario to the latest version
    Update {
        /// Scenario name to update (or --all for all scenarios)
        name: Option<String>,

        /// Update all installed scenarios
        #[arg(long)]
        all: bool,
    },
}

/// Handle scenario commands
pub async fn handle_scenario_command(command: ScenarioCommands) -> anyhow::Result<()> {
    match command {
        ScenarioCommands::Install {
            source,
            force,
            skip_validation,
            checksum,
        } => {
            println!("📦 Installing scenario from: {}", source);

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let options = InstallOptions {
                force,
                skip_validation,
                expected_checksum: checksum,
            };

            match installer.install(&source, options).await {
                Ok(scenario_id) => {
                    println!("✅ Scenario installed successfully: {}", scenario_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to install scenario: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Uninstall { name, version } => {
            let version = version.unwrap_or_else(|| "latest".to_string());
            println!("🗑️  Uninstalling scenario: {}@{}", name, version);

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            // If version is "latest", get the latest version
            let actual_version = if version == "latest" {
                installer
                    .get_latest(&name)
                    .map(|s| s.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", name))?
            } else {
                version
            };

            match installer.uninstall(&name, &actual_version).await {
                Ok(_) => {
                    println!("✅ Scenario uninstalled successfully");
                }
                Err(e) => {
                    eprintln!("❌ Failed to uninstall scenario: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::List { detailed } => {
            println!("📋 Installed scenarios:");

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let scenarios = installer.list_installed();

            if scenarios.is_empty() {
                println!("  No scenarios installed");
            } else {
                for scenario in scenarios {
                    if detailed {
                        println!("  - {}@{}", scenario.name, scenario.version);
                        println!("    Path: {}", scenario.path.display());
                        println!("    Description: {}", scenario.manifest.description);
                        println!("    Category: {:?}", scenario.manifest.category);
                        println!("    Installed: {}", scenario.installed_at);
                    } else {
                        println!("  - {}@{}", scenario.name, scenario.version);
                    }
                }
            }
        }

        ScenarioCommands::Info { name, version } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let scenario = if let Some(v) = version {
                installer.get(&name, &v)
            } else {
                installer.get_latest(&name)
            };

            match scenario {
                Some(s) => {
                    println!("ℹ️  Scenario information: {}@{}", s.name, s.version);
                    println!("   Title: {}", s.manifest.title);
                    println!("   Description: {}", s.manifest.description);
                    println!("   Author: {}", s.manifest.author);
                    println!("   Category: {:?}", s.manifest.category);
                    println!("   Tags: {}", s.manifest.tags.join(", "));
                    println!("   Path: {}", s.path.display());
                    println!("   Installed: {}", s.installed_at);
                    if let Some(updated) = s.updated_at {
                        println!("   Updated: {}", updated);
                    }
                }
                None => {
                    eprintln!("❌ Scenario '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Use { name, version } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let version_clone = version.clone();
            let scenario = if let Some(ref v) = version {
                installer.get(&name, v)
            } else {
                installer.get_latest(&name)
            };

            match scenario {
                Some(s) => {
                    println!("🎯 Applying scenario: {}@{}", s.name, s.version);

                    match installer.apply_to_workspace(&name, version_clone.as_deref()).await {
                        Ok(_) => {
                            println!("✅ Scenario applied successfully to workspace");
                            println!(
                                "   Files copied: config.yaml, openapi.json, fixtures/, examples/"
                            );
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to apply scenario: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    eprintln!("❌ Scenario '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Search {
            query,
            category,
            limit,
        } => {
            println!("🔍 Searching for scenarios: {}", query);

            let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

            let search_query = mockforge_scenarios::ScenarioSearchQuery {
                query: Some(query),
                category,
                tags: vec![],
                sort: mockforge_scenarios::ScenarioSortOrder::Relevance,
                page: 0,
                per_page: limit,
            };

            match registry.search(search_query).await {
                Ok(results) => {
                    if results.scenarios.is_empty() {
                        println!("  No scenarios found");
                    } else {
                        println!("  Found {} scenarios:", results.total);
                        for scenario in results.scenarios {
                            println!("  - {}@{}", scenario.name, scenario.version);
                            println!("    {}", scenario.description);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to search scenarios: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Publish { path, registry } => {
            println!("📤 Publishing scenario from: {}", path);

            // Load scenario package
            let package = mockforge_scenarios::ScenarioPackage::from_directory(&path)
                .map_err(|e| anyhow::anyhow!("Failed to load scenario package: {}", e))?;

            // Validate package
            let validation = package
                .validate()
                .map_err(|e| anyhow::anyhow!("Package validation failed: {}", e))?;

            if !validation.is_valid {
                eprintln!("❌ Package validation failed:");
                for error in &validation.errors {
                    eprintln!("   - {}", error);
                }
                std::process::exit(1);
            }

            // Warn about warnings
            if !validation.warnings.is_empty() {
                println!("⚠️  Package validation warnings:");
                for warning in &validation.warnings {
                    println!("   - {}", warning);
                }
            }

            // Create package archive
            println!("   Creating package archive...");
            let (archive_path, checksum, size) = create_scenario_archive(&package)
                .map_err(|e| anyhow::anyhow!("Failed to create archive: {}", e))?;

            println!("   Package size: {} bytes", size);
            println!("   Checksum: {}", checksum);

            // Read archive as base64
            let archive_data = std::fs::read(&archive_path)
                .map_err(|e| anyhow::anyhow!("Failed to read archive: {}", e))?;
            let archive_base64 = general_purpose::STANDARD.encode(&archive_data);

            // Serialize manifest
            let manifest_json = serde_json::to_string(&package.manifest)
                .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))?;

            // Create publish request
            let publish_request = mockforge_scenarios::ScenarioPublishRequest {
                manifest: manifest_json,
                package: archive_base64,
                checksum,
                size,
            };

            // Get registry URL and token
            let registry_url =
                registry.unwrap_or_else(|| "https://registry.mockforge.dev".to_string());
            let token = std::env::var("MOCKFORGE_REGISTRY_TOKEN")
                .map_err(|_| anyhow::anyhow!("MOCKFORGE_REGISTRY_TOKEN environment variable not set. Required for publishing."))?;

            // Create registry client and publish
            let registry_client =
                mockforge_scenarios::ScenarioRegistry::with_token(registry_url, token);

            println!("   Publishing to registry...");
            match registry_client.publish(publish_request).await {
                Ok(response) => {
                    println!("✅ Scenario published successfully!");
                    println!("   Name: {}@{}", response.name, response.version);
                    println!("   Download URL: {}", response.download_url);
                    println!("   Published at: {}", response.published_at);
                }
                Err(e) => {
                    eprintln!("❌ Failed to publish scenario: {}", e);
                    std::process::exit(1);
                }
            }

            // Clean up temp archive
            let _ = std::fs::remove_file(&archive_path);
        }

        ScenarioCommands::Update { name, all } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            if all {
                println!("🔄 Updating all scenarios...");

                match installer.update_all().await {
                    Ok(updated) => {
                        if updated.is_empty() {
                            println!("✅ All scenarios are up to date");
                        } else {
                            println!("✅ Updated {} scenarios:", updated.len());
                            for item in updated {
                                println!("   - {}", item);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to update scenarios: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if let Some(n) = name {
                println!("🔄 Updating scenario: {}", n);

                // Get current scenario info
                let scenario = installer
                    .get_latest(&n)
                    .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", n))?;

                let current_version = scenario.version.clone();
                let source_str = scenario.source.clone();

                // Parse source to check if it's from registry
                let source = mockforge_scenarios::ScenarioSource::parse(&source_str)?;

                match source {
                    mockforge_scenarios::ScenarioSource::Registry { .. } => {
                        // Update from registry
                        let mut installer_mut = installer;
                        match installer_mut.update_from_registry(&n, &current_version).await {
                            Ok(new_version) => {
                                if new_version == current_version {
                                    println!(
                                        "✅ Scenario is already up to date: {}@{}",
                                        n, current_version
                                    );
                                } else {
                                    println!(
                                        "✅ Scenario updated: {}@{} -> {}",
                                        n, current_version, new_version
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to update scenario: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    _ => {
                        // Reinstall from original source
                        println!("   Reinstalling from original source: {}", source_str);
                        let options = InstallOptions {
                            force: true,
                            skip_validation: false,
                            expected_checksum: None,
                        };

                        match installer.install(&source_str, options).await {
                            Ok(_) => {
                                println!("✅ Scenario updated successfully");
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to update scenario: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
            } else {
                eprintln!("❌ Please specify a scenario name or use --all");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Create a scenario package archive (ZIP format)
fn create_scenario_archive(
    package: &mockforge_scenarios::ScenarioPackage,
) -> anyhow::Result<(std::path::PathBuf, String, u64)> {
    use zip::write::FileOptions;
    use zip::ZipWriter;

    // Create temporary file for archive
    let temp_file = tempfile::NamedTempFile::new()?;
    let archive_path = temp_file.path().to_path_buf();
    drop(temp_file); // Close file so we can write to it

    // Create ZIP archive
    let file = fs::File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Add all files from package
    for file_path in &package.files {
        let full_path = package.root.join(file_path);

        if full_path.is_dir() {
            continue; // Skip directories, we'll add files individually
        }

        if !full_path.exists() {
            continue; // Skip missing files
        }

        // Get relative path for archive
        let archive_name = file_path.to_string_lossy().replace('\\', "/");

        // Read file contents
        let file_contents = fs::read(&full_path)?;

        // Add to ZIP
        zip.start_file(&archive_name, options)?;
        zip.write_all(&file_contents)?;
    }

    // Add all files from directories recursively
    add_directory_to_zip(&mut zip, &package.root, &package.root, options)?;

    // Finish writing the ZIP (consumes the writer)
    let _file = zip.finish()?;

    // Calculate checksum
    let archive_data = fs::read(&archive_path)?;
    let checksum = calculate_checksum(&archive_data);
    let size = archive_data.len() as u64;

    Ok((archive_path, checksum, size))
}

/// Recursively add directory contents to ZIP
fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::FileOptions<()>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        // Skip hidden files and common ignore patterns
        if file_name.to_string_lossy().starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Recursively add subdirectory
            add_directory_to_zip(zip, base, &path, options)?;
        } else {
            // Add file
            let relative_path = path
                .strip_prefix(base)
                .map_err(|e| anyhow::anyhow!("Failed to compute relative path: {}", e))?;
            let archive_name = relative_path.to_string_lossy().replace('\\', "/");

            let file_contents = fs::read(&path)?;
            zip.start_file(&archive_name, options)?;
            zip.write_all(&file_contents)?;
        }
    }

    Ok(())
}

/// Calculate SHA-256 checksum
fn calculate_checksum(data: &[u8]) -> String {
    use ring::digest::{Context, SHA256};
    let mut context = Context::new(&SHA256);
    context.update(data);
    let digest = context.finish();
    hex::encode(digest.as_ref())
}
